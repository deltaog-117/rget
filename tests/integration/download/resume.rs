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
use rget::features::download::download_file;

#[test]
fn a_partial_file_is_completed_from_where_it_stopped() {
    let data = payload(200_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-partial");
    let out = dir.join("out.bin");
    std::fs::write(&out, &data[..70_000]).unwrap();
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn resume_with_no_existing_file_is_a_normal_download() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-fresh");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_server_without_range_support_restarts_from_scratch() {
    let data = payload(20_000);
    let base = serve(data.clone(), false);
    let dir = scratch_dir("resume-norange");
    let out = dir.join("out.bin");
    std::fs::write(&out, vec![9u8; 5_000]).unwrap();
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

use super::serve_with;
use rget::features::download::Error;
use std::path::Path;

fn part_of(out: &Path) -> std::path::PathBuf {
    let mut name = out.as_os_str().to_owned();
    name.push(".part");
    name.into()
}

fn meta_of(out: &Path) -> std::path::PathBuf {
    let mut name = part_of(out).into_os_string();
    name.push(".meta");
    name.into()
}

/// Leaves what an interrupted download would: `name.part` plus its sidecar.
fn leave_partial(out: &Path, data: &[u8], url: &str, etag: &str) {
    std::fs::write(part_of(out), data).unwrap();
    std::fs::write(meta_of(out), format!("url = {url:?}\netag = {etag:?}\n")).unwrap();
}

fn resuming() -> rget::features::download::DownloadOptions {
    let mut opts = options();
    opts.resume = true;
    opts
}

#[test]
fn a_finished_download_leaves_no_part_file_or_sidecar() {
    let data = payload(30_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-clean");
    let out = dir.join("out.bin");

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(!part_of(&out).exists());
    assert!(!meta_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn resuming_a_complete_file_is_a_success_and_leaves_it_alone() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-complete");
    let out = dir.join("out.bin");
    std::fs::write(&out, &data).unwrap();

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(!part_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_part_file_from_an_earlier_run_is_continued_after_validation() {
    let data = payload(200_000);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("resume-part");
    let out = dir.join("out.bin");
    leave_partial(&out, &data[..70_000], &format!("{base}/file"), "\"v1\"");

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(
        stats.ranged(),
        1,
        "the transfer should have continued, not restarted"
    );
    assert!(!part_of(&out).exists() && !meta_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_changed_remote_file_restarts_instead_of_producing_a_corrupt_file() {
    let old = payload(200_000);
    let new: Vec<u8> = old.iter().map(|b| b.wrapping_add(1)).collect();
    let (base, _) = serve_with(new.clone(), true, "\"v2\"");
    let dir = scratch_dir("resume-changed");
    let out = dir.join("out.bin");
    leave_partial(&out, &old[..70_000], &format!("{base}/file"), "\"v1\"");

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), new);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_part_file_belonging_to_another_url_is_not_continued() {
    let data = payload(50_000);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("resume-otherurl");
    let out = dir.join("out.bin");
    leave_partial(
        &out,
        &vec![0xEE; 20_000],
        "http://example.invalid/other",
        "\"v1\"",
    );

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(stats.ranged(), 0);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_remote_file_that_shrank_is_downloaded_again() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-shrunk");
    let out = dir.join("out.bin");
    std::fs::write(&out, vec![0xAB; 50_000]).unwrap();

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &resuming(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn without_resume_a_stale_part_file_is_discarded() {
    let data = payload(40_000);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("resume-stale");
    let out = dir.join("out.bin");
    leave_partial(&out, &vec![0xEE; 30_000], &format!("{base}/file"), "\"v1\"");

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(stats.ranged(), 0);
    assert!(!part_of(&out).exists() && !meta_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_interrupted_download_keeps_its_progress_and_spares_the_existing_file() {
    let base = serve(payload(100_000), true);
    let dir = scratch_dir("resume-interrupted");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"previous version").unwrap();
    let mut opts = options();
    opts.timeout = 1;

    let err =
        download_file(&format!("{base}/stall"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::Stalled(1)), "got {err:?}");
    assert_eq!(std::fs::read(&out).unwrap(), b"previous version");
    assert_eq!(std::fs::metadata(part_of(&out)).unwrap().len(), 50_000);
    assert!(std::fs::read_to_string(meta_of(&out))
        .unwrap()
        .contains("/stall"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_device_is_written_directly_without_a_part_file() {
    let base = serve(payload(10_000), true);

    download_file(&format!("{base}/file"), "/dev/null", &options(), None).unwrap();

    assert!(!Path::new("/dev/null.part").exists());
}
