# Roadmap

This document outlines the future direction of rget, a safe, modern HTTP/HTTPS downloader for Linux written in Rust.
Items are organized by priority, not by timeline.

---

## ✅ Completed (Milestones Achieved)

- ✅ HTTP/HTTPS downloads with progress bar and resume support (`-c`) – initial commit, 2026-08-17
- ✅ `--follow-redirects false` handling with a dedicated `RedirectDisabled` error
- ✅ IPv6 loopback (`::1`) and private IPv6 range blocking
- ✅ Automatic retries (`-r`) with exponential backoff and jitter
- ✅ Quiet mode (`-q`)
- ✅ SHA‑256 checksum verification (`--sha256`) with a `ChecksumMismatch` error (MD5 deliberately dropped)
- ✅ Parallel downloads (`-j`) using worker threads and `MultiProgress`
- ✅ Configuration file (`~/.config/rget/config.toml`) and `--init` to generate it (works without URLs)
- ✅ Custom sanitization layer: URL validation plus injection, traversal, and sensitive-path blocking
- ✅ Rate limiting (`--limit-rate`) with `K`/`M`/`G`/`T` suffixes
- ✅ Segmented downloads (`--segments`) with corrected Content-Length parsing, range calculation, and single-thread fallback
- ✅ Output directory (`-P`, `--directory-prefix`)
- ✅ `--version`, batch input from file or stdin (`-i`), custom headers (`-H`), and default `~/Downloads` output folder
- ✅ v1.0.0 release: GPLv3 license, copyright headers, README, CHANGELOG, `.gitignore`
- ✅ **Cycle 0 · R – feature-first restructure**: `app/` composition root, `features/{download,validation,integrity,input,destination}/`, `shared/`, a `lib.rs` with a thin `main.rs`, per-feature error types, a `DownloadOptions` struct replacing the 12-argument calls, and mirrored `tests/unit` and `tests/integration` suites (67 tests). Behaviour verified identical to 1.0.0 across a 65-case end-to-end comparison.
- ✅ **Cycle 1 · A1 + A2 + A4 + B7 + B10 – streaming and HTTP correctness**: the body is streamed through one reusable buffer (400 MiB download: 437 MB peak memory before, 24 MB now); `-t` is now a connect/stall limit instead of a cap on the whole transfer; HTTP error statuses fail before the output file is touched; the segmented `HEAD` probe sends `-H`/`-A` and honours `--follow-redirects`; the `🔍 DEBUG:` prints became `log::debug!` (`RUST_LOG=debug`); segmented downloads join the `MultiProgress`; `--limit-rate` throttles the network and is shared across `--segments`. Adds property tests for the stream helper and a `criterion` throughput benchmark.
- ✅ **Cycle 2 · A5 + A6 + C5 – resume, retries and staged files**: downloads are written to `name.part` (with a `name.part.meta` sidecar holding the URL, ETag, Last-Modified and size) and renamed into place only when complete, so an interrupted or failed download never touches an existing file; `-c` validates with `If-Range` and restarts when the remote file changed, treats a `416` on a complete file as success, and continues from the last written byte on retries; only transient failures are retried (timeouts, connection errors, 408/425/429/500/502/503/504), a server's `Retry-After` is honoured (up to 60s), and backoff is capped; segmented resume no longer corrupts the file; symlinked and device outputs are handled.
- ✅ **Cycle 3 · B8 + B9 + B11 – segmented downloads you can trust**: each segment retries independently with the Cycle 2 policy and continues from its part file; the merge streams with `io::copy` (a 400 MiB, 4-segment download: 127 MB peak memory before, 34 MB now); the `.part.meta` sidecar records the segment layout and validators, so a resume is accepted only for the same URL, size, segment count and unchanged file, and workers send `If-Range`; oversized parts and leftovers beyond the segment count are deleted; every part is length-checked before merging; a failure returns the original error and keeps the parts for `-c`. Also fixed in the same code: a server that answers `200` to a range request used to have its whole body written into every part (a 300 KB file came out as 900 KB), and a file smaller than the segment count panicked.

---

## 🔥 High Priority (Critical)

Real-download correctness. Each item is a defect in the 1.0.0 behaviour, marked with a `FIXME(<id>)` at the code that needs to change. A1, A2, A4 (Cycle 1), A5, A6 (Cycle 2) and the whole B group (Cycles 1 and 3) are fixed; only A3 remains.

