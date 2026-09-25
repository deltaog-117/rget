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

//! A finished download is checked before it is put in place; a refusal replaces nothing.

use super::{options, payload, scratch_dir, serve, serve_with};
use rget::features::download::{download_file, Error, Outcome, Verifier};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn part_of(out: &Path) -> PathBuf {
    let mut name = out.as_os_str().to_owned();
    name.push(".part");
    name.into()
}

fn meta_of(out: &Path) -> PathBuf {
    let mut name = part_of(out).into_os_string();
    name.push(".meta");
    name.into()
}

/// A verifier that gives `verdict` and remembers which paths it was asked about.
fn verifier(verdict: Result<(), String>) -> (Verifier, Arc<Mutex<Vec<PathBuf>>>) {
    let seen: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let record = seen.clone();
    let verifier = Verifier::new(move |path| {
        record.lock().unwrap().push(path.to_path_buf());
        verdict.clone()
    });
    (verifier, seen)
}

#[test]
fn a_download_that_fails_verification_replaces_nothing_and_leaves_nothing_behind() {
    let base = serve(payload(20_000), true);
    let dir = scratch_dir("verify-fail");
    let out = dir.join("out.bin");
    std::fs::write(&out, b"the good file").unwrap();
    let (verify, _) = verifier(Err("Checksum mismatch: expected a, got b".into()));
    let mut opts = options();
    opts.verify = Some(verify);

    let err =
        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(
        matches!(&err, Error::Verification(m) if m.contains("expected a, got b")),
        "{err:?}"
    );
    assert_eq!(std::fs::read(&out).unwrap(), b"the good file");
    assert!(
        !part_of(&out).exists(),
        "a later -c must not find corrupt data to complete"
    );
    assert!(!meta_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_verifier_sees_the_finished_file_before_it_is_in_place() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("verify-pass");
    let out = dir.join("out.bin");
    let expected = data.clone();
    let seen: Arc<Mutex<Vec<(PathBuf, bool, bool)>>> = Arc::default();
    let record = seen.clone();
    let mut opts = options();
    opts.verify = Some(Verifier::new(move |path| {
        let complete = std::fs::read(path)
            .map(|bytes| bytes == expected)
            .unwrap_or(false);
        let final_exists = path.with_extension("").exists();
        record
            .lock()
            .unwrap()
            .push((path.to_path_buf(), complete, final_exists));
        Ok(())
    }));

    let outcome =
        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(outcome, Outcome::Saved(out.to_string_lossy().to_string()));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    let calls = seen.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        part_of(&out),
        "the staged file is what gets checked"
    );
    assert!(calls[0].1, "the verifier must see the complete file");
    assert!(!calls[0].2, "nothing may be in place yet");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_failed_verification_is_not_retried() {
    let base = serve(payload(5_000), true);
    let dir = scratch_dir("verify-noretry");
    let out = dir.join("out.bin");
    let (verify, seen) = verifier(Err("bad".into()));
    let mut opts = options();
    opts.verify = Some(verify);
    opts.retries = 3;

    assert!(download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).is_err());

    assert_eq!(seen.lock().unwrap().len(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn segmented_downloads_are_verified_too() {
    let base = serve(payload(90_000), true);
    let dir = scratch_dir("verify-segmented");
    let out = dir.join("out.bin");
    let (verify, seen) = verifier(Err("bad".into()));
    let mut opts = options();
    opts.verify = Some(verify);
    opts.segments = 3;

    let err =
        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::Verification(_)), "{err:?}");
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[part_of(&out)],
        "the merged file is checked once"
    );
    assert!(!out.exists() && !part_of(&out).exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_complete_part_file_from_an_earlier_run_is_verified_before_it_is_accepted() {
    let data = payload(30_000);
    let (base, _) = serve_with(data.clone(), true, "\"v1\"");
    let dir = scratch_dir("verify-complete-part");
    let out = dir.join("out.bin");
    let seed = |dir_out: &Path| {
        std::fs::write(part_of(dir_out), &data).unwrap();
        std::fs::write(
            meta_of(dir_out),
            format!(
                "url = \"{base}/file\"\netag = \"\\\"v1\\\"\"\ntotal = {}\n",
                data.len()
            ),
        )
        .unwrap();
    };
    let mut opts = options();
    opts.resume = true;

    seed(&out);
    let (refuse, _) = verifier(Err("corrupt".into()));
    opts.verify = Some(refuse);
    let err =
        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap_err();
    assert!(matches!(err, Error::Verification(_)), "{err:?}");
    assert!(!out.exists() && !part_of(&out).exists() && !meta_of(&out).exists());

    seed(&out);
    let (accept, _) = verifier(Ok(()));
    opts.verify = Some(accept);
    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn an_existing_complete_file_that_fails_verification_is_reported_but_never_deleted() {
    let data = payload(10_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("verify-existing");
    let out = dir.join("out.bin");
    std::fs::write(&out, &data).unwrap();
    let (verify, _) = verifier(Err("does not match".into()));
    let mut opts = options();
    opts.resume = true;
    opts.verify = Some(verify);

    let err =
        download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, Error::Verification(_)), "{err:?}");
    assert_eq!(
        std::fs::read(&out).unwrap(),
        data,
        "a file this run did not produce is left alone"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_device_cannot_be_read_back_so_it_is_not_verified() {
    let base = serve(payload(1_000), true);
    let (verify, seen) = verifier(Err("would fail".into()));
    let mut opts = options();
    opts.verify = Some(verify);

    download_file(&format!("{base}/file"), "/dev/null", &opts, None).unwrap();

    assert!(seen.lock().unwrap().is_empty());
}
