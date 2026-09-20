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


//! Character- and pattern-level URL sanitization.

use super::error::{Error, Result};
use regex::Regex;
use urlencoding::decode;

/// Custom sanitization: block dangerous patterns and characters
pub(super) fn sanitize_url(url_str: &str) -> Result<String> {
    // 1. Block dangerous characters (command injection)
    let dangerous_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>'];
    if url_str.chars().any(|c| dangerous_chars.contains(&c)) {
        return Err(Error::InvalidUrl(
            "Blocked: URL contains dangerous characters".to_string()
        ));
    }

    // 2. Block path traversal attempts
    if url_str.contains("../") || url_str.contains("..\\") {
        return Err(Error::InvalidUrl(
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
            return Err(Error::InvalidUrl(
                format!("Blocked: Access to sensitive file pattern '{}'", pattern)
            ));
        }
    }

    // 4. Block encoded injection attempts (basic)
    let decoded = decode(url_str).unwrap_or_else(|_| url_str.into());
    if decoded.contains(';') || decoded.contains('|') || decoded.contains('&') {
        return Err(Error::InvalidUrl(
            "Blocked: Encoded dangerous characters detected".to_string()
        ));
    }

    Ok(url_str.to_string())
}
