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

use super::client;
use super::error::{Error, Result};
use super::options::DownloadOptions;
use super::resume;
use super::throttle::Throttle;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::header::{ACCEPT_RANGES, RANGE};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

/// Splits `total_size` bytes into `segments` inclusive `(start, end)` ranges;
/// the last range absorbs the remainder.
///
/// FIXME(B): underflows when `total_size < segments` (`part_size` is 0).
fn plan_ranges(total_size: u64, segments: usize) -> Vec<(u64, u64)> {
    let part_size = total_size / segments as u64;
    let mut ranges = Vec::with_capacity(segments);
    let mut start = 0;
    for i in 0..segments {
        let end = if i == segments - 1 {
            total_size - 1
        } else {
            start + part_size - 1
        };
        if start >= total_size {
            break;
        }
        ranges.push((start, end));
        start = end + 1;
    }
    ranges
}

/// Concatenates the part files into `output_path`, deleting each part as it is consumed.
///
/// FIXME(B): reads each part fully into memory before writing it.
fn merge_parts(output_path: &str, part_paths: &[String]) -> Result<()> {
    let mut output_file = File::create(output_path)?;
    for part_path in part_paths {
        if !std::path::Path::new(part_path).exists() {
            return Err(Error::ProtocolError(format!(
                "Part file {} missing",
                part_path
            )));
        }
        let mut part_file = File::open(part_path)?;
        let mut buffer = Vec::new();
        part_file.read_to_end(&mut buffer)?;
        output_file.write_all(&buffer)?;
        let _ = std::fs::remove_file(part_path);
    }
    Ok(())
}

