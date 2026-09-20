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


//! The part files of a segmented download: how the file is split, which leftovers may be
//! reused, and how the parts are put back together.
//!
//! Part `i` lives in `name.part<i>` and holds bytes `ranges[i]` of the remote file. Because
//! the range of each part depends on the segment count and on the exact remote file, a
//! resume is only safe when both are unchanged (see [`resume_allowed`]).

use super::error::{Error, Result};
use super::partial::PartMeta;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

/// Splits `total_size` bytes into inclusive `(start, end)` ranges, one per segment; the last
/// range absorbs the remainder. Never returns more ranges than there are bytes.
pub(super) fn plan_ranges(total_size: u64, segments: usize) -> Vec<(u64, u64)> {
    if total_size == 0 {
        return Vec::new();
    }
    let segments = (segments as u64).clamp(1, total_size);
    let part_size = total_size / segments;
    let mut ranges = Vec::with_capacity(segments as usize);
    let mut start = 0;
    for i in 0..segments {
        let end = if i == segments - 1 { total_size - 1 } else { start + part_size - 1 };
        ranges.push((start, end));
        start = end + 1;
    }
    ranges
}

/// Splits `--limit-rate` across the segments so the limit holds for the whole download.
/// `0` (unlimited) stays unlimited, and no segment is ever starved down to zero.
pub(super) fn per_segment_limit(limit: Option<usize>, segments: usize) -> Option<usize> {
    limit.map(|l| if l == 0 { 0 } else { (l / segments.max(1)).max(1) })
}

pub(super) fn part_path(output_path: &str, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.part{}", output_path, index))
}

/// Every `name.part<N>` beside `output_path`, whatever N is. `name.part` (a single-connection
/// download) and `name.part.meta` (the sidecar) are not part files and never match.
pub(super) fn existing_parts(output_path: &str) -> Vec<(usize, PathBuf)> {
    let path = Path::new(output_path);
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let prefix = format!("{name}.part");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let digits = file_name.to_str()?.strip_prefix(&prefix)?;
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some((digits.parse().ok()?, entry.path()))
        })
        .collect()
}

/// Deletes the part files with index `from_index` or higher; `0` deletes them all.
pub(super) fn discard_parts(output_path: &str, from_index: usize) {
    for (index, path) in existing_parts(output_path) {
        if index >= from_index {
            let _ = fs::remove_file(path);
        }
    }
}

/// How many bytes of each part are already on disk. A part longer than its range cannot be
/// a prefix of anything we would ask for, so it is deleted and counts as empty.
pub(super) fn reconcile(part_paths: &[PathBuf], ranges: &[(u64, u64)]) -> Vec<u64> {
    part_paths
        .iter()
        .zip(ranges)
        .map(|(path, &(start, end))| {
            let expected = end - start + 1;
            match fs::metadata(path) {
                Ok(m) if m.len() > expected => {
                    let _ = fs::remove_file(path);
                    0
                }
                Ok(m) => m.len(),
                Err(_) => 0,
            }
        })
        .collect()
}

/// Whether parts left by an earlier run may be continued. `current` describes the remote
/// file as the probe sees it now; `saved` is the sidecar written by the earlier run.
///
/// With no sidecar (parts from 1.0.0, say) the parts are accepted as unvalidated prefixes, as
/// a single-connection resume does. With one, the URL, size and segment count must match and
/// the file must not have changed (same strong ETag, else same Last-Modified).
pub(super) fn resume_allowed(saved: Option<&PartMeta>, current: &PartMeta) -> bool {
    let Some(saved) = saved else {
        return true;
    };
    saved.url == current.url
        && saved.segments == current.segments
        && saved.total == current.total
        && same_version(saved, current)
}

