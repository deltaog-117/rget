use thiserror::Error;

#[derive(Error, Debug)]
pub enum RgetError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Timeout exceeded: {0} seconds")]
    Timeout(u64),
    
    #[error("Blocked URL: {0}")]
    BlockedUrl(String),
}

pub type Result<T> = std::result::Result<T, RgetError>;
