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


//! Host-level SSRF guard: refuse loopback, private and other non-public destinations.
//!
//! Only what the URL itself says can be judged here: an IP literal, or a name that always
//! means the local machine. What other names resolve to, and where redirects lead, is checked
//! while downloading (see the `download` feature).

use super::error::{Error, Result};
use crate::shared::address::{is_local_name, HostPolicy};
use std::net::IpAddr;
use url::{Host, Url};

/// Rejects a URL whose host is a non-public address, unless `policy` allows it.
pub(super) fn ensure_public(url: &Url, policy: HostPolicy) -> Result<()> {
    if !policy.blocks_private() {
        return Ok(());
    }
    let refused = match url.host() {
        Some(Host::Ipv4(ip)) => policy.refuses(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => policy.refuses(IpAddr::V6(ip)),
        Some(Host::Domain(name)) => is_local_name(name),
        None => false,
    };
    if refused {
        // `host()` is `Some` here; its text keeps the brackets of an IPv6 literal.
        let shown = url.host().map(|h| h.to_string()).unwrap_or_default();
        return Err(Error::BlockedUrl(format!(
            "Local/private IP addresses are blocked: {}",
            shown
        )));
    }
    Ok(())
}
