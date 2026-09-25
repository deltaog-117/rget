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

//! The host policy while downloading: redirects and DNS names that lead somewhere local or
//! private are refused. (The initial URL is `validation`'s concern, so a loopback test server
//! can stand in for "a host that was let through".)

use super::{options, payload, scratch_dir, serve, serve_with};
use rget::features::download::{download_file, DownloadOptions, Error};
use rget::shared::address::HostPolicy;
use std::time::{Duration, Instant};

fn guarded() -> DownloadOptions {
    let mut opts = options();
    opts.host_policy = HostPolicy::BlockPrivate;
    opts
}

fn message_of(err: Error) -> String {
    match err {
        Error::BlockedAddress(message) => message,
        other => panic!("expected BlockedAddress, got {other:?}"),
    }
}

#[test]
fn a_redirect_into_a_private_address_is_refused_and_never_followed() {
    let (base, stats) = serve_with(payload(1_000), true, "\"v1\"");
    let dir = scratch_dir("hosts-redirect");
    let out = dir.join("out.bin");

    let err = download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &guarded(),
        None,
    )
    .unwrap_err();

    let message = message_of(err);
    assert!(
        message.contains("redirect to") && message.contains("127.0.0.1"),
        "{message}"
    );
    assert_eq!(stats.hits(), 1, "the redirect target must not be requested");
    assert!(!out.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_redirect_to_the_cloud_metadata_address_is_refused_without_connecting() {
    let base = serve(payload(1_000), true);
    let dir = scratch_dir("hosts-metadata");
    let out = dir.join("out.bin");

    let started = Instant::now();
    let err = download_file(
        &format!("{base}/redirect-metadata"),
        out.to_str().unwrap(),
        &guarded(),
        None,
    )
    .unwrap_err();

    assert!(message_of(err).contains("169.254.169.254"));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "no connection attempt should have been made"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn allowing_private_hosts_follows_the_same_redirect() {
    let data = payload(5_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("hosts-allowed");
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
fn a_refused_address_is_not_retried() {
    let (base, stats) = serve_with(payload(10), true, "\"v1\"");
    let dir = scratch_dir("hosts-noretry");
    let out = dir.join("out.bin");
    let mut opts = guarded();
    opts.retries = 3;

    let started = Instant::now();
    assert!(download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &opts,
        None
    )
    .is_err());

    assert_eq!(stats.hits(), 1);
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "no backoff should have been slept"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_name_that_always_means_this_machine_is_refused_before_any_lookup() {
    let (base, stats) = serve_with(payload(10), true, "\"v1\"");
    let dir = scratch_dir("hosts-localname");
    let out = dir.join("out.bin");
    let url = base.replace("127.0.0.1", "localhost");
    assert!(url.contains("localhost"), "{url}");

    let err = download_file(
        &format!("{url}/file"),
        out.to_str().unwrap(),
        &guarded(),
        None,
    )
    .unwrap_err();

    assert!(message_of(err).contains("localhost"));
    assert_eq!(stats.hits(), 0, "the server must never have been contacted");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn allowing_private_hosts_reaches_a_local_name() {
    let data = payload(2_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("hosts-localname-allowed");
    let out = dir.join("out.bin");
    let url = base.replace("127.0.0.1", "localhost");

    download_file(
        &format!("{url}/file"),
        out.to_str().unwrap(),
        &options(),
        None,
    )
    .unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_segmented_download_is_guarded_too() {
    let (base, stats) = serve_with(payload(30_000), true, "\"v1\"");
    let dir = scratch_dir("hosts-segmented");
    let out = dir.join("out.bin");
    let mut opts = guarded();
    opts.segments = 3;

    let err = download_file(
        &format!("{base}/redirect"),
        out.to_str().unwrap(),
        &opts,
        None,
    )
    .unwrap_err();

    assert!(matches!(err, Error::BlockedAddress(_)), "got {err:?}");
    assert!(!out.exists());
    assert!(
        stats.hits() <= 2,
        "only the probe and the single-connection fallback may have reached the server"
    );
    std::fs::remove_dir_all(dir).unwrap();
}
