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


use proptest::prelude::*;
use rget::features::validation::{validate_url, Error};

fn assert_invalid(url: &str, expected_in_message: &str) {
    match validate_url(url) {
        Err(Error::InvalidUrl(message)) => assert!(
            message.contains(expected_in_message),
            "{url}: message {message:?} should mention {expected_in_message:?}"
        ),
        other => panic!("expected InvalidUrl for {url}, got {other:?}"),
    }
}

fn assert_ok(url: &str) {
    if let Err(e) = validate_url(url) {
        panic!("{url} should be accepted, got {e:?}");
    }
}

#[test]
fn ordinary_urls_pass() {
    assert_ok("https://example.com/file.zip");
    assert_ok("http://example.com/a/b/c.tar.gz");
    assert_ok("https://example.com:8443/x");
    assert_ok("https://user:secret@example.com/x");
}

#[test]
fn only_http_and_https_are_allowed() {
    assert_invalid("ftp://example.com/x", "Only HTTP/HTTPS");
    assert_invalid("file:///etc/hostname", "Only HTTP/HTTPS");
    assert_invalid("not a url", "not a url");
}

// ---- what used to be refused and is now accepted (roadmap item A3) ------------------------

#[test]
fn characters_that_are_legal_in_a_path_or_query_are_accepted() {
    for c in [';', '|', '&', '$', '(', ')', '\'', '!', '*', ',', '=', ':', '@', '+'] {
        assert_ok(&format!("http://example.com/a{c}b"));
        assert_ok(&format!("http://example.com/?q=a{c}b"));
    }
}

#[test]
fn characters_the_parser_percent_encodes_are_accepted_too() {
    for c in ['<', '>', '`', ' '] {
        assert_ok(&format!("http://example.com/a{c}b"));
    }
}

#[test]
fn encoded_reserved_characters_are_accepted() {
    for encoded in ["%26", "%3B", "%7C", "%24", "%28", "%29"] {
        assert_ok(&format!("http://example.com/a{encoded}b?q=a{encoded}b"));
    }
}

#[test]
fn query_strings_with_ampersands_are_accepted() {
    assert_ok("http://example.com/?a=1&b=2");
    assert_ok("https://www.example.com/search?q=rust+lang&page=2&sort=asc#results");
}

#[test]
fn parentheses_in_paths_are_accepted() {
    assert_ok("http://example.com/file(1).zip");
    assert_ok("https://en.wikipedia.org/wiki/Rust_(programming_language)");
}

#[test]
fn path_parameters_and_session_ids_are_accepted() {
    assert_ok("http://example.com/app;jsessionid=ABC123/page");
    assert_ok("http://example.com/x?token=$HOME|$(id)");
}

#[test]
fn hosts_that_merely_contain_a_sensitive_word_are_accepted() {
    assert_ok("http://foo.environment.com/x");
    assert_ok("http://foo.env.example.org/x");
    assert_ok("https://bashrc.dev/x");
}

#[test]
fn dots_that_are_not_traversal_are_accepted() {
    assert_ok("http://example.com/a..b");
    assert_ok("http://example.com/dir../file");
    assert_ok("http://example.com/.../x");
    assert_ok("http://example.com/file..txt");
    assert_ok("http://example.com/x?next=../home");
    assert_ok("http://example.com/x#../top");
    // A slash before the dots makes them a whole segment, but in the query or fragment that
    // is still just text.
    assert_ok("http://example.com/x?next=/../home");
    assert_ok("http://example.com/x?a=1&next=/..");
    assert_ok("http://example.com/x#/../top");
    assert_ok("http://example.com?next=/../home");
    assert_ok("http://example.com#/../top");
}

#[test]
fn names_that_only_resemble_secret_files_are_accepted() {
    assert_ok("http://example.com/.env.example");
    assert_ok("http://example.com/environment");
    assert_ok("http://example.com/.envrc");
    assert_ok("http://example.com/etc/passwd.bak");
    assert_ok("http://example.com/x?file=.env");
}

// ---- what is still refused ------------------------------------------------------------------

