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


//! Resume decisions, kept free of I/O so the whole state machine can be tested.
//!
//! Before the request, [`plan`] decides what to ask for. After the response, [`interpret`]
//! decides what the answer means. Together they cover every case:
//!
//! | Request                 | Response                         | Reply                          |
//! |-------------------------|----------------------------------|--------------------------------|
//! | full (no partial data)  | 2xx                              | `Full`                         |
//! | `Range: from-`          | 206 starting at `from`           | `Append`                       |
//! | `Range: from-`          | 206 starting elsewhere / no range| `WrongOffset` (error)          |
//! | `Range: from-` + `If-Range` | 200 (validator no longer matches) | `Restart { validator_failed }` |
//! | `Range: from-`          | 200 (server ignores ranges)      | `Restart { validator_failed }` |
//! | `Range: from-`          | 416, complete length == `from`   | `AlreadyComplete`              |
//! | `Range: from-`          | 416, any other length            | `Refetch` (partial is stale)   |
//! | anything                | any other status                 | `Other` (caller reports it)    |

use super::partial::PartMeta;
use reqwest::StatusCode;
use std::path::PathBuf;

/// What to ask the server for.
#[derive(Debug, PartialEq)]
pub(super) enum Plan {
    /// Start from byte 0 and replace whatever is in the partial file.
    Fresh,
    /// Continue after `from` bytes, guarded by `if_range` when a validator is known.
    Continue { from: u64, if_range: Option<String> },
}

/// Chooses the request. `partial_len` is the size of the usable partial data (0 if none),
/// `meta` the sidecar that belongs to it.
pub(super) fn plan(partial_len: u64, meta: Option<&PartMeta>, url: &str) -> Plan {
    if partial_len == 0 {
        return Plan::Fresh;
    }
    match meta {
        // The partial data belongs to a different download.
        Some(m) if m.url != url => Plan::Fresh,
        // More bytes than the file has: it shrank or the partial is corrupt.
        Some(m) if m.total.is_some_and(|total| partial_len > total) => Plan::Fresh,
        Some(m) => Plan::Continue { from: partial_len, if_range: m.validator().map(str::to_string) },
        // No sidecar (data from 1.0.0 or another tool): resume unvalidated, like wget and curl.
        None => Plan::Continue { from: partial_len, if_range: None },
    }
}

/// A parsed `Content-Range` header.
#[derive(Debug, PartialEq)]
pub(super) struct ContentRange {
    /// First and last byte served; `None` for the unsatisfied form `bytes */total`.
    pub(super) range: Option<(u64, u64)>,
    /// Complete length; `None` when the server sends `*`.
    pub(super) total: Option<u64>,
}

