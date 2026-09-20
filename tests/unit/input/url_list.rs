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


use rget::features::input::read_url_list;

#[test]
fn blank_and_whitespace_only_lines_are_skipped() {
    let path = std::env::temp_dir().join(format!("rget-urls-{}", std::process::id()));
    std::fs::write(&path, "http://a.example/1\n\n   \nhttp://b.example/2\n").unwrap();
    let urls = read_url_list(path.to_str().unwrap()).unwrap();
    assert_eq!(urls, vec!["http://a.example/1", "http://b.example/2"]);
    std::fs::remove_file(path).unwrap();
}

// Characterization of 1.0.0; roadmap item D6 trims lines and skips `#` comments.
#[test]
fn known_defect_lines_are_not_trimmed_and_comments_are_kept() {
    let path = std::env::temp_dir().join(format!("rget-urls-raw-{}", std::process::id()));
    std::fs::write(&path, "  http://a.example/1 \n# comment\n").unwrap();
    let urls = read_url_list(path.to_str().unwrap()).unwrap();
    assert_eq!(urls, vec!["  http://a.example/1 ", "# comment"]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_missing_file_is_reported() {
    let err = read_url_list("/definitely/not/here.txt").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}
