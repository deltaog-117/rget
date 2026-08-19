mod cli;
mod download;
mod validator;
mod progress;
mod error;
mod checksum;
mod config;

use clap::Parser;
use cli::Args;
use validator::validate_url;
use download::download_file;
use error::Result;
use indicatif::MultiProgress;
use crossbeam_channel::unbounded;
use std::sync::Arc;
use std::thread;
use std::path::Path;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.init {
        match config::Config::write_default_config() {
            Ok(()) => {
                eprintln!("🎉 You can now customize ~/.config/rget/config.toml");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("❌ Failed to write config: {}", e);
                std::process::exit(1);
            }
        }
    }

    if args.urls.is_empty() {
        eprintln!("Error: At least one URL is required");
        eprintln!("Usage: rget [OPTIONS] <URL> [URL...]");
        eprintln!("       rget --init");
        std::process::exit(1);
    }

    if args.segments > 1 && args.urls.len() > 1 {
        eprintln!("Error: --segments can only be used with a single URL");
        std::process::exit(1);
    }

    let config = if !args.no_config {
        config::Config::load()
    } else {
        config::Config::default()
    };

    let timeout = args.timeout.unwrap_or_else(|| config.timeout.unwrap_or(30));
    let retries = args.retries.unwrap_or_else(|| config.retries.unwrap_or(0));
    let user_agent = args.user_agent.or_else(|| config.user_agent.clone());
    let quiet = args.quiet || config.quiet.unwrap_or(false);
    let jobs = args.jobs.unwrap_or_else(|| config.jobs.unwrap_or(1));
    let follow_redirects = args.follow_redirects.unwrap_or_else(|| config.follow_redirects.unwrap_or(true));
    let resume = args.resume || config.resume.unwrap_or(false);
    let limit_rate = args.limit_rate.or_else(|| config.limit_rate);
    let segments = if args.segments > 1 { args.segments } else { config.segments.unwrap_or(1) };
    let directory_prefix = args.directory_prefix.or_else(|| config.directory_prefix.clone());

    if args.urls.len() > 1 && args.output.is_some() {
        eprintln!("Error: -O can only be used with a single URL");
        std::process::exit(1);
    }

    let mut tasks: Vec<(String, String)> = Vec::new();
    for url_str in &args.urls {
        let url = validate_url(url_str)?;
        let base_name = if let Some(name) = &args.output {
            name.clone()
        } else {
            url.path_segments()
                .and_then(|segments| segments.last())
                .filter(|&name| !name.is_empty())
                .unwrap_or("downloaded")
                .to_string()
        };

        let output_path = if let Some(prefix) = &directory_prefix {
            let dir = Path::new(prefix);
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
            dir.join(&base_name).to_string_lossy().to_string()
        } else {
            base_name
        };

        tasks.push((url_str.clone(), output_path));
    }

    if !quiet && args.verbose > 0 {
        eprintln!("🔍 Downloading {} URL(s)", tasks.len());
        eprintln!("⏱️  Timeout: {}s", timeout);
        if resume {
            eprintln!("🔄 Resume: enabled");
        }
        if retries > 0 {
            eprintln!("🔁 Retries: {}", retries);
        }
        if args.sha256.is_some() && tasks.len() == 1 {
            eprintln!("🔐 SHA‑256 verification: enabled");
        } else if args.sha256.is_some() && tasks.len() > 1 {
            eprintln!("⚠️  SHA‑256 verification only supported with a single URL, ignoring");
        }
        if jobs > 1 {
            eprintln!("📦 Parallel jobs: {}", jobs);
        }
        if segments > 1 {
            eprintln!("🧩 Segments: {}", segments);
        }
        if let Some(limit) = limit_rate {
            let limit_str = format_size(limit);
            eprintln!("🚀 Rate limit: {}/s", limit_str);
        }
        if let Some(prefix) = &directory_prefix {
            eprintln!("📂 Output directory: {}", prefix);
        }
        if !args.no_config {
            eprintln!("⚙️  Config: ~/.config/rget/config.toml");
        }
    }

    let config = Arc::new((
        resume,
        timeout,
        follow_redirects,
        user_agent,
        retries,
        quiet,
        limit_rate,
        segments,
    ));

    let multi_progress = if jobs > 1 {
        Some(MultiProgress::new())
    } else {
        None
    };

    let (task_sender, task_receiver) = unbounded::<(String, String)>();
    let (result_sender, result_receiver) = unbounded::<(String, String, Result<()>)>();

    let mut handles = Vec::new();
    for _ in 0..jobs {
        let task_receiver = task_receiver.clone();
        let result_sender = result_sender.clone();
        let config = config.clone();
        let multi_progress = multi_progress.as_ref().map(|mp| mp.clone());
        let handle = thread::spawn(move || {
            let (resume, timeout, follow_redirects, user_agent, retries, quiet, limit_rate, segments) = &*config;
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
                    *limit_rate,
                    *segments,
                );
                let _ = result_sender.send((url, output_path, result));
            }
        });
        handles.push(handle);
    }

    for task in tasks {
        let _ = task_sender.send(task);
    }
    drop(task_sender);

    let mut error_count = 0;
    for _ in 0..args.urls.len() {
        let (url, output, result) = result_receiver.recv().unwrap();
        match result {
            Ok(()) => {
                if !quiet {
                    eprintln!("✅ {} -> {}", url, output);
                }
            }
            Err(e) => {
                eprintln!("❌ {} -> {}", url, e);
                error_count += 1;
            }
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    if let Some(expected) = args.sha256 {
        if args.urls.len() == 1 {
            let url = validate_url(&args.urls[0])?;
            let base_name = if let Some(name) = args.output {
                name
            } else {
                url.path_segments()
                    .and_then(|segments| segments.last())
                    .filter(|&name| !name.is_empty())
                    .unwrap_or("downloaded")
                    .to_string()
            };
            let output_path = if let Some(prefix) = &directory_prefix {
                Path::new(prefix).join(&base_name).to_string_lossy().to_string()
            } else {
                base_name
            };
            if !quiet {
                eprintln!("🔐 Verifying SHA‑256 checksum...");
            }
            checksum::verify_sha256(&output_path, &expected)?;
            if !quiet {
                eprintln!("✅ SHA‑256 checksum verified");
            }
        }
    }

    if error_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn format_size(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}
