use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
#[serde(tag = "code", content = "message")]
pub enum AppError {
    // Audio / Whisper Errors
    AudioDeviceNotFound(String),
    AudioPermissionDenied(String),
    AudioBusy(String),
    AudioTooShort,
    AudioTooQuiet,
    ModelNotFound,
    ModelCorrupt,
    TranscriptionFailed(String),
    
    // Database / IO Errors
    DatabaseError(String),
    FileSystemError(String),
    
    // Generic
    Unknown(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for AppError {}

// Helper to map external errors easily
impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::FileSystemError(err.to_string())
    }
}

// Allow returning AppError from Tauri commands
impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        // Serialize to JSON string so frontend can parse { code: "...", message: "..." }
        serde_json::to_string(&err).unwrap_or_else(|_| "{\"code\":\"Unknown\",\"message\":\"Serialization failed\"}".to_string())
    }
}

