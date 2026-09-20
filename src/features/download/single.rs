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


//! Single-connection download attempt.

use super::client;
use super::error::{Error, Result};
use super::options::DownloadOptions;
use super::resume;
use super::throttle::Throttle;
use crate::shared::progress::ProgressBarWrapper;
use indicatif::MultiProgress;
use reqwest::header::RANGE;
use std::fs::OpenOptions;
use std::io::Write;

/// Performs one download attempt (retries are the caller's concern).
pub(super) fn attempt(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    multi_progress: Option<&MultiProgress>,
) -> Result<()> {
    let quiet = options.quiet;
    let client = client::build(options.timeout, options.follow_redirects)?;

    let existing_size = resume::existing_size(output_path, options.resume);

    let mut request_builder = client.get(url);
    if options.resume && existing_size > 0 {
        request_builder = request_builder.header(RANGE, resume::range_header(existing_size));
    }
    let request_builder =
        client::apply_headers(request_builder, options.user_agent.as_deref(), &options.headers);

    let response = request_builder.send()?;

    if !options.follow_redirects && response.status().is_redirection() {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        return Err(Error::RedirectDisabled(status, location.to_string()));
    }

    if options.resume && existing_size > 0 {
        resume::handle_response(response.status(), existing_size, output_path, quiet)?;
    }

    let total_size = response.content_length().unwrap_or(0);

    let mut file = if options.resume && existing_size > 0 {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(output_path)?
    } else {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?
    };

    let progress = if !quiet {
        let wrapper = ProgressBarWrapper::new(total_size, existing_size);
        if let Some(mp) = multi_progress {
            mp.add(wrapper.get_bar().clone());
        }
        Some(wrapper)
    } else {
        None
    };

    let mut throttle = Throttle::new(options.limit_rate);
    let chunk_size = 8192;

    // FIXME(A1): buffers the whole body in memory before writing anything.
    let bytes = response.bytes()?;
    for chunk in bytes.chunks(chunk_size) {
        throttle.wait(chunk.len());

        file.write_all(chunk)?;
        if let Some(ref p) = progress {
            p.inc(chunk.len() as u64);
        }
    }

    if let Some(p) = progress {
        p.finish();
    }

    if !quiet {
        eprintln!("\n✅ Download complete: {}", output_path);
    }

    Ok(())
}
