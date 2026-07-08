use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::app::session;
use crate::error::{AppError, Result};
use crate::storage::atomic_write::{atomic_write_json, cleanup_stale_temp_files};

pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const MIN_VERTICAL_TAB_WIDTH_PX: i32 = 80;
pub const MAX_VERTICAL_TAB_WIDTH_PX: i32 = 600;
pub const DEFAULT_VERTICAL_TAB_WIDTH_PX: i32 = 180;
pub const DEFAULT_EDITOR_DARK: bool = true;
pub const DEFAULT_SMART_HIGHLIGHT_ENABLED: bool = true;
pub const DEFAULT_SMART_HIGHLIGHT_MATCH_CASE: bool = false;
pub const DEFAULT_SMART_HIGHLIGHT_WHOLE_WORD: bool = true;
pub const DEFAULT_LARGE_FILE_THRESHOLD_MB: u32 = 20;
pub const MIN_LARGE_FILE_THRESHOLD_MB: u32 = 1;
pub const MAX_LARGE_FILE_THRESHOLD_MB: u32 = 1024;
pub const DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP: bool = true;
pub const DEFAULT_LARGE_FILE_DISABLE_SMART_HIGHLIGHT: bool = true;
pub const MAX_RECENT_FILES: usize = 10;
pub const DEFAULT_ZOOM_LEVEL: i32 = 0;
/// Scintilla's supported zoom range (points added to the base font size).
pub const MIN_ZOOM_LEVEL: i32 = -10;
pub const MAX_ZOOM_LEVEL: i32 = 20;
pub const DEFAULT_EDITOR_FONT_NAME: &str = "Consolas";
pub const DEFAULT_EDITOR_FONT_SIZE: i32 = 11;
pub const MIN_EDITOR_FONT_SIZE: i32 = 6;
pub const MAX_EDITOR_FONT_SIZE: i32 = 72;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TabPlacement {
    #[default]
    Top,
    Left,
    Right,
}

