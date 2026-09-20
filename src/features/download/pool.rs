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


//! Parallel downloads of several URLs (`-j`).

use super::download_file;
use super::error::Result;
use super::options::DownloadOptions;
use super::outcome::Outcome;
use crossbeam_channel::unbounded;
use indicatif::MultiProgress;
use std::sync::Arc;
use std::thread;

/// Downloads every `(url, output_path)` task on `jobs` worker threads.
///
/// `on_result` is called on the calling thread as each task finishes, in
/// completion order, with the URL, its output path, and the outcome.
///
/// FIXME(D): with `jobs == 0` no worker exists and this blocks forever.
pub fn run_pool<F>(
    tasks: Vec<(String, String)>,
    jobs: usize,
    options: Arc<DownloadOptions>,
    multi_progress: Option<MultiProgress>,
    mut on_result: F,
) where
    F: FnMut(String, String, Result<Outcome>),
{
    let total = tasks.len();
    let (task_sender, task_receiver) = unbounded::<(String, String)>();
    let (result_sender, result_receiver) = unbounded::<(String, String, Result<Outcome>)>();

    let mut handles = Vec::new();
    for _ in 0..jobs {
        let task_receiver = task_receiver.clone();
        let result_sender = result_sender.clone();
        let options = options.clone();
        let multi_progress = multi_progress.clone();
        let handle = thread::spawn(move || {
            while let Ok((url, output_path)) = task_receiver.recv() {
                let result = download_file(&url, &output_path, &options, multi_progress.as_ref());
                let _ = result_sender.send((url, output_path, result));
            }
        });
        handles.push(handle);
    }

    for task in tasks {
        let _ = task_sender.send(task);
    }
    drop(task_sender);

    for _ in 0..total {
        let (url, output, result) = result_receiver
            .recv()
            .expect("result channel closed before every task reported");
        on_result(url, output, result);
    }

    for handle in handles {
        let _ = handle.join();
    }
}
