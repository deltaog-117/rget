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


//! What happens when the target is occupied at the moment the finished download is put in place.

use super::{options, payload, scratch_dir, serve};
use rget::features::download::{download_file, OnOccupied, Outcome, Relocate};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn part_files(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains(".part"))
        .collect();
    found.sort();
    found
}

/// Offers `alternatives` one after another, remembering what it was asked to replace.
fn relocating(alternatives: Vec<PathBuf>) -> (OnOccupied, Arc<Mutex<Vec<String>>>) {
    let asked: Arc<Mutex<Vec<String>>> = Arc::default();
    let (record, queue) = (asked.clone(), Arc::new(Mutex::new(alternatives.into_iter())));
    let pick: Relocate = Arc::new(move |occupied| {
        record.lock().unwrap().push(occupied.to_string());
        queue.lock().unwrap().next().map(|p| text(&p))
    });
    (OnOccupied::Relocate(pick), asked)
}

#[test]
fn replace_is_the_default_and_overwrites_the_existing_file() {
    let data = payload(10_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-replace");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"old").unwrap();

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &options(), None).unwrap();

    assert_eq!(outcome, Outcome::Saved(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_free_target_is_used_directly_whatever_the_policy() {
    let data = payload(5_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-free");
    let mut opts = options();
    opts.on_occupied = OnOccupied::Skip;
    let out = dir.join("out.bin");

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Saved(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    assert!(part_files(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn skip_keeps_the_existing_file_and_discards_the_download() {
    let base = serve(payload(10_000), true);
    let dir = scratch_dir("place-skip");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"keep me").unwrap();
    let mut opts = options();
    opts.on_occupied = OnOccupied::Skip;

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Skipped(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");
    assert!(part_files(&dir).is_empty(), "the discarded download leaves nothing behind");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn relocate_saves_under_the_alternative_and_keeps_the_existing_file() {
    let data = payload(10_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-relocate");
    let out = dir.join("out.bin");
    let alternative = dir.join("out (1).bin");
    std::fs::write(&out, b"keep me").unwrap();
    let (policy, asked) = relocating(vec![alternative.clone()]);
    let mut opts = options();
    opts.on_occupied = policy;

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Saved(text(&alternative)));
    assert_eq!(std::fs::read(&alternative).unwrap(), data);
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");
    assert_eq!(asked.lock().unwrap().as_slice(), &[text(&out)]);
    assert!(part_files(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn relocate_is_asked_again_when_the_alternative_is_taken_too() {
    let data = payload(4_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-relocate-twice");
    let (out, first, second) = (dir.join("out.bin"), dir.join("out (1).bin"), dir.join("out (2).bin"));
    std::fs::write(&out, b"one").unwrap();
    std::fs::write(&first, b"two").unwrap();
    let (policy, asked) = relocating(vec![first.clone(), second.clone()]);
    let mut opts = options();
    opts.on_occupied = policy;

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Saved(text(&second)));
    assert_eq!(std::fs::read(&first).unwrap(), b"two");
    assert_eq!(asked.lock().unwrap().as_slice(), &[text(&out), text(&first)]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn relocate_giving_up_discards_the_download() {
    let base = serve(payload(4_000), true);
    let dir = scratch_dir("place-relocate-giveup");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"keep me").unwrap();
    let (policy, _) = relocating(Vec::new());
    let mut opts = options();
    opts.on_occupied = policy;

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Skipped(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");
    assert!(part_files(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

/// `/trickle` takes about three seconds; the file is created a second in, while the download
/// is still running and the target was free when it started.
fn create_after_a_second(path: PathBuf) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(1));
        std::fs::write(path, b"appeared meanwhile").unwrap();
    })
}

#[test]
fn a_file_that_appears_while_downloading_is_never_overwritten_under_skip() {
    let base = serve(payload(100_000), true);
    let dir = scratch_dir("place-race-skip");
    let out = dir.join("out.bin");
    let creator = create_after_a_second(out.clone());
    let mut opts = options();
    opts.on_occupied = OnOccupied::Skip;

    let outcome = download_file(&format!("{base}/trickle"), out.to_str().unwrap(), &opts, None).unwrap();
    creator.join().unwrap();

    assert_eq!(outcome, Outcome::Skipped(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), b"appeared meanwhile");
    assert!(part_files(&dir).is_empty());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_file_that_appears_while_downloading_is_never_overwritten_under_relocate() {
    let data = payload(100_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-race-relocate");
    let out = dir.join("out.bin");
    let alternative = dir.join("out (1).bin");
    let creator = create_after_a_second(out.clone());
    let (policy, _) = relocating(vec![alternative.clone()]);
    let mut opts = options();
    opts.on_occupied = policy;

    let outcome = download_file(&format!("{base}/trickle"), out.to_str().unwrap(), &opts, None).unwrap();
    creator.join().unwrap();

    assert_eq!(outcome, Outcome::Saved(text(&alternative)));
    assert_eq!(std::fs::read(&out).unwrap(), b"appeared meanwhile");
    assert_eq!(std::fs::read(&alternative).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_default_policy_still_replaces_a_file_that_appears_meanwhile() {
    let data = payload(100_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("place-race-replace");
    let out = dir.join("out.bin");
    let creator = create_after_a_second(out.clone());

    download_file(&format!("{base}/trickle"), out.to_str().unwrap(), &options(), None).unwrap();
    creator.join().unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data, "overwrite means overwrite");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn segmented_downloads_honour_the_policy_too() {
    let base = serve(payload(90_000), true);
    let dir = scratch_dir("place-segmented");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"keep me").unwrap();
    let mut opts = options();
    opts.segments = 3;
    opts.on_occupied = OnOccupied::Skip;

    let outcome = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Skipped(text(&out)));
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");
    assert!(part_files(&dir).is_empty(), "left over: {:?}", part_files(&dir));
    std::fs::remove_dir_all(dir).unwrap();
}
