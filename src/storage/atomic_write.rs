use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
#[cfg(windows)]
use windows::Win32::Security::{
    DACL_SECURITY_INFORMATION, GetFileSecurityW, PROTECTED_DACL_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, SetFileSecurityW,
};

#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_ENCRYPTED, MOVE_FILE_FLAGS, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    REPLACE_FILE_FLAGS, ReplaceFileW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);
const TEMP_MARKER: &str = ".rivet-tmp.";
const RECOVERY_MARKER: &str = ".rivet-recovery.";

pub fn atomic_write_bytes(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent directory.",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let temp_path = temp_path_for(dest);
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(windows)]
    if let Ok(metadata) = std::fs::metadata(dest) {
        // Never stage plaintext when replacing an EFS-encrypted file.
        options.attributes(metadata.file_attributes() & FILE_ATTRIBUTE_ENCRYPTED.0);
    }
    let mut file = options.open(&temp_path)?;
    let write_result = (|| {
        #[cfg(windows)]
        if dest.try_exists()? {
            protect_replacement(dest, &temp_path)?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok::<_, io::Error>(())
    })();
    drop(file);
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }
    // Keep the completed replacement on a commit failure for recovery.
    replace_file(dest, &temp_path).map_err(|error| {
        if !temp_path.exists() {
            return error;
        }
        // Recovery copies must not be purged by routine stale-temp cleanup.
        let recovery = unique_path_for(dest, RECOVERY_MARKER);
        let retained = if std::fs::rename(&temp_path, &recovery).is_ok() {
            recovery
        } else {
            temp_path
        };
        io::Error::new(
            error.kind(),
            format!("{error}; new contents retained at {}", retained.display()),
        )
    })
}

