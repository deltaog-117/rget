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
//!
//! Routes: `/file` (the payload, honouring `Range` unless disabled), `/redirect` (302 to
//! `/file`), `/status/<code>` (that status with a short body), `/trickle` (the payload in
//! ten pieces, 300 ms apart), `/stall` (half the payload, then silence), and `/guarded`
//! (like `/file`, but 403 unless the request carries `X-Token: ok` and `User-Agent: probe/1`).

mod resume;
mod segmented;
mod single;

use rget::features::download::DownloadOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

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

/// Serves the routes above on a random port and returns the base URL. With `ranges`
/// off the server ignores `Range` and does not advertise `Accept-Ranges`.
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
    let headers: Vec<String> = lines.map(|l| l.to_ascii_lowercase()).collect();
    let has = |h: &str| headers.iter().any(|l| l == h);
    let range = headers
        .iter()
        .find_map(|l| l.strip_prefix("range: bytes=").map(str::to_string));

    let head_only = method == "HEAD";
    let effective = match path.as_str() {
        "/guarded" if has("x-token: ok") && has("user-agent: probe/1") => "/file",
        "/guarded" => "/forbidden",
        other => other,
    };

    if effective == "/trickle" || effective == "/stall" {
        write_slowly(stream, payload, effective == "/stall", head_only);
        return;
    }

    let (status, extra, body): (String, String, &[u8]) = match (effective, range) {
        ("/redirect", _) => ("302 Found".into(), "Location: /file\r\n".into(), &[]),
        ("/forbidden", _) => ("403 Forbidden".into(), String::new(), b"forbidden"),
        ("/file", Some(spec)) if ranges => {
            let (start, end) = spec.split_once('-').unwrap();
            let start: usize = start.parse().unwrap();
            if start >= payload.len() {
                (
                    "416 Range Not Satisfiable".into(),
                    format!("Content-Range: bytes */{}\r\n", payload.len()),
                    &[],
                )
            } else {
                let end: usize = if end.is_empty() { payload.len() - 1 } else { end.parse().unwrap() };
                let end = end.min(payload.len() - 1);
                (
                    "206 Partial Content".into(),
                    format!(
                        "Content-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\n",
                        start,
                        end,
                        payload.len()
                    ),
                    &payload[start..=end],
                )
            }
        }
        ("/file", _) => (
            "200 OK".into(),
            if ranges { "Accept-Ranges: bytes\r\n".to_string() } else { String::new() },
            payload,
        ),
        (p, _) if p.starts_with("/status/") => {
            let code: u16 = p["/status/".len()..].parse().unwrap();
            (format!("{code} Test"), String::new(), b"error page")
        }
        _ => ("404 Not Found".into(), String::new(), b"not found"),
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

/// `/trickle`: the whole payload in ten pieces, 300 ms apart.
/// `/stall`: the first half, then nothing for five seconds.
fn write_slowly(mut stream: TcpStream, payload: &[u8], stall: bool, head_only: bool) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = stream.write_all(head.as_bytes());
    if head_only {
        return;
    }
    if stall {
        let _ = stream.write_all(&payload[..payload.len() / 2]);
        let _ = stream.flush();
        thread::sleep(Duration::from_secs(5));
        return;
    }
    let piece = payload.len().div_ceil(10);
    for chunk in payload.chunks(piece) {
        if stream.write_all(chunk).is_err() {
            return;
        }
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(300));
    }
}
