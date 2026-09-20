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


use rget::features::integrity::{compute_sha256, verify_sha256, Error};
use std::path::PathBuf;

const HELLO_SHA256: &str = "6e39426dd10db18f88f5c6b6be808b8d9f12929cd03b6ac41dba112de33ef099";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rget-sha-{}-{}", std::process::id(), name));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn hashes_match_known_vectors() {
    let hello = temp_file("hello", b"hello rget\n");
    let empty = temp_file("empty", b"");
    assert_eq!(compute_sha256(hello.to_str().unwrap()).unwrap(), HELLO_SHA256);
    assert_eq!(compute_sha256(empty.to_str().unwrap()).unwrap(), EMPTY_SHA256);
    std::fs::remove_file(hello).unwrap();
    std::fs::remove_file(empty).unwrap();
}

#[test]
fn files_larger_than_the_read_buffer_hash_correctly() {
    let big = temp_file("big", &vec![b'a'; 100_000]);
    assert_eq!(
        compute_sha256(big.to_str().unwrap()).unwrap(),
        "6d1cf22d7cc09b085dfc25ee1a1f3ae0265804c607bc2074ad253bcc82fd81ee"
    );
    std::fs::remove_file(big).unwrap();
}

#[test]
fn matching_checksum_verifies() {
    let path = temp_file("ok", b"hello rget\n");
    assert!(verify_sha256(path.to_str().unwrap(), HELLO_SHA256).is_ok());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn mismatch_reports_both_digests() {
    let path = temp_file("bad", b"hello rget\n");
    let wrong = "0".repeat(64);
    match verify_sha256(path.to_str().unwrap(), &wrong) {
        Err(Error::ChecksumMismatch { expected, actual }) => {
            assert_eq!(expected, wrong);
            assert_eq!(actual, HELLO_SHA256);
        }
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn missing_file_is_an_io_error() {
    assert!(matches!(
        verify_sha256("/definitely/not/here", HELLO_SHA256),
        Err(Error::Io(_))
    ));
}

#[test]
fn the_expected_digest_may_be_in_either_case() {
    let path = temp_file("case", b"hello rget\n");
    assert!(verify_sha256(path.to_str().unwrap(), &HELLO_SHA256.to_uppercase()).is_ok());
    assert!(verify_sha256(path.to_str().unwrap(), HELLO_SHA256).is_ok());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_whole_line_of_sha256sum_output_is_accepted() {
    let path = temp_file("line", b"hello rget\n");
    let line = format!("{HELLO_SHA256}  hello.txt\n");
    assert!(verify_sha256(path.to_str().unwrap(), &line).is_ok());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_malformed_expected_digest_is_refused_before_the_file_is_read() {
    // The path does not exist: if the file were read first, this would be an I/O error.
    for bad in ["", "deadbeef", &"g".repeat(64), &"a".repeat(63), &"a".repeat(65)] {
        assert!(
            matches!(verify_sha256("/definitely/not/here", bad), Err(Error::InvalidDigest(_))),
            "{bad:?}"
        );
    }
}

#[test]
fn a_mismatch_reports_both_digests_in_lower_case() {
    let path = temp_file("lower", b"hello rget\n");
    let wrong = "A".repeat(64);
    match verify_sha256(path.to_str().unwrap(), &wrong) {
        Err(Error::ChecksumMismatch { expected, actual }) => {
            assert_eq!(expected, "a".repeat(64));
            assert_eq!(actual, HELLO_SHA256);
        }
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
    std::fs::remove_file(path).unwrap();
}
