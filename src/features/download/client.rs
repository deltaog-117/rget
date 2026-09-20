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


//! HTTP client construction and request decoration.

use super::error::{Error, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::USER_AGENT;
use std::time::Duration;

/// Builds a blocking client with the given timeout and redirect policy.
///
/// `timeout` bounds connecting and receiving the response headers as a whole. While the
/// body is read with `Read::read`, the same duration applies afresh to *each* read, so
/// it acts as a stall limit and never caps the total transfer time.
pub(super) fn build(timeout: u64, follow_redirects: bool) -> Result<Client> {
    let mut client_builder = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(10));

    if !follow_redirects {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
    }

    Ok(client_builder.build()?)
}

/// Adds the User-Agent and every custom header to `request_builder`.
pub(super) fn apply_headers(
    mut request_builder: RequestBuilder,
    user_agent: Option<&str>,
    headers: &[(String, String)],
) -> RequestBuilder {
    if let Some(ua) = user_agent {
        request_builder = request_builder.header(USER_AGENT, ua);
    } else {
        request_builder = request_builder.header(USER_AGENT, "rget/0.1.0");
    }

    // Apply custom headers
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }

    request_builder
}

/// Turns any non-2xx response into [`Error::HttpStatus`] so that an error page is never
/// written to disk as if it were the file.
pub(super) fn ensure_success(response: Response) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(Error::HttpStatus(status))
    }
}
