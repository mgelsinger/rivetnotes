//! Native networking, installation identity, and process waiting for quiet updates.
use std::ffi::{OsString, c_void};
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Networking::WinHttp::*;
use windows::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6464KEY, RegGetValueW,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, GetProcessTimes, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{PCWSTR, w};

// Preserve the historical extra closing brace: changing it would break upgrades.
const UNINSTALL_KEY: PCWSTR = w!(
    "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{66885411-0CEF-459E-AA39-4B257B1A4D84}}_is1"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    User,
    Machine,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installation {
    pub directory: PathBuf,
    pub scope: Scope,
}

pub fn detect_installation(executable: &Path) -> Option<Installation> {
    if !executable
        .file_name()?
        .to_str()?
        .eq_ignore_ascii_case("rivet.exe")
    {
        return None;
    }
    let directory = executable.parent()?.canonicalize().ok()?;
    // A stale/user-writable HKCU entry must not turn a matching shared install
    // into a per-user update when both hives contain the same directory.
    for (root, scope) in [
        (HKEY_LOCAL_MACHINE, Scope::Machine),
        (HKEY_CURRENT_USER, Scope::User),
    ] {
        let mut buffer = [0u16; 32768];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        let result = unsafe {
            RegGetValueW(
                root,
                UNINSTALL_KEY,
                w!("InstallLocation"),
                RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
                None,
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut size),
            )
        };
        if result.is_ok() {
            let length = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
            let registered = PathBuf::from(String::from_utf16_lossy(&buffer[..length]));
            if registered.is_absolute()
                && let Ok(canonical) = registered.canonicalize()
                && canonical
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&directory.to_string_lossy())
            {
                return Some(Installation {
                    directory: registered,
                    scope,
                });
            }
        }
    }
    None
}

struct HttpHandle(*mut c_void);
impl HttpHandle {
    fn new(raw: *mut c_void) -> io::Result<Self> {
        if raw.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(raw))
        }
    }
}
impl Drop for HttpHandle {
    fn drop(&mut self) {
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

pub fn cancelled(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Relaxed) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Update cancelled",
        ))
    } else {
        Ok(())
    }
}

pub fn download(
    url: &str,
    output: &mut impl Write,
    limit: u64,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64),
) -> io::Result<u64> {
    cancelled(cancel)?;
    let url = url
        .strip_prefix("https://")
        .ok_or_else(|| io::Error::other("Updates require HTTPS"))?;
    let (host, path) = url
        .split_once('/')
        .ok_or_else(|| io::Error::other("Invalid update URL"))?;
    if host != "github.com" {
        return Err(io::Error::other("Invalid update host"));
    }
    let host: Vec<u16> = host.encode_utf16().chain(Some(0)).collect();
    let path: Vec<u16> = format!("/{path}").encode_utf16().chain(Some(0)).collect();
    unsafe {
        let session = HttpHandle::new(WinHttpOpen(
            w!("Rivet updater"),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        WinHttpSetTimeouts(session.0, 10000, 10000, 15000, 15000).map_err(io::Error::other)?;
        WinHttpSetOption(
            Some(session.0),
            WINHTTP_OPTION_SECURE_PROTOCOLS,
            Some(&WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2.to_ne_bytes()),
        )
        .map_err(io::Error::other)?;
        let connection = HttpHandle::new(WinHttpConnect(session.0, PCWSTR(host.as_ptr()), 443, 0))?;
        let request = HttpHandle::new(WinHttpOpenRequest(
            connection.0,
            w!("GET"),
            PCWSTR(path.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        WinHttpSetOption(
            Some(request.0),
            WINHTTP_OPTION_REDIRECT_POLICY,
            Some(&WINHTTP_OPTION_REDIRECT_POLICY_DISALLOW_HTTPS_TO_HTTP.to_ne_bytes()),
        )
        .map_err(io::Error::other)?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0).map_err(io::Error::other)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(io::Error::other)?;
        let mut status = 0u32;
        let mut status_size = 4;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&mut status as *mut u32).cast()),
            &mut status_size,
            std::ptr::null_mut(),
        )
        .map_err(io::Error::other)?;
        if status != 200 {
            return Err(io::Error::new(
                if status == 404 {
                    io::ErrorKind::NotFound
                } else {
                    io::ErrorKind::Other
                },
                format!("Update server returned HTTP {status}"),
            ));
        }
        let deadline = Instant::now() + Duration::from_secs(300);
        let mut total = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            cancelled(cancel)?;
            if Instant::now() > deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Update download timed out",
                ));
            }
            let mut read = 0u32;
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            )
            .map_err(io::Error::other)?;
            if read == 0 {
                break;
            }
            total += read as u64;
            if total > limit {
                return Err(io::Error::other("Update exceeds expected size"));
            }
            output.write_all(&buffer[..read as usize])?;
            progress(total);
        }
        cancelled(cancel)?;
        Ok(total)
    }
}

