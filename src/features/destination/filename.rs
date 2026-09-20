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


//! Choosing a file name.

use url::Url;

/// Uses `explicit` (`-O`) when given, otherwise the last path segment of `url`,
/// falling back to `"downloaded"` when the path has no file name.
pub fn file_name_for(url: &Url, explicit: Option<&str>) -> String {
    if let Some(name) = explicit {
        name.to_string()
    } else {
        url.path_segments()
            .and_then(|mut segments| segments.next_back())
            .filter(|&name| !name.is_empty())
            .unwrap_or("downloaded")
            .to_string()
    }
}
