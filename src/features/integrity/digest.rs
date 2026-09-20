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


//! A SHA-256 digest as a value that cannot be malformed.

use super::error::Error;
use std::fmt;
use std::str::FromStr;

/// Exactly 32 bytes. Built from text with [`Sha256Digest::parse`], so case never matters
/// afterwards and a wrong length or a non-hex character is refused before any work is done.
#[derive(Clone, PartialEq, Eq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Parses 64 hexadecimal digits in either case. Only the first whitespace-separated word
    /// is read, so a whole line of `sha256sum` output (`<digest>  <file name>`) can be pasted.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDigest`] unless that word is exactly 64 hexadecimal digits.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let word = text.split_whitespace().next().unwrap_or("");
        let invalid = || Error::InvalidDigest(word.chars().take(80).collect());
        if word.len() != 64 || !word.is_ascii() {
            return Err(invalid());
        }
        let mut bytes = [0u8; 32];
        for (byte, pair) in bytes.iter_mut().zip(word.as_bytes().chunks(2)) {
            let pair = std::str::from_utf8(pair).map_err(|_| invalid())?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| invalid())?;
        }
        Ok(Self(bytes))
    }

    /// Lower-case hexadecimal, the form `sha256sum` prints.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Digest({})", self.to_hex())
    }
}

impl FromStr for Sha256Digest {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        Self::parse(text)
    }
}
