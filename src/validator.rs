// SPDX-License-Identifier: GPL-3.0-or-later
// rget - A safe, modern downloader for Linux
// Copyright (C) 2026  Aeon Ennoia
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.


use url::{Url, Host};
use crate::error::{RgetError, Result};
use regex::Regex;
use urlencoding::decode;

/// Custom sanitization: block dangerous patterns and characters
fn sanitize_url(url_str: &str) -> Result<String> {
    // 1. Block dangerous characters (command injection)
    let dangerous_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>'];
    if url_str.chars().any(|c| dangerous_chars.contains(&c)) {
        return Err(RgetError::InvalidUrl(
            "Blocked: URL contains dangerous characters".to_string()
        ));
    }

    // 2. Block path traversal attempts
    if url_str.contains("../") || url_str.contains("..\\") {
        return Err(RgetError::InvalidUrl(
            "Blocked: Path traversal detected".to_string()
        ));
    }

    // 3. Block sensitive file access patterns
    let sensitive_patterns = [
        r"(?i)/etc/passwd",
        r"(?i)/etc/shadow",
        r"(?i)/etc/sudoers",
        r"(?i)\.env",
        r"(?i)\.git/config",
        r"(?i)\.aws/credentials",
        r"(?i)\.ssh/id_rsa",
        r"(?i)\.ssh/authorized_keys",
        r"(?i)\.bashrc",
        r"(?i)\.zshrc",
    ];
    for pattern in sensitive_patterns {
        let re = Regex::new(pattern).unwrap();
        if re.is_match(url_str) {
            return Err(RgetError::InvalidUrl(
                format!("Blocked: Access to sensitive file pattern '{}'", pattern)
            ));
        }
    }

    // 4. Block encoded injection attempts (basic)
    let decoded = decode(url_str).unwrap_or_else(|_| url_str.into());
    if decoded.contains(';') || decoded.contains('|') || decoded.contains('&') {
        return Err(RgetError::InvalidUrl(
            "Blocked: Encoded dangerous characters detected".to_string()
        ));
    }

    Ok(url_str.to_string())
}

pub fn validate_url(url_str: &str) -> Result<Url> {
    // --- 1. CUSTOM SANITIZATION ---
    let clean_url_str = sanitize_url(url_str)?;

    // --- 2. STANDARD URL VALIDATION ---
    let url = Url::parse(&clean_url_str)
        .map_err(|_| RgetError::InvalidUrl(clean_url_str.clone()))?;

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
        
        // localhost (IPv4 and IPv6)
        if host_str == "localhost" 
            || host_str == "127.0.0.1" 
            || host_str == "::1"
        {
            return Err(RgetError::BlockedUrl(format!(
                "Local/private IP addresses are blocked: {}",
                host_str
            )));
        }

        // Private IPv4 ranges
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

        // IPv6 private ranges
        if let Host::Ipv6(ip) = host {
            if ip == std::net::Ipv6Addr::LOCALHOST {
                return Err(RgetError::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Unique local addresses (fc00::/7)
            if ip.segments()[0] & 0xfe00 == 0xfc00 {
                return Err(RgetError::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Link-local (fe80::/10)
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
