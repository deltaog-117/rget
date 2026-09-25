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

use rget::features::destination::{default_download_dir, output_path};

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("rget-loc-{}-{}", std::process::id(), name))
}

#[test]
fn a_prefix_is_created_and_joined() {
    let dir = scratch("prefix").join("nested");
    let path = output_path("f.bin", Some(dir.to_str().unwrap()), false, false).unwrap();
    assert!(dir.is_dir());
    assert_eq!(path, dir.join("f.bin").to_string_lossy());
    std::fs::remove_dir_all(scratch("prefix")).unwrap();
}

#[test]
fn a_prefix_also_applies_to_an_explicit_output_name() {
    let dir = scratch("both");
    let path = output_path("mine.bin", Some(dir.to_str().unwrap()), true, false).unwrap();
    assert_eq!(path, dir.join("mine.bin").to_string_lossy());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_prefix_wins_over_the_downloads_flag_too() {
    let dir = scratch("prefix-over-downloads");
    let path = output_path("f.bin", Some(dir.to_str().unwrap()), false, true).unwrap();
    assert_eq!(path, dir.join("f.bin").to_string_lossy());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_explicit_output_without_a_prefix_is_used_as_given() {
    assert_eq!(
        output_path("out/mine.bin", None, true, false).unwrap(),
        "out/mine.bin"
    );
}

#[test]
fn an_explicit_output_wins_over_the_downloads_flag_too() {
    assert_eq!(
        output_path("out/mine.bin", None, true, true).unwrap(),
        "out/mine.bin"
    );
}

#[test]
fn an_uncreatable_prefix_is_an_io_error() {
    assert!(output_path("f.bin", Some("/proc/rget-cannot-create-this"), false, false).is_err());
}

// Roadmap: rget now matches wget/curl and defaults to the current directory instead of
// the platform Downloads folder.
#[test]
fn with_no_prefix_no_explicit_output_and_no_downloads_flag_the_current_directory_is_used() {
    assert_eq!(output_path("f.bin", None, false, false).unwrap(), "f.bin");
}

#[test]
fn the_downloads_flag_uses_the_platform_downloads_folder_when_one_exists() {
    let path = output_path("f.bin", None, false, true).unwrap();
    match default_download_dir() {
        Some(dir) => assert_eq!(path, dir.join("f.bin").to_string_lossy()),
        None => assert_eq!(path, "f.bin"),
    }
}
