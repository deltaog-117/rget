use url::Url;
use crate::error::{RgetError, Result};

pub fn validate_url(url_str: &str) -> Result<Url> {
    let url = Url::parse(url_str)
        .map_err(|_| RgetError::InvalidUrl(url_str.to_string()))?;

    // Only allow HTTP/HTTPS
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(RgetError::InvalidUrl(format!(
            "Only HTTP/HTTPS supported, got: {}",
            scheme
        )));
    }

    // Block localhost and private IPs
    if let Some(host) = url.host_str() {
        if host == "localhost" 
            || host == "127.0.0.1" 
            || host == "::1"
            || host.starts_with("192.168.")
            || host.starts_with("10.")
            || host.starts_with("172.16.")
            || host.starts_with("172.17.")
            || host.starts_with("172.18.")
            || host.starts_with("172.19.")
            || host.starts_with("172.20.")
            || host.starts_with("172.21.")
            || host.starts_with("172.22.")
            || host.starts_with("172.23.")
            || host.starts_with("172.24.")
            || host.starts_with("172.25.")
            || host.starts_with("172.26.")
            || host.starts_with("172.27.")
            || host.starts_with("172.28.")
            || host.starts_with("172.29.")
            || host.starts_with("172.30.")
            || host.starts_with("172.31.")
        {
            return Err(RgetError::BlockedUrl(format!(
                "Local/private IP addresses are blocked: {}",
                host
            )));
        }
    }

    Ok(url)
}
