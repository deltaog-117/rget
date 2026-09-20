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

use url::Url;

pub fn validate_url(url_str: &str) -> Result<Url> {
    // --- 1. CUSTOM SANITIZATION ---
    let clean_url_str = sanitize::sanitize_url(url_str)?;

    // --- 2. STANDARD URL VALIDATION ---
    let url = Url::parse(&clean_url_str)
        .map_err(|_| Error::InvalidUrl(clean_url_str.clone()))?;

    // Only allow HTTP/HTTPS
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(Error::InvalidUrl(format!(
            "Only HTTP/HTTPS supported, got: {}",
            scheme
        )));
    }

    // --- 3. HOST CHECKS ---
    host::ensure_public(&url)?;

    Ok(url)
}
