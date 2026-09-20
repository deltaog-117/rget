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


//! Retry policy: which failures are worth another attempt, and how long to wait.

use super::error::{Error, Result};
use std::thread;
use std::time::{Duration, SystemTime};

/// Longest we will sleep between attempts, whether by backoff or on a server's request.
const MAX_WAIT: Duration = Duration::from_secs(60);

/// Whether `error` may go away on its own: stalls and transport failures, and the HTTP
/// statuses that mean "try again" (the same set as `curl --retry`). Everything else
/// (404, a disabled redirect, a full disk, a protocol violation) would fail identically.
pub(super) fn is_retryable(error: &Error) -> bool {
    match error {
        Error::Stalled(_) => true,
        Error::Network(e) => !(e.is_builder() || e.is_redirect()),
        Error::HttpStatus { status, .. } => {
            matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
        }
        Error::Io(_)
        | Error::RedirectDisabled(..)
        | Error::ProtocolError(_)
        | Error::RangesUnsupported(_) => false,
    }
}

fn retry_after(error: &Error) -> Option<Duration> {
    match error {
        Error::HttpStatus { retry_after, .. } => *retry_after,
        _ => None,
    }
}

/// Parses a `Retry-After` value: a number of seconds, or an HTTP-date (a date in the
/// past means "now"). Anything else is ignored.
pub(super) fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse::<u64>().ok().map(Duration::from_secs);
    }
    let date = httpdate::parse_http_date(value).ok()?;
    Some(date.duration_since(now).unwrap_or(Duration::ZERO))
}

/// The pause before attempt number `attempt + 1`: exponential backoff with jitter, or the
/// server's `Retry-After` when that is longer. `None` means the server asked for more than
/// [`MAX_WAIT`], so waiting is pointless and the caller should give up.
fn next_delay(attempt: u32, jitter_ms: u64, retry_after: Option<Duration>) -> Option<Duration> {
    let backoff = Duration::from_secs(2u64.saturating_pow(attempt.saturating_sub(1))).min(MAX_WAIT);
    let delay = backoff + Duration::from_millis(jitter_ms);
    match retry_after {
        Some(asked) if asked > MAX_WAIT => None,
        Some(asked) => Some(delay.max(asked)),
        None => Some(delay),
    }
}

