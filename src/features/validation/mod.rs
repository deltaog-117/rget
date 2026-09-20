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


//! Validation: is this URL safe to fetch?

mod error;
mod host;
mod sanitize;

pub use error::{Error, Result};

use crate::shared::address::HostPolicy;
use url::Url;

/// Checks that `url_str` is a URL rget is willing to fetch and returns it parsed, refusing
/// local and private hosts.
///
/// See [`validate_url_with`] for the rules and the errors.
pub fn validate_url(url_str: &str) -> Result<Url> {
    validate_url_with(url_str, HostPolicy::BlockPrivate)
}

/// Checks that `url_str` is a URL rget is willing to fetch and returns it parsed.
///
/// The URL must parse, use `http` or `https`, pass the content checks (a plausible hostname,
/// no path traversal, no well-known secret file) and, unless `policy` allows private hosts,
/// not point at a loopback, private or otherwise non-public address.
///
/// # Errors
///
/// [`Error::InvalidUrl`] for a malformed or unsafe URL, [`Error::BlockedUrl`] for a local or
/// private host.
pub fn validate_url_with(url_str: &str, policy: HostPolicy) -> Result<Url> {
    let url = Url::parse(url_str).map_err(|_| Error::InvalidUrl(url_str.to_string()))?;

    // Only allow HTTP/HTTPS
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(Error::InvalidUrl(format!(
            "Only HTTP/HTTPS supported, got: {}",
            scheme
        )));
    }

    sanitize::check(url_str, &url)?;
    host::ensure_public(&url, policy)?;

    Ok(url)
}
