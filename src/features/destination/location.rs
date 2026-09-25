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

//! Choosing the output directory and full path.

use std::path::{Path, PathBuf};

/// The user's Downloads folder, when the platform defines one.
pub fn default_download_dir() -> Option<PathBuf> {
    dirs::download_dir()
}

/// Resolves the full output path for `base_name`.
///
/// `-P` wins, then an explicit `-O` (used as given), then, only when `use_downloads` asks
/// for it (`-d`/`--downloads`), the platform Downloads folder. Otherwise the current
/// directory, matching `wget` and `curl`. Missing directories are created.
///
/// # Errors
///
/// Returns the I/O error when a needed directory cannot be created.
pub fn output_path(
    base_name: &str,
    directory_prefix: Option<&str>,
    explicit_output: bool,
    use_downloads: bool,
) -> std::io::Result<String> {
    if let Some(prefix) = directory_prefix {
        let dir = Path::new(prefix);
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
        Ok(dir.join(base_name).to_string_lossy().to_string())
    } else if explicit_output {
        // If -O or -P was explicitly given, use that
        Ok(base_name.to_string())
    } else if use_downloads {
        if let Some(dir) = default_download_dir() {
            if !dir.exists() {
                std::fs::create_dir_all(&dir)?;
            }
            return Ok(dir.join(base_name).to_string_lossy().to_string());
        }
        // No platform Downloads folder (e.g. `dirs` found none): fall back to CWD.
        Ok(base_name.to_string())
    } else {
        // Current directory, the default.
        Ok(base_name.to_string())
    }
}
