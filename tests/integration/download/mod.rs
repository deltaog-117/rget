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


//! Shared fixtures for the download tests: a tiny in-process HTTP server.
//!
//! The download feature does not validate URLs (that is `validation`'s job), so it can
//! be pointed straight at 127.0.0.1.

mod resume;
mod segmented;
mod single;

use rget::features::download::DownloadOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;

/// Deterministic, non-repeating-looking bytes so a misplaced segment shows up.
pub(crate) fn payload(len: usize) -> Vec<u8> {
    (0..len).map(|i| ((i * 31 + i / 251) % 256) as u8).collect()
}

pub(crate) fn options() -> DownloadOptions {
    DownloadOptions {
        resume: false,
        timeout: 10,
        follow_redirects: true,
        user_agent: None,
        retries: 0,
        quiet: true,
        limit_rate: None,
        segments: 1,
        headers: Vec::new(),
    }
}

pub(crate) fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rget-it-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Serves `/file` (the payload) and `/redirect` (302 to `/file`) on a random port
/// and returns the base URL. With `ranges` off the server ignores `Range` and does
/// not advertise `Accept-Ranges`.
pub(crate) fn serve(payload: Vec<u8>, ranges: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let payload = payload.clone();
            thread::spawn(move || handle(stream, &payload, ranges));
        }
    });
    base
}

fn handle(mut stream: TcpStream, payload: &[u8], ranges: bool) {
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
    let text = String::from_utf8_lossy(&request).to_string();
    let mut lines = text.lines();
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default().to_string();
    let path = first.next().unwrap_or_default().to_string();
    let range = lines
        .find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=").map(str::to_string));

    let head_only = method == "HEAD";
    let (status, extra, body): (&str, String, &[u8]) = match (path.as_str(), range) {
        ("/redirect", _) => ("302 Found", "Location: /file\r\n".to_string(), &[]),
        ("/file", Some(spec)) if ranges => {
            let (start, end) = spec.split_once('-').unwrap();
            let start: usize = start.parse().unwrap();
            let end: usize = if end.is_empty() { payload.len() - 1 } else { end.parse().unwrap() };
            let end = end.min(payload.len() - 1);
            (
                "206 Partial Content",
                format!(
                    "Content-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\n",
                    start,
                    end,
                    payload.len()
                ),
                &payload[start..=end],
            )
        }
        ("/file", _) => (
            "200 OK",
            if ranges { "Accept-Ranges: bytes\r\n".to_string() } else { String::new() },
            payload,
        ),
        _ => ("404 Not Found", String::new(), b"not found"),
    };

    let head = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    if !head_only {
        let _ = stream.write_all(body);
    }
}
