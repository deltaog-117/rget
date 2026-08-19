mod cli;
mod download;
mod validator;
mod progress;
mod error;
mod checksum;

use clap::Parser;
use cli::Args;
use validator::validate_url;
use download::download_file;
use error::Result;
use indicatif::MultiProgress;
use crossbeam_channel::unbounded;
use std::sync::Arc;
use std::thread;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // Validate number of URLs and -O usage
    if args.urls.len() > 1 && args.output.is_some() {
        eprintln!("Error: -O can only be used with a single URL");
        std::process::exit(1);
    }

    // Build list of tasks: (url, output_path)
    let mut tasks: Vec<(String, String)> = Vec::new();
    for url_str in &args.urls {
        let url = validate_url(url_str)?;
        let output_path = if let Some(name) = &args.output {
            name.clone()
        } else {
            url.path_segments()
                .and_then(|segments| segments.last())
                .filter(|&name| !name.is_empty())
                .unwrap_or("downloaded")
                .to_string()
        };
        tasks.push((url_str.clone(), output_path));
    }

    // If verbose, print summary
    if !args.quiet && args.verbose > 0 {
        eprintln!("🔍 Downloading {} URL(s)", tasks.len());
        eprintln!("⏱️  Timeout: {}s", args.timeout);
        if args.resume {
            eprintln!("🔄 Resume: enabled");
        }
        if args.retries > 0 {
            eprintln!("🔁 Retries: {}", args.retries);
        }
        if args.sha256.is_some() && tasks.len() == 1 {
            eprintln!("🔐 SHA‑256 verification: enabled");
        } else if args.sha256.is_some() && tasks.len() > 1 {
            eprintln!("⚠️  SHA‑256 verification only supported with a single URL, ignoring");
        }
        if args.jobs > 1 {
            eprintln!("📦 Parallel jobs: {}", args.jobs);
        }
    }

    // Prepare shared config
    let config = Arc::new((
        args.resume,
        args.timeout,
        args.follow_redirects,
        args.user_agent,
        args.retries,
        args.quiet,
    ));

    // Create a MultiProgress if we have multiple jobs
    let multi_progress = if args.jobs > 1 {
        Some(MultiProgress::new())
    } else {
        None
    };

    // Create channels for tasks and results
    let (task_sender, task_receiver) = unbounded::<(String, String)>();
    let (result_sender, result_receiver) = unbounded::<(String, String, Result<()>)>();

    // Spawn worker threads
    let mut handles = Vec::new();
    for _ in 0..args.jobs {
        let task_receiver = task_receiver.clone();
        let result_sender = result_sender.clone();
        let config = config.clone();
        let multi_progress = multi_progress.as_ref().map(|mp| mp.clone());
        let handle = thread::spawn(move || {
            // Clone the config so we can move it into the closure
            let (resume, timeout, follow_redirects, user_agent, retries, quiet) = &*config;
            while let Ok((url, output_path)) = task_receiver.recv() {
                let result = download_file(
                    &url,
                    &output_path,
                    *resume,
                    *timeout,
                    *follow_redirects,
                    user_agent.as_deref(),
                    *retries,
                    *quiet,
                    multi_progress.as_ref(),
                );
                let _ = result_sender.send((url, output_path, result));
            }
        });
        handles.push(handle);
    }

    // Send tasks to workers
    for task in tasks {
        let _ = task_sender.send(task);
    }
    drop(task_sender);

    // Collect results
    let mut error_count = 0;
    for _ in 0..args.urls.len() {
        let (url, output, result) = result_receiver.recv().unwrap();
        match result {
            Ok(()) => {
                if !args.quiet {
                    eprintln!("✅ {} -> {}", url, output);
                }
            }
            Err(e) => {
                eprintln!("❌ {} -> {}", url, e);
                error_count += 1;
            }
        }
    }

    // Wait for threads to finish
    for handle in handles {
        let _ = handle.join();
    }

    // If checksum provided and only one URL, verify
    if let Some(expected) = args.sha256 {
        if args.urls.len() == 1 {
            let url = validate_url(&args.urls[0])?;
            let output_path = if let Some(name) = args.output {
                name
            } else {
                url.path_segments()
                    .and_then(|segments| segments.last())
                    .filter(|&name| !name.is_empty())
                    .unwrap_or("downloaded")
                    .to_string()
            };
            if !args.quiet {
                eprintln!("🔐 Verifying SHA‑256 checksum...");
            }
            checksum::verify_sha256(&output_path, &expected)?;
            if !args.quiet {
                eprintln!("✅ SHA‑256 checksum verified");
            }
        }
    }

    if error_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}
