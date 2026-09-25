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

//! Reading URLs from a file or from stdin.

use std::fs::File;
use std::io::{stdin, BufRead, BufReader};

/// Reads one URL per line from `source`; `"-"` means stdin.
///
/// Each line is trimmed of surrounding whitespace; a blank line or one whose first
/// non-whitespace character is `#` is treated as a comment and skipped.
///
/// # Errors
///
/// Returns the underlying I/O error when `source` cannot be opened. Lines that
/// cannot be read are skipped.
pub fn read_url_list(source: &str) -> std::io::Result<Vec<String>> {
    if source == "-" {
        let stdin = stdin();
        Ok(collect_urls(stdin.lock()))
    } else {
        let file = File::open(source)?;
        Ok(collect_urls(BufReader::new(file)))
    }
}

// `map_while(Result::ok)` would stop at the first undecodable line; skipping such
// lines and carrying on is the established behaviour.
#[allow(clippy::lines_filter_map_ok)]
fn collect_urls<R: BufRead>(reader: R) -> Vec<String> {
    reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}
