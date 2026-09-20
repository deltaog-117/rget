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


//! Retry loop with exponential backoff and jitter.

use super::error::Result;
use std::thread;
use std::time::Duration;

/// Runs `attempt_fn` until it succeeds or `retries + 1` attempts have failed.
///
/// FIXME(A6): every error is retried, including ones that can never succeed
/// (404, blocked URL). Classification comes with the retry cycle.
pub(super) fn run<F>(retries: u32, quiet: bool, mut attempt_fn: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    let mut attempt = 0;
    let max_attempts = retries + 1;

    loop {
        attempt += 1;

        match attempt_fn() {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt >= max_attempts {
                    if !quiet {
                        eprintln!("❌ Failed after {} attempts", attempt);
                    }
                    return Err(e);
                }

                let base_delay = 2u64.pow(attempt - 1);
                let jitter = rand::random::<u64>() % 1000;
                let delay = Duration::from_secs(base_delay) + Duration::from_millis(jitter);

                if !quiet {
                    eprintln!(
                        "⚠️  Attempt {} failed: {}. Retrying in {}s...",
                        attempt,
                        e,
                        delay.as_secs()
                    );
                }
                thread::sleep(delay);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::download::error::Error;

    fn failure() -> Error {
        Error::ProtocolError("boom".to_string())
    }

    #[test]
    fn success_on_first_attempt_runs_once() {
        let mut calls = 0;
        let result = run(3, true, || {
            calls += 1;
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(calls, 1);
    }

    #[test]
    fn zero_retries_means_a_single_attempt() {
        let mut calls = 0;
        let result = run(0, true, || {
            calls += 1;
            Err(failure())
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_later_success_wins() {
        let mut calls = 0;
        let result = run(1, true, || {
            calls += 1;
            if calls < 2 { Err(failure()) } else { Ok(()) }
        });
        assert!(result.is_ok());
        assert_eq!(calls, 2);
    }
}