impl TabPlacement {
    pub fn next(self) -> Self {
        match self {
            TabPlacement::Top => TabPlacement::Left,
            TabPlacement::Left => TabPlacement::Right,
            TabPlacement::Right => TabPlacement::Top,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UiSettings {
    #[serde(default)]
    pub tab_placement: TabPlacement,
    #[serde(default = "default_vertical_tab_width_px")]
    pub vertical_tab_width_px: i32,
    #[serde(default = "default_editor_dark")]
    pub editor_dark: bool,
    #[serde(default = "default_smart_highlight_enabled")]
    pub smart_highlight_enabled: bool,
    #[serde(default = "default_smart_highlight_match_case")]
    pub smart_highlight_match_case: bool,
    #[serde(default = "default_smart_highlight_whole_word")]
    pub smart_highlight_whole_word: bool,
    /// `settings.json`: files at or above this many MiB enter Large File Mode.
    #[serde(default = "default_large_file_threshold_mb")]
    pub large_file_threshold_mb: u32,
    /// `settings.json`: when true, Large File Mode suppresses word wrap for that tab.
    pub large_file_disable_word_wrap: bool,
    /// `settings.json`: when true, Large File Mode suppresses smart highlight for that tab.
    pub large_file_disable_smart_highlight: bool,
    /// `settings.json`: most-recently-opened file paths, newest first.
    #[serde(default)]
    pub recent_files: Vec<String>,
    /// `settings.json`: app-wide editor zoom level in points, clamped to
    /// [`MIN_ZOOM_LEVEL`]..=[`MAX_ZOOM_LEVEL`].
    #[serde(default)]
    pub zoom_level: i32,
    /// `settings.json`: editor font family name (e.g. "Consolas").
    #[serde(default = "default_editor_font_name")]
    pub editor_font_name: String,
    /// `settings.json`: editor base font size in points, clamped to
    /// [`MIN_EDITOR_FONT_SIZE`]..=[`MAX_EDITOR_FONT_SIZE`].
    #[serde(default = "default_editor_font_size")]
    pub editor_font_size: i32,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            tab_placement: TabPlacement::Top,
            vertical_tab_width_px: DEFAULT_VERTICAL_TAB_WIDTH_PX,
            editor_dark: DEFAULT_EDITOR_DARK,
            smart_highlight_enabled: DEFAULT_SMART_HIGHLIGHT_ENABLED,
            smart_highlight_match_case: DEFAULT_SMART_HIGHLIGHT_MATCH_CASE,
            smart_highlight_whole_word: DEFAULT_SMART_HIGHLIGHT_WHOLE_WORD,
            large_file_threshold_mb: DEFAULT_LARGE_FILE_THRESHOLD_MB,
            large_file_disable_word_wrap: DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP,
            large_file_disable_smart_highlight: DEFAULT_LARGE_FILE_DISABLE_SMART_HIGHLIGHT,
            recent_files: Vec::new(),
            zoom_level: DEFAULT_ZOOM_LEVEL,
            editor_font_name: DEFAULT_EDITOR_FONT_NAME.to_string(),
            editor_font_size: DEFAULT_EDITOR_FONT_SIZE,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UiSettingsWire {
    #[serde(default)]
    tab_placement: TabPlacement,
    #[serde(default = "default_vertical_tab_width_px")]
    vertical_tab_width_px: i32,
    #[serde(default = "default_editor_dark")]
    editor_dark: bool,
    #[serde(default = "default_smart_highlight_enabled")]
    smart_highlight_enabled: bool,
    #[serde(default = "default_smart_highlight_match_case")]
    smart_highlight_match_case: bool,
    #[serde(default = "default_smart_highlight_whole_word")]
    smart_highlight_whole_word: bool,
    #[serde(default = "default_large_file_threshold_mb")]
    large_file_threshold_mb: u32,
    #[serde(default)]
    large_file_disable_word_wrap: Option<bool>,
    #[serde(default)]
    large_file_disable_word_wrap_globally: Option<bool>,
    #[serde(default)]
    large_file_disable_smart_highlight: Option<bool>,
    #[serde(default)]
    large_file_allow_smart_highlight: Option<bool>,
    #[serde(default)]
    recent_files: Vec<String>,
    #[serde(default)]
    zoom_level: i32,
    #[serde(default = "default_editor_font_name")]
    editor_font_name: String,
    #[serde(default = "default_editor_font_size")]
    editor_font_size: i32,
}

impl From<UiSettingsWire> for UiSettings {
    fn from(value: UiSettingsWire) -> Self {
        Self {
            tab_placement: value.tab_placement,
            vertical_tab_width_px: value.vertical_tab_width_px,
            editor_dark: value.editor_dark,
            smart_highlight_enabled: value.smart_highlight_enabled,
            smart_highlight_match_case: value.smart_highlight_match_case,
            smart_highlight_whole_word: value.smart_highlight_whole_word,
            large_file_threshold_mb: value.large_file_threshold_mb,
            large_file_disable_word_wrap: value
                .large_file_disable_word_wrap
                .or(value.large_file_disable_word_wrap_globally)
                .unwrap_or(DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP),
            large_file_disable_smart_highlight: value
                .large_file_disable_smart_highlight
                .or_else(|| value.large_file_allow_smart_highlight.map(|allow| !allow))
                .unwrap_or(DEFAULT_LARGE_FILE_DISABLE_SMART_HIGHLIGHT),
            recent_files: value.recent_files,
            zoom_level: value.zoom_level,
            editor_font_name: value.editor_font_name,
            editor_font_size: value.editor_font_size,
        }
    }
}

impl<'de> Deserialize<'de> for UiSettings {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UiSettingsWire::deserialize(deserializer).map(|wire| wire.into())
    }
}

impl UiSettings {
    /// Insert `path` at the front of the recent-files list, removing any
    /// case-insensitive duplicate and capping the list at [`MAX_RECENT_FILES`].
    pub fn push_recent(&mut self, path: String) {
        self.recent_files.retain(|p| !p.eq_ignore_ascii_case(&path));
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT_FILES);
    }

    fn normalized(mut self) -> Self {
        self.vertical_tab_width_px = self
            .vertical_tab_width_px
            .clamp(MIN_VERTICAL_TAB_WIDTH_PX, MAX_VERTICAL_TAB_WIDTH_PX);
        self.large_file_threshold_mb = self
            .large_file_threshold_mb
            .clamp(MIN_LARGE_FILE_THRESHOLD_MB, MAX_LARGE_FILE_THRESHOLD_MB);
        self.recent_files.truncate(MAX_RECENT_FILES);
        self.zoom_level = self.zoom_level.clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
        if self.editor_font_name.trim().is_empty() {
            self.editor_font_name = DEFAULT_EDITOR_FONT_NAME.to_string();
        }
        self.editor_font_size = self
            .editor_font_size
            .clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE);
        self
    }
}

pub fn settings_file_path() -> Result<PathBuf> {
    Ok(session::data_dir()?.join(SETTINGS_FILE_NAME))
}

pub fn load_settings() -> Result<UiSettings> {
    ensure_settings_dir()?;
    let path = settings_file_path()?;
    if !path.exists() {
        return Ok(UiSettings::default());
    }
    let bytes = std::fs::read(&path)
        .map_err(|err| AppError::new(format!("Failed to read settings file: {err}")))?;
    let parsed: UiSettings = serde_json::from_slice(&bytes)
        .map_err(|err| AppError::new(format!("Failed to parse settings file: {err}")))?;
    Ok(parsed.normalized())
}

pub fn save_settings(settings: &UiSettings) -> Result<()> {
    ensure_settings_dir()?;
    let path = settings_file_path()?;
    atomic_write_json(&path, &settings.clone().normalized())
        .map_err(|err| AppError::new(format!("Failed to write settings file atomically: {err}")))
}

fn ensure_settings_dir() -> Result<()> {
    let dir = session::data_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|err| AppError::new(format!("Failed to create settings directory: {err}")))?;
    let _ = cleanup_stale_temp_files(&dir, Duration::from_secs(7 * 24 * 60 * 60));
    Ok(())
}