pub(super) fn parse_content_range(value: &str) -> Option<ContentRange> {
    let rest = value.trim().strip_prefix("bytes ")?;
    let (range, total) = rest.split_once('/')?;
    let total = match total.trim() {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    let range = match range.trim() {
        "*" => None,
        r => {
            let (start, end) = r.split_once('-')?;
            Some((start.parse().ok()?, end.parse().ok()?))
        }
    };
    Some(ContentRange { range, total })
}

/// What a response means for the download.
#[derive(Debug, PartialEq)]
pub(super) enum Reply {
    Full,
    Append { total: Option<u64> },
    Restart { validator_failed: bool },
    AlreadyComplete,
    Refetch,
    WrongOffset,
    Other,
}

pub(super) fn interpret(plan: &Plan, status: StatusCode, content_range: Option<&str>) -> Reply {
    let (from, if_range) = match plan {
        Plan::Fresh => {
            return if status.is_success() { Reply::Full } else { Reply::Other };
        }
        Plan::Continue { from, if_range } => (*from, if_range),
    };
    let content_range = content_range.and_then(parse_content_range);

    match status.as_u16() {
        206 => match content_range {
            Some(ContentRange { range: Some((start, _)), total }) if start == from => {
                Reply::Append { total }
            }
            _ => Reply::WrongOffset,
        },
        200 => Reply::Restart { validator_failed: if_range.is_some() },
        416 => match content_range {
            Some(ContentRange { range: None, total: Some(total) }) if total == from => {
                Reply::AlreadyComplete
            }
            _ => Reply::Refetch,
        },
        _ => Reply::Other,
    }
}

/// How many bytes of each segment are already on disk (a full part counts as its whole size).
///
/// FIXME(B11): parts are validated by size alone; a stale or oversized part is trusted.
pub(super) fn segment_positions(part_paths: &[PathBuf], ranges: &[(u64, u64)]) -> Vec<u64> {
    part_paths
        .iter()
        .zip(ranges)
        .map(|(path, &(start, end))| {
            let expected = end - start + 1;
            match std::fs::metadata(path) {
                Ok(m) => m.len().min(expected),
                Err(_) => 0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn meta(url: &str, etag: Option<&str>, last_modified: Option<&str>, total: Option<u64>) -> PartMeta {
        PartMeta {
            url: url.to_string(),
            etag: etag.map(str::to_string),
            last_modified: last_modified.map(str::to_string),
            total,
        }
    }

    fn sc(code: u16) -> StatusCode {
        StatusCode::from_u16(code).unwrap()
    }

    const URL: &str = "http://example.com/f";

    // ---- plan -------------------------------------------------------------------

    #[test]
    fn no_partial_data_means_a_fresh_download() {
        assert_eq!(plan(0, None, URL), Plan::Fresh);
        assert_eq!(plan(0, Some(&meta(URL, Some("\"a\""), None, Some(10))), URL), Plan::Fresh);
    }

    #[test]
    fn partial_data_without_a_sidecar_resumes_unvalidated() {
        assert_eq!(plan(500, None, URL), Plan::Continue { from: 500, if_range: None });
    }

    #[test]
    fn a_strong_etag_is_the_validator() {
        let m = meta(URL, Some("\"abc\""), Some("Wed, 01 Jan 2025 00:00:00 GMT"), Some(1000));
        assert_eq!(plan(500, Some(&m), URL), Plan::Continue { from: 500, if_range: Some("\"abc\"".into()) });
    }

    #[test]
    fn a_weak_etag_is_skipped_in_favour_of_last_modified() {
        let m = meta(URL, Some("W/\"abc\""), Some("Wed, 01 Jan 2025 00:00:00 GMT"), None);
        assert_eq!(
            plan(500, Some(&m), URL),
            Plan::Continue { from: 500, if_range: Some("Wed, 01 Jan 2025 00:00:00 GMT".into()) }
        );
        let weak_only = meta(URL, Some("W/\"abc\""), None, None);
        assert_eq!(plan(500, Some(&weak_only), URL), Plan::Continue { from: 500, if_range: None });
    }

    #[test]
    fn a_sidecar_for_another_url_is_ignored() {
        let m = meta("http://example.com/other", Some("\"a\""), None, Some(1000));
        assert_eq!(plan(500, Some(&m), URL), Plan::Fresh);
    }

    #[test]
    fn more_partial_data_than_the_file_has_restarts() {
        let m = meta(URL, Some("\"a\""), None, Some(400));
        assert_eq!(plan(500, Some(&m), URL), Plan::Fresh);
        let exact = meta(URL, Some("\"a\""), None, Some(500));
        assert!(matches!(plan(500, Some(&exact), URL), Plan::Continue { .. }));
    }

    // ---- content-range ----------------------------------------------------------

    #[test]
    fn content_range_forms_are_parsed() {
        assert_eq!(parse_content_range("bytes 0-99/1000"), Some(ContentRange { range: Some((0, 99)), total: Some(1000) }));
        assert_eq!(parse_content_range("bytes */1000"), Some(ContentRange { range: None, total: Some(1000) }));
        assert_eq!(parse_content_range("bytes 5-9/*"), Some(ContentRange { range: Some((5, 9)), total: None }));
    }

    #[test]
    fn malformed_content_range_is_rejected() {
        for bad in ["", "bytes", "0-99/1000", "bytes 0-99", "bytes a-b/c", "items 0-9/10", "bytes 0/10"] {
            assert_eq!(parse_content_range(bad), None, "{bad:?}");
        }
    }

    // ---- interpret --------------------------------------------------------------

    fn cont(from: u64, validated: bool) -> Plan {
        Plan::Continue { from, if_range: validated.then(|| "\"v\"".to_string()) }
    }

    #[test]
    fn a_fresh_request_only_cares_about_success() {
        assert_eq!(interpret(&Plan::Fresh, sc(200), None), Reply::Full);
        assert_eq!(interpret(&Plan::Fresh, sc(404), None), Reply::Other);
        assert_eq!(interpret(&Plan::Fresh, sc(416), Some("bytes */5")), Reply::Other);
    }

    #[test]
    fn a_206_at_the_requested_offset_appends() {
        assert_eq!(interpret(&cont(100, true), sc(206), Some("bytes 100-999/1000")), Reply::Append { total: Some(1000) });
        assert_eq!(interpret(&cont(100, false), sc(206), Some("bytes 100-999/*")), Reply::Append { total: None });
    }

    #[test]
    fn a_206_at_the_wrong_offset_or_without_a_range_is_rejected() {
        assert_eq!(interpret(&cont(100, false), sc(206), Some("bytes 0-999/1000")), Reply::WrongOffset);
        assert_eq!(interpret(&cont(100, false), sc(206), None), Reply::WrongOffset);
    }

    #[test]
    fn a_200_to_a_range_request_restarts_and_says_why() {
        assert_eq!(interpret(&cont(100, true), sc(200), None), Reply::Restart { validator_failed: true });
        assert_eq!(interpret(&cont(100, false), sc(200), None), Reply::Restart { validator_failed: false });
    }

    #[test]
    fn a_416_matching_our_length_means_the_file_is_complete() {
        assert_eq!(interpret(&cont(1000, false), sc(416), Some("bytes */1000")), Reply::AlreadyComplete);
    }

    #[test]
    fn any_other_416_means_the_partial_is_stale() {
        assert_eq!(interpret(&cont(1000, false), sc(416), Some("bytes */400")), Reply::Refetch);
        assert_eq!(interpret(&cont(1000, false), sc(416), Some("bytes */2000")), Reply::Refetch);
        assert_eq!(interpret(&cont(1000, false), sc(416), None), Reply::Refetch);
    }

    #[test]
    fn other_statuses_are_left_to_the_caller() {
        for code in [301, 403, 404, 500, 503] {
            assert_eq!(interpret(&cont(10, false), sc(code), None), Reply::Other, "{code}");
        }
    }

    // ---- properties -------------------------------------------------------------

    proptest! {
        #[test]
        fn a_continue_plan_always_resumes_exactly_where_the_data_ends(
            len in 0u64..1_000_000,
            total in proptest::option::of(0u64..2_000_000),
            same_url in any::<bool>(),
        ) {
            let m = meta(if same_url { URL } else { "http://other/" }, Some("\"e\""), None, total);
            match plan(len, Some(&m), URL) {
                Plan::Fresh => {}
                Plan::Continue { from, .. } => {
                    prop_assert_eq!(from, len);
                    prop_assert!(len > 0);
                    prop_assert!(same_url);
                    prop_assert!(total.is_none_or(|t| len <= t));
                }
            }
        }

        #[test]
        fn content_range_round_trips(start in 0u64..u64::MAX / 2, len in 1u64..1_000_000, total in 1u64..u64::MAX / 2) {
            let end = start + len - 1;
            let parsed = parse_content_range(&format!("bytes {start}-{end}/{total}")).unwrap();
            prop_assert_eq!(parsed, ContentRange { range: Some((start, end)), total: Some(total) });
        }

        #[test]
        fn interpret_never_appends_unless_the_offset_matches(from in 1u64..1_000_000, start in 0u64..1_000_000) {
            let header = format!("bytes {start}-{}/9999999", start + 10);
            let reply = interpret(&cont(from, false), sc(206), Some(&header));
            prop_assert_eq!(matches!(reply, Reply::Append { .. }), start == from);
        }
    }

    // ---- segments ---------------------------------------------------------------

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rget-resume-{}-{}", std::process::id(), name))
    }

    #[test]
    fn segment_positions_of_missing_parts_are_zero() {
        let ranges = vec![(0, 9), (10, 19)];
        let paths = vec![temp_path("missing-a"), temp_path("missing-b")];
        assert_eq!(segment_positions(&paths, &ranges), vec![0, 0]);
    }

    #[test]
    fn segment_positions_count_what_is_on_disk() {
        let full = temp_path("full");
        let partial = temp_path("partial");
        std::fs::write(&full, [0u8; 10]).unwrap();
        std::fs::write(&partial, [0u8; 4]).unwrap();
        let ranges = vec![(0, 9), (10, 19)];
        assert_eq!(segment_positions(&[full.clone(), partial.clone()], &ranges), vec![10, 4]);
        std::fs::remove_file(full).unwrap();
        std::fs::remove_file(partial).unwrap();
    }

    #[test]
    fn segment_positions_never_exceed_the_segment_size() {
        let oversized = temp_path("oversized");
        std::fs::write(&oversized, [0u8; 50]).unwrap();
        assert_eq!(segment_positions(std::slice::from_ref(&oversized), &[(0, 9)]), vec![10]);
        std::fs::remove_file(oversized).unwrap();
    }
}
