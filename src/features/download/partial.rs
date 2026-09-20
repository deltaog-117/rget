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


//! Where a download lives while it is in progress: `name.part` plus a small sidecar,
//! renamed into place only when the transfer is complete.

use reqwest::header::{HeaderMap, ETAG, LAST_MODIFIED};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Which file holds the bytes a resume continues from.
#[derive(Debug, PartialEq, Clone, Copy)]
pub(super) enum Source {
    /// `name.part`, left by an interrupted download.
    Part,
    /// `name` itself: a partial file from rget 1.0.0 or another tool.
    Final,
}

/// The paths of one download.
///
/// A target that already exists but is not a regular file (`/dev/null`, a FIFO) is written
/// directly: there is nothing to stage and nothing to rename into place.
pub(super) struct Target {
    final_path: PathBuf,
    part_path: PathBuf,
    meta_path: PathBuf,
    staged: bool,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

impl Target {
    pub(super) fn resolve(output_path: &str) -> Self {
        let mut final_path = PathBuf::from(output_path);
        // Write through a symlink, as a plain `open` would, instead of replacing the link
        // with a regular file when the download is renamed into place. (A dangling link
        // cannot be resolved and is replaced.)
        if fs::symlink_metadata(&final_path).is_ok_and(|m| m.file_type().is_symlink()) {
            final_path = fs::canonicalize(&final_path).unwrap_or(final_path);
        }
        let staged = fs::metadata(&final_path).map(|m| m.is_file()).unwrap_or(true);
        Self {
            part_path: with_suffix(&final_path, ".part"),
            meta_path: with_suffix(&final_path, ".part.meta"),
            final_path,
            staged,
        }
    }

    pub(super) fn is_staged(&self) -> bool {
        self.staged
    }

    /// Where bytes are written while the download runs.
    pub(super) fn work_path(&self) -> &Path {
        if self.staged { &self.part_path } else { &self.final_path }
    }

    pub(super) fn meta_path(&self) -> &Path {
        &self.meta_path
    }

    /// The bytes a resume can continue from, preferring `name.part`. `name` itself is only
    /// considered when `adopt_final` is set (the user asked for `-c`).
    pub(super) fn resume_source(&self, adopt_final: bool) -> Option<(Source, u64)> {
        let non_empty_file = |p: &Path| fs::metadata(p).ok().filter(|m| m.is_file() && m.len() > 0).map(|m| m.len());
        if let Some(len) = non_empty_file(&self.part_path) {
            return Some((Source::Part, len));
        }
        if adopt_final {
            if let Some(len) = non_empty_file(&self.final_path) {
                return Some((Source::Final, len));
            }
        }
        None
    }

    /// Moves a partial `name` aside so the resume can append to it as `name.part`.
    pub(super) fn adopt_final(&self) -> io::Result<()> {
        fs::rename(&self.final_path, &self.part_path)
    }

    /// Deletes any partial data and its sidecar. A missing file is not an error.
    pub(super) fn discard_partial(&self) {
        let _ = fs::remove_file(&self.part_path);
        let _ = fs::remove_file(&self.meta_path);
    }

    /// Puts the finished download in place and removes the sidecar.
    ///
    /// # Errors
    ///
    /// Returns the I/O error if the rename fails.
    pub(super) fn finish(&self) -> io::Result<()> {
        if self.staged && self.part_path.exists() {
            fs::rename(&self.part_path, &self.final_path)?;
        }
        let _ = fs::remove_file(&self.meta_path);
        Ok(())
    }
}

/// What we remember about the response a partial file came from, so that a later resume
/// can prove the remote file has not changed (`If-Range`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(super) struct PartMeta {
    pub(super) url: String,
    pub(super) etag: Option<String>,
    pub(super) last_modified: Option<String>,
    pub(super) total: Option<u64>,
    /// Number of `name.partN` files for a segmented download; `None` for a single connection.
    /// A resume needs the same count, because the byte range of each part depends on it.
    #[serde(default)]
    pub(super) segments: Option<usize>,
}

impl PartMeta {
    pub(super) fn from_headers(url: &str, headers: &HeaderMap, total: Option<u64>) -> Self {
        let text = |name| headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
        Self {
            url: url.to_string(),
            etag: text(ETAG),
            last_modified: text(LAST_MODIFIED),
            total,
            segments: None,
        }
    }

    pub(super) fn with_segments(mut self, segments: usize) -> Self {
        self.segments = Some(segments);
        self
    }

    /// Keeps the validators of `earlier` when a `206` response leaves them out.
    pub(super) fn inherit(mut self, earlier: Option<&PartMeta>) -> Self {
        if let Some(old) = earlier.filter(|old| old.url == self.url) {
            self.etag = self.etag.or_else(|| old.etag.clone());
            self.last_modified = self.last_modified.or_else(|| old.last_modified.clone());
        }
        self
    }

    /// The value for `If-Range`: a strong ETag, else Last-Modified. Weak ETags are never
    /// valid there.
    pub(super) fn validator(&self) -> Option<&str> {
        match self.etag.as_deref() {
            Some(etag) if !etag.starts_with("W/") => Some(etag),
            _ => self.last_modified.as_deref(),
        }
    }

    /// `None` when the file is missing or unreadable; a damaged sidecar simply means "no
    /// validators".
    pub(super) fn read(path: &Path) -> Option<Self> {
        toml::from_str(&fs::read_to_string(path).ok()?).ok()
    }

