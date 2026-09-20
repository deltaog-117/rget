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


use rget::features::destination::output_path;

fn scratch(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("rget-loc-{}-{}", std::process::id(), name))
}

#[test]
fn a_prefix_is_created_and_joined() {
    let dir = scratch("prefix").join("nested");
    let path = output_path("f.bin", Some(dir.to_str().unwrap()), false).unwrap();
    assert!(dir.is_dir());
    assert_eq!(path, dir.join("f.bin").to_string_lossy());
    std::fs::remove_dir_all(scratch("prefix")).unwrap();
}

#[test]
fn a_prefix_also_applies_to_an_explicit_output_name() {
    let dir = scratch("both");
    let path = output_path("mine.bin", Some(dir.to_str().unwrap()), true).unwrap();
    assert_eq!(path, dir.join("mine.bin").to_string_lossy());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_explicit_output_without_a_prefix_is_used_as_given() {
    assert_eq!(output_path("out/mine.bin", None, true).unwrap(), "out/mine.bin");
}

#[test]
fn an_uncreatable_prefix_is_an_io_error() {
    assert!(output_path("f.bin", Some("/proc/rget-cannot-create-this"), false).is_err());
}