pub fn process_birth() -> io::Result<u64> {
    birth(unsafe { GetCurrentProcess() })
}

fn birth(handle: windows::Win32::Foundation::HANDLE) -> io::Result<u64> {
    let mut created = FILETIME::default();
    let mut exited = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) }
        .map_err(io::Error::other)?;
    Ok((created.dwHighDateTime as u64) << 32 | created.dwLowDateTime as u64)
}

pub fn wait_for_parent(pid: u32, created: u64) -> io::Result<()> {
    let handle = match unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
    } {
        Ok(handle) => handle,
        Err(error) if error.code().0 as u32 == 0x80070057 => return Ok(()), // Process already exited.
        Err(error) => return Err(io::Error::other(error)),
    };
    let result = (|| {
        if birth(handle)? != created {
            return Ok(());
        } // PID was reused; original process is gone.
        if unsafe { WaitForSingleObject(handle, 300000) } != WAIT_OBJECT_0 {
            return Err(io::Error::other("Rivet did not exit; update postponed"));
        }
        Ok(())
    })();
    let _ = unsafe { CloseHandle(handle) };
    result
}

struct ComApartment;
impl ComApartment {
    fn new() -> io::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) }
            .ok()
            .map_err(io::Error::other)?;
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct ProcessHandle(HANDLE);
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

// ShellExecuteEx accepts a command-line string, not an argument array. Preserve
// UTF-16 exactly and escape quotes/backslashes using Windows argv conventions.
fn shell_parameters(arguments: &[OsString]) -> io::Result<Vec<u16>> {
    let mut output = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            output.push(b' ' as u16);
        }
        output.push(b'"' as u16);
        let mut slashes = 0;
        for unit in argument.encode_wide() {
            if unit == 0 {
                return Err(io::Error::other("Invalid installer argument"));
            }
            if unit == b'\\' as u16 {
                slashes += 1;
                continue;
            }
            output.extend(std::iter::repeat_n(
                b'\\' as u16,
                if unit == b'"' as u16 {
                    slashes * 2 + 1
                } else {
                    slashes
                },
            ));
            slashes = 0;
            output.push(unit);
        }
        output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        output.push(b'"' as u16);
    }
    output.push(0);
    Ok(output)
}

/// Called only by the unelevated helper after signature/hash verification and
/// parent exit. The caller retains its read-only installer lock for this call.
pub fn run_elevated_installer(path: &Path, arguments: &[OsString]) -> io::Result<u32> {
    let _com = ComApartment::new()?;
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let parameters = shell_parameters(arguments)?;
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(path.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }.map_err(elevation_error)?;
    if info.hProcess.is_invalid() {
        return Err(io::Error::other("Cannot track the elevated installer"));
    }
    let process = ProcessHandle(info.hProcess);
    // Do not abandon the verification lock or relaunch the editor while setup
    // is still replacing files. Never terminate an in-progress installer.
    if unsafe { WaitForSingleObject(process.0, u32::MAX) } != WAIT_OBJECT_0 {
        return Err(io::Error::last_os_error());
    }
    let mut code = 0;
    unsafe { GetExitCodeProcess(process.0, &mut code) }.map_err(io::Error::other)?;
    Ok(code)
}

