use std::fmt;
use std::mem::size_of;

use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;

#[derive(Debug, Clone)]
pub struct ClipboardError {
    message: String,
}

impl ClipboardError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ClipboardError {}

pub trait Clipboard {
    fn set_unicode_text(&mut self, text: &str) -> Result<(), ClipboardError>;
}

pub struct WinClipboard {
    owner: HWND,
}

impl WinClipboard {
    pub fn new(owner: HWND) -> Self {
        Self { owner }
    }
}

impl Clipboard for WinClipboard {
    fn set_unicode_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0);
        let bytes = utf16.len().saturating_mul(size_of::<u16>());

        let hmem = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }
            .map_err(|err| ClipboardError::new(format!("GlobalAlloc failed: {err}")))?;
        let ptr = unsafe { GlobalLock(hmem) as *mut u16 };
        if ptr.is_null() {
            unsafe {
                let _ = GlobalFree(hmem);
            }
            return Err(ClipboardError::new("GlobalLock failed."));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
            let _ = GlobalUnlock(hmem);
        }

        if let Err(err) = unsafe { OpenClipboard(self.owner) } {
            unsafe {
                let _ = GlobalFree(hmem);
            }
            return Err(ClipboardError::new(format!("OpenClipboard failed: {err}")));
        }

        if let Err(err) = unsafe { EmptyClipboard() } {
            unsafe {
                let _ = CloseClipboard();
                let _ = GlobalFree(hmem);
            }
            return Err(ClipboardError::new(format!("EmptyClipboard failed: {err}")));
        }

        if let Err(err) =
            unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(hmem.0 as isize)) }
        {
            unsafe {
                let _ = CloseClipboard();
                let _ = GlobalFree(hmem);
            }
            return Err(ClipboardError::new(format!(
                "SetClipboardData failed: {err}"
            )));
        }

        if let Err(err) = unsafe { CloseClipboard() } {
            return Err(ClipboardError::new(format!("CloseClipboard failed: {err}")));
        }

        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct TestClipboard {
    last_text: Option<String>,
}

#[cfg(test)]
impl TestClipboard {
    pub fn last_text(&self) -> Option<String> {
        self.last_text.clone()
    }
}

#[cfg(test)]
impl Clipboard for TestClipboard {
    fn set_unicode_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.last_text = Some(text.to_string());
        Ok(())
    }
}
