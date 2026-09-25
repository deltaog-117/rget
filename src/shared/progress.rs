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

//! Progress bar wrapper shared by every download strategy.

use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressBarWrapper {
    pub bar: ProgressBar,
}

impl ProgressBarWrapper {
    pub fn new(total_size: u64, existing_size: u64) -> Self {
        let bar = if total_size > 0 {
            let bar = ProgressBar::new(total_size);
            bar.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
                         {bytes}/{total_bytes} ({binary_bytes_per_sec}, {eta})",
                    )
                    .expect("valid template")
                    .progress_chars("━▸ "),
            );
            bar
        } else {
            // The size is unknown ahead of time (no `Content-Length`), so a determinate bar
            // and an ETA would be meaningless; a spinner reporting bytes and speed fits instead.
            let bar = ProgressBar::new_spinner();
            bar.set_style(
                ProgressStyle::default_spinner()
                    .template(
                        "{spinner:.green} [{elapsed_precise}] {bytes} ({binary_bytes_per_sec})",
                    )
                    .expect("valid template"),
            );
            bar
        };

        if existing_size > 0 {
            bar.set_position(existing_size);
        }

        Self { bar }
    }

    pub fn inc(&self, amount: u64) {
        self.bar.inc(amount);
    }

    pub fn finish(&self) {
        self.bar.finish();
    }

    pub fn get_bar(&self) -> &ProgressBar {
        &self.bar
    }
}
