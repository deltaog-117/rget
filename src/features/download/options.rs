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
use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Checks a finished download before it is put in place. `Err` carries the reason it is
/// refused. The caller supplies the check (a checksum, say), so `download` knows nothing
/// about what is being verified.
type Check = dyn Fn(&Path) -> Result<(), String> + Send + Sync;

#[derive(Clone)]
pub struct Verifier(Arc<Check>);

impl Verifier {
    pub fn new(check: impl Fn(&Path) -> Result<(), String> + Send + Sync + 'static) -> Self {
        Self(Arc::new(check))
    }

    pub(super) fn check(&self, path: &Path) -> Result<(), String> {
        (self.0)(path)
    }
}

impl fmt::Debug for Verifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Verifier(..)")
    }
}

/// Picks another path for a download whose target is occupied. Given the occupied path, it
/// returns the path to try instead, or `None` to give up.
pub type Relocate = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// What to do when the target path is occupied at the moment the finished download is put in
/// place. The check is atomic: a file that appears while downloading is never overwritten by
/// the last two variants.
#[derive(Clone, Default)]
pub enum OnOccupied {
    /// Replace whatever is there.
    #[default]
    Replace,
    /// Keep it and discard the download.
    Skip,
    /// Keep it and save under the path this returns.
    Relocate(Relocate),
}

impl fmt::Debug for OnOccupied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OnOccupied::Replace => f.write_str("Replace"),
            OnOccupied::Skip => f.write_str("Skip"),
            OnOccupied::Relocate(_) => f.write_str("Relocate(..)"),
        }
    }
}

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
    /// Checks the finished file before it replaces anything; a failure discards the download.
    pub verify: Option<Verifier>,
    /// What to do when the target turns out to be occupied.
    pub on_occupied: OnOccupied,
}
