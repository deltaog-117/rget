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

#[test]
fn downloads_the_whole_file() {
    let data = payload(200_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("single-full");
    let out = dir.join("out.bin");

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_existing_file_is_overwritten_without_resume() {
    let data = payload(10_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("single-overwrite");
    let out = dir.join("out.bin");
    std::fs::write(&out, vec![0u8; 50_000]).unwrap();

    download_file(
        &format!("{base}/file"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_redirect_is_followed_by_default() {
    let data = payload(5_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("single-redirect");
    let out = dir.join("out.bin");

    download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_redirect_is_an_error_when_following_is_disabled() {
    let base = serve(payload(10), true);
    let dir = scratch_dir("single-noredirect");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.follow_redirects = false;

    let err = download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    match err {
        Error::RedirectDisabled(status, location) => {
            assert_eq!(status, 302);
            assert_eq!(location, "/file");
        }
        other => panic!("expected RedirectDisabled, got {other:?}"),
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_refused_connection_is_a_network_error() {
    let dir = scratch_dir("single-refused");
    let out = dir.join("out.bin");

    let err = download_file(
        "http://127.0.0.1:1/file",
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap_err();

    assert!(matches!(err, Error::Network(_)));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_error_status_is_an_error_and_writes_nothing() {
    let base = serve(payload(10), true);
    for code in [404u16, 500, 403] {
        let dir = scratch_dir(&format!("single-status-{code}"));
        let out = dir.join("out.bin");

        let err = download_file(
            &format!("{base}/status/{code}"),
            out.to_str().unwrap(),
            &options(),
            None,
        )
        .unwrap_err();

        match err {
            Error::HttpStatus { status, .. } => assert_eq!(status.as_u16(), code),
            other => panic!("expected HttpStatus({code}), got {other:?}"),
        }
        assert!(!out.exists(), "an error page must not be saved as the file");
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn an_error_status_leaves_an_existing_file_untouched() {
    let base = serve(payload(10), true);
    let dir = scratch_dir("single-status-keep");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"precious").unwrap();

    assert!(download_file(
        &format!("{base}/status/404"),
        out.to_str().unwrap(),
        &options(),
        None
    )
    .is_err());

    assert_eq!(std::fs::read(&out).unwrap(), b"precious");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_download_may_outlast_the_timeout_while_data_keeps_flowing() {
    // Three seconds of trickling against a one second timeout: the timeout is a stall
    // limit, not a cap on the whole transfer.
    let data = payload(100_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("single-trickle");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.timeout = 1;

    let started = std::time::Instant::now();
    download_file(
        &format!("{base}/trickle"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap();

    assert!(started.elapsed() > std::time::Duration::from_millis(2500));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_stalled_transfer_fails_after_the_timeout() {
    let base = serve(payload(100_000), true);
    let dir = scratch_dir("single-stall");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.timeout = 1;

    let started = std::time::Instant::now();
    let err =
        download_file(&format!("{base}/stall"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::Stalled(1)), "got {err:?}");
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_rate_limit_slows_the_transfer() {
    let data = payload(200_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("single-limit");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.limit_rate = Some(100_000);

    let started = std::time::Instant::now();
    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();
    let elapsed = started.elapsed();

    assert!(
        elapsed > std::time::Duration::from_millis(900),
        "finished too fast: {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(6),
        "finished too slowly: {elapsed:?}"
    );
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_error_status_creates_no_part_file() {
    let base = serve(payload(10), true);
    let dir = scratch_dir("single-status-nopart");
    let out = dir.join("out.bin");

    assert!(download_file(
        &format!("{base}/status/500"),
        out.to_str().unwrap(),
        &options(),
        None
    )
    .is_err());

    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        0,
        "nothing should be left behind"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
