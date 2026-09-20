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


//! Application-level error: the sum of every feature error `run` can propagate.

use crate::features::{download, integrity, validation};
use std::fmt;
use thiserror::Error;

#[derive(Error)]
pub enum AppError {
    #[error(transparent)]
    Validation(#[from] validation::Error),

    #[error(transparent)]
    Download(#[from] download::Error),

    #[error(transparent)]
    Integrity(#[from] integrity::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// `main` prints `Error: {:?}`. Forwarding to the wrapped error's `Debug` keeps
// that output byte-for-byte what it was before errors were split per feature.
impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Validation(e) => fmt::Debug::fmt(e, f),
            AppError::Download(e) => fmt::Debug::fmt(e, f),
            AppError::Integrity(e) => fmt::Debug::fmt(e, f),
            AppError::Io(e) => write!(f, "Io({:?})", e),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
