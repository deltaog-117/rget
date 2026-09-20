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


use rget::features::validation::{validate_url, validate_url_with, Error};
use rget::shared::address::HostPolicy;

fn assert_blocked(url: &str) {
    match validate_url(url) {
        Err(Error::BlockedUrl(_)) => {}
        other => panic!("expected BlockedUrl for {url}, got {other:?}"),
    }
}

#[test]
fn loopback_hosts_are_blocked() {
    assert_blocked("http://localhost/x");
    assert_blocked("http://127.0.0.1/x");
    assert_blocked("http://[::1]/x");
}

#[test]
fn every_address_in_the_loopback_block_is_blocked() {
    for ip in ["127.0.0.2", "127.1.2.3", "127.255.255.255"] {
        assert_blocked(&format!("http://{ip}/x"));
    }
}

#[test]
fn private_ipv4_ranges_are_blocked() {
    assert_blocked("http://192.168.1.1/x");
    assert_blocked("http://10.0.0.1/x");
    assert_blocked("http://172.16.0.1/x");
    assert_blocked("http://172.31.255.255/x");
}

#[test]
fn the_cloud_metadata_address_and_other_link_local_addresses_are_blocked() {
    assert_blocked("http://169.254.169.254/latest/meta-data");
    assert_blocked("http://169.254.0.1/x");
}

#[test]
fn other_non_public_ipv4_ranges_are_blocked() {
    for ip in ["0.0.0.0", "0.1.2.3", "100.64.0.1", "100.127.255.254", "192.0.0.1", "198.18.0.1", "224.0.0.1", "240.0.0.1", "255.255.255.255"] {
        assert_blocked(&format!("http://{ip}/x"));
    }
}

#[test]
fn private_ipv6_ranges_are_blocked() {
    assert_blocked("http://[fd00::1]/x");
    assert_blocked("http://[fc00::1]/x");
    assert_blocked("http://[fe80::1]/x");
    assert_blocked("http://[::]/x");
    assert_blocked("http://[ff02::1]/x");
}

#[test]
fn ipv6_addresses_that_wrap_a_private_ipv4_address_are_blocked() {
    for host in ["::ffff:127.0.0.1", "::ffff:7f00:1", "::ffff:169.254.169.254", "::ffff:10.0.0.1", "64:ff9b::7f00:1", "2002:7f00:1::"] {
        assert_blocked(&format!("http://[{host}]/x"));
    }
}

#[test]
fn numeric_spellings_are_normalised_and_blocked() {
    assert_blocked("http://2130706433/x");
    assert_blocked("http://0x7f.1/x");
    assert_blocked("http://0177.0.0.1/x");
}

#[test]
fn localhost_and_everything_under_it_is_blocked() {
    for host in ["localhost", "LOCALHOST", "localhost.", "foo.localhost", "a.b.localhost."] {
        assert_blocked(&format!("http://{host}/x"));
    }
}

#[test]
fn public_addresses_and_names_pass() {
    assert!(validate_url("http://8.8.8.8/x").is_ok());
    assert!(validate_url("http://172.32.0.1/x").is_ok());
    assert!(validate_url("http://100.63.255.255/x").is_ok());
    assert!(validate_url("http://[2001:4860:4860::8888]/x").is_ok());
    assert!(validate_url("http://[::ffff:8.8.8.8]/x").is_ok());
    assert!(validate_url("https://example.com/x").is_ok());
    assert!(validate_url("https://localhost.example.com/x").is_ok());
    assert!(validate_url("https://notlocalhost/x").is_ok());
}

#[test]
fn a_public_looking_name_passes_because_only_downloading_can_tell_where_it_leads() {
    // Names are judged when they are resolved, and redirects when they are followed; both
    // happen in the download feature.
    assert!(validate_url("http://internal.example.com/x").is_ok());
}

#[test]
fn the_error_names_the_offending_host() {
    match validate_url("http://169.254.169.254/x") {
        Err(Error::BlockedUrl(message)) => assert!(message.contains("169.254.169.254"), "{message}"),
        other => panic!("{other:?}"),
    }
    match validate_url("http://[::1]/x") {
        Err(Error::BlockedUrl(message)) => assert!(message.contains("[::1]"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn allowing_private_hosts_lifts_every_host_block() {
    for url in [
        "http://localhost/x",
        "http://127.0.0.1/x",
        "http://127.0.0.2/x",
        "http://169.254.169.254/x",
        "http://192.168.1.1/x",
        "http://[::1]/x",
        "http://[::ffff:127.0.0.1]/x",
        "http://foo.localhost/x",
        "http://0.0.0.0/x",
    ] {
        assert!(validate_url_with(url, HostPolicy::AllowPrivate).is_ok(), "{url}");
    }
}

#[test]
fn allowing_private_hosts_does_not_relax_the_other_checks() {
    for url in ["ftp://127.0.0.1/x", "http://127.0.0.1/../etc", "http://127.0.0.1/.env", "http://exa;mple.com/x"] {
        assert!(matches!(validate_url_with(url, HostPolicy::AllowPrivate), Err(Error::InvalidUrl(_))), "{url}");
    }
}
