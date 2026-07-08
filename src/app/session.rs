use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::storage::atomic_write::{
    atomic_write_bytes, atomic_write_json, cleanup_stale_temp_files,
};

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

pub const DEFAULT_REMEMBER_SESSION: bool = true;
pub const DEFAULT_SESSION_SNAPSHOT_PERIODIC_BACKUP: bool = true;
pub const DEFAULT_BACKUP_INTERVAL_SECONDS: u32 = 7;
pub const DEFAULT_WORD_WRAP_ENABLED: bool = true;
pub const DEFAULT_ALWAYS_ON_TOP: bool = false;

const APP_DIR_NAME: &str = "Rivet";
const PORTABLE_DATA_DIR_NAME: &str = "data";
const SESSIONS_DIR_NAME: &str = "sessions";
const BACKUP_DIR_NAME: &str = "backup";
const SESSION_FILE_NAME: &str = "session.json";
const SESSION_SCHEMA_VERSION: u32 = 2;

const TEMP_CLEANUP_MAX_AGE_DAYS: u64 = 7;

#[cfg(test)]
static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
pub(crate) fn test_env_lock() -> &'static Mutex<()> {
    TEST_ENV_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrikeRange {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowPlacementData {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    #[serde(default)]
    pub maximized: bool,
}

impl WindowPlacementData {
    /// Rejects placements that could not have come from a real window:
    /// hand-edited JSON or the -32000 coordinates of a minimized window.
    pub fn sanitized(self) -> Option<Self> {
        let coord_ok = |v: i32| (-32_000..=32_000).contains(&v);
        if (200..=32_000).contains(&self.width)
            && (120..=32_000).contains(&self.height)
            && coord_ok(self.x)
            && coord_ok(self.y)
        {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionEntry {
    #[serde(rename = "tab_id", alias = "id")]
    pub id: Uuid,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub display_name: String,
    #[serde(rename = "backup_file", alias = "backup_path")]
    pub backup_path: PathBuf,
    #[serde(rename = "was_dirty_at_last_exit", alias = "is_dirty")]
    pub is_dirty: bool,
    #[serde(default)]
    pub cursor_pos: i64,
    #[serde(rename = "backup_timestamp", alias = "last_backup_timestamp")]
    pub backup_timestamp: Option<u64>,
    #[serde(default)]
    pub disk_timestamp_at_backup: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub strike_ranges: Vec<StrikeRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionData {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_app_version")]
    pub app_version: String,
    #[serde(default = "default_remember_session")]
    pub remember_session: bool,
    #[serde(
        default = "default_session_snapshot_periodic_backup",
        alias = "session_snapshot_enabled"
    )]
    pub session_snapshot_periodic_backup: bool,
    #[serde(
        default = "default_backup_interval_seconds",
        alias = "backup_interval_secs"
    )]
    pub backup_interval_seconds: u32,
    #[serde(default = "default_word_wrap_enabled")]
    pub word_wrap_enabled: bool,
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_placement: Option<WindowPlacementData>,
    #[serde(default)]
    pub active_tab_id: Option<Uuid>,
    #[serde(default)]
    pub entries: Vec<SessionEntry>,
}

impl SessionData {
    pub fn empty() -> Self {
        Self {
            schema_version: SESSION_SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            remember_session: DEFAULT_REMEMBER_SESSION,
            session_snapshot_periodic_backup: DEFAULT_SESSION_SNAPSHOT_PERIODIC_BACKUP,
            backup_interval_seconds: DEFAULT_BACKUP_INTERVAL_SECONDS,
            word_wrap_enabled: DEFAULT_WORD_WRAP_ENABLED,
            always_on_top: DEFAULT_ALWAYS_ON_TOP,
            window_placement: None,
            active_tab_id: None,
            entries: Vec::new(),
        }
    }

    fn normalized(mut self) -> Self {
        if self.backup_interval_seconds == 0 {
            self.backup_interval_seconds = DEFAULT_BACKUP_INTERVAL_SECONDS;
        }
        if self.schema_version == 0 {
            self.schema_version = SESSION_SCHEMA_VERSION;
        }
        if self.session_snapshot_periodic_backup && !self.remember_session {
            self.remember_session = true;
        }
        self.window_placement = self
            .window_placement
            .and_then(WindowPlacementData::sanitized);
        if !self.remember_session {
            self.entries.clear();
            self.active_tab_id = None;
        }
        self
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RestoreSource {
    Disk,
    Backup,
    Skip,
}

#[derive(Debug, Clone)]
pub struct RestoreDecisionInput {
    pub remember_session: bool,
    pub path: Option<PathBuf>,
    pub backup_path: PathBuf,
    pub was_dirty_at_exit: bool,
    pub backup_modified: Option<SystemTime>,
    pub disk_modified: Option<SystemTime>,
}

pub fn decide_restore_source(input: &RestoreDecisionInput) -> RestoreSource {
    match &input.path {
        Some(path) => {
            let has_disk = input.disk_modified.is_some() || path.exists();
            let has_backup = input.backup_modified.is_some() || input.backup_path.exists();

            if input.remember_session && input.was_dirty_at_exit && has_backup {
                return RestoreSource::Backup;
            }
            if has_disk {
                RestoreSource::Disk
            } else {
                RestoreSource::Skip
            }
        }
        None => {
            if input.backup_path.exists() {
                RestoreSource::Backup
            } else {
                RestoreSource::Skip
            }
        }
    }
}

/// Data directory next to the executable, used when that location is
/// writable (i.e. running as a portable build, not installed under a
/// protected directory like Program Files). Returns `None` if the exe path
/// can't be resolved or the directory isn't writable, in which case the
/// caller falls back to the per-user AppData location.
fn portable_data_dir() -> Option<PathBuf> {
    if cfg!(test) {
        // Tests exercise data_dir() via LOCALAPPDATA/APPDATA env overrides;
        // the test binary's own directory is writable, which would
        // short-circuit those overrides and scribble into target/.
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let candidate = exe_dir.join(PORTABLE_DATA_DIR_NAME);
    fs::create_dir_all(&candidate).ok()?;
    let probe = candidate.join(".write_test");
    fs::write(&probe, b"").ok()?;
    let _ = fs::remove_file(&probe);
    Some(candidate)
}

pub fn data_dir() -> Result<PathBuf> {
    if let Some(portable) = portable_data_dir() {
        return Ok(portable);
    }

    if let Ok(local) = std::env::var("LOCALAPPDATA")
        && !local.is_empty()
    {
        return Ok(Path::new(&local).join(APP_DIR_NAME));
    }

    if let Ok(roaming) = std::env::var("APPDATA")
        && !roaming.is_empty()
    {
        return Ok(Path::new(&roaming).join(APP_DIR_NAME));
    }

    let current = std::env::current_dir()
        .map_err(|err| AppError::new(format!("Failed to get current directory: {err}")))?;
    Ok(current.join(APP_DIR_NAME))
}

pub fn sessions_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join(SESSIONS_DIR_NAME))
}

pub fn backup_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join(BACKUP_DIR_NAME))
}