fn elevation_error(error: windows::core::Error) -> io::Error {
    if error.code().0 as u32 == 0x800704C7 {
        // ERROR_CANCELLED
        io::Error::new(
            io::ErrorKind::Interrupted,
            "Administrator approval was cancelled",
        )
    } else {
        io::Error::other(error)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn denied_admin_consent_is_a_postponement() {
        let cancelled = windows::core::Error::from(windows::core::HRESULT(0x800704C7u32 as i32));
        let error = elevation_error(cancelled);
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(error.to_string().contains("cancelled"));
        let denied = windows::core::Error::from(windows::core::HRESULT(0x80070005u32 as i32));
        assert_ne!(elevation_error(denied).kind(), io::ErrorKind::Interrupted);
    }

    #[test]
    fn elevated_parameters_roundtrip_through_windows_parser() {
        use std::os::windows::ffi::OsStringExt;
        use windows::Win32::Foundation::{HLOCAL, LocalFree};
        use windows::Win32::UI::Shell::CommandLineToArgvW;
        let args: Vec<OsString> = [
            "installer.exe",
            "/ALLUSERS",
            r"/DIR=C:\Program Files\Rivet\",
            r"/DIR=C:\Users\José\Rivet",
            "",
            "literal\"quote",
            "backslash\\\"quote",
        ]
        .into_iter()
        .map(Into::into)
        .collect();
        let parameters = shell_parameters(&args).unwrap();
        let mut count = 0;
        let argv = unsafe { CommandLineToArgvW(PCWSTR(parameters.as_ptr()), &mut count) };
        assert!(!argv.is_null());
        let parsed: Vec<OsString> = unsafe { std::slice::from_raw_parts(argv, count as usize) }
            .iter()
            .map(|arg| {
                let mut length = 0;
                while unsafe { *arg.0.add(length) } != 0 {
                    length += 1;
                }
                OsString::from_wide(unsafe { std::slice::from_raw_parts(arg.0, length) })
            })
            .collect();
        unsafe {
            LocalFree(HLOCAL(argv.cast()));
        }
        assert_eq!(parsed, args);
        assert!(shell_parameters(&[OsString::from("invalid\0argument")]).is_err());
    }

    #[test]
    fn rejects_non_https_foreign_hosts_and_cancelled_requests() {
        let mut output = Vec::new();
        let cancel = AtomicBool::new(false);
        for url in [
            "http://github.com/test",
            "https://example.org/test",
            "https://github.com.evil/test",
        ] {
            assert!(download(url, &mut output, 100, &cancel, |_| {}).is_err());
        }
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(
            download("https://github.com/test", &mut output, 100, &cancel, |_| {})
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert!(output.is_empty());
    }

    #[test]
    fn parent_identity_prevents_waiting_on_reused_process_ids() {
        let created = process_birth().unwrap();
        assert!(created > 0);
        wait_for_parent(std::process::id(), created - 1).unwrap();
    }

    #[test]
    fn unregistered_and_portable_paths_are_not_installed_copies() {
        let directory = tempfile::tempdir().unwrap();
        assert!(detect_installation(&directory.path().join("rivet.exe")).is_none());
        assert!(detect_installation(&std::env::current_exe().unwrap()).is_none());
    }

    #[test]
    #[ignore = "Requires internet access to the public release feed"]
    fn winhttp_release_feed_is_bounded() {
        let mut output = Vec::new();
        let cancel = AtomicBool::new(false);
        match download(
            rivet::update_protocol::FEED_URL,
            &mut output,
            rivet::update_protocol::MAX_METADATA_BYTES as u64,
            &cancel,
            |_| {},
        ) {
            Ok(_) => assert!(
                serde_json::from_slice::<rivet::update_protocol::SignedRelease>(&output).is_ok()
            ),
            Err(error) => assert_eq!(error.kind(), io::ErrorKind::NotFound), // Bootstrap release has no feed yet.
        }
        let mut too_small = Vec::new();
        assert!(
            download(
                "https://github.com/mgelsinger/rivetnotes",
                &mut too_small,
                1,
                &cancel,
                |_| {}
            )
            .is_err()
        );
        assert!(too_small.is_empty());
    }
}
