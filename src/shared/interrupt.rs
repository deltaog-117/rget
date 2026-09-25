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

//! A process-wide flag set once when the user presses Ctrl+C.
//!
//! Every download streams its body through one shared loop (`download::stream::copy`),
//! which checks [`requested`] between chunks and stops there instead of leaving the
//! process to die mid-write. Because the flag is global, one Ctrl+C stops every
//! in-flight download (a single URL, every segment, and every `-j` worker) without each
//! of them needing its own handler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static INSTALLED: Once = Once::new();

/// Installs the `SIGINT` handler. Safe to call more than once; only the first call has
/// an effect. A failure to install (there is already a handler) is not fatal: Ctrl+C then
/// falls back to the platform default of killing the process outright.
pub fn install() {
    INSTALLED.call_once(|| {
        let _ = ctrlc::set_handler(|| {
            if !INTERRUPTED.swap(true, Ordering::SeqCst) {
                eprintln!("\n⚠️  Interrupted; run again with -c to resume.");
            }
        });
    });
}

/// Whether Ctrl+C has been pressed since the process started.
///
/// Deliberately untested here: the flag is process-wide, and every test in this binary
/// runs in the same process, so flipping it in a unit test would race every other test
/// that streams a download body through [`super::super::features::download`]. The
/// behaviour it gates (`stream::copy` stopping between chunks) is exercised instead by
/// [`super::super::features::download::stream`]'s own tests, with the flag left alone.
pub fn requested() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}
