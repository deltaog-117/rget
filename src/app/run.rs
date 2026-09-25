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
use crate::features::destination::{Claims, ExistingFile, Placement, SkipReason};
use crate::features::download::{Outcome, Relocate, Verifier};
use crate::features::{destination, download, input, integrity, validation};
use crate::shared::interrupt;
use crate::shared::size::format_size;
use clap::{CommandFactory, Parser};
use indicatif::MultiProgress;
use std::sync::{Arc, Mutex};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> Result<()> {
    env_logger::init();
    interrupt::install();
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
        Config::load(args.quiet)
    } else {
        Config::default()
    };
    if let Some(message) = Settings::resume_conflict(&args) {
        Args::command()
            .error(clap::error::ErrorKind::ArgumentConflict, message)
            .exit();
    }
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
    // Names are settled one URL at a time, before any download starts, so parallel downloads
    // can never pick the same file and the result does not depend on timing.
    let claims = Arc::new(Mutex::new(Claims::new()));
    let mut tasks: Vec<(String, String)> = Vec::new();
    let mut skipped: Vec<(String, String, SkipReason)> = Vec::new();
    let mut error_count = 0;
    for url_str in &urls {
        // An invalid URL is reported and skipped, so one bad URL in a batch never stops the
        // others from downloading.
        let url = match validation::validate_url_with(url_str, settings.host_policy()) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("❌ {} -> {}", url_str, e);
                error_count += 1;
                continue;
            }
        };
        let base_name = destination::file_name_for(&url, args.output.as_deref());
        let output_path = destination::output_path(
            &base_name,
            settings.directory_prefix.as_deref(),
            args.output.is_some(),
        )?;
        let placement = claims
            .lock()
            .expect("the name claims are only locked briefly and never poisoned")
            .place(&output_path, settings.if_exists);
        match placement {
            Placement::Write(path) => tasks.push((url_str.clone(), path)),
            Placement::Skip { path, reason } => skipped.push((url_str.clone(), path, reason)),
        }
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
        match settings.if_exists {
            ExistingFile::Overwrite => {}
            ExistingFile::Skip => eprintln!("📄 Existing files: skipped"),
            ExistingFile::Rename => eprintln!("📄 Existing files: kept, new download numbered"),
        }
        if !args.no_config {
            eprintln!("⚙️  Config: ~/.config/rget/config.toml");
        }
    }

    // --- SKIPPED (the file is already there) ---
    // `--sha256` only applies to a single URL, as before.
    let digest = args.sha256.clone().filter(|_| urls.len() == 1);
    for (url, path, reason) in &skipped {
        if !quiet {
            match reason {
                SkipReason::Exists => eprintln!("⏭️  {} -> {} already exists, skipping", url, path),
                SkipReason::EarlierUrl => eprintln!(
                    "⏭️  {} -> {} has the same file name as an earlier URL, skipping",
                    url, path
                ),
                SkipReason::NoFreeName => {
                    eprintln!("⏭️  {} -> no free file name for {}, skipping", url, path)
                }
            }
        }
        // An existing file that was kept can still be checked against the digest.
        if let (SkipReason::Exists, Some(digest)) = (reason, &digest) {
            if !quiet {
                eprintln!("🔐 Verifying SHA‑256 checksum of the existing file...");
            }
            match integrity::verify_file(path, digest) {
                Ok(()) if !quiet => eprintln!("✅ SHA‑256 checksum verified"),
                Ok(()) => {}
                Err(e) => {
                    eprintln!("❌ {} -> Verification failed: {}", path, e);
                    error_count += 1;
                }
            }
        }
    }

    // --- DOWNLOAD ---
    let mut download_options = settings.download_options();
    // The digest is checked on the finished file *before* it is put in place, so a bad
    // download replaces nothing.
    download_options.verify = digest.map(|digest| {
        Verifier::new(move |path| {
            if !quiet {
                eprintln!("🔐 Verifying SHA‑256 checksum...");
            }
            integrity::verify_file(path, &digest).map_err(|e| e.to_string())?;
            if !quiet {
                eprintln!("✅ SHA‑256 checksum verified");
            }
            Ok(())
        })
    });
    download_options.on_occupied = match settings.if_exists {
        ExistingFile::Overwrite => download::OnOccupied::Replace,
        ExistingFile::Skip => download::OnOccupied::Skip,
        ExistingFile::Rename => {
            let claims = Arc::clone(&claims);
            let relocate: Relocate =
                Arc::new(move |occupied| claims.lock().ok()?.relocate(occupied));
            download::OnOccupied::Relocate(relocate)
        }
    };
    let options = Arc::new(download_options);

    let multi_progress = if settings.jobs > 1 {
        Some(MultiProgress::new())
    } else {
        None
    };

    download::run_pool(
        tasks,
        settings.jobs,
        options,
        multi_progress,
        |url, _requested, result| match result {
            Ok(Outcome::Saved(path)) => {
                if !quiet {
                    eprintln!("✅ {} -> {}", url, path);
                }
            }
            Ok(Outcome::Skipped(path)) => {
                if !quiet {
                    eprintln!("⏭️  {} -> {} already exists, skipped", url, path);
                }
            }
            // Already announced once by the Ctrl+C handler itself.
            Err(download::Error::Interrupted) => {
                error_count += 1;
            }
            Err(e) => {
                eprintln!("❌ {} -> {}", url, e);
                error_count += 1;
            }
        },
    );

    // Conventional shell exit code for a process that stopped on SIGINT.
    if interrupt::requested() {
        std::process::exit(130);
    }
    if error_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}
