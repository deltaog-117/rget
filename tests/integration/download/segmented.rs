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

use super::{options, payload, scratch_dir, serve};
use rget::features::download::{download_file, Error};

fn leftover_parts(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains(".part"))
        .collect()
}

#[test]
fn segments_are_reassembled_in_order() {
    for segments in [2usize, 4, 7] {
        let data = payload(300_017);
        let base = serve(data.clone(), true);
        let dir = scratch_dir(&format!("seg-{segments}"));
        let out = dir.join("out.bin");
        let mut opts = options();
        opts.segments = segments;

        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

        assert_eq!(std::fs::read(&out).unwrap(), data, "segments = {segments}");
        assert!(
            leftover_parts(&dir).is_empty(),
            "part files were not cleaned up"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn a_server_without_range_support_falls_back_to_one_connection() {
    let data = payload(50_000);
    let base = serve(data.clone(), false);
    let dir = scratch_dir("seg-norange");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 4;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(leftover_parts(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_unreachable_server_falls_back_and_then_fails() {
    let dir = scratch_dir("seg-refused");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;

    assert!(download_file(
        "http://127.0.0.1:1/file",
        out.to_str().unwrap(),
        &opts,
        None
    )
    .is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_probe_carries_the_custom_headers() {
    let data = payload(120_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-guarded");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;
    opts.user_agent = Some("probe/1".to_string());
    opts.headers = vec![("X-Token".to_string(), "ok".to_string())];

    download_file(
        &format!("{base}/guarded"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_probe_without_the_headers_is_refused_and_reported() {
    let base = serve(payload(120_000), true);
    let dir = scratch_dir("seg-forbidden");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;

    let err = download_file(
        &format!("{base}/guarded"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::HttpStatus { status: s, .. } if s.as_u16() == 403),
        "got {err:?}"
    );
    assert!(!out.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_probe_respects_a_disabled_redirect_policy() {
    let base = serve(payload(10_000), true);
    let dir = scratch_dir("seg-noredirect");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;
    opts.follow_redirects = false;

    let err = download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::RedirectDisabled(302, _)),
        "got {err:?}"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_error_status_on_the_probe_is_reported_not_saved() {
    let base = serve(payload(10), true);
    let dir = scratch_dir("seg-404");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;

    let err = download_file(
        &format!("{base}/status/404"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::HttpStatus { status: s, .. } if s.as_u16() == 404),
        "got {err:?}"
    );
    assert!(!out.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_segmented_resume_continues_partial_parts_at_the_right_offset() {
    // 300_017 bytes in 4 segments: (0,75003) (75004,150007) (150008,225011) (225012,300016).
    // Part 0 is partial, part 1 is missing, part 2 is complete-up-to-10_000 bytes.
    let data = payload(300_017);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-resume");
    let out = dir.join("out.bin");
    std::fs::write(dir.join("out.bin.part0"), &data[..30_000]).unwrap();
    std::fs::write(dir.join("out.bin.part2"), &data[150_008..160_008]).unwrap();
    let mut opts = options();
    opts.segments = 4;
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(
        leftover_parts(&dir).is_empty(),
        "leftover: {:?}",
        leftover_parts(&dir)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_segmented_download_never_shows_a_half_merged_final_file() {
    // The merge happens in `name.part`; until the rename, `name` keeps its old content.
    let data = payload(100_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-atomic");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"previous version").unwrap();
    let mut opts = options();
    opts.segments = 2;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(leftover_parts(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

// ---- retries, validation and cleanup of segmented downloads ---------------------------------

use super::serve_with;
use std::path::Path;

/// `name.part<N>` files only (not `name.part` or the sidecar).
fn numbered_parts(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| {
            n.rsplit_once(".part")
                .is_some_and(|(_, d)| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    found.sort();
    found
}

fn resuming(segments: usize) -> rget::features::download::DownloadOptions {
    let mut opts = options();
    opts.segments = segments;
    opts.resume = true;
    opts
}

const SEGMENTED_PAYLOAD: usize = 300_017; // two segments: (0,150007) and (150008,300016)

#[test]
fn a_failed_segment_is_retried_and_continues_where_it_stopped() {
    let data = payload(SEGMENTED_PAYLOAD);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("seg-retry");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 2;
    opts.retries = 2;

    download_file(
        &format!("{base}/dropseg"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    // Segment 0 once, segment 1 twice: the retry asks only for the missing tail.
    assert_eq!(stats.ranged(), 3);
    assert!(numbered_parts(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_segment_that_fails_without_retries_fails_the_download_with_its_own_error() {
    let data = payload(SEGMENTED_PAYLOAD);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-fail");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 2;

    let err = download_file(
        &format!("{base}/dropseg"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::Network(_)),
        "expected the original error, got {err:?}"
    );
    assert!(!out.exists());
    assert_eq!(numbered_parts(&dir), vec!["out.bin.part0", "out.bin.part1"]);
    assert_eq!(
        std::fs::metadata(dir.join("out.bin.part0")).unwrap().len(),
        150_008
    );
    let sidecar = std::fs::read_to_string(dir.join("out.bin.part.meta")).unwrap();
    assert!(sidecar.contains("segments = 2"), "sidecar: {sidecar}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_later_run_continues_the_parts_a_failure_left_behind() {
    let data = payload(SEGMENTED_PAYLOAD);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("seg-continue");
    let out = dir.join("out.bin");
    let url = format!("{base}/dropseg");
    let mut first = options();
    first.segments = 2;
    assert!(download_file(&url, out.to_str().unwrap(), &first, None).is_err());
    assert_eq!(stats.ranged(), 2);

    download_file(&url, out.to_str().unwrap(), &resuming(2), None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    // Segment 0 was already complete; only the rest of segment 1 was requested.
    assert_eq!(stats.ranged(), 3);
    assert!(numbered_parts(&dir).is_empty());
    assert!(!dir.join("out.bin.part.meta").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_different_segment_count_starts_over_instead_of_misaligning_the_parts() {
    let data = payload(SEGMENTED_PAYLOAD);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("seg-recount");
    let out = dir.join("out.bin");
    let url = format!("{base}/dropseg");
    let mut first = options();
    first.segments = 2;
    assert!(download_file(&url, out.to_str().unwrap(), &first, None).is_err());

    download_file(&url, out.to_str().unwrap(), &resuming(3), None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(
        stats.ranged(),
        2 + 3,
        "all three new segments must be fetched from scratch"
    );
    assert!(numbered_parts(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_changed_remote_file_discards_the_parts() {
    let old = payload(SEGMENTED_PAYLOAD);
    let new: Vec<u8> = old.iter().map(|b| b.wrapping_add(1)).collect();
    let (base, _) = serve_with(new.clone(), true, "\"v2\"");
    let dir = scratch_dir("seg-changed");
    let out = dir.join("out.bin");
    std::fs::write(dir.join("out.bin.part0"), &old[..30_000]).unwrap();
    std::fs::write(dir.join("out.bin.part1"), &old[150_008..160_008]).unwrap();
    std::fs::write(
        dir.join("out.bin.part.meta"),
        format!("url = \"{base}/file\"\netag = \"\\\"v1\\\"\"\ntotal = {SEGMENTED_PAYLOAD}\nsegments = 2\n"),
    )
    .unwrap();

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(2),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), new);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_server_that_answers_200_to_range_requests_falls_back_to_one_connection() {
    let data = payload(90_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-lying");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;

    download_file(&format!("{base}/lying"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(
        std::fs::read(&out).unwrap(),
        data,
        "the file must not contain three copies"
    );
    assert!(numbered_parts(&dir).is_empty());
    assert!(!dir.join("out.bin.part.meta").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_oversized_part_is_discarded_not_trusted() {
    let data = payload(SEGMENTED_PAYLOAD);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-oversized");
    let out = dir.join("out.bin");
    std::fs::write(dir.join("out.bin.part0"), vec![0xEE; 200_000]).unwrap();

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(2),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn leftover_parts_beyond_the_segment_count_are_deleted() {
    let data = payload(SEGMENTED_PAYLOAD);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-leftover");
    let out = dir.join("out.bin");
    std::fs::write(dir.join("out.bin.part5"), b"stale").unwrap();
    std::fs::write(dir.join("out.bin.part9"), b"stale").unwrap();
    let mut opts = options();
    opts.segments = 2;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(
        numbered_parts(&dir).is_empty(),
        "left over: {:?}",
        numbered_parts(&dir)
    );
    std::fs::remove_dir_all(dir).unwrap();
}

// Roadmap item D10: a segment that exhausts its retries used to fail the download only
// once every other segment finished on its own, wasting bandwidth on a result that gets
// discarded. `/slowfail` refuses the high half permanently and trickles the low half out
// over roughly 1.5s, so this proves the trickling segment is cancelled instead of running
// to completion.
#[test]
fn a_permanently_failed_segment_cancels_its_siblings_instead_of_letting_them_finish() {
    let data = payload(SEGMENTED_PAYLOAD);
    let base = serve(data, true);
    let dir = scratch_dir("seg-cancel-siblings");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 2;

    let started = std::time::Instant::now();
    let err = download_file(
        &format!("{base}/slowfail"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(
        matches!(err, Error::HttpStatus { status: s, .. } if s.as_u16() == 403),
        "got {err:?}"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_millis(1200),
        "the trickling segment was not cancelled: took {:?}",
        started.elapsed()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_smaller_than_the_segment_count_downloads_correctly() {
    let data = payload(5);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("seg-tiny");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 8;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}
