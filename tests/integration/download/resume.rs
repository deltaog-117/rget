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


use super::{options, payload, scratch_dir, serve};
use rget::features::download::download_file;

#[test]
fn a_partial_file_is_completed_from_where_it_stopped() {
    let data = payload(200_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-partial");
    let out = dir.join("out.bin");
    std::fs::write(&out, &data[..70_000]).unwrap();
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn resume_with_no_existing_file_is_a_normal_download() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-fresh");
    let out = dir.join("out.bin");
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_server_without_range_support_restarts_from_scratch() {
    let data = payload(20_000);
    let base = serve(data.clone(), false);
    let dir = scratch_dir("resume-norange");
    let out = dir.join("out.bin");
    std::fs::write(&out, vec![9u8; 5_000]).unwrap();
    let mut opts = options();
    opts.resume = true;

    download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap();

    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}

// Interim behaviour until roadmap item A5: a 416 on an already-complete file is now an
// error that leaves the file alone (it used to truncate the file to 0 bytes and report
// success). A5 turns this into a success.
#[test]
fn known_defect_resuming_a_complete_file_is_an_error_but_keeps_the_data() {
    let data = payload(20_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("resume-complete");
    let out = dir.join("out.bin");
    std::fs::write(&out, &data).unwrap();
    let mut opts = options();
    opts.resume = true;

    let err = download_file(&format!("{base}/file"), out.to_str().unwrap(), &opts, None).unwrap_err();

    assert!(matches!(err, rget::features::download::Error::HttpStatus(s) if s.as_u16() == 416));
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}
