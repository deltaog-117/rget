# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Library crate (`src/lib.rs`) alongside the binary, so the public API can be tested from `tests/`
- Characterization test suite pinning the 1.0.0 behaviour: 16 in-file unit tests, 40 mirrored unit tests (`tests/unit/`), and 11 integration tests that run the real download code against an in-process HTTP server (`tests/integration/`)
- `DownloadOptions` struct grouping the per-download settings
- Debug tracing through the `log` crate: run with `RUST_LOG=debug` to see the probe result, segment ranges and per-part byte counts (replaces the always-on `🔍 DEBUG:` lines)
- Property tests for the streaming helper (`proptest`) and a `criterion` throughput benchmark, `cargo bench`
- Integration tests for slow and stalled servers, HTTP error statuses, header-guarded servers, and rate limiting
- Downloads are staged as `name.part` next to a small `name.part.meta` sidecar (URL, ETag, Last-Modified, size) and renamed into place only when complete
- Resume validation with `If-Range`: a partial download is only continued when the remote file is unchanged
- `Retry-After` support (seconds or HTTP-date), and a retry policy that distinguishes transient from permanent failures
- Tests for the resume state machine and retry policy (property tests and a decision table), integration tests for ETag changes, dropped connections, `Retry-After`, and symlinked and device outputs

### Changed
- Restructured the source tree from a flat `src/` into a feature-first layout: `app/` (CLI, config, settings merge, wiring), `features/{download,validation,integrity,input,destination}/`, and `shared/` (progress bar, size parsing). `main.rs` is now a thin entry point. No user-visible behaviour change; verified identical to 1.0.0 across 65 end-to-end cases.
- `RgetError` split into per-feature error types (`download`, `validation`, `integrity`) aggregated by `AppError`; error messages and the `Error: …` output on failure are unchanged
- The checksum step now verifies the path the download actually wrote to, instead of recomputing it
- Removed clippy warnings from the carried-over code
- `-t`/`--timeout` now limits connecting and each pause in the incoming data, instead of the whole transfer; there is no cap on total download time
- `--limit-rate` combined with `--segments N` now applies to the download as a whole (each segment gets `1/N` of the limit) instead of giving every segment the full limit
- The segmented `HEAD` probe now sends `-H` headers and the User-Agent and honours `--follow-redirects false`; a non-2xx probe falls back to the single-connection path, which reports the real error
- Segmented downloads now take part in the `MultiProgress` display
- Errors gained two variants, `HttpStatus` and `Stalled`, so callers (and the retry logic) can tell them from other network failures
- An interrupted or failed download now leaves `name.part` (and its sidecar) instead of a partial file under the final name; an existing file is replaced only by the final rename
- `-c` continues from `name.part`, and still adopts a partial `name` left by rget 1.0.0 or another tool. Without `-c`, a leftover `name.part` from an earlier run is discarded
- Retries within a run continue from the bytes the previous attempt wrote (when the server supports ranges) instead of starting over
- Only transient failures are retried: timeouts, connection errors, and HTTP 408, 425, 429, 500, 502, 503 and 504. Other errors (404, a full disk, a disabled redirect) now fail at once; the backoff is capped at 60s
- The benchmark now writes to a real file, so the staging and rename are included in what it measures

### Fixed
- Downloads no longer buffer the whole file in memory: the body is streamed through a single 64 KiB buffer (a 400 MiB download peaked at 437 MB before and 24 MB now)
- Downloads that take longer than the timeout (default 30s) no longer fail while data is still arriving
- HTTP error responses (404, 500, …) are no longer saved as the output file and reported as complete; they fail with `HTTP error: <status>` and leave the destination untouched
- `--limit-rate` now throttles the network read itself, and never lets a single read overshoot a small limit
- Resuming an already-complete file no longer truncates it to 0 bytes: it now reports that the file is already fully retrieved and succeeds
- Resuming a download whose remote file changed no longer produces a corrupt file: the change is detected and the download restarts
- Segmented resume no longer skips bytes: a partially downloaded part used to be resumed twice as far in as it should have been, giving a short, corrupt file
- A failed re-download no longer destroys an existing file, and a stalled or interrupted one no longer leaves partial data under the final name
- A `Retry-After` from the server is honoured instead of being ignored, and a very large `-r` no longer overflows the backoff
- The progress bar on a resumed download now shows the full file size
- Segmented downloads from servers that require custom headers or a User-Agent on `HEAD` are now actually segmented instead of silently falling back to one connection

