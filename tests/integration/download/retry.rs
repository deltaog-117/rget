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


use super::{options, payload, scratch_dir, serve, serve_with};
use rget::features::download::{download_file, Error};
use std::time::{Duration, Instant};

#[test]
fn a_permanent_error_is_not_retried() {
    let (base, stats) = serve_with(payload(10), true, "\"v1\"");
    let dir = scratch_dir("retry-404");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 3;

    let started = Instant::now();
    let err = download_file(&format!("{base}/status/404"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::HttpStatus { .. }));
    assert_eq!(stats.hits(), 1, "a 404 must not be retried");
    assert!(started.elapsed() < Duration::from_millis(900), "no backoff should have been slept");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_transient_status_is_retried_until_it_succeeds() {
    let data = payload(20_000);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("retry-503");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 2;

    download_file(&format!("{base}/once/503"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(stats.hits(), 2);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn retry_after_is_honoured() {
    let data = payload(1_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("retry-after");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 1;

    // /once/429 says "Retry-After: 1"; the plain backoff for the first retry is also about
    // a second, so this checks the floor rather than the exact value.
    let started = Instant::now();
    download_file(&format!("{base}/once/429"), out.to_str().unwrap(), &opts, None).unwrap();

    assert!(started.elapsed() >= Duration::from_secs(1));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_unreasonably_long_retry_after_gives_up_at_once() {
    let (base, stats) = serve_with(payload(10), true, "\"v1\"");
    let dir = scratch_dir("retry-long");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 3;

    let started = Instant::now();
    let err = download_file(&format!("{base}/long/503"), out.to_str().unwrap(), &opts, None).unwrap_err();

    match err {
        Error::HttpStatus { status, retry_after } => {
            assert_eq!(status.as_u16(), 503);
            assert_eq!(retry_after, Some(Duration::from_secs(300)));
        }
        other => panic!("expected HttpStatus, got {other:?}"),
    }
    assert_eq!(stats.hits(), 1);
    assert!(started.elapsed() < Duration::from_secs(2));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_dropped_connection_is_resumed_not_restarted() {
    let data = payload(200_000);
    let (base, stats) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("retry-dropmid");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 2;

    download_file(&format!("{base}/dropmid"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert_eq!(stats.hits(), 2);
    assert_eq!(stats.ranged(), 1, "the retry should ask only for the missing bytes");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_dropped_connection_on_a_server_without_ranges_restarts_cleanly() {
    let data = payload(100_000);
    let (base, _) = serve_with(data.clone(), false, "\"v1\"");
    let dir = scratch_dir("retry-dropmid-noranges");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.retries = 2;

    download_file(&format!("{base}/dropmid"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}