pub fn session_file_path() -> Result<PathBuf> {
    Ok(sessions_dir()?.join(SESSION_FILE_NAME))
}

pub fn backup_path_for_id(id: Uuid) -> Result<PathBuf> {
    Ok(backup_dir()?.join(format!("{id}.bak")))
}

pub fn ensure_storage_dirs() -> Result<()> {
    let sessions = sessions_dir()?;
    let backups = backup_dir()?;
    std::fs::create_dir_all(&sessions)
        .map_err(|err| AppError::new(format!("Failed to create sessions directory: {err}")))?;
    std::fs::create_dir_all(&backups)
        .map_err(|err| AppError::new(format!("Failed to create backup directory: {err}")))?;
    let max_age = Duration::from_secs(TEMP_CLEANUP_MAX_AGE_DAYS * 24 * 60 * 60);
    let _ = cleanup_stale_temp_files(&sessions, max_age);
    let _ = cleanup_stale_temp_files(&backups, max_age);
    Ok(())
}

pub fn load_session() -> Result<SessionData> {
    ensure_storage_dirs()?;
    let path = session_file_path()?;
    if !path.exists() {
        return Ok(SessionData::empty());
    }

    let bytes = std::fs::read(&path)
        .map_err(|err| AppError::new(format!("Failed to read session file: {err}")))?;
    let session: SessionData = serde_json::from_slice(&bytes)
        .map_err(|err| AppError::new(format!("Failed to parse session file: {err}")))?;
    Ok(session.normalized())
}

