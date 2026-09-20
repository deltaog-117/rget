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

use crate::features::destination::ExistingFile;
use crate::features::integrity::Sha256Digest;
use crate::shared::size::parse_size;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

/// Parses `--sha256` at the command line, so a malformed digest is refused before any download.
fn parse_digest(text: &str) -> Result<Sha256Digest, String> {
    Sha256Digest::parse(text).map_err(|e| e.to_string())
}

/// What to do when the file to be written already exists (`--if-exists`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IfExists {
    /// Replace it (the default)
    Overwrite,
    /// Leave it alone and do not download
    Skip,
    /// Keep it and save the new download as `name (1).ext`, `name (2).ext`, ...
    Rename,
}

impl From<IfExists> for ExistingFile {
    fn from(choice: IfExists) -> Self {
        match choice {
            IfExists::Overwrite => ExistingFile::Overwrite,
            IfExists::Skip => ExistingFile::Skip,
            IfExists::Rename => ExistingFile::Rename,
        }
    }
}

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

    /// SHA‑256 checksum the download must match (only with a single URL). Either case; a whole
    /// `sha256sum` line may be pasted. A mismatch discards the download and replaces nothing
    #[arg(long, value_parser = parse_digest)]
    pub sha256: Option<Sha256Digest>,

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

    /// Allow downloads from loopback, private and link-local addresses (blocked by default)
    #[arg(long)]
    pub allow_private: bool,

    /// What to do when the file already exists (default: overwrite). Not combinable with -c
    #[arg(long, value_enum)]
    pub if_exists: Option<IfExists>,

    /// Do not download a file that already exists (same as --if-exists skip)
    #[arg(short = 'n', long, conflicts_with = "if_exists")]
    pub no_clobber: bool,

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
