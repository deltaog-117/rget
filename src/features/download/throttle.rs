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


//! Bandwidth limiting (`--limit-rate`).

use std::thread;
use std::time::{Duration, Instant};

/// Sleeps out the remainder of each second once `limit` bytes have been written.
pub(super) struct Throttle {
    limit: Option<usize>,
    bytes_written_this_second: usize,
    second_start: Instant,
}

impl Throttle {
    pub(super) fn new(limit: Option<usize>) -> Self {
        Self {
            limit,
            bytes_written_this_second: 0,
            second_start: Instant::now(),
        }
    }

    /// Accounts for a chunk about to be written, sleeping if the budget is spent.
    pub(super) fn wait(&mut self, chunk_len: usize) {
        if let Some(limit) = self.limit {
            if limit > 0 {
                self.bytes_written_this_second += chunk_len;
                if self.bytes_written_this_second >= limit {
                    let elapsed = self.second_start.elapsed();
                    if elapsed < Duration::from_secs(1) {
                        let sleep_time = Duration::from_secs(1) - elapsed;
                        thread::sleep(sleep_time);
                    }
                    self.bytes_written_this_second = 0;
                    self.second_start = Instant::now();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_limit_never_sleeps() {
        let mut throttle = Throttle::new(None);
        let start = Instant::now();
        for _ in 0..1000 {
            throttle.wait(1 << 20);
        }
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn zero_limit_means_unlimited() {
        let mut throttle = Throttle::new(Some(0));
        let start = Instant::now();
        throttle.wait(1 << 30);
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn spending_the_budget_waits_out_the_second() {
        let mut throttle = Throttle::new(Some(1000));
        let start = Instant::now();
        throttle.wait(1000);
        assert!(start.elapsed() >= Duration::from_millis(900));
    }
}