---

## [1.0.0] - 2026-08-30

### Added

#### Core Download Features
- HTTP/HTTPS downloads with modern progress bar (ETA, speed, percentage)
- Resume support for interrupted downloads (`-c`, `--resume`)
- Custom output filename (`-O`, `--output`)
- Custom output directory (`-P`, `--directory-prefix`)
- **Default Downloads folder** – files automatically save to `~/Downloads` when no `-P` or `-O` is specified
- Verbose mode for debugging (`-v`, `--verbose`)
- Timeout control for connection and read operations (`-t`, `--timeout`)
- Custom User-Agent header (`-A`, `--user-agent`)
- Custom HTTP headers (`-H`, `--header`) – can be used multiple times
- Follow redirects (enabled by default) with option to disable (`--follow-redirects`)

#### Performance Features
- **Segmented parallel downloads** (`--segments`) – split single file into parts and download concurrently for maximum speed
- **Parallel downloads** (`-j`, `--jobs`) – download multiple URLs concurrently
- **Rate limiting** (`--limit-rate`) – throttle bandwidth usage (supports human-readable sizes: `500k`, `2M`, `1G`)
- Connection pooling via reqwest for optimal performance

#### Reliability Features
- **Automatic retries** (`-r`, `--retries`) – exponential backoff with jitter for unstable networks
- **Resume support** – automatically resumes interrupted downloads when `-c` is used
- **SHA‑256 checksum verification** (`--sha256`) – verify file integrity after download
- Timeout handling with configurable limits

#### Security Features
- **Military‑grade input sanitization** – custom implementation that blocks:
  - Command injection (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `<`, `>`)
  - Path traversal (`../`, `..\\`)
  - Sensitive file patterns (`/etc/passwd`, `/etc/shadow`, `.env`, `.git/config`, `.aws/credentials`, `.ssh/id_rsa`)
  - Encoded injection attacks (URL‑decoded and re‑checked)
- **Private IP address blocking** – prevents SSRF attacks:
  - IPv4: `localhost`, `127.0.0.1`, `192.168.x.x`, `10.x.x.x`, `172.16-31.x.x`
  - IPv6: `::1` (loopback), `fc00::/7` (unique local), `fe80::/10` (link‑local)
- **TLS hardening** – uses rustls with modern cipher suites (no OpenSSL)
- **Memory safety** – zero `unsafe` code, pure Rust
- **No shell execution** – prevents command injection entirely

#### Usability Features
- **Configuration file** (`~/.config/rget/config.toml`) – set defaults for all options
- **`--init` flag** – generates a default configuration file with comments
- **`--version` flag** – displays version information
- **Batch downloads** (`-i`, `--input-file`) – read URLs from a file
- **Stdin support** (`-i -`) – pipe URLs from other commands
- **Quiet mode** (`-q`, `--quiet`) – suppress all non‑error output
- **Colored, modern progress bars** with ETA and transfer speed
- **Help text** – comprehensive `--help` output via clap

#### Documentation
- Professional `README.md` with:
  - Feature list with emoji icons
  - Installation instructions (Cargo, source, binary)
  - Quick start examples
  - Configuration guide
  - Security comparison table (rget vs curl vs wget)
  - Project structure overview
  - Contributing guidelines link
  - License information (GPLv3)
- `CHANGELOG.md` – this file
- `LICENSE` – GNU General Public License v3.0

### Changed
- (None – this is the initial release)

### Deprecated
- (None – this is the initial release)

### Removed
- (None – this is the initial release)

### Fixed
- (None – this is the initial release)

### Security
- Built‑in command injection protection
- Built‑in path traversal protection
- Private IP address blocking (IPv4 + IPv6)
- Sensitive file pattern blocking
- TLS hardening with rustls
- Memory safety guarantees via Rust
- No shell execution – eliminates entire class of vulnerabilities
- URL sanitization before any processing occurs
