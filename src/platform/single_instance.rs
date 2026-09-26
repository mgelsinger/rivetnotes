//! Single-instance support: a named mutex marks the first instance, and later
//! launches forward their command-line paths to it via `WM_COPYDATA`.

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;
use std::time::Duration;

use windows::Win32::Foundation::{
    BOOL, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, WPARAM,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SMTO_ABORTIFHUNG, SW_RESTORE, SendMessageTimeoutW, SetForegroundWindow,
    ShowWindow, WM_COPYDATA,
};
use windows::core::{PCWSTR, w};

use crate::logging;

/// Session-local (no `Global\` prefix): one instance per desktop session.
const MUTEX_NAME: PCWSTR = w!("RivetNotes_SingleInstance_66885411-0CEF-459E-AA39-4B257B1A4D84");
const MAIN_WINDOW_CLASS: PCWSTR = w!("rivet_main_window");

/// `COPYDATASTRUCT.dwData` magic ("RVOP") identifying an open-files request.
pub const COPYDATA_OPEN_FILES: usize = 0x5256_4F50;
pub const MAX_COPYDATA_BYTES: usize = 1024 * 1024;

const FIND_WINDOW_ATTEMPTS: u32 = 20;
const FIND_WINDOW_DELAY: Duration = Duration::from_millis(100);
const SEND_TIMEOUT_MS: u32 = 3000;

pub struct SingleInstance {
    /// Held (never waited on) for the process lifetime; the OS releases it on exit.
    _handle: Option<HANDLE>,
    pub already_running: bool,
}

pub fn acquire() -> SingleInstance {
    match unsafe { CreateMutexW(None, BOOL(0), MUTEX_NAME) } {
        Ok(handle) => {
            let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            SingleInstance {
                _handle: Some(handle),
                already_running,
            }
        }
        Err(err) => {
            logging::log_error(&format!("single-instance mutex unavailable: {err}"));
            SingleInstance {
                _handle: None,
                already_running: false,
            }
        }
    }
}

/// Paths passed on the command line, absolutized so Explorer-relative or
/// console-relative arguments resolve against the launch directory.
pub fn cli_paths() -> Vec<PathBuf> {
    std::env::args_os().skip(1).map(absolutize).collect()
}

fn absolutize(arg: OsString) -> PathBuf {
    let path = PathBuf::from(&arg);
    // Deliberately not fs::canonicalize: its \\?\ verbatim prefix would leak
    // into tab titles and the recent-files list.
    std::path::absolute(&path).unwrap_or(path)
}

/// UTF-16 paths joined and terminated by single NULs.
pub fn encode_paths(paths: &[PathBuf]) -> Vec<u16> {
    let mut buffer = Vec::new();
    for path in paths {
        buffer.extend(path.as_os_str().encode_wide().filter(|&unit| unit != 0));
        buffer.push(0);
    }
    buffer
}

pub fn decode_paths(buffer: &[u16]) -> Vec<PathBuf> {
    buffer
        .split(|&unit| unit == 0)
        .filter(|segment| !segment.is_empty())
        .map(|segment| PathBuf::from(OsString::from_wide(segment)))
        .collect()
}

/// Bring the existing instance to the foreground and hand it `paths`.
/// Returns false when no window was found or delivery failed; the caller
/// should then open its own window.
pub fn forward_to_existing(paths: &[PathBuf]) -> bool {
    let Some(hwnd) = find_existing_window() else {
        logging::log_error("single-instance: existing window not found; opening a new one");
        return false;
    };

    unsafe {
        if IsIconic(hwnd).as_bool() {
            ShowWindow(hwnd, SW_RESTORE);
        }
        SetForegroundWindow(hwnd);
    }

    if paths.is_empty() {
        return true;
    }

    let buffer = encode_paths(paths);
    if std::mem::size_of_val(buffer.as_slice()) > MAX_COPYDATA_BYTES {
        logging::log_error("single-instance: path message exceeds size limit");
        return false;
    }
    let cds = COPYDATASTRUCT {
        dwData: COPYDATA_OPEN_FILES,
        cbData: (buffer.len() * std::mem::size_of::<u16>()) as u32,
        lpData: buffer.as_ptr() as *mut _,
    };
    let mut result = 0usize;
    let delivered = unsafe {
        SendMessageTimeoutW(
            hwnd,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM(&cds as *const COPYDATASTRUCT as isize),
            SMTO_ABORTIFHUNG,
            SEND_TIMEOUT_MS,
            Some(&mut result),
        )
    };
    if delivered.0 == 0 || result != 1 {
        logging::log_error("single-instance: WM_COPYDATA forward failed; opening a new window");
        return false;
    }
    true
}

fn find_existing_window() -> Option<HWND> {
    // The first instance may hold the mutex before its window exists; retry
    // briefly to cover that startup race.
    for attempt in 0..FIND_WINDOW_ATTEMPTS {
        let hwnd = unsafe { FindWindowW(MAIN_WINDOW_CLASS, PCWSTR::null()) };
        if hwnd.0 != 0 {
            return Some(hwnd);
        }
        if attempt + 1 < FIND_WINDOW_ATTEMPTS {
            std::thread::sleep(FIND_WINDOW_DELAY);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let paths = vec![
            PathBuf::from(r"C:\notes\a.txt"),
            PathBuf::from(r"D:\Projects\rivetnotes\README.md"),
        ];
        assert_eq!(decode_paths(&encode_paths(&paths)), paths);
    }

    #[test]
    fn encode_empty_is_empty() {
        assert!(encode_paths(&[]).is_empty());
        assert!(decode_paths(&[]).is_empty());
    }

    #[test]
    fn decode_skips_empty_segments() {
        let buffer = [0u16, 0, b'x' as u16, 0, 0];
        assert_eq!(decode_paths(&buffer), vec![PathBuf::from("x")]);
    }

    #[test]
    fn absolutize_makes_relative_paths_absolute() {
        let path = absolutize(OsString::from("notes.txt"));
        assert!(path.is_absolute());
        assert!(path.ends_with("notes.txt"));
    }

    #[test]
    fn absolutize_keeps_absolute_paths() {
        let path = absolutize(OsString::from(r"C:\notes\a.txt"));
        assert_eq!(path, PathBuf::from(r"C:\notes\a.txt"));
    }
}
