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


//! Segmented (multi-connection) download of a single file.
//!
//! The remote file is split into byte ranges, each fetched by its own worker into
//! `name.part<i>`; the parts are then merged into `name.part` and renamed into place. A
//! sidecar (`name.part.meta`) records the layout and the remote file's validators, so an
//! interrupted download resumes only when it is safe (see [`parts::resume_allowed`]).

use super::client;
use super::error::{Error, Result};
use super::options::DownloadOptions;
use super::outcome::Outcome;
use super::parts;
use super::partial::{Finished, PartMeta, Target};
use super::resume::parse_content_range;
use super::retry;
use super::stream;
use super::throttle::Throttle;
use crate::shared::address::HostPolicy;
use crate::shared::progress::ProgressBarWrapper;
use indicatif::{MultiProgress, ProgressBar};
use reqwest::header::{ACCEPT_RANGES, CONTENT_RANGE, IF_RANGE, RANGE};
use reqwest::StatusCode;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

/// What every worker needs, shared between them.
struct Shared {
    url: String,
    timeout: u64,
    follow_redirects: bool,
    host_policy: HostPolicy,
    user_agent: Option<String>,
    headers: Vec<(String, String)>,
    /// Validator from the probe, so a file that changes mid-download is noticed.
    if_range: Option<String>,
    limit: Option<usize>,
    retries: u32,
    quiet: bool,
    progress: Option<ProgressBar>,
}

/// One attempt at one segment. Continues from whatever the part file already holds, so a
/// retry (or a resume) only asks for the missing bytes.
fn fetch_segment(shared: &Shared, index: usize, part_path: &Path, (start, end): (u64, u64)) -> Result<()> {
    let expected = end - start + 1;
    let mut have = fs::metadata(part_path).map(|m| m.len()).unwrap_or(0);
    if have > expected {
        let _ = fs::remove_file(part_path);
        have = 0;
    }
    if have == expected {
        return Ok(());
    }

    let first = start + have;
    let client = client::build(shared.timeout, shared.follow_redirects, shared.host_policy)?;
    let mut request_builder = client.get(&shared.url).header(RANGE, format!("bytes={}-{}", first, end));
    if let Some(validator) = &shared.if_range {
        request_builder = request_builder.header(IF_RANGE, validator);
    }
    let request_builder =
        client::apply_headers(request_builder, shared.user_agent.as_deref(), &shared.headers);

    let response = request_builder.send()?;
    if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
        return Err(Error::RangesUnsupported(format!(
            "part {}: 416 for bytes {}-{}",
            index, first, end
        )));
    }
    let mut response = client::ensure_success(response)?;

    match response.status().as_u16() {
        206 => {
            let at_offset = response
                .headers()
                .get(CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_content_range)
                .is_some_and(|cr| cr.range.is_some_and(|(s, _)| s == first));
            if !at_offset {
                return Err(Error::RangesUnsupported(format!(
                    "part {}: the reply does not start at byte {}",
                    index, first
                )));
            }
        }
        // The whole file instead of a range: the server ignores `Range`, or the validator
        // no longer matches. Either way the parts cannot be trusted.
        200 => {
            return Err(Error::RangesUnsupported(format!(
                "part {}: answered 200 to a range request",
                index
            )));
        }
        _ => {
            return Err(Error::ProtocolError(format!(
                "Unexpected status code: {}",
                response.status()
            )));
        }
    }

    let mut file = if have > 0 {
        OpenOptions::new().append(true).create(true).open(part_path)?
    } else {
        OpenOptions::new().write(true).create(true).truncate(true).open(part_path)?
    };

    let mut throttle = Throttle::new(shared.limit);
    let written = stream::copy(&mut response, &mut file, &mut throttle, shared.timeout, |n| {
        if let Some(ref bar) = shared.progress {
            bar.inc(n);
        }
    })?;
    log::debug!("part {} wrote {} bytes", index, written);

    if have + written != expected {
        return Err(Error::ProtocolError(format!(
            "part {} ended after {} of {} bytes",
            index,
            have + written,
            expected
        )));
    }

    if !shared.quiet {
        eprintln!("✅ Part {} complete", index);
    }
    Ok(())
}

