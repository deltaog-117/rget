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
use std::fs::File;
use std::io::{BufRead, BufReader, stdin};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // --- HANDLE --version ---
    if args.version {
        println!("rget version {}", VERSION);
        std::process::exit(0);
    }

    // --- HANDLE --init ---
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

    // --- LOAD CONFIG ---
    let config = if !args.no_config {
        config::Config::load()
    } else {
        config::Config::default()
    };

    // --- MERGE CONFIG & CLI ---
    let timeout = args.timeout.unwrap_or_else(|| config.timeout.unwrap_or(30));
    let retries = args.retries.unwrap_or_else(|| config.retries.unwrap_or(0));
    let user_agent = args.user_agent.or_else(|| config.user_agent.clone());
    let quiet = args.quiet || config.quiet.unwrap_or(false);
    let jobs = args.jobs.unwrap_or_else(|| config.jobs.unwrap_or(1));
    let follow_redirects = args.follow_redirects.unwrap_or_else(|| config.follow_redirects.unwrap_or(true));
    let resume = args.resume || config.resume.unwrap_or(false);
    let limit_rate = args.limit_rate.or_else(|| config.limit_rate);
    let segments = if args.segments > 1 { args.segments } else { config.segments.unwrap_or(1) };

    // Borrow directory_prefix without moving it
    let directory_prefix = args.directory_prefix.as_ref().or_else(|| config.directory_prefix.as_ref()).cloned();

    // --- PARSE HEADERS ---
    let custom_headers: Vec<(String, String)> = args
        .header
        .iter()
        .filter_map(|h| {
            let parts: Vec<&str> = h.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                eprintln!("⚠️  Ignoring malformed header: {}", h);
                None
            }
        })
        .collect();

    // --- COLLECT URLS FROM INPUT FILE OR ARGS ---
    let urls: Vec<String> = if let Some(input_file) = &args.input_file {
        if input_file == "-" {
            // Read from stdin
            let stdin = stdin();
            let reader = stdin.lock();
            reader
                .lines()
                .filter_map(|line| line.ok())
                .filter(|line| !line.trim().is_empty())
                .collect()
        } else {
            // Read from file
            let file = File::open(input_file)
                .unwrap_or_else(|e| {
                    eprintln!("❌ Failed to open input file '{}': {}", input_file, e);
                    std::process::exit(1);
                });
            let reader = BufReader::new(file);
            reader
                .lines()
                .filter_map(|line| line.ok())
                .filter(|line| !line.trim().is_empty())
                .collect()
        }
    } else {
        args.urls.clone()
    };

    if urls.is_empty() {
        eprintln!("Error: No URLs provided");
        eprintln!("Usage: rget [OPTIONS] <URL> [URL...]");
        eprintln!("       rget -i urls.txt");
        eprintln!("       rget --init");
        eprintln!("       rget --version");
        std::process::exit(1);
    }

    if segments > 1 && urls.len() > 1 {
        eprintln!("Error: --segments can only be used with a single URL");
        std::process::exit(1);
    }

    if urls.len() > 1 && args.output.is_some() {
        eprintln!("Error: -O can only be used with a single URL");
        std::process::exit(1);
    }

    // --- BUILD TASKS ---
    let mut tasks: Vec<(String, String)> = Vec::new();
    for url_str in &urls {
        let url = validate_url(url_str)?;
        // Borrow output without moving it
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
        } else if args.output.is_some() || directory_prefix.is_some() {
            // If -O or -P was explicitly given, use that (already handled above)
            base_name
        } else {
            // Default Downloads folder
            if let Some(download_dir) = dirs::download_dir() {
                let dir = download_dir;
                if !dir.exists() {
                    std::fs::create_dir_all(&dir)?;
                }
                dir.join(&base_name).to_string_lossy().to_string()
            } else {
                // Fallback to current directory
                base_name
            }
        };

        tasks.push((url_str.clone(), output_path));
    }

    // --- VERBOSE OUTPUT ---
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
        } else if args.output.is_none() && directory_prefix.is_none() {
            if let Some(download_dir) = dirs::download_dir() {
                eprintln!("📂 Default output directory: {}", download_dir.display());
            }
        }
        if !custom_headers.is_empty() {
            eprintln!("📋 Custom headers: {:?}", custom_headers);
        }
        if !args.no_config {
            eprintln!("⚙️  Config: ~/.config/rget/config.toml");
        }
    }

    // --- SHARED CONFIG ---
    let shared_config = Arc::new((
        resume,
        timeout,
        follow_redirects,
        user_agent,
        retries,
        quiet,
        limit_rate,
        segments,
        custom_headers,
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
        let config = shared_config.clone();
        let multi_progress = multi_progress.as_ref().map(|mp| mp.clone());
        let handle = thread::spawn(move || {
            let (resume, timeout, follow_redirects, user_agent, retries, quiet, limit_rate, segments, headers) = &*config;
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
                    headers,
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
    for _ in 0..urls.len() {
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

    // --- CHECKSUM VERIFICATION (single URL only) ---
    if let Some(expected) = args.sha256 {
        if urls.len() == 1 {
            let url = validate_url(&urls[0])?;
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
                Path::new(prefix).join(&base_name).to_string_lossy().to_string()
            } else if args.output.is_some() || directory_prefix.is_some() {
                base_name
            } else if let Some(download_dir) = dirs::download_dir() {
                download_dir.join(&base_name).to_string_lossy().to_string()
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