fn same_version(a: &PartMeta, b: &PartMeta) -> bool {
    let strong = |m: &PartMeta| m.etag.as_deref().filter(|e| !e.starts_with("W/")).map(str::to_string);
    if let (Some(x), Some(y)) = (strong(a), strong(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (&a.last_modified, &b.last_modified) {
        return x == y;
    }
    // Nothing comparable: trust the earlier run, as with an unvalidated single resume.
    true
}

/// Concatenates the parts into `output_path` and deletes each one as it is consumed, so disk
/// use stays at about the file size plus one part. `io::copy` between files is done inside
/// the kernel on Linux, so memory use does not depend on the part size.
///
/// Every part is checked against its range *before* anything is copied; a part of the wrong
/// length is deleted (so the next run downloads it again) and reported.
///
/// # Errors
///
/// [`Error::ProtocolError`] when a part is missing or has the wrong length, or an I/O error.
pub(super) fn merge_parts(output_path: &Path, part_paths: &[PathBuf], ranges: &[(u64, u64)]) -> Result<()> {
    for (path, &(start, end)) in part_paths.iter().zip(ranges) {
        let expected = end - start + 1;
        match fs::metadata(path) {
            Ok(m) if m.len() == expected => {}
            Ok(m) => {
                let _ = fs::remove_file(path);
                return Err(Error::ProtocolError(format!(
                    "Part file {} is {} bytes, expected {}",
                    path.display(),
                    m.len(),
                    expected
                )));
            }
            Err(_) => {
                return Err(Error::ProtocolError(format!("Part file {} missing", path.display())));
            }
        }
    }

    let mut output_file = File::create(output_path)?;
    for (path, &(start, end)) in part_paths.iter().zip(ranges) {
        let mut part_file = File::open(path)?;
        let copied = io::copy(&mut part_file, &mut output_file)?;
        if copied != end - start + 1 {
            return Err(Error::ProtocolError(format!(
                "Part file {} changed while merging ({} bytes copied)",
                path.display(),
                copied
            )));
        }
        let _ = fs::remove_file(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rget-parts-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn meta(url: &str, etag: Option<&str>, last_modified: Option<&str>, total: u64, segments: usize) -> PartMeta {
        PartMeta {
            url: url.to_string(),
            etag: etag.map(str::to_string),
            last_modified: last_modified.map(str::to_string),
            total: Some(total),
            segments: Some(segments),
        }
    }

    // ---- planning ---------------------------------------------------------------

    #[test]
    fn the_last_range_absorbs_the_remainder() {
        assert_eq!(
            plan_ranges(3_000_017, 4),
            vec![(0, 750_003), (750_004, 1_500_007), (1_500_008, 2_250_011), (2_250_012, 3_000_016)]
        );
    }

    #[test]
    fn a_single_segment_is_the_whole_file() {
        assert_eq!(plan_ranges(42, 1), vec![(0, 41)]);
    }

    #[test]
    fn there_are_never_more_segments_than_bytes() {
        assert_eq!(plan_ranges(3, 8), vec![(0, 0), (1, 1), (2, 2)]);
        assert_eq!(plan_ranges(1, 4), vec![(0, 0)]);
        assert!(plan_ranges(0, 4).is_empty());
    }

    proptest! {
        #[test]
        fn ranges_tile_the_file_exactly_once(total in 1u64..2_000_000, segments in 1usize..80) {
            let ranges = plan_ranges(total, segments);
            prop_assert_eq!(ranges.len(), segments.min(total as usize));
            prop_assert_eq!(ranges[0].0, 0);
            prop_assert_eq!(ranges.last().unwrap().1, total - 1);
            for pair in ranges.windows(2) {
                prop_assert_eq!(pair[0].1 + 1, pair[1].0);
            }
            for &(start, end) in &ranges {
                prop_assert!(start <= end);
            }
        }
    }

    #[test]
    fn the_limit_is_shared_between_segments() {
        assert_eq!(per_segment_limit(Some(4_000_000), 4), Some(1_000_000));
        assert_eq!(per_segment_limit(Some(1_000_000), 1), Some(1_000_000));
    }

    #[test]
    fn unlimited_stays_unlimited_and_no_segment_is_starved() {
        assert_eq!(per_segment_limit(None, 4), None);
        assert_eq!(per_segment_limit(Some(0), 4), Some(0));
        assert_eq!(per_segment_limit(Some(3), 8), Some(1));
    }

    // ---- discovery and cleanup --------------------------------------------------

    #[test]
    fn only_numbered_part_files_are_found() {
        let dir = scratch("discover");
        let out = dir.join("f.bin");
        for name in ["f.bin.part0", "f.bin.part3", "f.bin.part12", "f.bin.part", "f.bin.part.meta", "f.bin.partx", "f.bin.part1x", "other.bin.part1", "f.bin"] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        let mut found: Vec<usize> = existing_parts(out.to_str().unwrap()).into_iter().map(|(i, _)| i).collect();
        found.sort();
        assert_eq!(found, vec![0, 3, 12]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn discarding_removes_only_the_requested_indices_and_nothing_else() {
        let dir = scratch("discard");
        let out = dir.join("f.bin");
        for name in ["f.bin.part0", "f.bin.part1", "f.bin.part5", "f.bin.part", "f.bin.part.meta", "f.bin"] {
            fs::write(dir.join(name), b"x").unwrap();
        }
        discard_parts(out.to_str().unwrap(), 2);
        assert!(dir.join("f.bin.part0").exists() && dir.join("f.bin.part1").exists());
        assert!(!dir.join("f.bin.part5").exists());
        discard_parts(out.to_str().unwrap(), 0);
        assert!(!dir.join("f.bin.part0").exists() && !dir.join("f.bin.part1").exists());
        for keep in ["f.bin.part", "f.bin.part.meta", "f.bin"] {
            assert!(dir.join(keep).exists(), "{keep} must survive");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_bare_relative_name_is_searched_in_the_current_directory() {
        assert!(existing_parts("rget-no-such-file-here.bin").is_empty());
    }

    // ---- reconciling ------------------------------------------------------------

    #[test]
    fn missing_parts_are_empty() {
        let dir = scratch("missing");
        let paths = vec![dir.join("a"), dir.join("b")];
        assert_eq!(reconcile(&paths, &[(0, 9), (10, 19)]), vec![0, 0]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn what_is_on_disk_is_counted_and_never_more_than_the_range() {
        let dir = scratch("reconcile");
        let (full, partial, oversized) = (dir.join("full"), dir.join("partial"), dir.join("oversized"));
        fs::write(&full, [0u8; 10]).unwrap();
        fs::write(&partial, [0u8; 4]).unwrap();
        fs::write(&oversized, [0u8; 50]).unwrap();
        let ranges = [(0, 9), (10, 19), (20, 29)];
        assert_eq!(reconcile(&[full, partial, oversized.clone()], &ranges), vec![10, 4, 0]);
        assert!(!oversized.exists(), "an oversized part must be deleted");
        fs::remove_dir_all(dir).unwrap();
    }

    // ---- resume validation ------------------------------------------------------

    const URL: &str = "http://h/f";

    #[test]
    fn parts_without_a_sidecar_are_accepted_unvalidated() {
        assert!(resume_allowed(None, &meta(URL, Some("\"a\""), None, 100, 4)));
    }

    #[test]
    fn an_identical_layout_and_version_is_accepted() {
        let m = meta(URL, Some("\"a\""), Some("Wed, 01 Jan 2025 00:00:00 GMT"), 100, 4);
        assert!(resume_allowed(Some(&m), &m.clone()));
    }

    #[test]
    fn a_different_url_size_or_segment_count_is_refused() {
        let saved = meta(URL, Some("\"a\""), None, 100, 4);
        assert!(!resume_allowed(Some(&saved), &meta("http://h/other", Some("\"a\""), None, 100, 4)));
        assert!(!resume_allowed(Some(&saved), &meta(URL, Some("\"a\""), None, 101, 4)));
        assert!(!resume_allowed(Some(&saved), &meta(URL, Some("\"a\""), None, 100, 3)));
    }

    #[test]
    fn a_sidecar_from_a_single_connection_run_is_refused() {
        let mut single = meta(URL, Some("\"a\""), None, 100, 4);
        single.segments = None;
        assert!(!resume_allowed(Some(&single), &meta(URL, Some("\"a\""), None, 100, 4)));
    }

    #[test]
    fn a_changed_strong_etag_is_refused_and_a_weak_one_is_ignored() {
        let saved = meta(URL, Some("\"a\""), Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4);
        assert!(!resume_allowed(Some(&saved), &meta(URL, Some("\"b\""), Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4)));
        // Weak ETags cannot prove byte identity, so Last-Modified decides.
        let weak = meta(URL, Some("W/\"x\""), Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4);
        let weak_other = meta(URL, Some("W/\"y\""), Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4);
        assert!(resume_allowed(Some(&weak), &weak_other));
    }

    #[test]
    fn last_modified_is_used_when_there_is_no_strong_etag() {
        let saved = meta(URL, None, Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4);
        assert!(!resume_allowed(Some(&saved), &meta(URL, None, Some("Tue, 02 Jan 2024 00:00:00 GMT"), 100, 4)));
        assert!(resume_allowed(Some(&saved), &meta(URL, None, Some("Mon, 01 Jan 2024 00:00:00 GMT"), 100, 4)));
    }

    #[test]
    fn without_any_comparable_validator_the_earlier_run_is_trusted() {
        let saved = meta(URL, None, None, 100, 4);
        assert!(resume_allowed(Some(&saved), &meta(URL, Some("\"a\""), None, 100, 4)));
    }

    proptest! {
        #[test]
        fn any_change_of_url_size_or_layout_is_refused(
            total in 1u64..1_000_000, segments in 1usize..32, etag in "[a-z]{1,8}",
            tweak in 0usize..3,
        ) {
            let saved = meta(URL, Some(&format!("\"{etag}\"")), None, total, segments);
            prop_assert!(resume_allowed(Some(&saved), &saved.clone()));
            let mut other = saved.clone();
            match tweak {
                0 => other.url = "http://h/other".to_string(),
                1 => other.total = Some(total + 1),
                _ => other.segments = Some(segments + 1),
            }
            prop_assert!(!resume_allowed(Some(&saved), &other));
        }
    }

    // ---- merging ----------------------------------------------------------------

    #[test]
    fn parts_are_concatenated_in_order_and_consumed() {
        let dir = scratch("merge");
        let parts: Vec<PathBuf> = (0..3).map(|i| dir.join(format!("p{i}"))).collect();
        fs::write(&parts[0], b"aaaa").unwrap();
        fs::write(&parts[1], b"bb").unwrap();
        fs::write(&parts[2], b"ccc").unwrap();
        let out = dir.join("out");
        merge_parts(&out, &parts, &[(0, 3), (4, 5), (6, 8)]).unwrap();
        assert_eq!(fs::read(&out).unwrap(), b"aaaabbccc");
        assert!(parts.iter().all(|p| !p.exists()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_part_of_the_wrong_length_is_reported_deleted_and_nothing_is_merged() {
        let dir = scratch("merge-bad");
        let parts = vec![dir.join("p0"), dir.join("p1")];
        fs::write(&parts[0], b"aaaa").unwrap();
        fs::write(&parts[1], b"b").unwrap();
        let out = dir.join("out");
        let err = merge_parts(&out, &parts, &[(0, 3), (4, 5)]).unwrap_err();
        assert!(matches!(err, Error::ProtocolError(m) if m.contains("expected 2")));
        assert!(!out.exists(), "nothing should be written when a part is bad");
        assert!(parts[0].exists(), "good parts are kept for the next run");
        assert!(!parts[1].exists(), "the bad part is removed so it is fetched again");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_part_is_reported() {
        let dir = scratch("merge-missing");
        let err = merge_parts(&dir.join("out"), &[dir.join("nope")], &[(0, 3)]).unwrap_err();
        assert!(matches!(err, Error::ProtocolError(m) if m.contains("missing")));
        fs::remove_dir_all(dir).unwrap();
    }
}
