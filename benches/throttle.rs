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


//! Throughput of the streaming download path over loopback, including the staged
//! `name.part` write and the rename into place.
//!
//! Run with `cargo bench`. To gate a change on regressions, record a baseline first
//! (`cargo bench -- --save-baseline before`), then compare (`cargo bench -- --baseline before`)
//! and treat a slowdown above 5% in `unthrottled` as a failure.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rget::features::download::{download_file, DownloadOptions, OnOccupied};
use rget::shared::address::HostPolicy;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

/// Serves `payload` for every request on a random local port; returns the base URL.
fn serve(payload: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a loopback port for the benchmark server");
    let base = format!("http://{}/file", listener.local_addr().expect("listener has a local address"));
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let payload = payload.clone();
            thread::spawn(move || respond(stream, &payload));
        }
    });
    base
}

fn respond(mut stream: TcpStream, payload: &[u8]) {
    let mut request = [0u8; 1024];
    let _ = stream.read(&mut request);
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(payload);
}

fn options(limit_rate: Option<usize>) -> DownloadOptions {
    DownloadOptions {
        resume: false,
        timeout: 30,
        follow_redirects: true,
        user_agent: None,
        retries: 0,
        quiet: true,
        limit_rate,
        segments: 1,
        headers: Vec::new(),
        host_policy: HostPolicy::AllowPrivate, // the benchmark server is on loopback
        verify: None,
        on_occupied: OnOccupied::Replace,
    }
}

fn bench_download(c: &mut Criterion) {
    let url = serve(vec![0xA5; PAYLOAD_BYTES]);
    let mut group = c.benchmark_group("download");
    group.throughput(Throughput::Bytes(PAYLOAD_BYTES as u64));
    group.sample_size(10);

    // A real file, so the `.part` staging and the final rename are part of what is measured.
    let out = std::env::temp_dir().join(format!("rget-bench-{}.bin", std::process::id()));
    let out = out.to_str().expect("the temp directory path is valid UTF-8");

    let unthrottled = options(None);
    group.bench_function("unthrottled", |b| {
        b.iter(|| download_file(&url, out, &unthrottled, None).expect("unthrottled download"));
    });

    // A limit far above loopback speed: measures the cost of the throttle bookkeeping.
    let generous = options(Some(4 * 1024 * 1024 * 1024));
    group.bench_function("limit_4GiB_per_s", |b| {
        b.iter(|| download_file(&url, out, &generous, None).expect("throttled download"));
    });

    group.finish();
    let _ = std::fs::remove_file(out);
}

criterion_group!(benches, bench_download);
criterion_main!(benches);
