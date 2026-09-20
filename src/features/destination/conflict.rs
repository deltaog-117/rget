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


//! What to do when the file we are about to write already exists.

use super::filename::split_extension;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The most numbered names (`file (1).zip` … `file (9999).zip`) tried before giving up.
const MAX_NUMBER: usize = 9_999;

/// The policy for a target that already exists (`--if-exists`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExistingFile {
    /// Replace it. The default, as before.
    #[default]
    Overwrite,
    /// Leave it alone and do not download.
    Skip,
    /// Keep it and save the new download as `name (1).ext`, `name (2).ext`, …
    Rename,
}

/// Why a download is not going ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// The file is already on disk.
    Exists,
    /// An earlier URL in the same run maps to the same file name.
    EarlierUrl,
    /// Every numbered alternative is taken.
    NoFreeName,
}

/// Where a download goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// Write here (possibly a numbered alternative to the path asked for).
    Write(String),
    /// Do not download; `path` is the file that is in the way.
    Skip { path: String, reason: SkipReason },
}

/// Whether something a download could clobber is at `path`. A device or a FIFO
/// (`-O /dev/null`) is never in the way.
fn occupied(path: &str) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|m| m.is_file() || m.is_dir() || m.file_type().is_symlink())
}

/// `dir/name.ext` -> `dir/name (n).ext`, keeping a short or compound extension intact.
fn numbered_name(path: &str, n: usize) -> String {
    let p = Path::new(path);
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return format!("{} ({})", path, n);
    };
    let (stem, extension) = split_extension(name);
    let numbered = format!("{} ({}){}", stem, n, extension);
    match p.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        Some(dir) => dir.join(numbered).to_string_lossy().to_string(),
        None => numbered,
    }
}

/// The file names settled so far in one run.
///
/// Names are claimed one URL at a time, before any download starts, so that parallel
/// downloads (`-j`) can never pick the same name and the outcome does not depend on timing.
#[derive(Debug, Default)]
pub struct Claims {
    claimed: HashSet<String>,
    /// For a numbered name, the name it was derived from (so `file (1).zip` never becomes
    /// `file (1) (1).zip`).
    origin: HashMap<String, String>,
}

impl Claims {
    pub fn new() -> Self {
        Self::default()
    }

    /// Settles where the download for `path` goes under `policy`.
    ///
    /// Two URLs that map to the same path never share it: whatever the policy, the later one
    /// is numbered (or, under [`ExistingFile::Skip`], skipped), because overwriting a file
    /// fetched moments ago in the same command is never what was meant.
    pub fn place(&mut self, path: &str, policy: ExistingFile) -> Placement {
        let claimed = self.claimed.contains(path);
        let exists = occupied(path);
        match policy {
            ExistingFile::Skip if claimed => Placement::Skip { path: path.to_string(), reason: SkipReason::EarlierUrl },
            ExistingFile::Skip if exists => Placement::Skip { path: path.to_string(), reason: SkipReason::Exists },
            ExistingFile::Rename if claimed || exists => self.next_free(path),
            ExistingFile::Overwrite if claimed => self.next_free(path),
            _ => {
                self.claimed.insert(path.to_string());
                Placement::Write(path.to_string())
            }
        }
    }

    /// Another name for a download whose target turned out to be occupied at the last moment.
    /// `None` when every alternative is taken.
    pub fn relocate(&mut self, occupied_path: &str) -> Option<String> {
        match self.next_free(occupied_path) {
            Placement::Write(path) => Some(path),
            Placement::Skip { .. } => None,
        }
    }

    /// Whether `path`, or anything a download of it leaves beside it, is in the way.
    fn taken(&self, path: &str) -> bool {
        self.claimed.contains(path)
            || occupied(path)
            || occupied(&format!("{}.part", path))
            || occupied(&format!("{}.part.meta", path))
    }

    fn next_free(&mut self, path: &str) -> Placement {
        let base = self.origin.get(path).cloned().unwrap_or_else(|| path.to_string());
        for n in 1..=MAX_NUMBER {
            let candidate = numbered_name(&base, n);
            if !self.taken(&candidate) {
                self.claimed.insert(candidate.clone());
                self.origin.insert(candidate.clone(), base);
                return Placement::Write(candidate);
            }
        }
        Placement::Skip { path: path.to_string(), reason: SkipReason::NoFreeName }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_goes_before_the_extension() {
        assert_eq!(numbered_name("file.zip", 1), "file (1).zip");
        assert_eq!(numbered_name("/d/e/file.zip", 12), "/d/e/file (12).zip");
        assert_eq!(numbered_name("archive.tar.gz", 2), "archive (2).tar.gz");
        assert_eq!(numbered_name("README", 1), "README (1)");
        assert_eq!(numbered_name("/d/README", 3), "/d/README (3)");
        assert_eq!(numbered_name(".env.example", 1), ".env (1).example");
        assert_eq!(numbered_name("a.b.c.d", 1), "a.b.c (1).d");
        assert_eq!(numbered_name("my.file.v2.zip", 1), "my.file.v2 (1).zip");
        assert_eq!(numbered_name("version.1.2.zip", 4), "version.1.2 (4).zip");
    }

    #[test]
    fn a_relative_path_without_a_directory_stays_relative() {
        assert_eq!(numbered_name("file.bin", 1), "file (1).bin");
    }
}
