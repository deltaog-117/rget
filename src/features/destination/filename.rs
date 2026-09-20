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


//! Choosing a file name.
//!
//! A name taken from a URL is untrusted text: once percent-decoded it can contain `/`, control
//! characters, `..`, or a leading `-` that a shell would read as an option. It is made safe
//! here, so that joining it to the download directory always yields a file *inside* that
//! directory. A name the user gives explicitly (`-O`) is theirs and is used as written.

use url::Url;

/// Used when the URL offers no usable name.
const FALLBACK_NAME: &str = "downloaded";

/// Longest URL-derived name, in bytes. Filesystems allow 255, but the download appends
/// `.part` and `.part.meta` (and `.part<N>` for segments), so that headroom is left.
const MAX_NAME_BYTES: usize = 240;

/// The longest extension (dot included) kept intact when a long name is shortened.
const MAX_KEPT_EXTENSION_BYTES: usize = 16;

/// Uses `explicit` (`-O`) when given, otherwise a safe version of the last path segment of
/// `url`, falling back to `"downloaded"` when the path has no usable name.
pub fn file_name_for(url: &Url, explicit: Option<&str>) -> String {
    if let Some(name) = explicit {
        return name.to_string();
    }
    let segment = url.path_segments().and_then(|mut s| s.next_back()).unwrap_or("");
    safe_name(&decode(segment))
}

/// Percent-decodes `segment`. Bytes that are not valid UTF-8 are left as written rather than
/// guessed at.
fn decode(segment: &str) -> String {
    let bytes = urlencoding::decode_binary(segment.as_bytes()).into_owned();
    String::from_utf8(bytes).unwrap_or_else(|_| segment.to_string())
}

/// Invisible characters that reorder or hide text, used to disguise an extension.
fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Turns arbitrary text into a single, harmless path component.
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' || c.is_control() || is_bidi_control(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return FALLBACK_NAME.to_string();
    }
    // A leading `-` would be taken for an option by `rm *` and friends.
    let unambiguous = match trimmed.strip_prefix('-') {
        Some(rest) => format!("_{}", rest),
        None => trimmed.to_string(),
    };
    shorten(&unambiguous, MAX_NAME_BYTES)
}

/// `text` cut to at most `max` bytes on a character boundary.
fn cut(text: &str, max: usize) -> &str {
    let mut end = max.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Where the extension of a long `name` starts: the last dot, or the one before it when that
/// gives a short compound extension such as `.tar.gz`. `None` when there is nothing to keep.
fn extension_start(name: &str) -> Option<usize> {
    let last = name.rfind('.').filter(|&dot| dot > 0)?;
    let compound = name[..last]
        .rfind('.')
        .filter(|&prev| prev > 0 && last - prev <= 5 && name.len() - prev <= MAX_KEPT_EXTENSION_BYTES);
    let start = compound.unwrap_or(last);
    (name.len() - start <= MAX_KEPT_EXTENSION_BYTES).then_some(start)
}

/// Shortens `name` to at most `max` bytes, keeping a short extension (`.gz`, `.tar.gz`).
fn shorten(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_string();
    }
    match extension_start(name) {
        Some(dot) => format!("{}{}", cut(&name[..dot], max - (name.len() - dot)), &name[dot..]),
        None => cut(name, max).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slashes_and_control_characters_become_underscores() {
        assert_eq!(safe_name("a/b\\c"), "a_b_c");
        assert_eq!(safe_name("a\0b\nc\td\u{7f}"), "a_b_c_d_");
        assert_eq!(safe_name("evil\u{202E}gpj.exe"), "evil_gpj.exe");
    }

    #[test]
    fn dot_names_and_blanks_fall_back() {
        for name in ["", " ", "   ", ".", "..", " .. "] {
            assert_eq!(safe_name(name), FALLBACK_NAME, "{name:?}");
        }
    }

    #[test]
    fn a_leading_dash_is_replaced_but_a_dash_inside_is_not() {
        assert_eq!(safe_name("-rf"), "_rf");
        // Only the first character can make a shell read the name as an option.
        assert_eq!(safe_name("--help.txt"), "_-help.txt");
        assert_eq!(safe_name("a-b.txt"), "a-b.txt");
    }

    #[test]
    fn legal_but_unusual_characters_are_kept() {
        for name in ["file(1).zip", "a;b.txt", "a$b.txt", "a b.txt", "café.txt", ".env.example", "a&b|c.txt", "日本語.pdf"] {
            assert_eq!(safe_name(name), name);
        }
    }

    #[test]
    fn a_long_name_is_cut_on_a_character_boundary_and_keeps_its_extension() {
        let long = format!("{}.tar.gz", "é".repeat(200));
        let short = safe_name(&long);
        assert!(short.len() <= MAX_NAME_BYTES, "{}", short.len());
        assert!(short.ends_with(".tar.gz") && short.starts_with('é'));
    }

    #[test]
    fn a_compound_extension_is_kept_whole_but_a_long_one_is_not() {
        let name = |ext: &str| safe_name(&format!("{}{}", "x".repeat(300), ext));
        for ext in [".tar.gz", ".tar.xz", ".tar.bz2", ".zip", ".iso"] {
            assert!(name(ext).ends_with(ext) && name(ext).len() == MAX_NAME_BYTES, "{ext}");
        }
        // Not an extension worth keeping: a dot followed by a long tail.
        assert_eq!(name(".averylongsuffixthatisnotanextension").len(), MAX_NAME_BYTES);
    }

    #[test]
    fn a_long_name_without_a_usable_extension_is_simply_cut() {
        assert_eq!(safe_name(&"a".repeat(500)).len(), MAX_NAME_BYTES);
        let long_extension = format!("x.{}", "b".repeat(300));
        assert_eq!(safe_name(&long_extension).len(), MAX_NAME_BYTES);
    }
}
