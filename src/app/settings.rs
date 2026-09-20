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


//! Merges CLI arguments and the config file into one resolved set of settings.

use super::cli::Args;
use super::config::Config;
use crate::features::download::DownloadOptions;
use crate::shared::address::HostPolicy;

/// Effective settings: a CLI value wins over the config file, which wins over the default.
#[derive(Debug)]
pub struct Settings {
    pub timeout: u64,
    pub retries: u32,
    pub user_agent: Option<String>,
    pub quiet: bool,
    pub jobs: usize,
    pub follow_redirects: bool,
    pub resume: bool,
    pub limit_rate: Option<usize>,
    pub segments: usize,
    pub directory_prefix: Option<String>,
    pub headers: Vec<(String, String)>,
    pub allow_private: bool,
}

impl Settings {
    pub fn resolve(args: &Args, config: &Config) -> Self {
        Self {
            timeout: args.timeout.unwrap_or_else(|| config.timeout.unwrap_or(30)),
            retries: args.retries.unwrap_or_else(|| config.retries.unwrap_or(0)),
            user_agent: args.user_agent.clone().or_else(|| config.user_agent.clone()),
            quiet: args.quiet || config.quiet.unwrap_or(false),
            jobs: args.jobs.unwrap_or_else(|| config.jobs.unwrap_or(1)),
            follow_redirects: args
                .follow_redirects
                .unwrap_or_else(|| config.follow_redirects.unwrap_or(true)),
            resume: args.resume || config.resume.unwrap_or(false),
            limit_rate: args.limit_rate.or(config.limit_rate),
            segments: if args.segments > 1 {
                args.segments
            } else {
                config.segments.unwrap_or(1)
            },
            directory_prefix: args
                .directory_prefix
                .as_ref()
                .or(config.directory_prefix.as_ref())
                .cloned(),
            headers: parse_headers(&args.header),
            allow_private: args.allow_private || config.allow_private.unwrap_or(false),
        }
    }

    /// Whether local and private destinations may be contacted (`--allow-private`).
    pub fn host_policy(&self) -> HostPolicy {
        if self.allow_private { HostPolicy::AllowPrivate } else { HostPolicy::BlockPrivate }
    }

    /// The subset of settings the download feature cares about.
    pub fn download_options(&self) -> DownloadOptions {
        DownloadOptions {
            resume: self.resume,
            timeout: self.timeout,
            follow_redirects: self.follow_redirects,
            user_agent: self.user_agent.clone(),
            retries: self.retries,
            quiet: self.quiet,
            limit_rate: self.limit_rate,
            segments: self.segments,
            headers: self.headers.clone(),
            host_policy: self.host_policy(),
        }
    }
}

/// Splits `"Key: Value"` strings; malformed entries are reported and skipped.
fn parse_headers(raw: &[String]) -> Vec<(String, String)> {
    raw.iter()
        .filter_map(|h| {
            let parts: Vec<&str> = h.splitn(2, ':').collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                eprintln!("⚠️  Ignoring malformed header: {}", h);
                None
            }
        })
        .collect()
}
