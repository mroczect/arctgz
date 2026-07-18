use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArctgzError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Project already initialized (arctgz.init exists)")]
    AlreadyInitialized,

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Path not allowed: {0}")]
    PathNotAllowed(String),

    #[error("Directory not empty: {0}")]
    DirectoryNotEmpty(String),

    #[error("Config validation failed: {0}")]
    ConfigValidation(String),

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(String),

    #[error("Failed to load configuration: {0}")]
    ConfigLoadError(String),

    #[error("Failed to save configuration: {0}")]
    ConfigSaveError(String),

    #[error("Symlink not allowed: {0}")]
    SymlinkNotAllowed(String),

    #[error("Include file not found: {0}")]
    IncludeFileNotFound(String),
    #[error("Manifest not found in archive")]
    ManifestNotFound,

    #[error("Checksum mismatch for file {0}: expected {1}, got {2}")]
    ChecksumMismatch(String, String, String),

    #[error("Extract error: {0}")]
    ExtractError(String),
}