pub(super) fn download(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    multi_progress: Option<&MultiProgress>,
) -> Result<Outcome> {
    let quiet = options.quiet;
    let segments = options.segments;

    let client = client::build(options.timeout, options.follow_redirects, options.host_policy)?;

    // The probe carries the same headers as the real requests, so servers that need
    // auth or a User-Agent answer it, and a disabled redirect policy is respected.
    let probe = client::apply_headers(
        client.head(url),
        options.user_agent.as_deref(),
        &options.headers,
    );
    let head_response = match probe.send() {
        Ok(r) => r,
        Err(e) => {
            if !quiet {
                eprintln!("⚠️  HEAD request failed: {}; falling back to single-thread.", e);
            }
            return super::download_single(url, output_path, options, multi_progress);
        }
    };

    if !head_response.status().is_success() {
        // The single-connection path reports the real problem (redirect, 403, 404, ...).
        if !quiet {
            eprintln!(
                "⚠️  HEAD request returned {}; falling back to single-thread.",
                head_response.status()
            );
        }
        return super::download_single(url, output_path, options, multi_progress);
    }

    let total_size = head_response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    log::debug!("total_size = {}, segments = {}", total_size, segments);

    if total_size == 0 {
        if !quiet {
            eprintln!("⚠️  Content-Length header missing or zero; falling back to single-thread.");
        }
        return super::download_single(url, output_path, options, multi_progress);
    }

    let accept_ranges = head_response
        .headers()
        .get(ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("bytes"))
        .unwrap_or(false);

    if !accept_ranges {
        if !quiet {
            eprintln!("⚠️  Server does not support range requests; falling back to single-thread.");
        }
        return super::download_single(url, output_path, options, multi_progress);
    }

    let ranges = parts::plan_ranges(total_size, segments);

    for (i, (start, end)) in ranges.iter().enumerate() {
        log::debug!("range {}: {}-{} (length: {})", i, start, end, end - start + 1);
    }

    if ranges.is_empty() {
        if !quiet {
            eprintln!("⚠️  Invalid range calculation; falling back to single-thread.");
        }
        return super::download_single(url, output_path, options, multi_progress);
    }

    // What the probe says the remote file is, and the layout we are about to use.
    let target = Target::resolve(output_path);
    let current = PartMeta::from_headers(url, head_response.headers(), Some(total_size))
        .with_segments(ranges.len());
    let saved = PartMeta::read(target.meta_path());

    let resuming = options.resume && parts::resume_allowed(saved.as_ref(), &current);
    if options.resume && !resuming && !quiet {
        eprintln!("Existing parts do not match the remote file, starting from scratch");
    }
    // Parts from a run we will not continue, and leftovers beyond the current count.
    parts::discard_parts(output_path, if resuming { ranges.len() } else { 0 });

    let part_paths: Vec<PathBuf> = (0..ranges.len()).map(|i| parts::part_path(output_path, i)).collect();
    let positions = if resuming {
        parts::reconcile(&part_paths, &ranges)
    } else {
        vec![0u64; ranges.len()]
    };

    if target.is_staged() {
        if let Err(e) = current.write(target.meta_path()) {
            // Only costs the ability to validate a later resume.
            log::debug!("could not write {}: {}", target.meta_path().display(), e);
        }
    }

    let progress = if !quiet {
        let wrapper = ProgressBarWrapper::new(total_size, positions.iter().sum());
        if let Some(mp) = multi_progress {
            mp.add(wrapper.get_bar().clone());
        }
        Some(wrapper)
    } else {
        None
    };

    let shared = Arc::new(Shared {
        url: url.to_string(),
        timeout: options.timeout,
        follow_redirects: options.follow_redirects,
        host_policy: options.host_policy,
        user_agent: options.user_agent.clone(),
        headers: options.headers.clone(),
        if_range: current.validator().map(str::to_string),
        limit: parts::per_segment_limit(options.limit_rate, ranges.len()),
        retries: options.retries,
        quiet,
        progress: progress.as_ref().map(|p| p.get_bar().clone()),
    });

    let handles: Vec<_> = ranges
        .iter()
        .copied()
        .enumerate()
        .map(|(i, range)| {
            let shared = Arc::clone(&shared);
            let part_path = part_paths[i].clone();
            thread::spawn(move || {
                let label = format!("part {}: ", i);
                retry::run(shared.retries, shared.quiet, &label, |_attempt| {
                    fetch_segment(&shared, i, &part_path, range)
                })
            })
        })
        .collect();

    let mut ranges_unsupported = false;
    let mut first_error: Option<Error> = None;
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join() {
            Ok(Ok(())) => {}
            Ok(Err(Error::RangesUnsupported(reason))) => {
                log::debug!("{}", reason);
                ranges_unsupported = true;
            }
            Ok(Err(e)) => {
                eprintln!("❌ Part {}: {}", i, e);
                first_error.get_or_insert(e);
            }
            Err(_) => {
                eprintln!("❌ Part {}: worker thread panicked", i);
                first_error.get_or_insert(Error::ProtocolError(format!("part {} worker thread panicked", i)));
            }
        }
    }

    if ranges_unsupported {
        if !quiet {
            eprintln!("⚠️  Server rejected byte ranges; falling back to single-thread.");
        }
        parts::discard_parts(output_path, 0);
        let _ = fs::remove_file(target.meta_path());
        return super::download_single(url, output_path, options, multi_progress);
    }

    // The parts and the sidecar stay on disk, so `-c` can continue from them.
    if let Some(e) = first_error {
        return Err(e);
    }

    // The merged file is staged as `name.part` and renamed, like a single-connection download.
    parts::merge_parts(target.work_path(), &part_paths, &ranges)?;
    // Checked before anything is put in place, so a bad download replaces nothing.
    target.verify(options.verify.as_ref())?;
    let finished = target.finish(&options.on_occupied)?;

    if let Some(p) = progress {
        p.finish();
    }

    if !quiet {
        match &finished {
            Finished::Placed(path) => eprintln!("\n✅ Download complete: {}", path.display()),
            Finished::Skipped(path) => eprintln!(
                "\nA file appeared at {} while downloading; keeping it and discarding this download",
                path.display()
            ),
        }
    }

    Ok(finished.into_outcome())
}
