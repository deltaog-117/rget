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


use rget::features::validation::{validate_url, Error};

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
fn private_ipv4_ranges_are_blocked() {
    assert_blocked("http://192.168.1.1/x");
    assert_blocked("http://10.0.0.1/x");
    assert_blocked("http://172.16.0.1/x");
    assert_blocked("http://172.31.255.255/x");
}

#[test]
fn private_ipv6_ranges_are_blocked() {
    assert_blocked("http://[fd00::1]/x");
    assert_blocked("http://[fc00::1]/x");
    assert_blocked("http://[fe80::1]/x");
}

#[test]
fn public_addresses_pass() {
    assert!(validate_url("http://8.8.8.8/x").is_ok());
    assert!(validate_url("http://172.32.0.1/x").is_ok());
    assert!(validate_url("http://[2001:4860:4860::8888]/x").is_ok());
}

// Characterization of 1.0.0: the check is string-prefix matching. Roadmap item C1
// replaces it with real address classification, and these tests flip to `assert_blocked`.

#[test]
fn known_defect_other_loopback_addresses_are_allowed() {
    assert!(validate_url("http://127.0.0.2/x").is_ok());
    assert!(validate_url("http://0.0.0.0/x").is_ok());
}

#[test]
fn known_defect_link_local_metadata_address_is_allowed() {
    assert!(validate_url("http://169.254.169.254/latest/meta-data").is_ok());
}