    pub(super) fn write(&self, path: &Path) -> io::Result<()> {
        fs::write(path, toml::to_string(self).map_err(io::Error::other)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rget-partial-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn paths_sit_next_to_the_final_file() {
        let t = Target::resolve("/tmp/rget-nonexistent-dir/file.bin");
        assert_eq!(t.work_path(), Path::new("/tmp/rget-nonexistent-dir/file.bin.part"));
        assert_eq!(t.meta_path(), Path::new("/tmp/rget-nonexistent-dir/file.bin.part.meta"));
        assert!(t.is_staged());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_written_through_not_replaced() {
        let dir = scratch("symlink");
        let real = dir.join("real.bin");
        let link = dir.join("link.bin");
        fs::write(&real, b"old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let t = Target::resolve(link.to_str().unwrap());
        fs::write(t.work_path(), b"new").unwrap();
        t.finish().unwrap();

        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&real).unwrap(), b"new");
        assert_eq!(fs::read(&link).unwrap(), b"new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_device_is_written_directly() {
        let t = Target::resolve("/dev/null");
        assert!(!t.is_staged());
        assert_eq!(t.work_path(), Path::new("/dev/null"));
    }

    #[test]
    fn finish_renames_the_part_file_and_drops_the_sidecar() {
        let dir = scratch("finish");
        let out = dir.join("f.bin");
        let t = Target::resolve(out.to_str().unwrap());
        fs::write(t.work_path(), b"data").unwrap();
        fs::write(t.meta_path(), b"url = \"x\"").unwrap();
        t.finish().unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"data");
        assert!(!t.work_path().exists());
        assert!(!t.meta_path().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn finish_replaces_an_existing_file_only_at_the_end() {
        let dir = scratch("replace");
        let out = dir.join("f.bin");
        fs::write(&out, b"old").unwrap();
        let t = Target::resolve(out.to_str().unwrap());
        fs::write(t.work_path(), b"new").unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"old");
        t.finish().unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"new");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resume_prefers_the_part_file_and_adopts_the_final_one_only_on_request() {
        let dir = scratch("source");
        let out = dir.join("f.bin");
        let t = Target::resolve(out.to_str().unwrap());
        assert_eq!(t.resume_source(true), None);

        fs::write(&out, [0u8; 7]).unwrap();
        assert_eq!(t.resume_source(false), None);
        assert_eq!(t.resume_source(true), Some((Source::Final, 7)));

        fs::write(t.work_path(), [0u8; 3]).unwrap();
        assert_eq!(t.resume_source(true), Some((Source::Part, 3)));
        assert_eq!(t.resume_source(false), Some((Source::Part, 3)));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_empty_partial_is_no_partial() {
        let dir = scratch("empty");
        let t = Target::resolve(dir.join("f.bin").to_str().unwrap());
        fs::write(t.work_path(), b"").unwrap();
        assert_eq!(t.resume_source(true), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn adopting_moves_the_final_file_to_part() {
        let dir = scratch("adopt");
        let out = dir.join("f.bin");
        fs::write(&out, b"half").unwrap();
        let t = Target::resolve(out.to_str().unwrap());
        t.adopt_final().unwrap();
        assert!(!out.exists());
        assert_eq!(fs::read(t.work_path()).unwrap(), b"half");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn discarding_removes_both_files_and_tolerates_missing_ones() {
        let dir = scratch("discard");
        let t = Target::resolve(dir.join("f.bin").to_str().unwrap());
        t.discard_partial();
        fs::write(t.work_path(), b"x").unwrap();
        fs::write(t.meta_path(), b"y").unwrap();
        t.discard_partial();
        assert!(!t.work_path().exists() && !t.meta_path().exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_sidecar_round_trips_and_omits_missing_fields() {
        let dir = scratch("meta");
        let path = dir.join("m.meta");
        let full = PartMeta {
            url: "http://h/x".into(),
            etag: Some("\"abc\"".into()),
            last_modified: Some("Wed, 01 Jan 2025 00:00:00 GMT".into()),
            total: Some(123),
            segments: Some(4),
        };
        full.write(&path).unwrap();
        assert_eq!(PartMeta::read(&path), Some(full));

        let bare = PartMeta { url: "http://h/y".into(), etag: None, last_modified: None, total: None, segments: None };
        bare.write(&path).unwrap();
        assert_eq!(PartMeta::read(&path), Some(bare));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_or_damaged_sidecar_reads_as_none() {
        let dir = scratch("damaged");
        let path = dir.join("m.meta");
        assert_eq!(PartMeta::read(&path), None);
        fs::write(&path, "this is = not [valid").unwrap();
        assert_eq!(PartMeta::read(&path), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_206_that_omits_validators_inherits_the_earlier_ones() {
        let earlier = PartMeta { url: "http://h/x".into(), etag: Some("\"e\"".into()), last_modified: None, total: Some(10), segments: None };
        let new = PartMeta { url: "http://h/x".into(), etag: None, last_modified: None, total: Some(10), segments: None };
        assert_eq!(new.clone().inherit(Some(&earlier)).etag.as_deref(), Some("\"e\""));
        let other_url = PartMeta { url: "http://h/other".into(), ..new };
        assert_eq!(other_url.inherit(Some(&earlier)).etag, None);
    }
}
