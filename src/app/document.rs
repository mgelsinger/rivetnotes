use std::path::{Path, PathBuf};
use std::time::SystemTime;

use uuid::Uuid;

use crate::error::{AppError, Result};

pub const LARGE_FILE_THRESHOLD: u64 = 100 * 1024 * 1024;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Eol {
    Crlf,
    Lf,
}

#[derive(Clone, Debug)]
pub struct FileStamp {
    pub modified: SystemTime,
    pub size: u64,
}

impl FileStamp {
    pub fn from_path(path: &Path) -> Result<Self> {
        let meta = std::fs::metadata(path)
            .map_err(|err| AppError::new(format!("Failed to read metadata: {err}")))?;
        let modified = meta
            .modified()
            .map_err(|err| AppError::new(format!("Failed to read modified time: {err}")))?;
        Ok(Self {
            modified,
            size: meta.len(),
        })
    }
}

#[derive(Debug)]
pub struct Document {
    pub id: Uuid,
    pub path: Option<PathBuf>,
    pub display_name: String,
    pub is_dirty: bool,
    pub backup_path: PathBuf,
    pub first_backup_write: Option<SystemTime>,
    pub last_backup_write: Option<SystemTime>,
    pub cursor_pos: i64,
    pub scroll_pos: i64,
    pub encoding_hint: Option<TextEncoding>,
    pub encoding: TextEncoding,
    pub eol: Eol,
    pub stamp: Option<FileStamp>,
    pub large_file_mode: bool,
}

impl Document {
    pub fn new_empty() -> Self {
        Self::with_id(Uuid::new_v4())
    }

    pub fn with_id(id: Uuid) -> Self {
        Self {
            id,
            path: None,
            display_name: "new 001".to_string(),
            is_dirty: false,
            backup_path: PathBuf::new(),
            first_backup_write: None,
            last_backup_write: None,
            cursor_pos: 0,
            scroll_pos: 0,
            encoding_hint: None,
            encoding: TextEncoding::Utf8,
            eol: Eol::Crlf,
            stamp: None,
            large_file_mode: false,
        }
    }

    pub fn update_from_load(
        &mut self,
        path: PathBuf,
        encoding: TextEncoding,
        eol: Eol,
        stamp: FileStamp,
        large_file_mode: bool,
    ) {
        self.path = Some(path);
        self.encoding = encoding;
        self.encoding_hint = Some(encoding);
        self.eol = eol;
        self.stamp = Some(stamp);
        self.large_file_mode = large_file_mode;
        self.is_dirty = false;
    }

    pub fn update_after_save(&mut self, encoding: TextEncoding, eol: Eol, stamp: FileStamp) {
        self.encoding = encoding;
        self.encoding_hint = Some(encoding);
        self.eol = eol;
        self.stamp = Some(stamp);
        self.is_dirty = false;
    }
}

pub fn is_large_file(size: u64) -> bool {
    size >= LARGE_FILE_THRESHOLD
}

pub fn detect_eol(text: &str) -> Eol {
    if let Some(index) = text.find('\n') {
        if index > 0 && text.as_bytes()[index - 1] == b'\r' {
            return Eol::Crlf;
        }
        return Eol::Lf;
    }
    Eol::Crlf
}

pub fn normalize_eol(text: &str, eol: Eol) -> String {
    match eol {
        Eol::Crlf => {
            let mut out = String::with_capacity(text.len());
            let mut chars = text.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '\r' {
                    if let Some('\n') = chars.peek().copied() {
                        chars.next();
                    }
                    out.push('\r');
                    out.push('\n');
                } else if ch == '\n' {
                    out.push('\r');
                    out.push('\n');
                } else {
                    out.push(ch);
                }
            }
            out
        }
        Eol::Lf => {
            let mut out = String::with_capacity(text.len());
            let mut chars = text.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '\r' {
                    if let Some('\n') = chars.peek().copied() {
                        chars.next();
                    }
                    out.push('\n');
                } else {
                    out.push(ch);
                }
            }
            out
        }
    }
}

pub fn decode_bytes(bytes: &[u8]) -> Result<(String, TextEncoding)> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let text = std::str::from_utf8(&bytes[3..])
            .map_err(|err| AppError::new(format!("Invalid UTF-8 BOM text: {err}")))?;
        return Ok((text.to_string(), TextEncoding::Utf8Bom));
    }

    if bytes.starts_with(&[0xFF, 0xFE]) {
        return decode_utf16(&bytes[2..], true).map(|text| (text, TextEncoding::Utf16Le));
    }

    if bytes.starts_with(&[0xFE, 0xFF]) {
        return decode_utf16(&bytes[2..], false).map(|text| (text, TextEncoding::Utf16Be));
    }

    let text = std::str::from_utf8(bytes)
        .map_err(|err| AppError::new(format!("Unsupported encoding (not UTF-8): {err}")))?;
    Ok((text.to_string(), TextEncoding::Utf8))
}

