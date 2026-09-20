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

---

## 🔥 High Priority (Critical)

Real-download correctness. Each item is a defect in the 1.0.0 behaviour, marked with a `FIXME(<id>)` at the code that needs to change. A1, A2 and A4 are fixed (Cycle 1).

- [ ] **A3 · Fix URL validator false positives** – `&`, `(`, `)`, `;`, `$` are rejected, so common query-string URLs are refused; the `\.env` pattern also blocks hosts like `foo.environment.com`, and the `../` check is too blunt. Keep the security intent, stop rejecting legitimate URLs.
- [ ] **A5 · Harden resume** – resuming an already-complete file gets a 416. Since Cycle 1 that is reported as an error and the file is left intact (it used to be truncated to 0 bytes and reported as a success); it should be a success. Also validate with `If-Range`/ETag so a changed remote file does not produce a corrupt result, make retries continue from the last written byte, and fix segmented resume, which advances a partial part's start twice (once when scanning parts, again in the worker) and so produces a short, corrupt file.
- [ ] **A6 · Retry only what is retryable** – today every error (404, blocked URL, …) is retried with backoff. Retry network errors, timeouts, 5xx and 429 only, and honor `Retry-After`.

---

## 🟡 Medium Priority (Important)

- [ ] **B · Segmented download cleanup** (B7 and B10 are done) – segments have no individual retries (needs A6's classification first, B8); merging loads each part fully into memory (use `io::copy`, or write each segment straight into the final file, B9); stale `.partN` files are not cleaned up on failure and parts are validated by size only (B11); `plan_ranges` underflows when the file is smaller than the segment count (`part_size` is 0); by reading the code, a server that answers a ranged request with `200` has its whole body written into a single part.
- [ ] **C1 · Correct private-address blocking** – the check is string-prefix matching and misses `127.0.0.0/8`, `0.0.0.0`, `169.254.0.0/16` (cloud metadata), and IPv4-mapped IPv6. Parse addresses and use `std::net` classification, and re-validate redirect targets.
- [ ] **C2 · Opt-out for local/private hosts** – an explicit flag (e.g. `--allow-private`) for dev servers and home NAS, keeping the block as the default.
- [ ] **C3 · Safe output filenames** – percent-decode the name taken from the URL, sanitize it, and honor `Content-Disposition`.
- [ ] **C4 · Don't silently overwrite** – add `--no-clobber` or auto-rename when the target exists and `-c` is not set.
- [ ] **C5 · Write to `name.part`, rename on success** – an interrupted download should never look like a finished file. More visible since Cycle 1: a stalled or interrupted transfer now leaves the bytes received so far under the final name (a stalled download used to leave an empty file). Best done together with A5, since resume reads the partial file.
- [ ] **C6 · SHA‑256 polish** – case-insensitive comparison, and decide what to do with the file on mismatch. (The duplicated output-path logic was removed in Cycle 0; the check now reuses the download's real path.)
- [ ] **F · Tests** – Cycle 0 added characterization tests, Cycle 1 added property tests for the stream helper and a `criterion` throughput benchmark (`benches/throttle.rs`; record a baseline with `cargo bench -- --save-baseline before`). Known defects are pinned by tests named `known_defect_*` that flip when their item lands. Still to do: property tests for `parse_size`, the validator and range planning, and a throughput regression gate if CI is added.

---

## 🟢 Low Priority (Nice‑to‑Have)

- [ ] **D1 · `--init` default** – it writes `limit_rate = 1048576` uncommented, so new users are throttled to 1 MB/s without knowing. Comment it out.
- [ ] **D2 · Warn on unreadable config** – a malformed `config.toml` is silently ignored; report the parse error.
- [ ] **D3 · Cleaner errors and exit codes** – `main() -> Result` prints Rust `Debug` output, and one invalid URL aborts the whole batch.
- [ ] **D4 · User-Agent from `CARGO_PKG_VERSION`** – it is hardcoded to `rget/0.1.0` while the crate is 1.0.0.
- [ ] **D5 · Progress polish** – show transfer speed, fit the unknown-size case, and handle Ctrl+C.
- [ ] **D6 · Input file parsing** – trim lines and skip `#` comments in `-i` files.
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

1. **Cycle 2 – A5 + A6 (+ C5)**: resume hardening, retry classification (using the new `HttpStatus` and `Stalled` errors, and `Retry-After`), and writing to `name.part`.
2. **Cycle 3 – B8, B9, B11**: per-segment retries, the merge step, and part-file validation and cleanup, now that A5 and A6 exist.
3. **Cycle 4 – A3**: validator false positives, with tests (flips the `known_defect_*` validator tests).
