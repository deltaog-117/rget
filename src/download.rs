use std::fs::OpenOptions;
use std::io::Write;
use std::thread;
use std::time::Duration;
use reqwest::blocking::Client;
use reqwest::header::{RANGE, USER_AGENT};
use crate::error::{Result, RgetError};
use crate::progress::ProgressBarWrapper;

pub fn download_file(
    url: &str,
    output_path: &str,
    resume: bool,
    timeout: u64,
    follow_redirects: bool,
    user_agent: Option<&str>,
    retries: u32,
) -> Result<()> {
    let mut attempt = 0;
    let max_attempts = retries + 1; // Initial attempt + retries

    loop {
        attempt += 1;
        let result = attempt_download(
            url,
            output_path,
            resume,
            timeout,
            follow_redirects,
            user_agent,
        );

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                // If we've used all attempts, return the error
                if attempt >= max_attempts {
                    eprintln!("❌ Failed after {} attempts", attempt);
                    return Err(e);
                }

                // Calculate backoff with jitter
                let base_delay = 2u64.pow(attempt - 1); // 1, 2, 4, 8, 16...
                let jitter = rand::random::<u64>() % 1000; // 0-999ms jitter
                let delay = Duration::from_secs(base_delay) + Duration::from_millis(jitter);

                eprintln!(
                    "⚠️  Attempt {} failed: {}. Retrying in {}s...",
                    attempt,
                    e,
                    delay.as_secs()
                );
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
) -> Result<()> {
    // Build client
    let mut client_builder = Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(10));

    if !follow_redirects {
        client_builder = client_builder.redirect(reqwest::redirect::Policy::none());
    }

    let client = client_builder.build()?;

    // Check if file exists for resume
    let existing_size = if resume {
        std::fs::metadata(output_path)
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    // Build request with Range header if resuming
    let mut request_builder = client.get(url);
    if resume && existing_size > 0 {
        request_builder = request_builder.header(RANGE, format!("bytes={}-", existing_size));
    }

    // Set user-agent
    if let Some(ua) = user_agent {
        request_builder = request_builder.header(USER_AGENT, ua);
    } else {
        request_builder = request_builder.header(USER_AGENT, "rget/0.1.0");
    }

    // Send request
    let response = request_builder.send()?;

    // If redirects are disabled, check if we got a redirect
    if !follow_redirects && response.status().is_redirection() {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");
        return Err(RgetError::RedirectDisabled(status, location.to_string()));
    }

    // Check if server supports resume
    if resume && existing_size > 0 {
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            eprintln!("Resuming download from byte {}", existing_size);
        } else {
            eprintln!("Server doesn't support resume, starting from scratch");
            // Re-open file without append
            let _ = std::fs::write(output_path, &[])?;
        }
    }

    // Get total size from Content-Length header
    let total_size = response.content_length().unwrap_or(0);

    // Open file (append if resuming)
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
    let progress = ProgressBarWrapper::new(total_size, existing_size);

    // Stream response body
    let bytes = response.bytes()?;
    for chunk in bytes.chunks(8192) {
        file.write_all(chunk)?;
        progress.inc(chunk.len() as u64);
    }

    progress.finish();
    eprintln!("\n✅ Download complete: {}", output_path);

    Ok(())
}
