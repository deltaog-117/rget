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


//! Content checks on a URL that has already parsed.
//!
//! What matters is what a URL *means* once parsed, not which characters appear in its text:
//! rget never hands a URL to a shell, and `& ; $ ( ) |` are ordinary sub-delimiters in a path
//! or query (`/file(1).zip?a=1&b=2`). The parser also normalises `..` segments, strips
//! CR/LF and percent-encodes the rest. Three things are left for us to check:
//!
//! - the host, where the parser accepts characters that no hostname can contain;
//! - path traversal, judged on the raw text because the parser has already resolved it;
//! - a short list of well-known secret files, matched as whole decoded path segments.

use super::error::{Error, Result};
use url::{Host, Url};

/// Whole path segments that name a secret file.
const SENSITIVE_NAMES: [&str; 3] = [".env", ".bashrc", ".zshrc"];

/// Two consecutive path segments that name a secret file.
const SENSITIVE_PAIRS: [(&str, &str); 7] = [
    ("etc", "passwd"),
    ("etc", "shadow"),
    ("etc", "sudoers"),
    (".git", "config"),
    (".aws", "credentials"),
    (".ssh", "id_rsa"),
    (".ssh", "authorized_keys"),
];

/// Runs every check. `raw` is the text as the user typed it, `url` what it parsed to.
pub(super) fn check(raw: &str, url: &Url) -> Result<()> {
    check_host_characters(url)?;
    check_traversal(raw)?;
    check_sensitive_path(url)?;
    Ok(())
}

fn blocked(reason: String) -> Error {
    Error::InvalidUrl(format!("Blocked: {}", reason))
}

/// A hostname is letters, digits, `.`, `-` and `_` (an internationalised name has already
/// been converted to ASCII). The parser accepts more, such as `exa;mple.com`.
fn check_host_characters(url: &Url) -> Result<()> {
    if let Some(Host::Domain(domain)) = url.host() {
        let invalid = domain
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')));
        if let Some(c) = invalid {
            return Err(blocked(format!("hostname contains invalid character '{}'", c)));
        }
    }
    Ok(())
}

fn decode(text: &str) -> String {
    String::from_utf8_lossy(&urlencoding::decode_binary(text.as_bytes())).into_owned()
}

/// The path of `raw` exactly as written: what follows the authority, up to `?` or `#`.
/// Tabs and newlines are dropped first, as the parser drops them.
fn raw_path(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !matches!(c, '\t' | '\n' | '\r')).collect();
    let after_scheme = cleaned.trim().split_once(':').map_or("", |(_, rest)| rest);
    // A query or fragment ends the authority and the path alike, so cut there first: text
    // after a `?` must never be mistaken for a path.
    let before_query = after_scheme
        .trim_start_matches(['/', '\\'])
        .split(['?', '#'])
        .next()
        .unwrap_or("");
    before_query
        .find(['/', '\\'])
        .map_or("", |start| &before_query[start..])
        .to_string()
}

/// Whether any path segment is `..` once percent-decoded (`%2e%2e`, `.%2E`, `..%2f`) and
/// split on `/` and `\`. Only the path counts; `a..b` and `?next=../x` are ordinary.
fn has_dot_dot_segment(path: &str) -> bool {
    path.split(['/', '\\'])
        .any(|segment| decode(segment).split(['/', '\\']).any(|piece| piece == ".."))
}

fn check_traversal(raw: &str) -> Result<()> {
    if has_dot_dot_segment(&raw_path(raw)) {
        return Err(blocked("Path traversal detected".to_string()));
    }
    Ok(())
}

/// The decoded, lower-cased, non-empty pieces of the path.
fn path_pieces(url: &Url) -> Vec<String> {
    let mut pieces = Vec::new();
    for segment in url.path_segments().into_iter().flatten() {
        for piece in decode(segment).to_lowercase().split(['/', '\\']) {
            if !piece.is_empty() {
                pieces.push(piece.to_string());
            }
        }
    }
    pieces
}

/// The sensitive file the path names, if any.
fn sensitive_file(url: &Url) -> Option<String> {
    let pieces = path_pieces(url);
    if let Some(name) = pieces.iter().find(|p| SENSITIVE_NAMES.contains(&p.as_str())) {
        return Some(name.clone());
    }
    pieces.windows(2).find_map(|pair| {
        SENSITIVE_PAIRS
            .iter()
            .find(|(a, b)| pair[0] == *a && pair[1] == *b)
            .map(|(a, b)| format!("{}/{}", a, b))
    })
}

