use std::path::{Path, PathBuf};

use crate::app::document::{Eol, TextEncoding};
use crate::error::{AppError, Result};

#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub path: PathBuf,
    pub caret: usize,
    pub encoding: TextEncoding,
    pub eol: Eol,
    pub wrap: bool,
}

#[derive(Debug, Clone)]
pub struct SessionData {
    pub active: usize,
    pub entries: Vec<SessionEntry>,
}

impl SessionData {
    pub fn empty() -> Self {
        Self {
            active: 0,
            entries: Vec::new(),
        }
    }
}

pub fn load_session() -> Result<SessionData> {
    let path = session_path()?;
    if !path.exists() {
        return Ok(SessionData::empty());
    }

    let content = std::fs::read_to_string(&path).map_err(|err| AppError::new(format!("{err}")))?;
    let mut data = SessionData::empty();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(value) = line.strip_prefix("active=") {
            if let Ok(index) = value.parse::<usize>() {
                data.active = index;
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("doc=")
            && let Some(entry) = parse_entry(rest)
        {
            data.entries.push(entry);
        }
    }

    Ok(data)
}

pub fn save_session(data: &SessionData) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| AppError::new(format!("Failed to create session directory: {err}")))?;
    }

    let mut output = String::new();
    output.push_str(&format!("active={}\n", data.active));
    for entry in &data.entries {
        let path = percent_encode(&entry.path.to_string_lossy());
        output.push_str("doc=");
        output.push_str(&path);
        output.push_str(&format!(
            ";caret={};encoding={};eol={};wrap={}\n",
            entry.caret,
            encoding_to_str(entry.encoding),
            eol_to_str(entry.eol),
            if entry.wrap { 1 } else { 0 }
        ));
    }

    std::fs::write(&path, output)
        .map_err(|err| AppError::new(format!("Failed to write session file: {err}")))?;
    Ok(())
}

fn session_path() -> Result<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        Ok(Path::new(&appdata).join("Rivet").join("session.txt"))
    } else {
        let current = std::env::current_dir()
            .map_err(|err| AppError::new(format!("Failed to get current directory: {err}")))?;
        Ok(current.join("session.txt"))
    }
}

fn parse_entry(rest: &str) -> Option<SessionEntry> {
    let mut path: Option<PathBuf> = None;
    let mut caret = 0usize;
    let mut encoding = TextEncoding::Utf8;
    let mut eol = Eol::Crlf;
    let mut wrap = true;

    for part in rest.split(';') {
        let mut iter = part.splitn(2, '=');
        let key = iter.next()?.trim();
        let value = iter.next().unwrap_or("").trim();
        match key {
            "caret" => {
                if let Ok(parsed) = value.parse::<usize>() {
                    caret = parsed;
                }
            }
            "encoding" => {
                if let Some(parsed) = parse_encoding(value) {
                    encoding = parsed;
                }
            }
            "eol" => {
                if let Some(parsed) = parse_eol(value) {
                    eol = parsed;
                }
            }
            "wrap" => {
                wrap = value == "1";
            }
            _ => {
                if path.is_none()
                    && let Ok(decoded) = percent_decode(key)
                {
                    path = Some(PathBuf::from(decoded));
                }
            }
        }
    }

    path.map(|path| SessionEntry {
        path,
        caret,
        encoding,
        eol,
        wrap,
    })
}

fn encoding_to_str(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::Utf8Bom => "utf8bom",
        TextEncoding::Utf16Le => "utf16le",
        TextEncoding::Utf16Be => "utf16be",
    }
}

fn parse_encoding(value: &str) -> Option<TextEncoding> {
    match value {
        "utf8" => Some(TextEncoding::Utf8),
        "utf8bom" => Some(TextEncoding::Utf8Bom),
        "utf16le" => Some(TextEncoding::Utf16Le),
        "utf16be" => Some(TextEncoding::Utf16Be),
        _ => None,
    }
}

fn eol_to_str(eol: Eol) -> &'static str {
    match eol {
        Eol::Crlf => "crlf",
        Eol::Lf => "lf",
    }
}

fn parse_eol(value: &str) -> Option<Eol> {
    match value {
        "crlf" => Some(Eol::Crlf),
        "lf" => Some(Eol::Lf),
        _ => None,
    }
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for &byte in value.as_bytes() {
        let is_unreserved =
            matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.');
        if is_unreserved {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", byte));
        }
    }
    out
}

fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(AppError::new("Invalid percent encoding."));
            }
            let hex = &value[i + 1..i + 3];
            let parsed = u8::from_str_radix(hex, 16)
                .map_err(|_| AppError::new("Invalid percent encoding."))?;
            out.push(parsed);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|err| AppError::new(format!("Invalid UTF-8 path: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_temp_appdata<F>(action: F)
    where
        F: FnOnce(PathBuf),
    {
        let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.lock().unwrap();
        let temp = TempDir::new().unwrap();
        let original = std::env::var("APPDATA").ok();
        unsafe {
            std::env::set_var("APPDATA", temp.path());
        }

        action(temp.path().to_path_buf());

        if let Some(value) = original {
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
    fn percent_encode_roundtrip() {
        let value = "C:\\Users\\Jane Doe\\notes.txt";
        let encoded = percent_encode(value);
        let decoded = percent_decode(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn parse_entry_reads_fields() {
        let input = "C%3A%5CTest%5Cfile.txt;caret=12;encoding=utf16le;eol=lf;wrap=0";
        let entry = parse_entry(input).unwrap();
        assert_eq!(entry.path, PathBuf::from("C:\\Test\\file.txt"));
        assert_eq!(entry.caret, 12);
        assert_eq!(entry.encoding, TextEncoding::Utf16Le);
        assert_eq!(entry.eol, Eol::Lf);
        assert!(!entry.wrap);
    }

    #[test]
    fn save_load_session_roundtrip() {
        with_temp_appdata(|_root| {
            let data = SessionData {
                active: 1,
                entries: vec![
                    SessionEntry {
                        path: PathBuf::from("C:\\notes\\a.txt"),
                        caret: 3,
                        encoding: TextEncoding::Utf8,
                        eol: Eol::Crlf,
                        wrap: true,
                    },
                    SessionEntry {
                        path: PathBuf::from("C:\\notes\\b.txt"),
                        caret: 7,
                        encoding: TextEncoding::Utf16Le,
                        eol: Eol::Lf,
                        wrap: false,
                    },
                ],
            };

            save_session(&data).unwrap();
            let loaded = load_session().unwrap();

            assert_eq!(loaded.active, 1);
            assert_eq!(loaded.entries.len(), 2);
            assert_eq!(loaded.entries[0].path, data.entries[0].path);
            assert_eq!(loaded.entries[0].caret, data.entries[0].caret);
            assert_eq!(loaded.entries[0].encoding, data.entries[0].encoding);
            assert_eq!(loaded.entries[0].eol, data.entries[0].eol);
            assert_eq!(loaded.entries[0].wrap, data.entries[0].wrap);
            assert_eq!(loaded.entries[1].path, data.entries[1].path);
            assert_eq!(loaded.entries[1].caret, data.entries[1].caret);
            assert_eq!(loaded.entries[1].encoding, data.entries[1].encoding);
            assert_eq!(loaded.entries[1].eol, data.entries[1].eol);
            assert_eq!(loaded.entries[1].wrap, data.entries[1].wrap);
        });
    }
}