#[cfg(windows)]
fn read_dacl(path: &Path) -> io::Result<Vec<u32>> {
    let wide = path_to_wide(path);
    let mut needed = 0;
    unsafe {
        GetFileSecurityW(
            PCWSTR(wide.as_ptr()),
            DACL_SECURITY_INFORMATION.0,
            PSECURITY_DESCRIPTOR::default(),
            0,
            &mut needed,
        );
    }
    if needed == 0 {
        return Err(io::Error::last_os_error());
    }
    // The self-relative descriptor needs DWORD alignment.
    let mut descriptor = vec![0u32; (needed as usize).div_ceil(4)];
    if !unsafe {
        GetFileSecurityW(
            PCWSTR(wide.as_ptr()),
            DACL_SECURITY_INFORMATION.0,
            PSECURITY_DESCRIPTOR(descriptor.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
    }
    .as_bool()
    {
        return Err(io::Error::last_os_error());
    }
    Ok(descriptor)
}

#[cfg(windows)]
fn protect_replacement(dest: &Path, temp: &Path) -> io::Result<()> {
    let mut descriptor = read_dacl(dest)?;
    let wide = path_to_wide(temp);
    // Apply the original access list before writing any contents. Protect it
    // against broader permissions inherited from the containing directory.
    if !unsafe {
        SetFileSecurityW(
            PCWSTR(wide.as_ptr()),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR(descriptor.as_mut_ptr().cast()),
        )
    }
    .as_bool()
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub fn atomic_write_json<T: Serialize>(dest: &Path, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    atomic_write_bytes(dest, &bytes)
}

pub fn cleanup_stale_temp_files(dir: &Path, max_age: Duration) -> io::Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let now = SystemTime::now();
    let mut removed = 0usize;

    for entry in std::fs::read_dir(dir)? {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(value) => value,
            None => continue,
        };
        let Some((_, suffix)) = file_name.rsplit_once(TEMP_MARKER) else {
            continue;
        };
        let parts: Vec<_> = suffix.split('.').collect();
        if parts.len() != 3
            || parts[0].parse::<u32>().is_err()
            || u128::from_str_radix(parts[1], 16).is_err()
            || u64::from_str_radix(parts[2], 16).is_err()
        {
            continue;
        }

        let modified = match entry.metadata().and_then(|meta| meta.modified()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let age = match now.duration_since(modified) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if age < max_age {
            continue;
        }

        if std::fs::remove_file(&path).is_ok() {
            removed = removed.saturating_add(1);
        }
    }

    Ok(removed)
}

fn temp_path_for(dest: &Path) -> PathBuf {
    unique_path_for(dest, TEMP_MARKER)
}

fn unique_path_for(dest: &Path, marker: &str) -> PathBuf {
    let parent = dest.parent().unwrap_or_else(|| Path::new("."));
    let base = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("snapshot");
    let pid = std::process::id();
    let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    parent.join(format!("{base}{marker}{pid}.{nanos:x}.{seq:x}"))
}

#[cfg(windows)]
fn replace_file(dest: &Path, temp: &Path) -> io::Result<()> {
    let dest_w = path_to_wide(dest);
    let temp_w = path_to_wide(temp);

    if dest.try_exists()? {
        // Preserve the original even for ReplaceFileW's partial-failure cases.
        // Reserve the name so we never replace someone else's recovery file.
        let previous = unique_path_for(dest, RECOVERY_MARKER);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&previous)?;
        let previous_w = path_to_wide(&previous);
        match unsafe {
            ReplaceFileW(
                PCWSTR(dest_w.as_ptr()),
                PCWSTR(temp_w.as_ptr()),
                PCWSTR(previous_w.as_ptr()),
                REPLACE_FILE_FLAGS(0),
                None,
                None,
            )
        } {
            Ok(_) => {
                let _ = std::fs::remove_file(previous);
                return Ok(());
            }
            Err(replace_err) => {
                // Never fall back to a rename that bypasses ACL/stream merging.
                return Err(io::Error::other(format!(
                    "ReplaceFileW failed for {}: {replace_err}; any original-file recovery copy is at {}",
                    dest.display(),
                    previous.display()
                )));
            }
        }
    }

    // A file created by another writer since the existence check must survive.
    let flags = MOVE_FILE_FLAGS(MOVEFILE_WRITE_THROUGH.0);
    match unsafe { MoveFileExW(PCWSTR(temp_w.as_ptr()), PCWSTR(dest_w.as_ptr()), flags) } {
        Ok(_) => Ok(()),
        Err(err) => Err(io::Error::other(format!(
            "MoveFileExW failed for {}: {err}",
            dest.display()
        ))),
    }
}

#[cfg(not(windows))]
fn replace_file(dest: &Path, temp: &Path) -> io::Result<()> {
    std::fs::rename(temp, dest)
}

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_replaces_whole_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.txt");
        atomic_write_bytes(&target, b"first-content").unwrap();
        atomic_write_bytes(&target, b"second").unwrap();
        let bytes = std::fs::read(&target).unwrap();
        assert_eq!(bytes, b"second");
    }

    #[test]
    fn atomic_write_json_replaces_whole_file() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("session.json");
        atomic_write_json(&target, &serde_json::json!({ "value": 1 })).unwrap();
        atomic_write_json(&target, &serde_json::json!({ "value": 2 })).unwrap();

        let parsed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(parsed["value"], 2);

        let leftovers = std::fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.contains(TEMP_MARKER))
            })
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn cleanup_removes_old_temp_files() {
        let temp = TempDir::new().unwrap();
        let stale = temp.path().join(format!("session.json{TEMP_MARKER}1.2.3"));
        std::fs::write(&stale, b"x").unwrap();
        let removed = cleanup_stale_temp_files(temp.path(), Duration::ZERO).unwrap();
        assert_eq!(removed, 1);
        assert!(!stale.exists());
    }

    #[test]
    fn cleanup_ignores_non_atomic_temp_files() {
        let temp = TempDir::new().unwrap();
        let unrelated = temp.path().join("session.json.tmp.1.2.3");
        std::fs::write(&unrelated, b"x").unwrap();
        let removed = cleanup_stale_temp_files(temp.path(), Duration::ZERO).unwrap();
        assert_eq!(removed, 0);
        assert!(unrelated.exists());
        let similar = temp.path().join("notes.rivet-tmp.keep-this.txt");
        std::fs::write(&similar, b"keep this note").unwrap();
        assert_eq!(
            cleanup_stale_temp_files(temp.path(), Duration::ZERO).unwrap(),
            0
        );
        assert!(similar.exists());
    }

    #[cfg(windows)]
    #[test]
    fn failed_replacement_preserves_original_and_completed_new_contents() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("locked.txt");
        std::fs::write(&target, b"original").unwrap();
        let _locked = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&target)
            .unwrap();
        assert!(atomic_write_bytes(&target, b"new contents").is_err());
        cleanup_stale_temp_files(temp.path(), Duration::ZERO).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"original");
        assert!(
            std::fs::read_dir(temp.path())
                .unwrap()
                .filter_map(Result::ok)
                .any(|entry| std::fs::read(entry.path()).ok().as_deref() == Some(b"new contents"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn replacement_preserves_named_streams() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("streams.txt");
        std::fs::write(&target, b"original").unwrap();
        let stream = format!("{}:rivet_test_metadata", target.display());
        std::fs::write(&stream, b"preserve this metadata").unwrap();
        atomic_write_bytes(&target, b"replacement").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"replacement");
        assert_eq!(std::fs::read(&stream).unwrap(), b"preserve this metadata");
    }

    #[cfg(windows)]
    #[test]
    fn staging_and_replacement_preserve_access_list() {
        use windows::Win32::Foundation::BOOL;
        use windows::Win32::Security::{ACL, GetSecurityDescriptorDacl};
        fn acl_bytes(path: &Path) -> Vec<u8> {
            let mut descriptor = read_dacl(path).unwrap();
            let mut present = BOOL(0);
            let mut defaulted = BOOL(0);
            let mut acl: *mut ACL = std::ptr::null_mut();
            unsafe {
                GetSecurityDescriptorDacl(
                    PSECURITY_DESCRIPTOR(descriptor.as_mut_ptr().cast()),
                    &mut present,
                    &mut acl,
                    &mut defaulted,
                )
                .unwrap();
                assert!(present.as_bool() && !acl.is_null());
                std::slice::from_raw_parts(acl.cast::<u8>(), (*acl).AclSize as usize).to_vec()
            }
        }
        let directory = TempDir::new().unwrap();
        let original = directory.path().join("protected.txt");
        let replacement = directory.path().join("empty.txt");
        std::fs::write(&original, b"original").unwrap();
        std::fs::write(&replacement, b"").unwrap();
        protect_replacement(&original, &original).unwrap();
        let acl = acl_bytes(&original);
        protect_replacement(&original, &replacement).unwrap();
        assert_eq!(acl_bytes(&replacement), acl);
        atomic_write_bytes(&original, b"new contents").unwrap();
        assert_eq!(acl_bytes(&original), acl);
        assert_eq!(std::fs::read(&original).unwrap(), b"new contents");
    }
}