/// Runs `attempt_fn` (given the zero-based attempt number) until it succeeds, fails with
/// an error that is not worth retrying, or `retries + 1` attempts have failed. `label`
/// prefixes every message, so concurrent segments can be told apart.
pub(super) fn run<F>(retries: u32, quiet: bool, label: &str, mut attempt_fn: F) -> Result<()>
where
    F: FnMut(usize) -> Result<()>,
{
    let max_attempts = retries.saturating_add(1);
    let mut attempt: u32 = 0;

    loop {
        let index = attempt as usize;
        attempt += 1;

        let e = match attempt_fn(index) {
            Ok(()) => return Ok(()),
            Err(e) => e,
        };

        if attempt >= max_attempts || !is_retryable(&e) {
            // "Ranges unsupported" is not a failure: the caller falls back to one connection.
            if !quiet && !matches!(e, Error::RangesUnsupported(_)) {
                eprintln!("❌ {}Failed after {} attempts", label, attempt);
            }
            return Err(e);
        }

        let jitter = rand::random::<u64>() % 1000;
        match next_delay(attempt, jitter, retry_after(&e)) {
            Some(delay) => {
                if !quiet {
                    eprintln!(
                        "⚠️  {}Attempt {} failed: {}. Retrying in {}s...",
                        label,
                        attempt,
                        e,
                        delay.as_secs()
                    );
                }
                thread::sleep(delay);
            }
            None => {
                if !quiet {
                    eprintln!(
                        "⚠️  {}Attempt {} failed: {}. The server asked to wait longer than {}s; giving up.",
                        label,
                        attempt,
                        e,
                        MAX_WAIT.as_secs()
                    );
                }
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use reqwest::StatusCode;

    fn status(code: u16) -> Error {
        Error::HttpStatus { status: StatusCode::from_u16(code).unwrap(), retry_after: None }
    }

    #[test]
    fn transient_http_statuses_are_retried() {
        for code in [408, 425, 429, 500, 502, 503, 504] {
            assert!(is_retryable(&status(code)), "{code} should be retried");
        }
    }

    #[test]
    fn permanent_http_statuses_are_not_retried() {
        for code in [400, 401, 403, 404, 410, 416, 501, 505] {
            assert!(!is_retryable(&status(code)), "{code} should not be retried");
        }
    }

    #[test]
    fn local_and_protocol_errors_are_not_retried() {
        assert!(!is_retryable(&Error::Io(std::io::Error::other("disk full"))));
        assert!(!is_retryable(&Error::RedirectDisabled(302, "/x".into())));
        assert!(!is_retryable(&Error::ProtocolError("bad".into())));
        assert!(!is_retryable(&Error::RangesUnsupported("200".into())));
    }

    #[test]
    fn stalls_and_connection_failures_are_retried() {
        assert!(is_retryable(&Error::Stalled(30)));
        // Nothing listens on port 1, so this is a genuine refused connection.
        let refused = reqwest::blocking::get("http://127.0.0.1:1/").unwrap_err();
        assert!(is_retryable(&Error::Network(refused)));
    }

    #[test]
    fn a_malformed_request_is_not_retried() {
        let builder = reqwest::blocking::Client::new().get("not a url").send().unwrap_err();
        assert!(builder.is_builder());
        assert!(!is_retryable(&Error::Network(builder)));
    }

    #[test]
    fn retry_after_accepts_seconds() {
        let now = SystemTime::UNIX_EPOCH;
        assert_eq!(parse_retry_after("120", now), Some(Duration::from_secs(120)));
        assert_eq!(parse_retry_after("  7 ", now), Some(Duration::from_secs(7)));
        assert_eq!(parse_retry_after("0", now), Some(Duration::ZERO));
    }

    #[test]
    fn retry_after_accepts_http_dates() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let later = httpdate::fmt_http_date(now + Duration::from_secs(30));
        let earlier = httpdate::fmt_http_date(now - Duration::from_secs(30));
        assert_eq!(parse_retry_after(&later, now), Some(Duration::from_secs(30)));
        assert_eq!(parse_retry_after(&earlier, now), Some(Duration::ZERO));
    }

    #[test]
    fn retry_after_ignores_garbage() {
        let now = SystemTime::UNIX_EPOCH;
        for bad in ["", "soon", "-5", "1.5", "12abc"] {
            assert_eq!(parse_retry_after(bad, now), None, "{bad:?}");
        }
    }

    #[test]
    fn backoff_doubles_from_one_second() {
        assert_eq!(next_delay(1, 0, None), Some(Duration::from_secs(1)));
        assert_eq!(next_delay(2, 0, None), Some(Duration::from_secs(2)));
        assert_eq!(next_delay(3, 500, None), Some(Duration::from_millis(4500)));
    }

    #[test]
    fn backoff_is_capped_and_never_overflows() {
        assert_eq!(next_delay(20, 0, None), Some(MAX_WAIT));
        assert_eq!(next_delay(u32::MAX, 0, None), Some(MAX_WAIT));
    }

    #[test]
    fn a_longer_retry_after_wins_and_a_shorter_one_does_not() {
        assert_eq!(next_delay(1, 0, Some(Duration::from_secs(5))), Some(Duration::from_secs(5)));
        assert_eq!(next_delay(3, 0, Some(Duration::from_secs(1))), Some(Duration::from_secs(4)));
    }

    #[test]
    fn an_unreasonable_retry_after_means_give_up() {
        assert_eq!(next_delay(1, 0, Some(Duration::from_secs(300))), None);
        assert_eq!(next_delay(1, 0, Some(MAX_WAIT)), Some(MAX_WAIT));
    }

    proptest! {
        #[test]
        fn a_delay_never_exceeds_the_cap_plus_jitter(
            attempt in 1u32..10_000,
            jitter in 0u64..1000,
            asked in proptest::option::of(0u64..200),
        ) {
            if let Some(delay) = next_delay(attempt, jitter, asked.map(Duration::from_secs)) {
                prop_assert!(delay <= MAX_WAIT + Duration::from_millis(999));
            }
        }
    }

    #[test]
    fn success_on_first_attempt_runs_once() {
        let mut calls = 0;
        let result = run(3, true, "", |_| {
            calls += 1;
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(calls, 1);
    }

    #[test]
    fn zero_retries_means_a_single_attempt() {
        let mut calls = 0;
        let result = run(0, true, "", |_| {
            calls += 1;
            Err(Error::Stalled(1))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_permanent_error_is_not_retried_even_when_retries_remain() {
        let mut calls = 0;
        let result = run(5, true, "", |_| {
            calls += 1;
            Err(status(404))
        });
        assert!(result.is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_transient_error_is_retried_and_the_attempt_number_is_passed_on() {
        let mut seen = Vec::new();
        let result = run(1, true, "", |attempt| {
            seen.push(attempt);
            if attempt == 0 { Err(Error::Stalled(1)) } else { Ok(()) }
        });
        assert!(result.is_ok());
        assert_eq!(seen, vec![0, 1]);
    }
}
