use std::fs::File;
use std::io::Read;
use sha2::{Sha256, Digest};

pub fn compute_sha256(file_path: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn verify_sha256(file_path: &str, expected: &str) -> Result<(), super::error::RgetError> {
    let actual = compute_sha256(file_path)
        .map_err(super::error::RgetError::Io)?;
    if actual == expected {
        Ok(())
    } else {
        Err(super::error::RgetError::ChecksumMismatch {
            expected: expected.to_string(),
            actual,
        })
    }
}
