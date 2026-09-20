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


//! Resume bookkeeping for single-connection and segmented downloads.

use super::error::Result;
use reqwest::StatusCode;

/// Bytes already on disk at `output_path`, or 0 when not resuming.
pub(super) fn existing_size(output_path: &str, resume: bool) -> u64 {
    if resume {
        std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    }
}

/// `Range` header value that asks for everything after `existing_size` bytes.
pub(super) fn range_header(existing_size: u64) -> String {
    format!("bytes={}-", existing_size)
}

/// Reports whether the server honoured the resume request; if not, empties the
/// output file so the download starts over.
///
/// FIXME(A5): a 416 on an already-complete file lands in the "not supported"
/// branch and truncates the finished file. Fixed in the resume cycle.
pub(super) fn handle_response(
    status: StatusCode,
    existing_size: u64,
    output_path: &str,
    quiet: bool,
) -> Result<()> {
    if status == StatusCode::PARTIAL_CONTENT {
        if !quiet {
            eprintln!("Resuming download from byte {}", existing_size);
        }
    } else {
        if !quiet {
            eprintln!("Server doesn't support resume, starting from scratch");
        }
        std::fs::write(output_path, [])?;
    }
    Ok(())
}

/// How many bytes of each segment are already on disk. Also advances the start
/// of each partially-downloaded range.
///
/// FIXME(A5/B): the range start is advanced here *and* again by the worker
/// (`start + resume_pos`), so a partial part resumes `size` bytes too far and
/// the merged file is corrupt. Preserved as-is by the restructure cycle.
pub(super) fn segment_positions(part_paths: &[String], ranges: &mut [(u64, u64)]) -> Vec<u64> {
    let mut resume_positions = vec![0u64; ranges.len()];
    for (i, path) in part_paths.iter().enumerate() {
        if let Ok(metadata) = std::fs::metadata(path) {
            let size = metadata.len();
            let (expected_start, expected_end) = ranges[i];
            let expected_size = expected_end - expected_start + 1;
            if size >= expected_size {
                resume_positions[i] = expected_size;
            } else {
                resume_positions[i] = size;
                ranges[i].0 = expected_start + size;
            }
        }
    }
    resume_positions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("rget-resume-{}-{}", std::process::id(), name))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn existing_size_is_zero_when_not_resuming() {
        assert_eq!(existing_size("/definitely/not/here", false), 0);
    }

    #[test]
    fn existing_size_is_zero_for_a_missing_file() {
        assert_eq!(existing_size("/definitely/not/here", true), 0);
    }

    #[test]
    fn existing_size_reads_the_file_length() {
        let path = temp_path("size");
        std::fs::write(&path, b"12345").unwrap();
        assert_eq!(existing_size(&path, true), 5);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn range_header_is_open_ended() {
        assert_eq!(range_header(1000), "bytes=1000-");
    }

    #[test]
    fn segment_positions_of_missing_parts_are_zero() {
        let mut ranges = vec![(0, 9), (10, 19)];
        let paths = vec![temp_path("missing-a"), temp_path("missing-b")];
        assert_eq!(segment_positions(&paths, &mut ranges), vec![0, 0]);
        assert_eq!(ranges, vec![(0, 9), (10, 19)]);
    }

    #[test]
    fn segment_positions_of_a_complete_part_are_its_full_size() {
        let path = temp_path("complete");
        std::fs::write(&path, [0u8; 10]).unwrap();
        let mut ranges = vec![(0, 9)];
        assert_eq!(segment_positions(std::slice::from_ref(&path), &mut ranges), vec![10]);
        assert_eq!(ranges, vec![(0, 9)]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn segment_positions_of_a_partial_part_advance_the_range_start() {
        let path = temp_path("partial");
        std::fs::write(&path, [0u8; 4]).unwrap();
        let mut ranges = vec![(10, 19)];
        assert_eq!(segment_positions(std::slice::from_ref(&path), &mut ranges), vec![4]);
        assert_eq!(ranges, vec![(14, 19)]);
        std::fs::remove_file(&path).unwrap();
    }
}
