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
        assert!(leftover_parts(&dir).is_empty(), "part files were not cleaned up");
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

    assert!(download_file("http://127.0.0.1:1/file", out.to_str().unwrap(), &opts, None).is_err());
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

    download_file(&format!("{base}/guarded"), out.to_str().unwrap(), &opts, None).unwrap();

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

    let err = download_file(&format!("{base}/guarded"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::HttpStatus { status: s, .. } if s.as_u16() == 403), "got {err:?}");
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

    let err = download_file(&format!("{base}/redirect"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::RedirectDisabled(302, _)), "got {err:?}");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_error_status_on_the_probe_is_reported_not_saved() {
    let base = serve(payload(10), true);
    let dir = scratch_dir("seg-404");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.segments = 3;

    let err = download_file(&format!("{base}/status/404"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::HttpStatus { status: s, .. } if s.as_u16() == 404), "got {err:?}");
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
    assert!(leftover_parts(&dir).is_empty(), "leftover: {:?}", leftover_parts(&dir));
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
