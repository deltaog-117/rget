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

fn assert_invalid(url: &str) {
    match validate_url(url) {
        Err(Error::InvalidUrl(_)) => {}
        other => panic!("expected InvalidUrl for {url}, got {other:?}"),
    }
}

#[test]
fn ordinary_urls_pass() {
    assert!(validate_url("https://example.com/file.zip").is_ok());
    assert!(validate_url("http://example.com/a/b/c.tar.gz").is_ok());
    assert!(validate_url("https://example.com:8443/x").is_ok());
}

#[test]
fn only_http_and_https_are_allowed() {
    assert_invalid("ftp://example.com/x");
    assert_invalid("file:///etc/hostname");
    assert_invalid("not a url");
}

#[test]
fn shell_metacharacters_are_rejected() {
    for c in [';', '|', '&', '$', '`', '(', ')', '<', '>'] {
        assert_invalid(&format!("http://example.com/a{c}b"));
    }
}

#[test]
fn encoded_shell_metacharacters_are_rejected() {
    assert_invalid("http://example.com/a%3Bb");
    assert_invalid("http://example.com/a%7Cb");
    assert_invalid("http://example.com/a%26b");
}

#[test]
fn path_traversal_is_rejected() {
    assert_invalid("http://example.com/../etc/passwd");
    assert_invalid("http://example.com/a/..\\b");
}

#[test]
fn sensitive_file_patterns_are_rejected() {
    for path in [
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/.env",
        "/.git/config",
        "/.aws/credentials",
        "/.ssh/id_rsa",
        "/.ssh/authorized_keys",
        "/.bashrc",
        "/.zshrc",
    ] {
        assert_invalid(&format!("http://example.com{path}"));
    }
    assert_invalid("http://example.com/.ENV");
}

// Characterization of 1.0.0: the checks below are too blunt. Roadmap item A3 fixes
// them, and these three tests then flip to `is_ok()`.

#[test]
fn known_defect_query_strings_with_ampersands_are_rejected() {
    assert_invalid("http://example.com/?a=1&b=2");
}

#[test]
fn known_defect_hosts_containing_dot_env_are_rejected() {
    assert_invalid("http://foo.environment.com/x");
}

#[test]
fn known_defect_parentheses_in_paths_are_rejected() {
    assert_invalid("http://example.com/file(1).zip");
}
