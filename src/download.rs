use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};
use reqwest::blocking::Client;
use reqwest::header::{RANGE, USER_AGENT};
use crate::error::{Result, RgetError};
use crate::progress::ProgressBarWrapper;
use indicatif::MultiProgress;

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
) -> Result<()> {
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

    // Create progress bar
    let progress = if !quiet {
        let wrapper = ProgressBarWrapper::new(total_size, existing_size);
        if let Some(mp) = multi_progress {
            mp.add(wrapper.get_bar().clone());
        }
        Some(wrapper)
    } else {
        None
    };

    // --- RATE LIMITING ---
    let mut bytes_written_this_second = 0;
    let mut second_start = Instant::now();
    let chunk_size = 8192;

    let bytes = response.bytes()?;
    for chunk in bytes.chunks(chunk_size) {
        // Apply rate limiting
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