pub fn encode_text(text: &str, encoding: TextEncoding) -> Result<Vec<u8>> {
    match encoding {
        TextEncoding::Utf8 => Ok(text.as_bytes().to_vec()),
        TextEncoding::Utf8Bom => {
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(text.as_bytes());
            Ok(bytes)
        }
        TextEncoding::Utf16Le => Ok(encode_utf16(text, true)),
        TextEncoding::Utf16Be => Ok(encode_utf16(text, false)),
    }
}

pub fn check_stamp(path: &Path, stamp: &Option<FileStamp>) -> Result<Option<FileStamp>> {
    let new_stamp = FileStamp::from_path(path)?;
    let changed = match stamp {
        Some(old) => old.modified != new_stamp.modified || old.size != new_stamp.size,
        None => true,
    };
    if changed {
        Ok(Some(new_stamp))
    } else {
        Ok(None)
    }
}

fn decode_utf16(bytes: &[u8], le: bool) -> Result<String> {
    if !bytes.len().is_multiple_of(2) {
        return Err(AppError::new("Invalid UTF-16 byte length."));
    }

    let mut words = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        let value = if le {
            u16::from_le_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], chunk[1]])
        };
        words.push(value);
    }

    String::from_utf16(&words).map_err(|err| AppError::new(format!("Invalid UTF-16 text: {err}")))
}

fn encode_utf16(text: &str, le: bool) -> Vec<u8> {
    let mut bytes = if le {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for unit in text.encode_utf16() {
        let pair = if le {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        };
        bytes.extend_from_slice(&pair);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn detect_eol_prefers_crlf() {
        assert_eq!(detect_eol("one\r\ntwo\n"), Eol::Crlf);
    }

    #[test]
    fn detect_eol_lf() {
        assert_eq!(detect_eol("one\ntwo\n"), Eol::Lf);
    }

    #[test]
    fn detect_eol_default_is_crlf() {
        assert_eq!(detect_eol("no newlines"), Eol::Crlf);
    }

    #[test]
    fn normalize_eol_crlf() {
        let input = "one\ntwo\r\nthree\rfour";
        let expected = "one\r\ntwo\r\nthree\r\nfour";
        assert_eq!(normalize_eol(input, Eol::Crlf), expected);
    }

    #[test]
    fn normalize_eol_lf() {
        let input = "one\r\ntwo\rthree\n";
        let expected = "one\ntwo\nthree\n";
        assert_eq!(normalize_eol(input, Eol::Lf), expected);
    }

    #[test]
    fn encode_decode_utf8_roundtrip() {
        let text = "hello";
        let bytes = encode_text(text, TextEncoding::Utf8).unwrap();
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);
        assert_eq!(encoding, TextEncoding::Utf8);
    }

    #[test]
    fn encode_decode_utf8_bom_roundtrip() {
        let text = "hello";
        let bytes = encode_text(text, TextEncoding::Utf8Bom).unwrap();
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);
        assert_eq!(encoding, TextEncoding::Utf8Bom);
    }

    #[test]
    fn encode_decode_utf16_le_roundtrip() {
        let text = "hello";
        let bytes = encode_text(text, TextEncoding::Utf16Le).unwrap();
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);
        assert_eq!(encoding, TextEncoding::Utf16Le);
    }

    #[test]
    fn encode_decode_utf16_be_roundtrip() {
        let text = "hello";
        let bytes = encode_text(text, TextEncoding::Utf16Be).unwrap();
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(decoded, text);
        assert_eq!(encoding, TextEncoding::Utf16Be);
    }

    #[test]
    fn decode_utf16_invalid_length() {
        let bytes = [0xFF, 0xFE, 0x00];
        assert!(decode_bytes(&bytes).is_err());
    }

    #[test]
    fn large_file_threshold() {
        assert!(is_large_file(LARGE_FILE_THRESHOLD));
        assert!(!is_large_file(LARGE_FILE_THRESHOLD - 1));
    }

    #[test]
    fn check_stamp_detects_changes() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "hello").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        let stamp = FileStamp::from_path(&path).unwrap();
        assert!(check_stamp(&path, &Some(stamp.clone())).unwrap().is_none());

        write!(file, "world").unwrap();
        file.flush().unwrap();
        assert!(check_stamp(&path, &Some(stamp)).unwrap().is_some());
    }
}
