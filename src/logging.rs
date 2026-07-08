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
    verbose: bool,
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
    let file = open_log_file().ok();
    let logger = Logger { file, verbose };
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
    let Some(file) = logger.file.as_mut() else {
        return;
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "{timestamp} [{level}] {message}");
    let _ = file.flush();
}

fn open_log_file() -> Result<File> {
    let dir = log_directory()?;
    fs::create_dir_all(&dir)
        .map_err(|err| AppError::new(format!("Failed to create log directory: {err}")))?;
    let log_path = dir.join("rivet.log");
    rotate_logs(&log_path)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
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