- [ ] **A3 · Fix URL validator false positives** – `&`, `(`, `)`, `;`, `$` are rejected, so common query-string URLs are refused; the `\.env` pattern also blocks hosts like `foo.environment.com`, and the `../` check is too blunt. Keep the security intent, stop rejecting legitimate URLs.

---

## 🟡 Medium Priority (Important)

- [ ] **C1 · Correct private-address blocking** – the check is string-prefix matching and misses `127.0.0.0/8`, `0.0.0.0`, `169.254.0.0/16` (cloud metadata), and IPv4-mapped IPv6. Parse addresses and use `std::net` classification, and re-validate redirect targets.
- [ ] **C2 · Opt-out for local/private hosts** – an explicit flag (e.g. `--allow-private`) for dev servers and home NAS, keeping the block as the default.
- [ ] **C3 · Safe output filenames** – percent-decode the name taken from the URL, sanitize it, and honor `Content-Disposition`.
- [ ] **C4 · Don't silently overwrite** – add `--no-clobber` or auto-rename when the target exists and `-c` is not set.
- [ ] **C6 · SHA‑256 polish** – case-insensitive comparison, and decide what to do with the file on mismatch. (The duplicated output-path logic was removed in Cycle 0; the check now reuses the download's real path.)
- [ ] **F · Tests** – Cycle 0 added characterization tests, Cycle 1 property tests for the stream helper and a `criterion` benchmark (`benches/throttle.rs`), Cycle 2 a state table and property tests for the resume and retry decisions, Cycle 3 property tests for segment planning and resume validation plus integration tests for dropped segments, lying servers, changed files and changed segment counts. Known defects are pinned by tests named `known_defect_*` that flip when their item lands. Still to do: property tests for `parse_size` and the validator, and a throughput regression gate if CI is added.

---

## 🟢 Low Priority (Nice‑to‑Have)

- [ ] **D1 · `--init` default** – it writes `limit_rate = 1048576` uncommented, so new users are throttled to 1 MB/s without knowing. Comment it out.
- [ ] **D2 · Warn on unreadable config** – a malformed `config.toml` is silently ignored; report the parse error.
- [ ] **D3 · Cleaner errors and exit codes** – `main() -> Result` prints Rust `Debug` output, and one invalid URL aborts the whole batch.
- [ ] **D4 · User-Agent from `CARGO_PKG_VERSION`** – it is hardcoded to `rget/0.1.0` while the crate is 1.0.0.
- [ ] **D5 · Progress polish** – show transfer speed, fit the unknown-size case, and handle Ctrl+C.
- [ ] **D6 · Input file parsing** – trim lines and skip `#` comments in `-i` files.
- [ ] **D8 · Don't retry permanent network errors** – connection errors are all retried, including ones that cannot heal (an invalid certificate, a name that does not exist), because `reqwest` does not distinguish them. Harmless but slow with `-r`.
- [ ] **D9 · `--max-time`** – `-t` no longer caps the total transfer (Cycle 1); an optional overall limit could return as its own flag.
- [ ] **D10 · Stop the other segments when one fails** – a segment that exhausts its retries fails the download, but the other segments keep downloading until they finish. Their parts stay resumable with `-c`, so nothing is lost; it only wastes bandwidth. A shared cancel flag checked in the stream loop would fix it.
- [ ] **D7 · `-j 0` hangs** – no worker threads are started, so the result loop waits forever. Reject 0 (or treat it as 1).
- [ ] **G · Project hygiene** – remove unused dependencies (`anyhow`, `digest`) and commit `Cargo.lock` (this is a binary crate). The options-struct part is done (`DownloadOptions`). Existing code is not `rustfmt`-formatted; run `cargo fmt` as its own commit so blame stays readable.

---

## 🔭 Long‑Term Vision

Ideas only; none are committed.

- Proxy support (`--proxy`) and HTTP basic auth (`--user`)
- `--no-check-certificate` escape hatch
- Cookie handling
- Spider/head-only mode
- Machine-readable (JSON) progress output

---

## 🎯 Next Actions (Immediate)

1. **Cycle 4 – A3**: validator false positives (`&`, `(`, `)`, `;`, `$`, `.env` in hostnames), with tests that flip the `known_defect_*` validator tests. This is the last item in the high-priority group.
2. **Then**: the medium-priority C items (C1 private-address blocking, C2 an opt-out flag, C3 safe filenames, C4 no-clobber) and the release cleanup in G (`Cargo.lock`, unused dependencies, `cargo fmt`).
