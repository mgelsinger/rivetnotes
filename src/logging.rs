use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::session;
use crate::error::{AppError, Result};

const MAX_LOG_SIZE: u64 = 512 * 1024;
const MAX_LOG_FILES: usize = 3;

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

struct Logger {
    file: Option<File>,
    path: PathBuf,
    bytes_written: u64,
    verbose: bool,
}

impl Logger {
    fn new(path: PathBuf, verbose: bool) -> Self {
        let file = open_log_file(&path).ok();
        let bytes_written = file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |metadata| metadata.len());
        Self {
            file,
            path,
            bytes_written,
            verbose,
        }
    }

    fn append(&mut self, line: &str) {
        if self.bytes_written >= MAX_LOG_SIZE {
            // Close before renaming on Windows. Long-running instances need
            // rotation too, not only applications that are frequently restarted.
            self.file.take();
            self.file = open_log_file(&self.path).ok();
            self.bytes_written = 0;
        }
        if let Some(file) = self.file.as_mut()
            && file.write_all(line.as_bytes()).is_ok()
        {
            self.bytes_written = self.bytes_written.saturating_add(line.len() as u64);
            let _ = file.flush();
        }
    }
}

pub fn verbose_from_env() -> bool {
    match std::env::var("RIVET_VERBOSE") {
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            matches!(value.as_str(), "1" | "true" | "yes" | "on" | "verbose")
        }
        Err(_) => false,
    }
}

pub fn init(verbose: bool) -> Result<()> {
    let logger = Logger::new(log_directory()?.join("rivet.log"), verbose);
    let _ = LOGGER.set(Mutex::new(logger));
    log_info("logging initialized");
    Ok(())
}

pub fn log_error(message: &str) {
    write_line("ERROR", message, true);
}

pub fn log_info(message: &str) {
    write_line("INFO", message, false);
}

fn write_line(level: &str, message: &str, force: bool) {
    let Some(logger) = LOGGER.get() else {
        return;
    };
    let Ok(mut logger) = logger.lock() else {
        return;
    };
    if !force && !logger.verbose {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let mut end = message.len().min(16 * 1024);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    let suffix = if end < message.len() {
        " [truncated]"
    } else {
        ""
    };
    logger.append(&format!(
        "{timestamp} [{level}] {}{suffix}\n",
        &message[..end]
    ));
}

fn open_log_file(log_path: &Path) -> Result<File> {
    let dir = log_path
        .parent()
        .ok_or_else(|| AppError::new("Invalid log path"))?;
    fs::create_dir_all(dir)
        .map_err(|err| AppError::new(format!("Failed to create log directory: {err}")))?;
    rotate_logs(log_path)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|err| AppError::new(format!("Failed to open log file: {err}")))
}

fn log_directory() -> Result<PathBuf> {
    let path = session::data_dir().unwrap_or_else(|_| std::env::temp_dir().join("Rivet"));
    Ok(path.join("logs"))
}

fn rotate_logs(path: &Path) -> Result<()> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(_) => return Ok(()),
    };
    if size < MAX_LOG_SIZE {
        return Ok(());
    }
    for index in (1..=MAX_LOG_FILES).rev() {
        let from = if index == 1 {
            path.to_path_buf()
        } else {
            path.with_extension(format!("log.{}", index - 1))
        };
        let to = path.with_extension(format!("log.{index}"));
        if index == MAX_LOG_FILES {
            let _ = fs::remove_file(&to);
        }
        if from.exists() {
            let _ = fs::rename(from, to);
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn logging_rotates_while_the_process_remains_open() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rivet.log");
        let mut logger = Logger::new(path.clone(), true);
        let line = "x".repeat(16 * 1024);
        for _ in 0..200 {
            logger.append(&line);
        }
        drop(logger);
        let files: Vec<_> = fs::read_dir(directory.path()).unwrap().collect();
        assert_eq!(files.len(), MAX_LOG_FILES + 1);
        for file in files {
            assert!(file.unwrap().metadata().unwrap().len() <= MAX_LOG_SIZE);
        }
        assert!(path.exists());
    }
}
