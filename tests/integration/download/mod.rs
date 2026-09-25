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
//! Routes:
//! - `/file`: the payload, with an `ETag`, honouring `Range` (and `If-Range`) unless disabled
//! - `/redirect`: 302 to `/file`
//! - `/redirect-metadata`: 302 to `http://169.254.169.254:9/latest`, the cloud metadata address
//! - `/status/<code>`: that status with a short body
//! - `/once/<code>`: that status the first time (with `Retry-After: 1` for 429), then `/file`
//! - `/long/503`: always 503 with `Retry-After: 300`
//! - `/dropmid`: the first request promises the whole payload but closes after half, then `/file`
//! - `/dropseg`: like `/file`, but the first range request that starts in the second half of
//!   the payload closes the connection after half of its bytes
//! - `/lying`: advertises `Accept-Ranges: bytes` but answers every request with `200` and the whole payload
//! - `/trickle`: the payload in ten pieces, 300 ms apart
//! - `/stall`: half the payload, then silence
//! - `/guarded`: like `/file`, but 403 unless the request carries `X-Token: ok` and `User-Agent: probe/1`
//! - `/slowfail`: like `/file`, but a range starting in the low half trickles out 300 ms
//!   apart and one starting in the high half is refused with a permanent `403`

mod hosts;
mod placement;
mod pool;
mod resume;
mod retry;
mod segmented;
mod single;
mod verification;

use rget::features::download::{DownloadOptions, OnOccupied};
use rget::shared::address::HostPolicy;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
        // The test server is on loopback.
        host_policy: HostPolicy::AllowPrivate,
        verify: None,
        on_occupied: OnOccupied::Replace,
    }
}

pub(crate) fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rget-it-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// What the server saw, for tests that assert on the requests themselves.
#[derive(Default)]
pub(crate) struct Stats {
    /// Every request, including HEAD.
    pub hits: AtomicUsize,
    /// Requests that carried a `Range` header.
    pub ranged: AtomicUsize,
}

impl Stats {
    pub(crate) fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
    pub(crate) fn ranged(&self) -> usize {
        self.ranged.load(Ordering::SeqCst)
    }
}

struct Config {
    payload: Vec<u8>,
    ranges: bool,
    etag: String,
}

/// Serves the routes above on a random port and returns the base URL. With `ranges`
/// off the server ignores `Range` and does not advertise `Accept-Ranges`.
pub(crate) fn serve(payload: Vec<u8>, ranges: bool) -> String {
    serve_with(payload, ranges, "\"v1\"").0
}

/// Like [`serve`], with a chosen `ETag` and request statistics.
pub(crate) fn serve_with(payload: Vec<u8>, ranges: bool, etag: &str) -> (String, Arc<Stats>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let config = Arc::new(Config {
        payload,
        ranges,
        etag: etag.to_string(),
    });
    let stats = Arc::new(Stats::default());
    let seen: Arc<Mutex<HashMap<String, usize>>> = Arc::default();
    let shared = stats.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (config, stats, seen) = (config.clone(), shared.clone(), seen.clone());
            thread::spawn(move || handle(stream, &config, &stats, &seen));
        }
    });
    (base, stats)
}

fn respond(stream: &mut TcpStream, status: &str, extra: &str, body: &[u8], head_only: bool) {
    let head = format!(
        "HTTP/1.1 {status}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    if !head_only {
        let _ = stream.write_all(body);
    }
}