#[test]
fn hostnames_no_hostname_can_contain_are_rejected() {
    for c in [';', '&', '$', '(', ')', '`', '!', '*', '\'', '=', '|'] {
        match validate_url(&format!("http://exa{c}mple.com/x")) {
            Err(Error::InvalidUrl(_)) => {}
            other => panic!("host with {c:?} should be rejected, got {other:?}"),
        }
    }
    assert_invalid("http://exa;mple.com/x", "hostname contains invalid character ';'");
}

#[test]
fn path_traversal_is_rejected_in_every_spelling() {
    for url in [
        "http://example.com/../etc/passwd",
        "http://example.com/a/../b",
        "http://example.com/a/..",
        "http://example.com/a/%2e%2e/b",
        "http://example.com/a/%2E%2E/b",
        "http://example.com/a/.%2e/b",
        "http://example.com/a/..\\b",
        "http://example.com/a/..%2fb",
        "http://example.com/a/..%5cb",
    ] {
        assert_invalid(url, "Path traversal");
    }
}

#[test]
fn sensitive_file_paths_are_rejected() {
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
        "/site/.ENV",
        "/home/user/.zshrc",
        "/%2eenv",
        "/etc%2fpasswd",
    ] {
        assert_invalid(&format!("http://example.com{path}"), "sensitive file");
    }
}

#[test]
fn the_message_names_the_sensitive_file() {
    assert_invalid("http://example.com/x/.env", "'.env'");
    assert_invalid("http://example.com/etc/passwd", "'etc/passwd'");
}

// ---- properties -----------------------------------------------------------------------------

const SEGMENT: &str = "[a-z0-9][a-z0-9._~!$&'()*+,;=:@-]{0,10}";
const QUERY: &str = "([a-zA-Z0-9._~!$&'()*+,;=:@/?-]|%[0-9a-f]{2}){0,20}";

proptest! {
    #[test]
    fn no_input_can_make_validation_panic(text in any::<String>()) {
        let _ = validate_url(&text);
    }

    #[test]
    fn urlish_text_never_panics_either(text in "[a-zA-Z0-9:/?#\\[\\]@!$&'()*+,;=%.\\\\ -]{0,60}") {
        let _ = validate_url(&text);
        let _ = validate_url(&format!("http://{text}"));
        let _ = validate_url(&format!("https://example.com/{text}"));
    }

    #[test]
    fn any_legal_path_and_query_is_accepted(
        segments in proptest::collection::vec(SEGMENT, 0..5),
        query in QUERY,
        // Dot-dot segments are fine in a query or fragment; only the path is judged.
        tail in prop::sample::select(vec!["", "/../x", "/..", "&next=/../", "#/../top"]),
    ) {
        prop_assume!(!segments.iter().any(|s| s == "etc"));
        let url = format!("http://example.com/{}?{}{}", segments.join("/"), query, tail);
        prop_assert!(validate_url(&url).is_ok(), "{url}");
    }

    #[test]
    fn a_dot_dot_segment_is_rejected_whatever_the_encoding(
        before in SEGMENT,
        after in SEGMENT,
        dots in prop::sample::select(vec!["..", "%2e%2e", "%2E%2E", ".%2e", "%2e.", "..%2f", "..%5c"]),
        separator in prop::sample::select(vec!["/", "\\"]),
    ) {
        let url = format!("http://example.com/{before}{separator}{dots}{separator}{after}");
        prop_assert!(
            matches!(validate_url(&url), Err(Error::InvalidUrl(m)) if m.contains("traversal")),
            "{url}"
        );
    }

    #[test]
    fn dots_inside_a_name_never_count_as_traversal(
        before in SEGMENT,
        name in "[a-z]{1,4}(\\.\\.|\\.\\.\\.)[a-z]{0,4}",
        after in SEGMENT,
    ) {
        prop_assume!(before != "etc");
        let url = format!("http://example.com/{before}/{name}/{after}");
        prop_assert!(validate_url(&url).is_ok(), "{url}");
    }

    #[test]
    fn a_forbidden_character_in_the_host_is_always_rejected(
        c in prop::sample::select(vec![';', '&', '$', '(', ')', '`', '!', '*', '\'', '=', '|']),
        left in "[a-z]{1,6}",
        right in "[a-z]{1,6}",
    ) {
        let url = format!("http://{left}{c}{right}.com/x");
        prop_assert!(matches!(validate_url(&url), Err(Error::InvalidUrl(_))), "{url}");
    }
}
