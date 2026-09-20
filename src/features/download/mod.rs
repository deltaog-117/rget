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


//! Download: fetching bytes from a URL to a file.
//!
//! Entry points are [`download_file`] (one URL, single- or multi-connection) and
//! [`run_pool`] (many URLs on worker threads).

mod client;
mod error;
mod options;
mod parts;
mod partial;
mod pool;
mod resume;
mod retry;
mod segmented;
mod single;
mod stream;
mod throttle;

pub use error::{Error, Result};
pub use options::DownloadOptions;
pub use pool::run_pool;

use indicatif::MultiProgress;

/// Downloads `url` to `output_path`, splitting it into `options.segments`
/// parallel ranges when that is greater than one.
///
/// # Errors
///
/// Returns the last error once every attempt (`options.retries + 1`) has failed.
pub fn download_file(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    multi_progress: Option<&MultiProgress>,
) -> Result<()> {
    if options.segments > 1 {
        return segmented::download(url, output_path, options, multi_progress);
    }

    download_single(url, output_path, options, multi_progress)
}

/// Single-connection download with the retry loop. Also the fallback whenever a
/// segmented download turns out not to be possible.
fn download_single(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    multi_progress: Option<&MultiProgress>,
) -> Result<()> {
    retry::run(options.retries, options.quiet, "", |attempt| {
        single::attempt(url, output_path, options, attempt, multi_progress)
    })
}
