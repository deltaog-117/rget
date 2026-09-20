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


//! Everything a download needs besides the URL and the destination.

use crate::shared::address::HostPolicy;

/// Per-download settings, resolved once by the caller.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub resume: bool,
    pub timeout: u64,
    pub follow_redirects: bool,
    pub user_agent: Option<String>,
    pub retries: u32,
    pub quiet: bool,
    pub limit_rate: Option<usize>,
    pub segments: usize,
    pub headers: Vec<(String, String)>,
    /// Whether redirects and DNS answers that lead to local or private addresses are refused.
    pub host_policy: HostPolicy,
}
