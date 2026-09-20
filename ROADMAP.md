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

---

## 🔥 High Priority (Critical)

Real-download correctness. Each item is a defect in the 1.0.0 behaviour; the restructure kept them all deliberately, and each is marked with a `FIXME(<id>)` at the code that needs to change.

- [ ] **A1 · Stream the response body** – `response.bytes()` buffers the whole file in RAM before writing; the progress bar only moves at the end and `--limit-rate` throttles disk writes, not the network. Read in chunks straight to disk instead (single-file and segmented paths).
- [ ] **A2 · Replace the total-request timeout** – the blocking client's `timeout()` covers the entire body, so any download longer than the timeout (default 30s) fails. Use a connect timeout plus a read/stall timeout.
- [ ] **A3 · Fix URL validator false positives** – `&`, `(`, `)`, `;`, `$` are rejected, so common query-string URLs are refused; the `\.env` pattern also blocks hosts like `foo.environment.com`, and the `../` check is too blunt. Keep the security intent, stop rejecting legitimate URLs.
- [ ] **A4 · Check the HTTP status** – a 404 or 500 body is currently saved as the output file and reported as complete. Fail on error statuses before touching the output file.
- [ ] **A5 · Harden resume** – resuming an already-complete file currently gets a 416, which is treated as "server doesn't support resume": the finished file is **truncated to 0 bytes** and reported as a success. Treat 416 on a complete file as success. Also validate with `If-Range`/ETag so a changed remote file does not produce a corrupt result, make retries continue from the last written byte, and fix segmented resume, which advances a partial part's start twice (once when scanning parts, again in the worker) and so produces a short, corrupt file.
- [ ] **A6 · Retry only what is retryable** – today every error (404, blocked URL, …) is retried with backoff. Retry network errors, timeouts, 5xx and 429 only, and honor `Retry-After`.

---

## 🟡 Medium Priority (Important)

- [ ] **B · Segmented download cleanup** – the `HEAD` probe ignores `-H`, `-A`, and `--follow-redirects`; segments have no individual retries; merging loads each part fully into memory (use `io::copy`); leftover `🔍 DEBUG:` output prints on every non-quiet run; segmented mode ignores `MultiProgress`; stale `.partN` files are not cleaned up on failure; `plan_ranges` underflows when the file is smaller than the segment count (`part_size` is 0).
- [ ] **C1 · Correct private-address blocking** – the check is string-prefix matching and misses `127.0.0.0/8`, `0.0.0.0`, `169.254.0.0/16` (cloud metadata), and IPv4-mapped IPv6. Parse addresses and use `std::net` classification, and re-validate redirect targets.
- [ ] **C2 · Opt-out for local/private hosts** – an explicit flag (e.g. `--allow-private`) for dev servers and home NAS, keeping the block as the default.
- [ ] **C3 · Safe output filenames** – percent-decode the name taken from the URL, sanitize it, and honor `Content-Disposition`.
- [ ] **C4 · Don't silently overwrite** – add `--no-clobber` or auto-rename when the target exists and `-c` is not set.
- [ ] **C5 · Write to `name.part`, rename on success** – an interrupted download should never look like a finished file.
- [ ] **C6 · SHA‑256 polish** – case-insensitive comparison, and decide what to do with the file on mismatch. (The duplicated output-path logic was removed in Cycle 0; the check now reuses the download's real path.)
- [ ] **F · Tests** – Cycle 0 added characterization tests (validator, sizes, SHA‑256, input, destination, range planning, throttle, retry loop, and single/resume/segmented downloads against a local server). Known defects are pinned by tests named `known_defect_*` that flip when their item lands. Still to do: property tests (`proptest` is in the local cargo cache) and the throttle benchmark stub (`criterion`, likewise cached) that `benches/throttle.rs` is reserved for.

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

1. **Cycle 1 – A1 + A2 + A4**: streaming download path, connect/stall timeouts, and HTTP status check (they touch the same code in `features/download/`). Adds the `criterion` throttle benchmark.
2. **Cycle 2 – A5 + A6**: resume hardening and retry classification.
3. **Cycle 3 – A3**: validator false positives, with tests (flips the `known_defect_*` validator tests).