pub(super) fn download(
    url: &str,
    output_path: &str,
    options: &DownloadOptions,
    multi_progress: Option<&MultiProgress>,
) -> Result<()> {
    let quiet = options.quiet;
    let segments = options.segments;

    // FIXME(B): the probe ignores custom headers, the User-Agent and --follow-redirects.
    let client = client::build(options.timeout, true)?;

    let head_response = match client.head(url).send() {
        Ok(r) => r,
        Err(e) => {
            if !quiet {
                eprintln!("⚠️  HEAD request failed: {}; falling back to single-thread.", e);
            }
            return super::download_single(url, output_path, options, multi_progress);
        }
    };

    let total_size = head_response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if !quiet {
        eprintln!("🔍 DEBUG: total_size = {}", total_size);
        eprintln!("🔍 DEBUG: segments = {}", segments);
    }

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

    let mut ranges = plan_ranges(total_size, segments);

    if !quiet {
        for (i, (start, end)) in ranges.iter().enumerate() {
            eprintln!("🔍 DEBUG: Range {}: {}-{} (length: {})", i, start, end, end - start + 1);
        }
    }

    if ranges.is_empty() {
        if !quiet {
            eprintln!("⚠️  Invalid range calculation; falling back to single-thread.");
            eprintln!(
                "   (total_size={}, segments={}, part_size={})",
                total_size,
                segments,
                total_size / segments as u64
            );
        }
        return super::download_single(url, output_path, options, multi_progress);
    }

    let part_paths: Vec<String> = (0..ranges.len())
        .map(|i| format!("{}.part{}", output_path, i))
        .collect();

    let resume_positions = if options.resume {
        resume::segment_positions(&part_paths, &mut ranges)
    } else {
        vec![0u64; ranges.len()]
    };

    let progress_bar = if !quiet {
        let bar = ProgressBar::new(total_size);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .expect("valid template")
                .progress_chars("━▸ "),
        );
        if options.resume {
            let downloaded: u64 = resume_positions.iter().sum();
            bar.set_position(downloaded);
        }
        Some(Arc::new(Mutex::new(bar)))
    } else {
        None
    };

    let mut handles = Vec::with_capacity(ranges.len());
    for i in 0..ranges.len() {
        let url = url.to_string();
        let part_path = part_paths[i].clone();
        let (start, end) = ranges[i];
        if start > end || start >= total_size {
            continue;
        }
        let user_agent = options.user_agent.clone();
        let timeout = options.timeout;
        let limit_rate = options.limit_rate;
        let progress_bar = progress_bar.clone();
        let resume_pos = resume_positions[i];
        let headers = options.headers.clone();

        let handle = thread::spawn(move || -> Result<()> {
            let client = client::build(timeout, true)?;

            let mut request_builder = client.get(&url);
            if resume_pos > 0 {
                let new_start = start + resume_pos;
                if new_start <= end {
                    request_builder = request_builder.header(RANGE, format!("bytes={}-{}", new_start, end));
                } else {
                    return Ok(());
                }
            } else {
                request_builder = request_builder.header(RANGE, format!("bytes={}-{}", start, end));
            }

            let request_builder =
                client::apply_headers(request_builder, user_agent.as_deref(), &headers);

            let response = request_builder.send()?;

            if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                return Err(Error::ProtocolError(format!(
                    "Range not satisfiable: {}-{} (total: {})",
                    start, end, total_size
                )));
            }

            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                && response.status() != reqwest::StatusCode::OK
            {
                return Err(Error::ProtocolError(format!(
                    "Unexpected status code: {}",
                    response.status()
                )));
            }

            let mut file = if resume_pos > 0 {
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&part_path)?
            } else {
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&part_path)?
            };

            let mut throttle = Throttle::new(limit_rate);
            let chunk_size = 8192;

            // FIXME(A1): buffers the whole part in memory before writing anything.
            let bytes = response.bytes()?;
            for chunk in bytes.chunks(chunk_size) {
                throttle.wait(chunk.len());

                file.write_all(chunk)?;
                if let Some(ref pb) = progress_bar {
                    let bar = pb.lock().unwrap();
                    bar.inc(chunk.len() as u64);
                }
            }

            if !quiet {
                eprintln!("✅ Part {} complete", i);
            }
            Ok(())
        });

        handles.push(handle);
    }

    let mut error_count = 0;
    let mut thread_errors = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(Ok(())) => { /* ok */ }
            Ok(Err(e)) => {
                eprintln!("❌ Thread error: {}", e);
                error_count += 1;
                thread_errors.push(e);
            }
            Err(_) => {
                eprintln!("❌ Thread panicked!");
                error_count += 1;
            }
        }
    }

    if error_count > 0 {
        let all_416 = thread_errors.iter().all(|e| {
            if let Error::ProtocolError(msg) = e {
                msg.contains("Range not satisfiable")
            } else {
                false
            }
        });
        if all_416 {
            if !quiet {
                eprintln!("⚠️  Server rejected byte ranges; falling back to single-thread.");
            }
            for path in &part_paths {
                let _ = std::fs::remove_file(path);
            }
            return super::download_single(url, output_path, options, multi_progress);
        } else {
            return Err(Error::ProtocolError(format!(
                "{} segments failed: {:?}",
                error_count,
                thread_errors
            )));
        }
    }

    merge_parts(output_path, &part_paths)?;

    if let Some(pb) = progress_bar {
        let bar = pb.lock().unwrap();
        bar.finish();
    }

    if !quiet {
        eprintln!("\n✅ Download complete: {}", output_path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_the_file_exactly_once() {
        for (total, segments) in [(100u64, 4usize), (3_000_017, 4), (3_000_017, 7), (10, 3), (5, 5)] {
            let ranges = plan_ranges(total, segments);
            assert_eq!(ranges.len(), segments);
            assert_eq!(ranges[0].0, 0);
            assert_eq!(ranges.last().unwrap().1, total - 1);
            for pair in ranges.windows(2) {
                assert_eq!(pair[0].1 + 1, pair[1].0);
            }
        }
    }

    #[test]
    fn the_last_range_absorbs_the_remainder() {
        assert_eq!(
            plan_ranges(3_000_017, 4),
            vec![(0, 750_003), (750_004, 1_500_007), (1_500_008, 2_250_011), (2_250_012, 3_000_016)]
        );
    }

    #[test]
    fn a_single_segment_is_the_whole_file() {
        assert_eq!(plan_ranges(42, 1), vec![(0, 41)]);
    }
}
