use url::{Url, Host};
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
    if let Some(host) = url.host() {
        let host_str = host.to_string();
        
        // Check for localhost (IPv4 and IPv6)
        if host_str == "localhost" 
            || host_str == "127.0.0.1" 
            || host_str == "::1"
        {
            return Err(RgetError::BlockedUrl(format!(
                "Local/private IP addresses are blocked: {}",
                host_str
            )));
        }

        // Check for private IPv4 ranges
        if host_str.starts_with("192.168.")
            || host_str.starts_with("10.")
            || host_str.starts_with("172.16.")
            || host_str.starts_with("172.17.")
            || host_str.starts_with("172.18.")
            || host_str.starts_with("172.19.")
            || host_str.starts_with("172.20.")
            || host_str.starts_with("172.21.")
            || host_str.starts_with("172.22.")
            || host_str.starts_with("172.23.")
            || host_str.starts_with("172.24.")
            || host_str.starts_with("172.25.")
            || host_str.starts_with("172.26.")
            || host_str.starts_with("172.27.")
            || host_str.starts_with("172.28.")
            || host_str.starts_with("172.29.")
            || host_str.starts_with("172.30.")
            || host_str.starts_with("172.31.")
        {
            return Err(RgetError::BlockedUrl(format!(
                "Local/private IP addresses are blocked: {}",
                host_str
            )));
        }

        // Check for IPv6 loopback and private ranges (::1 is already checked above)
        // We can use the Host enum to properly detect IPv6
        if let Host::Ipv6(ip) = host {
            // Check for loopback (::1)
            if ip == std::net::Ipv6Addr::LOCALHOST {
                return Err(RgetError::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Check for unique local addresses (fc00::/7) - private IPv6 range
            if ip.segments()[0] & 0xfe00 == 0xfc00 {
                return Err(RgetError::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Check for link-local (fe80::/10)
            if ip.segments()[0] & 0xffc0 == 0xfe80 {
                return Err(RgetError::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
        }
    }

    Ok(url)
}
