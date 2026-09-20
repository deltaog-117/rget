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
use rget::features::destination::{Claims, ExistingFile, Placement, SkipReason};
use std::collections::HashSet;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rget-conflict-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn text(dir: &std::path::Path, name: &str) -> String {
    dir.join(name).to_string_lossy().to_string()
}

fn written(placement: Placement) -> String {
    match placement {
        Placement::Write(path) => path,
        other => panic!("expected Write, got {other:?}"),
    }
}

// ---- overwrite (the default) ------------------------------------------------------------

#[test]
fn overwrite_uses_the_path_even_when_the_file_exists() {
    let dir = scratch("overwrite");
    std::fs::write(dir.join("f.bin"), b"old").unwrap();
    let path = text(&dir, "f.bin");
    assert_eq!(Claims::new().place(&path, ExistingFile::Overwrite), Placement::Write(path));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_second_url_for_the_same_path_is_numbered_even_under_overwrite() {
    let dir = scratch("dup-overwrite");
    let path = text(&dir, "f.bin");
    let mut claims = Claims::new();
    assert_eq!(claims.place(&path, ExistingFile::Overwrite), Placement::Write(path.clone()));
    assert_eq!(claims.place(&path, ExistingFile::Overwrite), Placement::Write(text(&dir, "f (1).bin")));
    assert_eq!(claims.place(&path, ExistingFile::Overwrite), Placement::Write(text(&dir, "f (2).bin")));
    std::fs::remove_dir_all(dir).unwrap();
}

// ---- skip -------------------------------------------------------------------------------

#[test]
fn skip_leaves_an_existing_file_alone() {
    let dir = scratch("skip");
    std::fs::write(dir.join("f.bin"), b"old").unwrap();
    let path = text(&dir, "f.bin");
    assert_eq!(
        Claims::new().place(&path, ExistingFile::Skip),
        Placement::Skip { path, reason: SkipReason::Exists }
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn skip_writes_when_nothing_is_in_the_way_and_skips_a_later_duplicate() {
    let dir = scratch("skip-dup");
    let path = text(&dir, "f.bin");
    let mut claims = Claims::new();
    assert_eq!(claims.place(&path, ExistingFile::Skip), Placement::Write(path.clone()));
    assert_eq!(
        claims.place(&path, ExistingFile::Skip),
        Placement::Skip { path, reason: SkipReason::EarlierUrl }
    );
    std::fs::remove_dir_all(dir).unwrap();
}

// ---- rename -----------------------------------------------------------------------------

#[test]
fn rename_keeps_the_existing_file_and_picks_the_first_free_number() {
    let dir = scratch("rename");
    for name in ["f.zip", "f (1).zip"] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let mut claims = Claims::new();
    assert_eq!(written(claims.place(&text(&dir, "f.zip"), ExistingFile::Rename)), text(&dir, "f (2).zip"));
    assert_eq!(std::fs::read(dir.join("f.zip")).unwrap(), b"x");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rename_uses_the_plain_name_when_it_is_free() {
    let dir = scratch("rename-free");
    let path = text(&dir, "f.zip");
    assert_eq!(Claims::new().place(&path, ExistingFile::Rename), Placement::Write(path));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rename_never_takes_a_name_whose_download_is_in_progress() {
    let dir = scratch("rename-part");
    std::fs::write(dir.join("f.zip"), b"x").unwrap();
    std::fs::write(dir.join("f (1).zip.part"), b"half").unwrap();
    std::fs::write(dir.join("f (2).zip.part.meta"), b"url = \"x\"").unwrap();
    let placed = written(Claims::new().place(&text(&dir, "f.zip"), ExistingFile::Rename));
    assert_eq!(placed, text(&dir, "f (3).zip"), "someone else's partial download must not be trampled");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn rename_keeps_a_tarball_extension_whole() {
    let dir = scratch("rename-tar");
    std::fs::write(dir.join("a.tar.gz"), b"x").unwrap();
    assert_eq!(written(Claims::new().place(&text(&dir, "a.tar.gz"), ExistingFile::Rename)), text(&dir, "a (1).tar.gz"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_directory_in_the_way_counts_as_occupied() {
    let dir = scratch("rename-dir");
    std::fs::create_dir(dir.join("f.bin")).unwrap();
    assert_eq!(written(Claims::new().place(&text(&dir, "f.bin"), ExistingFile::Rename)), text(&dir, "f (1).bin"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_device_is_never_in_the_way() {
    let mut claims = Claims::new();
    assert_eq!(claims.place("/dev/null", ExistingFile::Skip), Placement::Write("/dev/null".into()));
    assert_eq!(Claims::new().place("/dev/null", ExistingFile::Rename), Placement::Write("/dev/null".into()));
}

// ---- relocating at the last moment ------------------------------------------------------

#[test]
fn a_name_that_became_occupied_is_relocated_to_the_next_free_number() {
    let dir = scratch("relocate");
    let path = text(&dir, "f.zip");
    let mut claims = Claims::new();
    assert_eq!(written(claims.place(&path, ExistingFile::Rename)), path);
    // Someone creates f.zip after we claimed it:
    std::fs::write(dir.join("f.zip"), b"appeared").unwrap();
    assert_eq!(claims.relocate(&path), Some(text(&dir, "f (1).zip")));
    // ... and if that one is then taken as well, the number moves on from the *original* name.
    std::fs::write(dir.join("f (1).zip"), b"appeared too").unwrap();
    assert_eq!(claims.relocate(&text(&dir, "f (1).zip")), Some(text(&dir, "f (2).zip")));
    std::fs::remove_dir_all(dir).unwrap();
}

// ---- properties -------------------------------------------------------------------------

proptest! {
    #[test]
    fn no_two_requests_in_one_run_ever_get_the_same_file(
        policy in prop::sample::select(vec![ExistingFile::Overwrite, ExistingFile::Skip, ExistingFile::Rename]),
        names in proptest::collection::vec("[a-c]{1,2}(\\.[a-z]{1,3})?", 1..25),
        existing in proptest::collection::vec("[a-c]{1,2}(\\.[a-z]{1,3})?", 0..6),
    ) {
        let dir = scratch(&format!("prop-{:x}", names.iter().chain(existing.iter()).fold(0u64, |h, n| h.wrapping_mul(31).wrapping_add(n.len() as u64 * 7 + n.bytes().map(u64::from).sum::<u64>()))));
        for name in &existing {
            std::fs::write(dir.join(name), b"pre-existing").unwrap();
        }
        let mut claims = Claims::new();
        let mut used = HashSet::new();
        for name in &names {
            if let Placement::Write(path) = claims.place(&text(&dir, name), policy) {
                prop_assert!(used.insert(path.clone()), "{path} was handed out twice");
                if policy != ExistingFile::Overwrite {
                    prop_assert!(!existing.iter().any(|e| text(&dir, e) == path) , "{path} already existed");
                }
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
