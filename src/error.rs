use std::fmt;

#[cfg(target_os = "macos")]
use crate::types::*;

#[derive(Debug, Clone)]
pub enum AuError {
    OsStatus { function: &'static str, code: i32 },
    NullComponent,
    InvalidBuffer(String),
}

pub type Result<T> = std::result::Result<T, AuError>;

impl AuError {
    pub fn message(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        {
            let code = match self {
                AuError::OsStatus { code, .. } => *code,
                AuError::NullComponent => return "null component",
                AuError::InvalidBuffer(_) => return "invalid buffer",
            };
            match code {
                K_AUDIO_UNIT_ERR_INVALID_PROPERTY => "invalid property",
                K_AUDIO_UNIT_ERR_INVALID_PARAMETER => "invalid parameter",
                K_AUDIO_UNIT_ERR_INVALID_ELEMENT => "invalid element",
                K_AUDIO_UNIT_ERR_NO_CONNECTION => "no connection",
                K_AUDIO_UNIT_ERR_FAILED_INITIALIZATION => "failed initialization",
                K_AUDIO_UNIT_ERR_TOO_MANY_FRAMES_TO_PROCESS => "too many frames to process",
                K_AUDIO_UNIT_ERR_INVALID_FILE => "invalid file",
                K_AUDIO_UNIT_ERR_UNKNOWN_FILE_TYPE => "unknown file type",
                K_AUDIO_UNIT_ERR_FILE_NOT_SPECIFIED => "file not specified",
                K_AUDIO_UNIT_ERR_FORMAT_NOT_SUPPORTED => "format not supported",
                K_AUDIO_UNIT_ERR_UNINITIALIZED => "uninitialized",
                K_AUDIO_UNIT_ERR_INVALID_SCOPE => "invalid scope",
                K_AUDIO_UNIT_ERR_PROPERTY_NOT_WRITABLE => "property not writable",
                K_AUDIO_UNIT_ERR_CANNOT_DO_IN_CURRENT_CONTEXT => "cannot do in current context",
                K_AUDIO_UNIT_ERR_INVALID_PROPERTY_VALUE => "invalid property value",
                K_AUDIO_UNIT_ERR_PROPERTY_NOT_IN_USE => "property not in use",
                K_AUDIO_UNIT_ERR_INITIALIZED => "already initialized",
                K_AUDIO_UNIT_ERR_INVALID_OFFLINE_RENDER => "invalid offline render",
                K_AUDIO_UNIT_ERR_UNAUTHORIZED => "unauthorized",
                _ => "unknown error",
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            match self {
                AuError::NullComponent => "null component",
                AuError::InvalidBuffer(_) => "invalid buffer",
                _ => "unknown error",
            }
        }
    }
}

impl fmt::Display for AuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuError::OsStatus { function, code } => write!(
                f,
                "AudioUnit error in {}: OSStatus {} ({})",
                function,
                code,
                self.message()
            ),
            AuError::NullComponent => write!(f, "null AudioComponent handle"),
            AuError::InvalidBuffer(msg) => write!(f, "invalid buffer: {msg}"),
        }
    }
}

impl std::error::Error for AuError {}
