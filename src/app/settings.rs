use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::session;
use crate::error::{AppError, Result};
use crate::storage::atomic_write::atomic_write_json;

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
pub const DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP_GLOBALLY: bool = false;
pub const DEFAULT_LARGE_FILE_ALLOW_SMART_HIGHLIGHT: bool = false;

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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    #[serde(default = "default_large_file_threshold_mb")]
    pub large_file_threshold_mb: u32,
    #[serde(default = "default_large_file_disable_word_wrap_globally")]
    pub large_file_disable_word_wrap_globally: bool,
    #[serde(default = "default_large_file_allow_smart_highlight")]
    pub large_file_allow_smart_highlight: bool,
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
            large_file_disable_word_wrap_globally: DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP_GLOBALLY,
            large_file_allow_smart_highlight: DEFAULT_LARGE_FILE_ALLOW_SMART_HIGHLIGHT,
        }
    }
}

impl UiSettings {
    fn normalized(mut self) -> Self {
        self.vertical_tab_width_px = self
            .vertical_tab_width_px
            .clamp(MIN_VERTICAL_TAB_WIDTH_PX, MAX_VERTICAL_TAB_WIDTH_PX);
        self.large_file_threshold_mb = self
            .large_file_threshold_mb
            .clamp(MIN_LARGE_FILE_THRESHOLD_MB, MAX_LARGE_FILE_THRESHOLD_MB);
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
    atomic_write_json(&path, &settings.normalized())
        .map_err(|err| AppError::new(format!("Failed to write settings file atomically: {err}")))
}

fn ensure_settings_dir() -> Result<()> {
    let dir = session::data_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|err| AppError::new(format!("Failed to create settings directory: {err}")))
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

fn default_smart_highlight_match_case() -> bool {
    DEFAULT_SMART_HIGHLIGHT_MATCH_CASE
}

fn default_smart_highlight_whole_word() -> bool {
    DEFAULT_SMART_HIGHLIGHT_WHOLE_WORD
}

fn default_large_file_threshold_mb() -> u32 {
    DEFAULT_LARGE_FILE_THRESHOLD_MB
}

fn default_large_file_disable_word_wrap_globally() -> bool {
    DEFAULT_LARGE_FILE_DISABLE_WORD_WRAP_GLOBALLY
}

fn default_large_file_allow_smart_highlight() -> bool {
    DEFAULT_LARGE_FILE_ALLOW_SMART_HIGHLIGHT
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
            large_file_disable_word_wrap_globally: true,
            large_file_allow_smart_highlight: true,
        };
        let json = serde_json::to_string_pretty(&settings).unwrap();
        assert!(json.contains("\"tab_placement\": \"right\""));
        assert!(json.contains("\"vertical_tab_width_px\": 320"));
        assert!(json.contains("\"editor_dark\": false"));
        assert!(json.contains("\"smart_highlight_enabled\": false"));
        assert!(json.contains("\"large_file_threshold_mb\": 50"));

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
}
