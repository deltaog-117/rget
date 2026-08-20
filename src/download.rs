use std::fs::{File, OpenOptions};
use std::io::{Write, Read};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use reqwest::blocking::Client;
use reqwest::header::{RANGE, USER_AGENT, ACCEPT_RANGES};
use crate::error::{Result, RgetError};
use crate::progress::ProgressBarWrapper;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub fn download_file(
    url: &str,
    output_path: &str,
    resume: bool,
    timeout: u64,
    follow_redirects: bool,
    user_agent: Option<&str>,
    retries: u32,
    quiet: bool,
    multi_progress: Option<&MultiProgress>,
    limit_rate: Option<usize>,
    segments: usize,
    headers: &[(String, String)],
) -> Result<()> {
    if segments > 1 {
        return download_segmented(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
            retries,
            quiet,
            multi_progress,
            limit_rate,
            segments,
            headers,
        );
    }

    let mut attempt = 0;
    let max_attempts = retries + 1;

    loop {
        attempt += 1;
        let result = attempt_download(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
            quiet,
            multi_progress,
            limit_rate,
            headers,
        );

        match result {
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

fn attempt_download(
    url: &str,
    output_path: &str,
    resume: bool,
    timeout: u64,
    follow_redirects: bool,
    user_agent: Option<&str>,
    quiet: bool,
    multi_progress: Option<&MultiProgress>,
    limit_rate: Option<usize>,
    headers: &[(String, String)],
) -> Result<()> {
    let mut client_builder = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(10));

    if !follow_redirects {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
    }

    let client = client_builder.build()?;

    let existing_size = if resume {
        std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    let mut request_builder = client.get(url);
    if resume && existing_size > 0 {
        request_builder = request_builder.header(RANGE, format!("bytes={}-", existing_size));
    }

    if let Some(ua) = user_agent {
        request_builder = request_builder.header(USER_AGENT, ua);
    } else {
        request_builder = request_builder.header(USER_AGENT, "rget/0.1.0");
    }

    // Apply custom headers
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }

    let response = request_builder.send()?;

    if !follow_redirects && response.status().is_redirection() {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        return Err(RgetError::RedirectDisabled(status, location.to_string()));
    }

    if resume && existing_size > 0 {
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            if !quiet {
                eprintln!("Resuming download from byte {}", existing_size);
            }
        } else {
            if !quiet {
                eprintln!("Server doesn't support resume, starting from scratch");
            }
            let _ = std::fs::write(output_path, &[])?;
        }
    }

    let total_size = response.content_length().unwrap_or(0);

    let mut file = if resume && existing_size > 0 {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(output_path)?
    } else {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?
    };

    let progress = if !quiet {
        let wrapper = ProgressBarWrapper::new(total_size, existing_size);
        if let Some(mp) = multi_progress {
            mp.add(wrapper.get_bar().clone());
        }
        Some(wrapper)
    } else {
        None
    };

    let mut bytes_written_this_second = 0;
    let mut second_start = Instant::now();
    let chunk_size = 8192;

    let bytes = response.bytes()?;
    for chunk in bytes.chunks(chunk_size) {
        if let Some(limit) = limit_rate {
            if limit > 0 {
                bytes_written_this_second += chunk.len() as usize;
                if bytes_written_this_second >= limit {
                    let elapsed = second_start.elapsed();
                    if elapsed < Duration::from_secs(1) {
                        let sleep_time = Duration::from_secs(1) - elapsed;
                        thread::sleep(sleep_time);
                    }
                    bytes_written_this_second = 0;
                    second_start = Instant::now();
                }
            }
        }

        file.write_all(chunk)?;
        if let Some(ref p) = progress {
            p.inc(chunk.len() as u64);
        }
    }

    if let Some(p) = progress {
        p.finish();
    }

    if !quiet {
        eprintln!("\n✅ Download complete: {}", output_path);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Segmented download with headers support
// ---------------------------------------------------------------------------

fn download_segmented(
    url: &str,
    output_path: &str,
    resume: bool,
    timeout: u64,
    follow_redirects: bool,
    user_agent: Option<&str>,
    retries: u32,
    quiet: bool,
    multi_progress: Option<&MultiProgress>,
    limit_rate: Option<usize>,
    segments: usize,
    headers: &[(String, String)],
) -> Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let head_response = match client.head(url).send() {
        Ok(r) => r,
        Err(e) => {
            if !quiet {
                eprintln!("⚠️  HEAD request failed: {}; falling back to single-thread.", e);
            }
            return download_file(
                url,
                output_path,
                resume,
                timeout,
                follow_redirects,
                user_agent,
                retries,
                quiet,
                multi_progress,
                limit_rate,
                1,
                headers,
            );
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
        return download_file(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
            retries,
            quiet,
            multi_progress,
            limit_rate,
            1,
            headers,
        );
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
        return download_file(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
            retries,
            quiet,
            multi_progress,
            limit_rate,
            1,
            headers,
        );
    }

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

    if !quiet {
        for (i, (start, end)) in ranges.iter().enumerate() {
            eprintln!("🔍 DEBUG: Range {}: {}-{} (length: {})", i, start, end, end - start + 1);
        }
    }

    if ranges.is_empty() {
        if !quiet {
            eprintln!("⚠️  Invalid range calculation; falling back to single-thread.");
            eprintln!("   (total_size={}, segments={}, part_size={})", total_size, segments, part_size);
        }
        return download_file(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
            retries,
            quiet,
            multi_progress,
            limit_rate,
            1,
            headers,
        );
    }

    let part_paths: Vec<String> = (0..ranges.len())
        .map(|i| format!("{}.part{}", output_path, i))
        .collect();

    let mut resume_positions = vec![0u64; ranges.len()];
    if resume {
        for (i, path) in part_paths.iter().enumerate() {
            if let Ok(metadata) = std::fs::metadata(path) {
                let size = metadata.len();
                let (expected_start, expected_end) = ranges[i];
                let expected_size = expected_end - expected_start + 1;
                if size >= expected_size {
                    resume_positions[i] = expected_size;
                } else {
                    resume_positions[i] = size;
                    ranges[i].0 = expected_start + size;
                }
            }
        }
    }

    let progress_bar = if !quiet {
        let bar = ProgressBar::new(total_size);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .expect("valid template")
                .progress_chars("━▸ "),
        );
        if resume {
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
        let user_agent = user_agent.map(|s| s.to_string());
        let timeout = timeout;
        let limit_rate = limit_rate;
        let quiet = quiet;
        let progress_bar = progress_bar.clone();
        let resume_pos = resume_positions[i];
        let headers = headers.to_vec();

        let handle = thread::spawn(move || -> Result<()> {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(timeout))
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()?;

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

            if let Some(ua) = &user_agent {
                request_builder = request_builder.header(USER_AGENT, ua);
            } else {
                request_builder = request_builder.header(USER_AGENT, "rget/0.1.0");
            }

            // Apply custom headers
            for (key, value) in &headers {
                request_builder = request_builder.header(key, value);
            }

            let response = request_builder.send()?;

            if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                return Err(RgetError::ProtocolError(format!(
                    "Range not satisfiable: {}-{} (total: {})",
                    start, end, total_size
                )));
            }

            if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                && response.status() != reqwest::StatusCode::OK
            {
                return Err(RgetError::ProtocolError(format!(
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

            let mut bytes_written_this_second = 0;
            let mut second_start = Instant::now();
            let chunk_size = 8192;

            let bytes = response.bytes()?;
            for chunk in bytes.chunks(chunk_size) {
                if let Some(limit) = limit_rate {
                    if limit > 0 {
                        bytes_written_this_second += chunk.len() as usize;
                        if bytes_written_this_second >= limit {
                            let elapsed = second_start.elapsed();
                            if elapsed < Duration::from_secs(1) {
                                let sleep_time = Duration::from_secs(1) - elapsed;
                                thread::sleep(sleep_time);
                            }
                            bytes_written_this_second = 0;
                            second_start = Instant::now();
                        }
                    }
                }

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
            if let RgetError::ProtocolError(msg) = e {
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
            return download_file(
                url,
                output_path,
                resume,
                timeout,
                follow_redirects,
                user_agent,
                retries,
                quiet,
                multi_progress,
                limit_rate,
                1,
                headers,
            );
        } else {
            return Err(RgetError::ProtocolError(format!(
                "{} segments failed: {:?}",
                error_count,
                thread_errors
            )));
        }
    }

    let mut output_file = File::create(output_path)?;
    for part_path in part_paths {
        if !std::path::Path::new(&part_path).exists() {
            return Err(RgetError::ProtocolError(format!(
                "Part file {} missing",
                part_path
            )));
        }
        let mut part_file = File::open(&part_path)?;
        let mut buffer = Vec::new();
        part_file.read_to_end(&mut buffer)?;
        output_file.write_all(&buffer)?;
        let _ = std::fs::remove_file(&part_path);
    }

    if let Some(pb) = progress_bar {
        let bar = pb.lock().unwrap();
        bar.finish();
    }

    if !quiet {
        eprintln!("\n✅ Download complete: {}", output_path);
    }

    Ok(())
}
