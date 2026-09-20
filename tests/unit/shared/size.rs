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


use rget::shared::size::{format_size, parse_size};

#[test]
fn zero_means_no_limit() {
    assert_eq!(parse_size("0"), Ok(0));
}

#[test]
fn plain_numbers_are_bytes() {
    assert_eq!(parse_size("512"), Ok(512));
    assert_eq!(parse_size("512b"), Ok(512));
}

#[test]
fn suffixes_are_binary_and_case_insensitive() {
    assert_eq!(parse_size("1k"), Ok(1024));
    assert_eq!(parse_size("1K"), Ok(1024));
    assert_eq!(parse_size("1kb"), Ok(1024));
    assert_eq!(parse_size("2M"), Ok(2 * 1024 * 1024));
    assert_eq!(parse_size("1g"), Ok(1024 * 1024 * 1024));
    assert_eq!(parse_size("1t"), Ok(1024usize.pow(4)));
}

#[test]
fn fractions_and_surrounding_whitespace_are_accepted() {
    assert_eq!(parse_size("1.5k"), Ok(1536));
    assert_eq!(parse_size(".5k"), Ok(512));
    assert_eq!(parse_size("  3  "), Ok(3));
}

#[test]
fn malformed_sizes_are_rejected_with_a_reason() {
    assert_eq!(parse_size(""), Err("Empty size string".to_string()));
    assert_eq!(parse_size("abc"), Err("Invalid size format: abc".to_string()));
    assert_eq!(parse_size("12zz"), Err("Unknown size suffix: zz".to_string()));
    assert_eq!(
        parse_size("0.5"),
        Err("Rate limit must be at least 1 byte".to_string())
    );
    assert!(parse_size("0k").is_err());
}

#[test]
fn format_size_picks_the_largest_fitting_unit() {
    assert_eq!(format_size(0), "0.0 B");
    assert_eq!(format_size(1023), "1023.0 B");
    assert_eq!(format_size(1024), "1.0 KB");
    assert_eq!(format_size(1536), "1.5 KB");
    assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
}

#[test]
fn format_size_saturates_at_terabytes() {
    assert_eq!(format_size(1024usize.pow(4) * 2048), "2048.0 TB");
}