fn handle(
    mut stream: TcpStream,
    config: &Config,
    stats: &Stats,
    seen: &Mutex<HashMap<String, usize>>,
) {
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
    let header = |name: &str| {
        headers
            .iter()
            .find_map(|l| l.strip_prefix(name).map(|v| v.trim().to_string()))
    };
    let range = header("range: bytes=");
    let if_range = text
        .lines()
        .find_map(|l| {
            l.strip_prefix("If-Range: ")
                .or_else(|| l.strip_prefix("if-range: "))
        })
        .map(|v| v.trim().to_string());

    stats.hits.fetch_add(1, Ordering::SeqCst);
    if range.is_some() {
        stats.ranged.fetch_add(1, Ordering::SeqCst);
    }
    let head_only = method == "HEAD";
    let times_seen = |key: &str| {
        let mut map = seen.lock().unwrap();
        let n = map.entry(key.to_string()).or_insert(0);
        *n += 1;
        *n
    };

    let mut effective = path.as_str();
    let mut drop_high_range = false;
    match path.as_str() {
        "/dropseg" => {
            effective = "/file";
            drop_high_range = true;
        }
        "/guarded" => {
            effective = if has("x-token: ok") && has("user-agent: probe/1") {
                "/file"
            } else {
                "/forbidden"
            };
        }
        "/dropmid" => {
            if times_seen("dropmid") == 1 {
                let head = format!(
                    "HTTP/1.1 200 OK\r\nETag: {}\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    config.etag,
                    config.payload.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&config.payload[..config.payload.len() / 2]);
                return;
            }
            effective = "/file";
        }
        p if p.starts_with("/once/") => {
            if times_seen(p) == 1 {
                let code: u16 = p["/once/".len()..].parse().unwrap();
                let extra = if code == 429 {
                    "Retry-After: 1\r\n"
                } else {
                    ""
                };
                respond(
                    &mut stream,
                    &format!("{code} Test"),
                    extra,
                    b"try again",
                    head_only,
                );
                return;
            }
            effective = "/file";
        }
        _ => {}
    }

    if effective == "/trickle" || effective == "/stall" {
        write_slowly(stream, &config.payload, effective == "/stall", head_only);
        return;
    }

    let payload = &config.payload;
    match effective {
        "/redirect" => respond(
            &mut stream,
            "302 Found",
            "Location: /file\r\n",
            &[],
            head_only,
        ),
        "/redirect-metadata" => respond(
            &mut stream,
            "302 Found",
            "Location: http://169.254.169.254:9/latest\r\n",
            &[],
            head_only,
        ),
        "/forbidden" => respond(&mut stream, "403 Forbidden", "", b"forbidden", head_only),
        "/long/503" => respond(
            &mut stream,
            "503 Test",
            "Retry-After: 300\r\n",
            b"later",
            head_only,
        ),
        "/lying" => respond(
            &mut stream,
            "200 OK",
            &format!("ETag: {}\r\nAccept-Ranges: bytes\r\n", config.etag),
            payload,
            head_only,
        ),
        "/file" => {
            let etag_line = format!("ETag: {}\r\n", config.etag);
            // A validator that no longer matches means: ignore the Range, send everything.
            let honour_range =
                config.ranges && if_range.as_deref().is_none_or(|v| v == config.etag);
            match range {
                Some(spec) if honour_range => {
                    let (start, end) = spec.split_once('-').unwrap();
                    let start: usize = start.parse().unwrap();
                    if start >= payload.len() {
                        let extra = format!("Content-Range: bytes */{}\r\n", payload.len());
                        respond(
                            &mut stream,
                            "416 Range Not Satisfiable",
                            &extra,
                            &[],
                            head_only,
                        );
                    } else {
                        let end: usize = if end.is_empty() {
                            payload.len() - 1
                        } else {
                            end.parse().unwrap()
                        };
                        let end = end.min(payload.len() - 1);
                        let extra = format!(
                            "{etag_line}Content-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\n",
                            start,
                            end,
                            payload.len()
                        );
                        let body = &payload[start..=end];
                        if drop_high_range
                            && start >= payload.len() / 2
                            && times_seen("dropseg") == 1
                        {
                            let head = format!(
                                "HTTP/1.1 206 Partial Content\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(head.as_bytes());
                            let _ = stream.write_all(&body[..body.len() / 2]);
                            // The other segment's own request is on loopback and unthrottled,
                            // so it finishes in a few milliseconds; delaying the close of this
                            // one keeps a test that expects it to have already finished from
                            // racing that segment's cancellation (D10) under a loaded machine.
                            thread::sleep(Duration::from_millis(300));
                            return;
                        }
                        respond(&mut stream, "206 Partial Content", &extra, body, head_only);
                    }
                }
                _ => {
                    let accept = if config.ranges {
                        "Accept-Ranges: bytes\r\n"
                    } else {
                        ""
                    };
                    respond(
                        &mut stream,
                        "200 OK",
                        &format!("{etag_line}{accept}"),
                        payload,
                        head_only,
                    );
                }
            }
        }
        "/slowfail" => {
            let etag_line = format!("ETag: {}\r\n", config.etag);
            match range {
                Some(spec) if config.ranges => {
                    let (start, end) = spec.split_once('-').unwrap();
                    let start: usize = start.parse().unwrap();
                    let end: usize = if end.is_empty() {
                        payload.len() - 1
                    } else {
                        end.parse().unwrap()
                    };
                    let end = end.min(payload.len() - 1);
                    if start >= payload.len() / 2 {
                        // The high half is refused outright, and permanently: this is the
                        // segment meant to end the whole download.
                        respond(&mut stream, "403 Forbidden", "", b"forbidden", head_only);
                    } else {
                        let body = &payload[start..=end];
                        let extra = format!(
                            "{etag_line}Content-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\n",
                            start,
                            end,
                            payload.len()
                        );
                        if head_only {
                            respond(&mut stream, "206 Partial Content", &extra, body, true);
                        } else {
                            // The low half trickles out, so a test can tell whether it kept
                            // going to completion or stopped as soon as the high half failed.
                            let head = format!(
                                "HTTP/1.1 206 Partial Content\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            let _ = stream.write_all(head.as_bytes());
                            for chunk in body.chunks(body.len().div_ceil(6).max(1)) {
                                if stream.write_all(chunk).is_err() {
                                    return;
                                }
                                let _ = stream.flush();
                                thread::sleep(Duration::from_millis(300));
                            }
                        }
                    }
                }
                _ => {
                    let accept = if config.ranges {
                        "Accept-Ranges: bytes\r\n"
                    } else {
                        ""
                    };
                    respond(
                        &mut stream,
                        "200 OK",
                        &format!("{etag_line}{accept}"),
                        payload,
                        head_only,
                    );
                }
            }
        }
        p if p.starts_with("/status/") => {
            let code: u16 = p["/status/".len()..].parse().unwrap();
            respond(
                &mut stream,
                &format!("{code} Test"),
                "",
                b"error page",
                head_only,
            );
        }
        _ => respond(&mut stream, "404 Not Found", "", b"not found", head_only),
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
