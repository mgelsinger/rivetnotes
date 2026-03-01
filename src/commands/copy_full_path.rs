use std::fmt;
use std::path::{MAIN_SEPARATOR, Path};

use crate::platform::clipboard::{Clipboard, ClipboardError};

#[cfg(test)]
use crate::platform::clipboard::TestClipboard;

#[derive(Debug)]
pub enum Error {
    Clipboard(ClipboardError),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CopyPathKind {
    FullPath,
    FileName,
    DirectoryPath,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clipboard(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ClipboardError> for Error {
    fn from(value: ClipboardError) -> Self {
        Self::Clipboard(value)
    }
}

pub fn can_copy_full_path(doc_path: Option<&Path>) -> bool {
    doc_path.is_some()
}

pub fn can_copy_filename(doc_path: Option<&Path>) -> bool {
    doc_path.and_then(|path| path.file_name()).is_some()
}

pub fn can_copy_directory_path(doc_path: Option<&Path>) -> bool {
    doc_path.and_then(|path| path.parent()).is_some()
}

pub fn copy_full_path(
    doc_path: Option<&Path>,
    clipboard: &mut dyn Clipboard,
) -> Result<bool, Error> {
    copy_path(doc_path, clipboard, CopyPathKind::FullPath)
}

pub fn copy_filename(
    doc_path: Option<&Path>,
    clipboard: &mut dyn Clipboard,
) -> Result<bool, Error> {
    copy_path(doc_path, clipboard, CopyPathKind::FileName)
}

pub fn copy_directory_path(
    doc_path: Option<&Path>,
    clipboard: &mut dyn Clipboard,
) -> Result<bool, Error> {
    copy_path(doc_path, clipboard, CopyPathKind::DirectoryPath)
}

pub fn copy_path(
    doc_path: Option<&Path>,
    clipboard: &mut dyn Clipboard,
    kind: CopyPathKind,
) -> Result<bool, Error> {
    let Some(text) = copy_text(doc_path, kind) else {
        return Ok(false);
    };
    clipboard.set_unicode_text(&text)?;
    Ok(true)
}

fn copy_text(doc_path: Option<&Path>, kind: CopyPathKind) -> Option<String> {
    let path = doc_path?;
    match kind {
        CopyPathKind::FullPath => Some(path.to_string_lossy().to_string()),
        CopyPathKind::FileName => path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        CopyPathKind::DirectoryPath => path.parent().map(directory_with_trailing_separator),
    }
}

fn directory_with_trailing_separator(path: &Path) -> String {
    let mut text = path.to_string_lossy().to_string();
    if !text.ends_with('\\') && !text.ends_with('/') {
        text.push(MAIN_SEPARATOR);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn copy_full_path_no_path_returns_false_and_does_not_touch_clipboard() {
        let mut clipboard = TestClipboard::default();
        let copied = copy_full_path(None, &mut clipboard).unwrap();
        assert!(!copied);
        assert_eq!(clipboard.last_text(), None);
    }

    #[test]
    fn copy_full_path_saved_file_copies_exact_unicode_path() {
        let mut clipboard = TestClipboard::default();
        let path = Path::new(r"C:\x\日本語\y.txt");
        let copied = copy_full_path(Some(path), &mut clipboard).unwrap();
        assert!(copied);
        assert_eq!(
            clipboard.last_text(),
            Some(path.to_string_lossy().to_string())
        );
    }

    #[test]
    fn can_copy_full_path_follows_path_presence() {
        assert!(!can_copy_full_path(None));
        assert!(can_copy_full_path(Some(Path::new(r"C:\x\y.txt"))));
    }

    #[test]
    fn copy_filename_uses_leaf_name() {
        let mut clipboard = TestClipboard::default();
        let path = Path::new(r"C:\x\日本語\y.txt");
        let copied = copy_filename(Some(path), &mut clipboard).unwrap();
        assert!(copied);
        assert_eq!(clipboard.last_text(), Some("y.txt".to_string()));
    }

    #[test]
    fn copy_directory_path_appends_separator() {
        let mut clipboard = TestClipboard::default();
        let path = Path::new(r"C:\x\y.txt");
        let copied = copy_directory_path(Some(path), &mut clipboard).unwrap();
        assert!(copied);
        assert_eq!(
            clipboard.last_text(),
            Some(format!("C:\\x{MAIN_SEPARATOR}"))
        );
    }

    #[test]
    fn can_copy_filename_and_directory_follow_path_shape() {
        assert!(!can_copy_filename(None));
        assert!(!can_copy_directory_path(None));
        assert!(can_copy_filename(Some(Path::new(r"C:\x\y.txt"))));
        assert!(can_copy_directory_path(Some(Path::new(r"C:\x\y.txt"))));
    }
}
