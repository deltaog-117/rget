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


//! Command-line interface definition.

use crate::shared::size::parse_size;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rget")]
#[command(about = "A safe, modern downloader for Linux", long_about = None)]
pub struct Args {
    /// URLs to download (can specify multiple)
    #[arg()]
    pub urls: Vec<String>,

    /// Output filename (only valid with a single URL)
    #[arg(short = 'O', long)]
    pub output: Option<String>,

    /// Output directory prefix
    #[arg(short = 'P', long)]
    pub directory_prefix: Option<String>,

    /// Resume an incomplete download
    #[arg(short = 'c', long)]
    pub resume: bool,

    /// Verbose output
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Timeout in seconds: for connecting, and for each pause in the incoming data (default: 30)
    #[arg(short = 't', long)]
    pub timeout: Option<u64>,

    /// Follow redirects (default: true)
    #[arg(long)]
    pub follow_redirects: Option<bool>,

    /// Custom user-agent
    #[arg(short = 'A', long)]
    pub user_agent: Option<String>,

    /// Number of retries on failure (default: 0)
    #[arg(short = 'r', long)]
    pub retries: Option<u32>,

    /// Quiet mode (no output except errors)
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// SHA‑256 checksum to verify (only with single URL)
    #[arg(long)]
    pub sha256: Option<String>,

    /// Number of parallel downloads (default: 1)
    #[arg(short = 'j', long)]
    pub jobs: Option<usize>,

    /// Rate limit in bytes per second (e.g., 500k, 2M, 1G). Use 0 for no limit.
    #[arg(long, value_parser = parse_size)]
    pub limit_rate: Option<usize>,

    /// Number of parallel segments for a single file (default: 1)
    #[arg(long, default_value = "1")]
    pub segments: usize,

    /// Custom HTTP headers (can be used multiple times: -H "Key: Value")
    #[arg(short = 'H', long)]
    pub header: Vec<String>,

    /// Read URLs from a file (use - for stdin)
    #[arg(short = 'i', long)]
    pub input_file: Option<String>,

    /// Ignore config file
    #[arg(long)]
    pub no_config: bool,

    /// Generate a default configuration file and exit
    #[arg(long)]
    pub init: bool,

    /// Display version information and exit
    #[arg(long)]
    pub version: bool,
}
