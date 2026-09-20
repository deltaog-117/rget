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


//! SHA-256 file verification.

use super::digest::Sha256Digest;
use super::error::Error;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// The SHA-256 of the file at `path`, read in constant memory.
///
/// # Errors
///
/// Returns the I/O error if the file cannot be opened or read.
pub fn digest_of_file(path: impl AsRef<Path>) -> Result<Sha256Digest, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(Sha256Digest::from_bytes(hasher.finalize().into()))
}

/// The SHA-256 of the file at `path` as lower-case hexadecimal.
pub fn compute_sha256(file_path: &str) -> Result<String, std::io::Error> {
    Ok(digest_of_file(file_path)?.to_hex())
}

/// Checks the file at `path` against `expected`.
///
/// # Errors
///
/// [`Error::ChecksumMismatch`] (naming both digests) or an I/O error.
pub fn verify_file(path: impl AsRef<Path>, expected: &Sha256Digest) -> Result<(), Error> {
    let actual = digest_of_file(path)?;
    if &actual == expected {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            expected: expected.to_hex(),
            actual: actual.to_hex(),
        })
    }
}

/// Like [`verify_file`], taking the expected digest as text.
///
/// # Errors
///
/// [`Error::InvalidDigest`] when `expected` is not a SHA-256 digest, otherwise as [`verify_file`].
pub fn verify_sha256(file_path: &str, expected: &str) -> Result<(), Error> {
    verify_file(file_path, &Sha256Digest::parse(expected)?)
}
