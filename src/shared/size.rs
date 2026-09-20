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


//! Human-readable byte sizes.

/// Parse human-readable size strings like "500k", "2M", "1G" (or "0" for no limit)
pub fn parse_size(s: &str) -> Result<usize, String> {
    let s = s.trim();
    if s == "0" {
        return Ok(0);
    }
    if s.is_empty() {
        return Err("Empty size string".to_string());
    }

    let (num_str, suffix) = {
        let chars: Vec<char> = s.chars().collect();
        let mut num_end = 0;
        for (i, c) in chars.iter().enumerate() {
            if c.is_ascii_digit() || *c == '.' {
                num_end = i + 1;
            } else {
                break;
            }
        }
        if num_end == 0 {
            return Err(format!("Invalid size format: {}", s));
        }
        let num_str: String = chars[..num_end].iter().collect();
        let suffix: String = chars[num_end..].iter().collect();
        (num_str, suffix.to_lowercase())
    };

    let num: f64 = num_str.parse().map_err(|_| format!("Invalid number: {}", num_str))?;

    let bytes = match suffix.as_str() {
        "" => num,
        "b" => num,
        "k" | "kb" => num * 1024.0,
        "m" | "mb" => num * 1024.0 * 1024.0,
        "g" | "gb" => num * 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => num * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return Err(format!("Unknown size suffix: {}", suffix)),
    };

    if bytes < 1.0 {
        return Err("Rate limit must be at least 1 byte".to_string());
    }

    Ok(bytes as usize)
}

/// Format a byte count with a binary unit, e.g. `1536` -> `"1.5 KB"`.
pub fn format_size(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}