fn check_sensitive_path(url: &Url) -> Result<()> {
    match sensitive_file(url) {
        Some(name) => Err(blocked(format!("Access to sensitive file '{}'", name))),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    #[test]
    fn the_raw_path_is_what_follows_the_authority() {
        assert_eq!(raw_path("http://example.com/a/b?x=1#f"), "/a/b");
        assert_eq!(raw_path("http://example.com"), "");
        assert_eq!(raw_path("http://example.com?x=/../"), "");
        assert_eq!(raw_path("http://example.com#/../"), "");
        assert_eq!(raw_path("http://example.com/a?x=/../#/../"), "/a");
        assert_eq!(raw_path("https://user:pw@example.com:8080/a/../b"), "/a/../b");
        assert_eq!(raw_path("  http://example.com/a  "), "/a");
    }

    #[test]
    fn the_raw_path_survives_the_odd_spellings_the_parser_accepts() {
        assert_eq!(raw_path("http:example.com/a/.."), "/a/..");
        assert_eq!(raw_path("http:\\\\example.com\\a\\.."), "\\a\\..");
        assert_eq!(raw_path("http://example.com/a/.\t./b"), "/a/../b");
    }

    #[test]
    fn dot_dot_segments_are_found_in_every_encoding() {
        for path in ["/../x", "/a/../b", "/a/..", "/a/%2e%2e/b", "/a/%2E%2E/b", "/a/.%2e/b", "/a/%2e./b", "/a\\..\\b", "/a/..%2fb", "/a/..%5cb"] {
            assert!(has_dot_dot_segment(path), "{path}");
        }
    }

    #[test]
    fn dots_inside_a_name_are_not_traversal() {
        for path in ["/a..b", "/..a", "/a../b", "/...", "/a/./b", "/file..txt", "/v1.2..3/x", "/.", "/%2e", "/.hidden"] {
            assert!(!has_dot_dot_segment(path), "{path}");
        }
    }

    #[test]
    fn hostnames_allow_letters_digits_dots_hyphens_and_underscores() {
        for host in ["example.com", "a-b.example.co.uk", "ex_ample.com", "xn--mnchen-3ya.de", "localhost", "1.2.3.4", "[::1]"] {
            assert!(check_host_characters(&parse(&format!("http://{host}/"))).is_ok(), "{host}");
        }
    }

    #[test]
    fn characters_no_hostname_can_contain_are_refused_even_though_the_parser_accepts_them() {
        for c in [';', '&', '$', '(', ')', '`', '!', '*', '\'', '='] {
            let url = Url::parse(&format!("http://exa{c}mple.com/x"));
            if let Ok(url) = url {
                let err = check_host_characters(&url).unwrap_err();
                assert!(matches!(&err, Error::InvalidUrl(m) if m.contains(&format!("'{c}'"))), "{c}: {err:?}");
            }
        }
        assert!(check_host_characters(&parse("http://exa;mple.com/")).is_err());
    }

    #[test]
    fn sensitive_names_match_whole_decoded_segments_only() {
        assert_eq!(sensitive_file(&parse("http://h/.env")), Some(".env".into()));
        assert_eq!(sensitive_file(&parse("http://h/a/.ENV")), Some(".env".into()));
        assert_eq!(sensitive_file(&parse("http://h/%2eenv")), Some(".env".into()));
        assert_eq!(sensitive_file(&parse("http://h/home/.bashrc")), Some(".bashrc".into()));
        for ok in ["http://h/.env.example", "http://h/environment", "http://h/my.env", "http://h/.envrc", "http://h/bashrc", "http://h/a?f=.env", "http://h/a#.env"] {
            assert_eq!(sensitive_file(&parse(ok)), None, "{ok}");
        }
    }

    #[test]
    fn sensitive_pairs_need_both_segments_in_order() {
        assert_eq!(sensitive_file(&parse("http://h/etc/passwd")), Some("etc/passwd".into()));
        assert_eq!(sensitive_file(&parse("http://h/x/etc/shadow")), Some("etc/shadow".into()));
        assert_eq!(sensitive_file(&parse("http://h/repo/.git/config")), Some(".git/config".into()));
        assert_eq!(sensitive_file(&parse("http://h/.ssh/id_rsa")), Some(".ssh/id_rsa".into()));
        assert_eq!(sensitive_file(&parse("http://h/etc%2fpasswd")), Some("etc/passwd".into()));
        for ok in ["http://h/passwd/etc", "http://h/etc/passwd.bak", "http://h/etc/x/passwd", "http://h/.git/configuration", "http://h/etc", "http://h/passwd"] {
            assert_eq!(sensitive_file(&parse(ok)), None, "{ok}");
        }
    }
}
