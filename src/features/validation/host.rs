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


//! Host-level SSRF guard: refuse loopback and private addresses.

use super::error::{Error, Result};
use url::{Host, Url};

/// Rejects localhost and private/link-local hosts.
pub(super) fn ensure_public(url: &Url) -> Result<()> {
    // Block localhost and private IPs
    if let Some(host) = url.host() {
        let host_str = host.to_string();

        // localhost (IPv4 and IPv6)
        if host_str == "localhost"
            || host_str == "127.0.0.1"
            || host_str == "::1"
        {
            return Err(Error::BlockedUrl(format!(
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
            return Err(Error::BlockedUrl(format!(
                "Local/private IP addresses are blocked: {}",
                host_str
            )));
        }

        // IPv6 private ranges
        if let Host::Ipv6(ip) = host {
            if ip == std::net::Ipv6Addr::LOCALHOST {
                return Err(Error::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Unique local addresses (fc00::/7)
            if ip.segments()[0] & 0xfe00 == 0xfc00 {
                return Err(Error::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
            // Link-local (fe80::/10)
            if ip.segments()[0] & 0xffc0 == 0xfe80 {
                return Err(Error::BlockedUrl(format!(
                    "Local/private IP addresses are blocked: {}",
                    host_str
                )));
            }
        }
    }

    Ok(())
}
