use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    MOVE_FILE_FLAGS, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    REPLACE_FILE_FLAGS, ReplaceFileW,
};
#[cfg(windows)]
use windows::core::PCWSTR;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn atomic_write_bytes(dest: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = dest.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Destination path has no parent directory.",
        )
    })?;
    std::fs::create_dir_all(parent)?;

    let temp_path = temp_path_for(dest);
    {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }

    let replace_result = replace_file(dest, &temp_path);
    if replace_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    replace_result
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
        if !file_name.contains(".tmp.") {
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
    parent.join(format!("{base}.tmp.{pid}.{nanos:x}.{seq:x}"))
}

#[cfg(windows)]
fn replace_file(dest: &Path, temp: &Path) -> io::Result<()> {
    let dest_w = path_to_wide(dest);
    let temp_w = path_to_wide(temp);

    if dest.exists() {
        let replaced = unsafe {
            ReplaceFileW(
                PCWSTR(dest_w.as_ptr()),
                PCWSTR(temp_w.as_ptr()),
                PCWSTR::null(),
                REPLACE_FILE_FLAGS(0),
                None,
                None,
            )
        };
        if replaced.is_ok() {
            return Ok(());
        }
    }

    let flags = MOVE_FILE_FLAGS(MOVEFILE_REPLACE_EXISTING.0 | MOVEFILE_WRITE_THROUGH.0);
    let moved = unsafe { MoveFileExW(PCWSTR(temp_w.as_ptr()), PCWSTR(dest_w.as_ptr()), flags) };
    if moved.is_ok() {
        return Ok(());
    }

    Err(io::Error::other(format!(
        "ReplaceFileW/MoveFileExW failed for {}",
        dest.display()
    )))
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
    fn cleanup_removes_old_temp_files() {
        let temp = TempDir::new().unwrap();
        let stale = temp.path().join("session.json.tmp.1.2.3");
        std::fs::write(&stale, b"x").unwrap();
        let removed = cleanup_stale_temp_files(temp.path(), Duration::ZERO).unwrap();
        assert_eq!(removed, 1);
        assert!(!stale.exists());
    }
}
