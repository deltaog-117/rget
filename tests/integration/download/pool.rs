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
use rget::features::download::run_pool;
use std::sync::Arc;

// Roadmap item D7: `-j 0` used to start no worker thread, so the result of every task
// waited forever. A hang here would time out the whole test binary rather than fail this
// test alone, which is the point: it proves the clamp, not just documents the symptom.
#[test]
fn zero_jobs_is_treated_as_one_instead_of_hanging() {
    let data = payload(1_000);
    let base = serve(data.clone(), true);
    let dir = scratch_dir("pool-zero-jobs");
    let out = dir.join("out.bin");
    let tasks = vec![(format!("{base}/file"), out.to_str().unwrap().to_string())];

    let mut result = None;
    run_pool(tasks, 0, Arc::new(options()), None, |_url, _path, r| {
        result = Some(r)
    });

    assert!(result.expect("on_result must have been called").is_ok());
    assert_eq!(std::fs::read(&out).unwrap(), data);
    std::fs::remove_dir_all(dir).unwrap();
}