fn default_vertical_tab_width_px() -> i32 {
    DEFAULT_VERTICAL_TAB_WIDTH_PX
}

fn default_smart_highlight_enabled() -> bool {
    DEFAULT_SMART_HIGHLIGHT_ENABLED
}

fn default_editor_dark() -> bool {
    DEFAULT_EDITOR_DARK
}

fn default_editor_font_name() -> String {
    DEFAULT_EDITOR_FONT_NAME.to_string()
}

fn default_editor_font_size() -> i32 {
    DEFAULT_EDITOR_FONT_SIZE
}

fn default_smart_highlight_match_case() -> bool {
    DEFAULT_SMART_HIGHLIGHT_MATCH_CASE
}

fn default_smart_highlight_whole_word() -> bool {
    DEFAULT_SMART_HIGHLIGHT_WHOLE_WORD
}

fn default_large_file_threshold_mb() -> u32 {
    DEFAULT_LARGE_FILE_THRESHOLD_MB
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn with_temp_local_appdata<F>(action: F)
    where
        F: FnOnce(),
    {
        let lock = session::test_env_lock();
        let _guard = lock.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original_local = std::env::var("LOCALAPPDATA").ok();
        let original_appdata = std::env::var("APPDATA").ok();

        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
            std::env::set_var("APPDATA", temp.path().join("roaming"));
        }

        action();

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
    fn push_recent_dedupes_and_moves_to_front() {
        let mut settings = UiSettings::default();
        settings.push_recent("C:\\a.txt".to_string());
        settings.push_recent("C:\\b.txt".to_string());
        // Re-opening 'a' (different case) moves it to the front without a dup.
        settings.push_recent("c:\\A.TXT".to_string());
        assert_eq!(settings.recent_files, vec!["c:\\A.TXT", "C:\\b.txt"]);
    }

    #[test]
    fn push_recent_caps_at_max() {
        let mut settings = UiSettings::default();
        for i in 0..(MAX_RECENT_FILES + 5) {
            settings.push_recent(format!("C:\\file{i}.txt"));
        }
        assert_eq!(settings.recent_files.len(), MAX_RECENT_FILES);
        // Newest is first.
        assert_eq!(
            settings.recent_files[0],
            format!("C:\\file{}.txt", MAX_RECENT_FILES + 4)
        );
    }

    #[test]
    fn tab_placement_cycles() {
        assert_eq!(TabPlacement::Top.next(), TabPlacement::Left);
        assert_eq!(TabPlacement::Left.next(), TabPlacement::Right);
        assert_eq!(TabPlacement::Right.next(), TabPlacement::Top);
    }

    #[test]
    fn settings_width_normalizes() {
        let settings = UiSettings {
            tab_placement: TabPlacement::Top,
            vertical_tab_width_px: 1000,
            ..UiSettings::default()
        };
        assert_eq!(
            settings.normalized().vertical_tab_width_px,
            MAX_VERTICAL_TAB_WIDTH_PX
        );
    }

    #[test]
    fn tab_placement_serializes_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&TabPlacement::Top).unwrap(),
            "\"top\""
        );
        assert_eq!(
            serde_json::to_string(&TabPlacement::Left).unwrap(),
            "\"left\""
        );
        assert_eq!(
            serde_json::to_string(&TabPlacement::Right).unwrap(),
            "\"right\""
        );
    }

    #[test]
    fn settings_json_roundtrip_uses_expected_fields() {
        let settings = UiSettings {
            tab_placement: TabPlacement::Right,
            vertical_tab_width_px: 320,
            editor_dark: false,
            smart_highlight_enabled: false,
            smart_highlight_match_case: true,
            smart_highlight_whole_word: false,
            large_file_threshold_mb: 50,
            large_file_disable_word_wrap: true,
            large_file_disable_smart_highlight: true,
            recent_files: vec!["C:\\a.txt".to_string(), "C:\\b.md".to_string()],
            zoom_level: 3,
            editor_font_name: "Consolas".to_string(),
            editor_font_size: 11,
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("\"tab_placement\": \"right\""));
        assert!(json.contains("\"recent_files\""));
        assert!(json.contains("\"vertical_tab_width_px\": 320"));
        assert!(json.contains("\"editor_dark\": false"));
        assert!(json.contains("\"smart_highlight_enabled\": false"));
        assert!(json.contains("\"large_file_threshold_mb\": 50"));
        assert!(json.contains("\"large_file_disable_word_wrap\": true"));
        assert!(json.contains("\"large_file_disable_smart_highlight\": true"));
        assert!(json.contains("\"zoom_level\": 3"));

        let parsed: UiSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, settings);
    }

    #[test]
    fn load_settings_defaults_when_missing_file() {
        with_temp_local_appdata(|| {
            let settings = load_settings().unwrap();
            assert_eq!(settings, UiSettings::default());
        });
    }

    #[test]
    fn save_and_load_settings_roundtrip() {
        with_temp_local_appdata(|| {
            let settings = UiSettings {
                tab_placement: TabPlacement::Left,
                vertical_tab_width_px: 240,
                ..UiSettings::default()
            };
            save_settings(&settings).unwrap();
            let loaded = load_settings().unwrap();
            assert_eq!(loaded, settings);
        });
    }

    #[test]
    fn load_clamps_vertical_width_from_file() {
        with_temp_local_appdata(|| {
            let path = settings_file_path().unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                r#"{
  "tab_placement": "left",
  "vertical_tab_width_px": 9999,
  "large_file_threshold_mb": 0
}"#,
            )
            .unwrap();
            let loaded = load_settings().unwrap();
            assert_eq!(loaded.tab_placement, TabPlacement::Left);
            assert_eq!(loaded.vertical_tab_width_px, MAX_VERTICAL_TAB_WIDTH_PX);
            assert_eq!(loaded.editor_dark, DEFAULT_EDITOR_DARK);
            assert_eq!(loaded.large_file_threshold_mb, MIN_LARGE_FILE_THRESHOLD_MB);
        });
    }

    #[test]
    fn load_clamps_zoom_from_file() {
        with_temp_local_appdata(|| {
            let path = settings_file_path().unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, r#"{ "zoom_level": 99 }"#).unwrap();
            let loaded = load_settings().unwrap();
            assert_eq!(loaded.zoom_level, MAX_ZOOM_LEVEL);
        });
    }

    #[test]
    fn load_accepts_legacy_large_file_setting_keys() {
        with_temp_local_appdata(|| {
            let path = settings_file_path().unwrap();
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(
                &path,
                r#"{
  "large_file_threshold_mb": 32,
  "large_file_disable_word_wrap_globally": true,
  "large_file_allow_smart_highlight": true
}"#,
            )
            .unwrap();
            let loaded = load_settings().unwrap();
            assert_eq!(loaded.large_file_threshold_mb, 32);
            assert!(loaded.large_file_disable_word_wrap);
            assert!(!loaded.large_file_disable_smart_highlight);
        });
    }
}
