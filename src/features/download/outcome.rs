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


//! What became of a download.

/// Where a finished download ended up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The file is at this path. It differs from the requested path when the requested one was
    /// taken and the policy asked for a numbered alternative.
    Saved(String),
    /// The download was discarded because a file was already at this path.
    Skipped(String),
}

impl Outcome {
    /// The path the outcome is about.
    pub fn path(&self) -> &str {
        match self {
            Outcome::Saved(path) | Outcome::Skipped(path) => path,
        }
    }
}
