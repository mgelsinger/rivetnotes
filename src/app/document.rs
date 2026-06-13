use std::path::{Path, PathBuf};
use std::time::SystemTime;

use uuid::Uuid;

use crate::error::{AppError, Result};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    /// Windows-1252 ("ANSI"). The fallback for legacy files that are neither
    /// valid UTF-8 nor BOM-tagged UTF-16.
    Ansi,
}

impl TextEncoding {
    /// Short label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            TextEncoding::Utf8 => "UTF-8",
            TextEncoding::Utf8Bom => "UTF-8 BOM",
            TextEncoding::Utf16Le => "UTF-16 LE",
            TextEncoding::Utf16Be => "UTF-16 BE",
            TextEncoding::Ansi => "ANSI",
        }
    }
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

pub fn large_file_threshold_bytes(threshold_mb: u32) -> u64 {
    (threshold_mb as u64).saturating_mul(1024 * 1024)
}

pub fn is_large_file_size(threshold_mb: u32, size_bytes: u64) -> bool {
    size_bytes >= large_file_threshold_bytes(threshold_mb)
}

pub fn encoded_size_for_text(text: &str, encoding: TextEncoding) -> u64 {
    match encoding {
        TextEncoding::Utf8 => text.len() as u64,
        TextEncoding::Utf8Bom => text.len() as u64 + 3,
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            2 + (text.encode_utf16().count() as u64).saturating_mul(2)
        }
        // One byte per char when representable; an upper-bound estimate that
        // matches the real size for any text that can actually be saved as ANSI.
        TextEncoding::Ansi => text.chars().count() as u64,
    }
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

    // No BOM: prefer UTF-8, but fall back to Windows-1252 so legacy ANSI files
    // open instead of erroring. CP1252 maps every byte, so this never fails.
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok((text.to_string(), TextEncoding::Utf8)),
        Err(_) => Ok((decode_cp1252(bytes), TextEncoding::Ansi)),
    }
}

/// Windows-1252 mappings for bytes 0x80–0x9F. The five positions undefined in
/// CP1252 (0x81, 0x8D, 0x8F, 0x90, 0x9D) map to the matching C1 control code
/// point so decoding is lossless and reversible.
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

fn decode_cp1252(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        let code = match b {
            0x80..=0x9F => CP1252_HIGH[(b - 0x80) as usize] as u32,
            other => other as u32,
        };
        // Every mapped value is a valid non-surrogate scalar.
        out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
    }
    out
}

fn encode_cp1252(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len());
    for ch in text.chars() {
        let code = ch as u32;
        let byte = if code <= 0x7F || (0xA0..=0xFF).contains(&code) {
            code as u8
        } else if let Some(idx) = CP1252_HIGH.iter().position(|&c| u32::from(c) == code) {
            0x80 + idx as u8
        } else {
            return Err(AppError::new(format!(
                "Character '{ch}' (U+{code:04X}) cannot be saved as ANSI. Use Save As (UTF-8)."
            )));
        };
        out.push(byte);
    }
    Ok(out)
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
        TextEncoding::Ansi => encode_cp1252(text),
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
    fn non_utf8_bytes_fall_back_to_ansi() {
        // 0xE9 is 'é' in Windows-1252 but an invalid lone UTF-8 lead byte.
        let bytes = [b'c', b'a', b'f', b'\xE9'];
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(encoding, TextEncoding::Ansi);
        assert_eq!(decoded, "café");
    }

    #[test]
    fn ansi_roundtrips_smart_punctuation() {
        // “quoted” — em dash, smart quotes, euro, trademark all live in 0x80–0x9F.
        let text = "\u{201C}quoted\u{201D} \u{2014} \u{20AC}99 \u{2122}";
        let bytes = encode_text(text, TextEncoding::Ansi).unwrap();
        // Each character is exactly one CP1252 byte.
        assert_eq!(bytes.len(), text.chars().count());
        let (decoded, encoding) = decode_bytes(&bytes).unwrap();
        assert_eq!(encoding, TextEncoding::Ansi);
        assert_eq!(decoded, text);
    }

    #[test]
    fn ansi_encode_rejects_unrepresentable_char() {
        // An emoji has no CP1252 byte; saving as ANSI must fail loudly.
        assert!(encode_text("hi \u{1F600}", TextEncoding::Ansi).is_err());
    }

    #[test]
    fn ansi_full_byte_range_roundtrips() {
        let bytes: Vec<u8> = (0u8..=255).collect();
        let decoded = decode_cp1252(&bytes);
        let reencoded = encode_cp1252(&decoded).unwrap();
        assert_eq!(reencoded, bytes);
    }

    #[test]
    fn large_file_threshold() {
        assert_eq!(large_file_threshold_bytes(100), 100 * 1024 * 1024);
        assert!(is_large_file_size(100, 100 * 1024 * 1024));
        assert!(!is_large_file_size(100, (100 * 1024 * 1024) - 1));
    }

    #[test]
    fn encoded_size_matches_encoding() {
        assert_eq!(encoded_size_for_text("hello", TextEncoding::Utf8), 5);
        assert_eq!(encoded_size_for_text("hello", TextEncoding::Utf8Bom), 8);
        assert_eq!(encoded_size_for_text("hello", TextEncoding::Utf16Le), 12);
        assert_eq!(encoded_size_for_text("hello", TextEncoding::Utf16Be), 12);
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
