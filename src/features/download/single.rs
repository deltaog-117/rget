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

//! Single-connection download attempt.

use super::client;
use super::error::{Error, Result};
use super::options::DownloadOptions;
use super::outcome::Outcome;
use super::partial::{Finished, PartMeta, Source, Target};
use super::resume::{self, Plan, Reply};
use super::stream;
use super::throttle::Throttle;
use crate::shared::progress::ProgressBarWrapper;
use indicatif::MultiProgress;
use reqwest::header::{CONTENT_RANGE, IF_RANGE, RANGE};
use std::fs::OpenOptions;
use std::time::Instant;

/// One download in progress: everything `fetch` needs besides the resume plan.
struct Job<'a> {
    url: &'a str,
    output_path: &'a str,
    options: &'a DownloadOptions,
    target: &'a Target,
    multi_progress: Option<&'a MultiProgress>,
}

/// Performs one download attempt (retries are the caller's concern).
///
/// Data goes to `name.part` and is renamed to `name` only once complete. The first attempt
/// continues an earlier partial download only when `-c` was given; later attempts of the
/// same run always continue from the bytes the previous attempt wrote.
pub(super) fn attempt(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    attempt_index: usize,
    multi_progress: Option<&MultiProgress>,
) -> Result<Outcome> {
    let target = Target::resolve(output_path);
    let resume_wanted = target.is_staged() && (options.resume || attempt_index > 0);

    if target.is_staged() && !resume_wanted {
        // Without -c a leftover partial download is stale, exactly as an old file would be.
        target.discard_partial();
    }

    let source = if resume_wanted {
        target.resume_source(options.resume)
    } else {
        None
    };
    let saved_meta = PartMeta::read(target.meta_path());
    // A sidecar only describes `name.part`; it says nothing about a file we are adopting.
    let meta_for_plan = match source {
        Some((Source::Part, _)) => saved_meta.as_ref(),
        _ => None,
    };
    let plan = resume::plan(source.map_or(0, |(_, len)| len), meta_for_plan, url);

    let job = Job {
        url,
        output_path,
        options,
        target: &target,
        multi_progress,
    };
    fetch(&job, source.map(|(s, _)| s), saved_meta.as_ref(), plan)
}

fn fetch(
    job: &Job,
    source: Option<Source>,
    saved_meta: Option<&PartMeta>,
    plan: Plan,
) -> Result<Outcome> {
    let options = job.options;
    let quiet = options.quiet;
    let started = Instant::now();
    let client = client::build(
        options.timeout,
        options.follow_redirects,
        options.host_policy,
    )?;

    let mut request_builder = client.get(job.url);
    if let Plan::Continue { from, if_range } = &plan {
        request_builder = request_builder.header(RANGE, format!("bytes={}-", from));
        if let Some(validator) = if_range {
            request_builder = request_builder.header(IF_RANGE, validator);
        }
    }
    let request_builder = client::apply_headers(
        request_builder,
        options.user_agent.as_deref(),
        &options.headers,
    );

    let response = request_builder.send()?;

    if !options.follow_redirects && response.status().is_redirection() {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        return Err(Error::RedirectDisabled(status, location.to_string()));
    }

    let content_range = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let reply = resume::interpret(&plan, response.status(), content_range.as_deref());

    match reply {
        Reply::AlreadyComplete => {
            job.target.verify(options.verify.as_ref())?;
            let finished = job.target.finish(&options.on_occupied)?;
            if !quiet {
                eprintln!("File is already fully retrieved, nothing to download");
                eprintln!("\n✅ Download complete: {}", job.output_path);
            }
            return Ok(finished.into_outcome());
        }
        Reply::Refetch => {
            // The partial data no longer matches the remote file; the 416 body is not the file.
            if !quiet {
                eprintln!("Partial data does not match the remote file, starting from scratch");
            }
            job.target.discard_partial();
            return fetch(job, None, None, Plan::Fresh);
        }
        Reply::WrongOffset => {
            return Err(Error::ProtocolError(
                "server answered a resume request at the wrong offset".to_string(),
            ));
        }
        _ => {}
    }

    // Checked before any file is opened, so an error page never replaces the download.
    let mut response = client::ensure_success(response)?;
    log::debug!("{} answered {}", job.url, response.status());

    let (start, total) = match (&reply, &plan) {
        (Reply::Append { total }, Plan::Continue { from, .. }) => {
            if !quiet {
                eprintln!("Resuming download from byte {}", from);
            }
            (*from, *total)
        }
        (Reply::Restart { validator_failed }, _) => {
            if !quiet {
                if *validator_failed {
                    eprintln!("The remote file changed, starting from scratch");
                } else {
                    eprintln!("Server doesn't support resume, starting from scratch");
                }
            }
            (0, response.content_length())
        }
        _ => (0, response.content_length()),
    };

    let appending = matches!(reply, Reply::Append { .. });
    if appending && source == Some(Source::Final) {
        job.target.adopt_final()?;
    }

    let mut file = if appending {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(job.target.work_path())?
    } else {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(job.target.work_path())?
    };

    if job.target.is_staged() {
        let meta = PartMeta::from_headers(job.url, response.headers(), total).inherit(saved_meta);
        if let Err(e) = meta.write(job.target.meta_path()) {
            // Only costs the ability to validate a later resume.
            log::debug!(
                "could not write {}: {}",
                job.target.meta_path().display(),
                e
            );
        }
    }

    let remaining = response.content_length().unwrap_or(0);
    let progress = if !quiet {
        let bar_total = if remaining > 0 { start + remaining } else { 0 };
        let wrapper = ProgressBarWrapper::new(bar_total, start);
        if let Some(mp) = job.multi_progress {
            mp.add(wrapper.get_bar().clone());
        }
        Some(wrapper)
    } else {
        None
    };

    let mut throttle = Throttle::new(options.limit_rate);
    let written = stream::copy(
        &mut response,
        &mut file,
        &mut throttle,
        options.timeout,
        None,
        |n| {
            if let Some(ref p) = progress {
                p.inc(n);
            }
        },
    )?;
    log::debug!(
        "{} bytes written to {} in {:?}",
        written,
        job.target.work_path().display(),
        started.elapsed()
    );

    // Checked before anything is put in place, so a bad download replaces nothing.
    job.target.verify(options.verify.as_ref())?;
    let finished = job.target.finish(&options.on_occupied)?;

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
