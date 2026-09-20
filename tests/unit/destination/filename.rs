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
use rget::features::destination::file_name_for;
use std::path::{Component, Path};
use url::Url;

fn name(url: &str, explicit: Option<&str>) -> String {
    file_name_for(&Url::parse(url).unwrap(), explicit)
}

#[test]
fn uses_the_last_path_segment() {
    assert_eq!(name("http://example.com/a/b/file.zip", None), "file.zip");
}

#[test]
fn the_query_string_is_not_part_of_the_name() {
    assert_eq!(name("http://example.com/file.zip?token=1", None), "file.zip");
}

#[test]
fn a_trailing_slash_or_bare_host_falls_back_to_downloaded() {
    assert_eq!(name("http://example.com/dir/", None), "downloaded");
    assert_eq!(name("http://example.com", None), "downloaded");
}

#[test]
fn an_explicit_name_wins_and_is_used_exactly_as_given() {
    assert_eq!(name("http://example.com/file.zip", Some("mine.bin")), "mine.bin");
    assert_eq!(name("http://example.com/file.zip", Some("sub/dir/../-x")), "sub/dir/../-x");
}

// ---- percent-decoding (roadmap item C3) -----------------------------------------------------

#[test]
fn percent_escapes_are_decoded() {
    assert_eq!(name("http://example.com/a%20b.txt", None), "a b.txt");
    assert_eq!(name("http://example.com/caf%C3%A9.txt", None), "café.txt");
    assert_eq!(name("http://example.com/100%25.txt", None), "100%.txt");
}

#[test]
fn bytes_that_are_not_valid_utf8_are_left_as_written() {
    assert_eq!(name("http://example.com/bad%FFname.txt", None), "bad%FFname.txt");
}

#[test]
fn an_encoded_slash_cannot_leave_the_directory() {
    assert_eq!(name("http://example.com/a%2Fb.txt", None), "a_b.txt");
    assert_eq!(name("http://example.com/a%5Cb.txt", None), "a_b.txt");
    assert_eq!(name("http://example.com/..%2F..%2Fetc%2Fcron.d%2Fx", None), ".._.._etc_cron.d_x");
}

#[test]
fn control_characters_and_nul_are_replaced() {
    assert_eq!(name("http://example.com/a%00b.txt", None), "a_b.txt");
    assert_eq!(name("http://example.com/a%0Ab.txt", None), "a_b.txt");
}

#[test]
fn a_leading_dash_is_replaced_so_a_shell_glob_cannot_read_it_as_an_option() {
    assert_eq!(name("http://example.com/-rf", None), "_rf");
    assert_eq!(name("http://example.com/%2Drf", None), "_rf");
}

#[test]
fn names_that_decode_to_nothing_useful_fall_back() {
    for encoded in ["%20", "%2E", "%2E%2E"] {
        assert_eq!(name(&format!("http://example.com/{encoded}"), None), "downloaded", "{encoded}");
    }
    // A name made only of replaced characters is harmless and simply kept.
    assert_eq!(name("http://example.com/%09", None), "_");
}

#[test]
fn unusual_but_legal_names_survive() {
    assert_eq!(name("http://example.com/file(1).zip", None), "file(1).zip");
    assert_eq!(name("http://example.com/app;v=1.tgz", None), "app;v=1.tgz");
    assert_eq!(name("http://example.com/.env.example", None), ".env.example");
}

#[test]
fn a_very_long_name_is_shortened_but_keeps_its_extension() {
    let long = format!("http://example.com/{}.tar.gz", "x".repeat(400));
    let shortened = name(&long, None);
    assert!(shortened.len() <= 240 && shortened.ends_with(".tar.gz"), "{}", shortened.len());
}

// ---- properties -----------------------------------------------------------------------------

fn assert_safe(candidate: &str) {
    let path = Path::new(candidate);
    let parts: Vec<_> = path.components().collect();
    assert!(
        matches!(parts.as_slice(), [Component::Normal(_)]),
        "{candidate:?} must be exactly one ordinary path component, got {parts:?}"
    );
    assert!(candidate != "." && candidate != "..", "{candidate:?}");
    assert!(candidate.len() <= 240, "{candidate:?} is {} bytes", candidate.len());
    assert!(!candidate.starts_with('-'), "{candidate:?}");
    assert!(!candidate.chars().any(char::is_control), "{candidate:?}");
    // The property that matters: joined to a directory, it stays directly inside it.
    let dir = Path::new("/home/user/Downloads");
    assert_eq!(dir.join(candidate).parent(), Some(dir), "{candidate:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4_000))]

    #[test]
    fn any_segment_however_encoded_yields_a_name_that_stays_inside_the_directory(segment in any::<String>()) {
        // Fully encoded, so every character (including `/` and NUL) arrives as an escape.
        let encoded = urlencoding::encode(&segment).into_owned();
        assert_safe(&name(&format!("http://example.com/{encoded}"), None));

        // And pushed through the URL library's own segment encoder.
        let mut url = Url::parse("http://example.com/").unwrap();
        url.path_segments_mut().unwrap().push(&segment);
        assert_safe(&file_name_for(&url, None));
    }

    #[test]
    fn traversal_attempts_in_every_encoding_stay_inside_the_directory(
        prefix in "[a-z]{0,5}",
        dots in prop::sample::select(vec!["..", "%2e%2e", "%2E%2E", ".%2e", "..%2f", "%2e%2e%2f", "..%5c"]),
        suffix in "[a-z/%0-9A-F]{0,12}",
    ) {
        assert_safe(&name(&format!("http://example.com/{prefix}{dots}{suffix}"), None));
    }

    #[test]
    fn encoded_names_round_trip_to_themselves_when_already_safe(
        // Starts and ends with a non-space, so nothing here is subject to trimming.
        stem in "[a-zA-Z0-9]([a-zA-Z0-9 _.()+,=-]{0,30}[a-zA-Z0-9_.()+,=-])?",
    ) {
        let encoded = urlencoding::encode(&stem).into_owned();
        prop_assert_eq!(name(&format!("http://example.com/{encoded}"), None), stem);
    }
}
