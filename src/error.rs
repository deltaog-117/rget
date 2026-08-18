use thiserror::Error;

#[derive(Error, Debug)]
pub enum RgetError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Blocked URL: {0}")]
    BlockedUrl(String),

    #[error("Redirect disabled but server returned redirect {0} to {1}")]
    RedirectDisabled(u16, String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

pub type Result<T> = std::result::Result<T, RgetError>;
