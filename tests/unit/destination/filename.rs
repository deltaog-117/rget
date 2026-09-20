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


use rget::features::destination::file_name_for;
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
fn an_explicit_name_wins() {
    assert_eq!(name("http://example.com/file.zip", Some("mine.bin")), "mine.bin");
}

// Characterization of 1.0.0; roadmap item C3 percent-decodes and sanitizes the name.
#[test]
fn known_defect_percent_escapes_are_kept_verbatim() {
    assert_eq!(name("http://example.com/a%20b.txt", None), "a%20b.txt");
}
