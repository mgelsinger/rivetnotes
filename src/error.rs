use std::fmt;

#[derive(Debug)]
pub struct AppError {
    message: String,
}

impl AppError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn win32(context: &str) -> Self {
        let err = windows::core::Error::from_win32();
        Self::new(format!("{context}: {err}"))
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<windows::core::Error> for AppError {
    fn from(value: windows::core::Error) -> Self {
        Self::new(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
