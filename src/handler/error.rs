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
}