pub fn save_session(data: &SessionData) -> Result<()> {
    ensure_storage_dirs()?;
    let path = session_file_path()?;
    let normalized = data.clone().normalized();
    atomic_write_json(&path, &normalized)
        .map_err(|err| AppError::new(format!("Failed to write session file atomically: {err}")))
}

pub fn write_backup(backup_path: &Path, bytes: &[u8]) -> Result<SystemTime> {
    atomic_write_bytes(backup_path, bytes)
        .map_err(|err| AppError::new(format!("Failed to write backup file atomically: {err}")))?;
    modified_time(backup_path)
}

pub fn modified_time(path: &Path) -> Result<SystemTime> {
    let meta = std::fs::metadata(path)
        .map_err(|err| AppError::new(format!("Failed to read file metadata: {err}")))?;
    meta.modified()
        .map_err(|err| AppError::new(format!("Failed to read file modified time: {err}")))
}

pub fn unix_timestamp(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

pub fn delete_backup(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(path)
        .map_err(|err| AppError::new(format!("Failed to delete backup file: {err}")))
}

fn default_schema_version() -> u32 {
    SESSION_SCHEMA_VERSION
}

fn default_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_remember_session() -> bool {
    DEFAULT_REMEMBER_SESSION
}

fn default_session_snapshot_periodic_backup() -> bool {
    DEFAULT_SESSION_SNAPSHOT_PERIODIC_BACKUP
}

fn default_backup_interval_seconds() -> u32 {
    DEFAULT_BACKUP_INTERVAL_SECONDS
}

fn default_word_wrap_enabled() -> bool {
    DEFAULT_WORD_WRAP_ENABLED
}

fn default_always_on_top() -> bool {
    DEFAULT_ALWAYS_ON_TOP
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_local_appdata<F>(action: F)
    where
        F: FnOnce(PathBuf),
    {
        let lock = test_env_lock();
        let _guard = lock.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_local = std::env::var("LOCALAPPDATA").ok();
        let original_appdata = std::env::var("APPDATA").ok();

        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
            std::env::set_var("APPDATA", temp.path().join("roaming"));
        }

        action(temp.path().to_path_buf());

        if let Some(value) = original_local {
            unsafe {
                std::env::set_var("LOCALAPPDATA", value);
            }
        } else {
            unsafe {
                std::env::remove_var("LOCALAPPDATA");
            }
        }

        if let Some(value) = original_appdata {
            unsafe {
                std::env::set_var("APPDATA", value);
            }
        } else {
            unsafe {
                std::env::remove_var("APPDATA");
            }
        }
    }

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
    fn session_json_roundtrip() {
        with_temp_local_appdata(|_| {
            let id = Uuid::new_v4();
            let backup = backup_path_for_id(id).unwrap();
            let data = SessionData {
                schema_version: SESSION_SCHEMA_VERSION,
                app_version: "0.2.1".to_string(),
                remember_session: true,
                session_snapshot_periodic_backup: true,
                backup_interval_seconds: 7,
                word_wrap_enabled: true,
                always_on_top: true,
                window_placement: Some(WindowPlacementData {
                    x: 64,
                    y: 48,
                    width: 1024,
                    height: 768,
                    maximized: false,
                }),
                active_tab_id: Some(id),
                entries: vec![SessionEntry {
                    id,
                    path: Some(PathBuf::from("C:\\notes\\sample.txt")),
                    display_name: "sample.txt".to_string(),
                    backup_path: backup,
                    is_dirty: true,
                    cursor_pos: 11,
                    backup_timestamp: Some(123),
                    disk_timestamp_at_backup: Some(456),
                    strike_ranges: vec![
                        StrikeRange { start: 2, end: 7 },
                        StrikeRange { start: 10, end: 12 },
                    ],
                }],
            };

            save_session(&data).unwrap();
            let loaded = load_session().unwrap();
            assert_eq!(loaded, data);
        });
    }

    #[test]
    fn backup_path_uses_tab_id_scheme() {
        with_temp_local_appdata(|_| {
            let id = Uuid::new_v4();
            let path = backup_path_for_id(id).unwrap();
            let file_name = path.file_name().and_then(|name| name.to_str()).unwrap();
            assert_eq!(file_name, format!("{id}.bak"));
        });
    }

    #[test]
    fn periodic_backup_implies_remember_session() {
        let data = SessionData {
            remember_session: false,
            session_snapshot_periodic_backup: true,
            ..SessionData::empty()
        };
        let normalized = data.normalized();
        assert!(normalized.remember_session);
        assert!(normalized.session_snapshot_periodic_backup);
    }

    #[test]
    fn always_on_top_defaults_false_when_missing() {
        let json = r#"{
            "schema_version":1,
            "app_version":"0.2.1",
            "remember_session":true,
            "session_snapshot_periodic_backup":true,
            "backup_interval_seconds":7,
            "word_wrap_enabled":true,
            "entries":[]
        }"#;
        let parsed: SessionData = serde_json::from_str(json).unwrap();
        assert!(!parsed.always_on_top);
    }

    #[test]
    fn window_placement_round_trips_through_json() {
        let data = SessionData {
            window_placement: Some(WindowPlacementData {
                x: 120,
                y: 80,
                width: 1200,
                height: 800,
                maximized: true,
            }),
            ..SessionData::empty()
        };
        let json = serde_json::to_string(&data).unwrap();
        let parsed: SessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.window_placement, data.window_placement);
    }

    #[test]
    fn window_placement_defaults_none_when_missing() {
        let json = r#"{
            "schema_version":2,
            "app_version":"0.4.21",
            "remember_session":true,
            "session_snapshot_periodic_backup":true,
            "backup_interval_seconds":7,
            "word_wrap_enabled":true,
            "always_on_top":false,
            "entries":[]
        }"#;
        let parsed: SessionData = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.window_placement, None);
    }

    #[test]
    fn window_placement_sanitized_accepts_normal_rect() {
        let placement = WindowPlacementData {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
        };
        assert_eq!(placement.sanitized(), Some(placement));
    }

    #[test]
    fn window_placement_sanitized_rejects_nonsense() {
        let base = WindowPlacementData {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
        };
        assert_eq!(WindowPlacementData { width: 0, ..base }.sanitized(), None);
        assert_eq!(
            WindowPlacementData {
                width: -500,
                ..base
            }
            .sanitized(),
            None
        );
        assert_eq!(WindowPlacementData { height: 50, ..base }.sanitized(), None);
        assert_eq!(
            WindowPlacementData {
                x: 1_000_000,
                ..base
            }
            .sanitized(),
            None
        );
        assert_eq!(
            WindowPlacementData {
                y: -100_000,
                ..base
            }
            .sanitized(),
            None
        );
    }

    #[test]
    fn normalized_drops_invalid_window_placement() {
        let data = SessionData {
            window_placement: Some(WindowPlacementData {
                x: 99_999,
                y: 0,
                width: 1200,
                height: 800,
                maximized: false,
            }),
            ..SessionData::empty()
        };
        assert_eq!(data.normalized().window_placement, None);
    }

    #[test]
    fn normalized_keeps_window_placement_without_remember_session() {
        let placement = WindowPlacementData {
            x: 10,
            y: 20,
            width: 900,
            height: 600,
            maximized: false,
        };
        let data = SessionData {
            remember_session: false,
            session_snapshot_periodic_backup: false,
            window_placement: Some(placement),
            ..SessionData::empty()
        };
        assert_eq!(data.normalized().window_placement, Some(placement));
    }

    #[test]
    fn restore_decision_prefers_backup_when_marked_dirty() {
        let now = SystemTime::now();
        let input = RestoreDecisionInput {
            remember_session: true,
            path: Some(PathBuf::from("C:\\tmp\\file.txt")),
            backup_path: PathBuf::from("C:\\tmp\\backup.bak"),
            was_dirty_at_exit: true,
            backup_modified: Some(now),
            disk_modified: None,
        };
        assert_eq!(decide_restore_source(&input), RestoreSource::Backup);
    }

    #[test]
    fn restore_decision_uses_disk_when_not_dirty_and_backup_not_newer() {
        let now = SystemTime::now();
        let input = RestoreDecisionInput {
            remember_session: true,
            path: Some(PathBuf::from("C:\\tmp\\file.txt")),
            backup_path: PathBuf::from("C:\\tmp\\backup.bak"),
            was_dirty_at_exit: false,
            backup_modified: Some(now),
            disk_modified: Some(now),
        };
        assert_eq!(decide_restore_source(&input), RestoreSource::Disk);
    }

    #[test]
    fn restore_decision_clean_named_without_disk_skips_even_with_backup() {
        let now = SystemTime::now();
        let input = RestoreDecisionInput {
            remember_session: true,
            path: Some(PathBuf::from("C:\\tmp\\missing.txt")),
            backup_path: PathBuf::from("C:\\tmp\\backup.bak"),
            was_dirty_at_exit: false,
            backup_modified: Some(now),
            disk_modified: None,
        };
        assert_eq!(decide_restore_source(&input), RestoreSource::Skip);
    }

    #[test]
    fn restore_decision_without_remember_session_uses_disk_for_named_files() {
        let now = SystemTime::now();
        let input = RestoreDecisionInput {
            remember_session: false,
            path: Some(PathBuf::from("C:\\tmp\\file.txt")),
            backup_path: PathBuf::from("C:\\tmp\\backup.bak"),
            was_dirty_at_exit: true,
            backup_modified: Some(now),
            disk_modified: Some(now),
        };
        assert_eq!(decide_restore_source(&input), RestoreSource::Disk);
    }

    #[test]
    fn restore_decision_untitled_uses_backup() {
        let temp = TempDir::new().unwrap();
        let backup = temp.path().join("untitled.bak");
        std::fs::write(&backup, b"untitled content").unwrap();
        let input = RestoreDecisionInput {
            remember_session: true,
            path: None,
            backup_path: backup,
            was_dirty_at_exit: true,
            backup_modified: Some(SystemTime::now()),
            disk_modified: None,
        };
        assert_eq!(decide_restore_source(&input), RestoreSource::Backup);
    }

    #[test]
    fn headless_backup_restore_roundtrip() {
        with_temp_local_appdata(|root| {
            let id = Uuid::new_v4();
            let disk_path = root.join("doc.txt");
            std::fs::write(&disk_path, b"on-disk").unwrap();
            let backup_path = backup_path_for_id(id).unwrap();
            write_backup(&backup_path, b"from-backup").unwrap();

            let data = SessionData {
                schema_version: SESSION_SCHEMA_VERSION,
                app_version: "0.2.1".to_string(),
                remember_session: true,
                session_snapshot_periodic_backup: true,
                backup_interval_seconds: 7,
                word_wrap_enabled: true,
                always_on_top: false,
                window_placement: None,
                active_tab_id: Some(id),
                entries: vec![SessionEntry {
                    id,
                    path: Some(disk_path.clone()),
                    display_name: "doc.txt".to_string(),
                    backup_path: backup_path.clone(),
                    is_dirty: true,
                    cursor_pos: 0,
                    backup_timestamp: None,
                    disk_timestamp_at_backup: None,
                    strike_ranges: vec![StrikeRange { start: 1, end: 4 }],
                }],
            };
            save_session(&data).unwrap();
            let loaded = load_session().unwrap();
            let entry = &loaded.entries[0];
            let decision = decide_restore_source(&RestoreDecisionInput {
                remember_session: loaded.remember_session,
                path: entry.path.clone(),
                backup_path: entry.backup_path.clone(),
                was_dirty_at_exit: entry.is_dirty,
                backup_modified: modified_time(&entry.backup_path).ok(),
                disk_modified: entry
                    .path
                    .as_ref()
                    .and_then(|path| modified_time(path).ok()),
            });
            assert_eq!(decision, RestoreSource::Backup);
            let restored = std::fs::read_to_string(&entry.backup_path).unwrap();
            assert_eq!(restored, "from-backup");
        });
    }

    #[test]
    fn strike_ranges_default_to_empty_when_missing() {
        let json = r#"{
            "schema_version":1,
            "app_version":"0.2.1",
            "remember_session":true,
            "session_snapshot_periodic_backup":true,
            "backup_interval_seconds":7,
            "word_wrap_enabled":true,
            "always_on_top":false,
            "entries":[{
                "tab_id":"00000000-0000-0000-0000-000000000001",
                "path":"C:\\notes\\sample.txt",
                "display_name":"sample.txt",
                "backup_file":"C:\\notes\\sample.bak",
                "was_dirty_at_last_exit":false,
                "cursor_pos":0
            }]
        }"#;
        let parsed: SessionData = serde_json::from_str(json).unwrap();
        assert!(parsed.entries[0].strike_ranges.is_empty());
    }
}
