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


//! Wiring: input → validate → destination → download → integrity.

use super::cli::Args;
use super::config::Config;
use super::error::Result;
use super::settings::Settings;
use crate::features::{destination, download, input, integrity, validation};
use crate::shared::size::format_size;
use clap::Parser;
use indicatif::MultiProgress;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    // --- HANDLE --version ---
    if args.version {
        println!("rget version {}", VERSION);
        std::process::exit(0);
    }

    // --- HANDLE --init ---
    if args.init {
        match Config::write_default_config() {
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

    // --- LOAD CONFIG & MERGE WITH CLI ---
    let config = if !args.no_config {
        Config::load()
    } else {
        Config::default()
    };
    let settings = Settings::resolve(&args, &config);
    let quiet = settings.quiet;

    // --- COLLECT URLS FROM INPUT FILE OR ARGS ---
    let urls: Vec<String> = if let Some(input_file) = &args.input_file {
        match input::read_url_list(input_file) {
            Ok(urls) => urls,
            Err(e) => {
                eprintln!("❌ Failed to open input file '{}': {}", input_file, e);
                std::process::exit(1);
            }
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

    if settings.segments > 1 && urls.len() > 1 {
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
        let url = validation::validate_url_with(url_str, settings.host_policy())?;
        let base_name = destination::file_name_for(&url, args.output.as_deref());
        let output_path = destination::output_path(
            &base_name,
            settings.directory_prefix.as_deref(),
            args.output.is_some(),
        )?;
        tasks.push((url_str.clone(), output_path));
    }

    // --- VERBOSE OUTPUT ---
    if !quiet && args.verbose > 0 {
        eprintln!("🔍 Downloading {} URL(s)", tasks.len());
        eprintln!("⏱️  Timeout: {}s", settings.timeout);
        if settings.resume {
            eprintln!("🔄 Resume: enabled");
        }
        if settings.retries > 0 {
            eprintln!("🔁 Retries: {}", settings.retries);
        }
        if args.sha256.is_some() && tasks.len() == 1 {
            eprintln!("🔐 SHA‑256 verification: enabled");
        } else if args.sha256.is_some() && tasks.len() > 1 {
            eprintln!("⚠️  SHA‑256 verification only supported with a single URL, ignoring");
        }
        if settings.jobs > 1 {
            eprintln!("📦 Parallel jobs: {}", settings.jobs);
        }
        if settings.segments > 1 {
            eprintln!("🧩 Segments: {}", settings.segments);
        }
        if let Some(limit) = settings.limit_rate {
            let limit_str = format_size(limit);
            eprintln!("🚀 Rate limit: {}/s", limit_str);
        }
        if let Some(prefix) = &settings.directory_prefix {
            eprintln!("📂 Output directory: {}", prefix);
        } else if args.output.is_none() && settings.directory_prefix.is_none() {
            if let Some(download_dir) = destination::default_download_dir() {
                eprintln!("📂 Default output directory: {}", download_dir.display());
            }
        }
        if !settings.headers.is_empty() {
            eprintln!("📋 Custom headers: {:?}", settings.headers);
        }
        if settings.allow_private {
            eprintln!("🛡️  Private addresses: allowed (--allow-private)");
        }
        if !args.no_config {
            eprintln!("⚙️  Config: ~/.config/rget/config.toml");
        }
    }

    // --- DOWNLOAD ---
    let options = Arc::new(settings.download_options());

    let multi_progress = if settings.jobs > 1 {
        Some(MultiProgress::new())
    } else {
        None
    };

    // Path of the only download, kept for checksum verification below.
    let single_output_path = tasks.first().map(|(_, output)| output.clone());

    let mut error_count = 0;
    download::run_pool(tasks, settings.jobs, options, multi_progress, |url, output, result| {
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
    });

    // --- CHECKSUM VERIFICATION (single URL only) ---
    if let Some(expected) = args.sha256 {
        if let (1, Some(output_path)) = (urls.len(), single_output_path) {
            if !quiet {
                eprintln!("🔐 Verifying SHA‑256 checksum...");
            }
            integrity::verify_sha256(&output_path, &expected)?;
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
