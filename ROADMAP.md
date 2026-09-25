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
- ✅ **Cycle 4 · A3 – URL validation that judges what a URL means**: the character blacklist and substring patterns are replaced by checks on the parsed URL. Ordinary URLs with `&`, `;`, `$`, `|`, `(`, `)` and `%26` (`?a=1&b=2`, `file(1).zip`, `Rust_(programming_language)`) are accepted, as are hosts containing `.env` and names like `.env.example`; path traversal (`..` in any encoding, including `%2e%2e` and `%2f`) and well-known secret files (exact decoded path segments) are still refused, and now the encoded spellings the old checks missed are caught too; hostnames containing characters no hostname can hold (`exa;mple.com`) are refused. The `regex` dependency is gone.
- ✅ **Cycle 5 · C1 + C2 + C3 – SSRF protection and safe file names**: network addresses are classified by one pure function (`shared/address.rs`: loopback, private, link-local including the cloud metadata address, carrier-grade NAT, unspecified, multicast, reserved and documentation ranges; IPv6 addresses that wrap an IPv4 address are judged by it), replacing string-prefix matching that let 9 of 15 tested spellings through; redirects are re-checked at every hop and every client resolves names through a filter that drops non-public answers, so a redirect or a hostname that leads to `127.0.0.1` or `169.254.169.254` is refused at connection time (no DNS-rebinding gap); `--allow-private` and `allow_private` in the config opt out; a name taken from a URL is percent-decoded and cleaned so it can only name a file inside the download directory.
- ✅ **Cycle 6 · C4 + C6 – putting a finished download in place safely**: `--if-exists overwrite|skip|rename` (with `-n/--no-clobber` for skip, a config key, and `-c` taking precedence); `rename` saves as `file (1).zip`, `archive (1).tar.gz`, never taking a name whose download is in progress; names are claimed one URL at a time before any download starts, so two URLs that map to the same file name are never lost and `-j` is deterministic; under `skip` and `rename` the file is put in place with an atomic no-replace, so a file that appears while the download runs is never overwritten. `--sha256` is now checked on the finished file *before* it is put in place, so a bad download replaces nothing and leaves nothing behind; the digest is a validated value type that accepts either case and a pasted `sha256sum` line, and a malformed one is refused before any request.
- ✅ **Cycle 7 · D1 + D2 + D3 + D4 + D5 – smaller correctness and polish fixes**: `--init` no longer writes `limit_rate = 1048576` uncommented, so a new user is not silently throttled to 1 MB/s; a `config.toml` that cannot be read or parsed is now reported on stderr (unless `-q`) instead of falling back to defaults with no explanation; the CLI no longer aborts the whole batch on the first invalid URL — it is reported and skipped, and every other URL still downloads; `main` reports failures through `Display` instead of the default runtime's raw `Debug` dump; the default `User-Agent` is built from `CARGO_PKG_VERSION` instead of the hardcoded `rget/0.1.0`; the progress bar shows a smoothed transfer rate, a size-unknown download gets its own spinner template instead of a bar with a meaningless ETA, and Ctrl+C is now caught (`ctrlc`, checked once per chunk in the one shared `stream::copy` loop, so it covers single, segmented and `-j` downloads alike) — it prints an interrupt notice, exits `130`, and leaves the `.part`/`.part.meta` sidecar in place for `-c` to continue.
- ✅ **Cycle 8 · D6 + D7 + D8 + D10 – low-priority defects that were quick to fix and easy to get wrong**: `-i`/`--input-file` lines are trimmed and a line starting with `#` is treated as a comment, matching `curl`/`wget`; `-j 0` no longer hangs forever (no worker thread would ever have claimed a task) — it is treated as `1`; a connection failure is only retried when it might heal, since a DNS name that does not exist or a certificate that fails validation now fails immediately instead of waiting through `-r`'s backoff for something that cannot succeed (judged by the absence of an OS error number underneath the failure, which is what tells the two apart from an ordinary refused or timed-out connection — `reqwest` itself reports all three alike); and a segment that exhausts its retries now sets a flag its siblings check between chunks, so they stop instead of finishing a transfer whose result is discarded anyway (their part files stay valid for `-c`, exactly as before).

---

## 🔥 High Priority (Critical)

Real-download correctness. Each item is a defect in the 1.0.0 behaviour, marked with a `FIXME(<id>)` at the code that needs to change. **Nothing is outstanding here:** A1, A2, A4 (Cycle 1), A5, A6 (Cycle 2), the whole B group (Cycles 1 and 3) and A3 (Cycle 4) are all fixed.


---

## 🟡 Medium Priority (Important)

- [ ] **F · Tests** – Cycle 0 added characterization tests, Cycle 1 property tests for the stream helper and a `criterion` benchmark (`benches/throttle.rs`), Cycle 2 a state table and property tests for the resume and retry decisions, Cycle 3 property tests for segment planning and resume validation, Cycle 4 property tests for URL validation, Cycle 5 a classifier checked against an independent CIDR table and properties for file-name safety, Cycle 6 properties for digest parsing and for name claiming (no two requests in a run ever get the same file), option-merging tests, and integration tests for verification before placement and for a file appearing mid-download. The only `known_defect_*` test left is for untrimmed input-file lines (D6). Still to do: a property test for `parse_size`, and a throughput regression gate if CI is added.

---

## 🟢 Low Priority (Nice‑to‑Have)

- [ ] **D9 · `--max-time`** – `-t` no longer caps the total transfer (Cycle 1); an optional overall limit could return as its own flag.
- [ ] **D12 · `Content-Disposition` file names** – honour the name a server suggests, as `curl -J` and `wget --content-disposition` do. Deferred from C3 because the name must be known before the request: resume looks for `name.part` by name, and no-clobber (C4) needs it too. It needs either an extra `HEAD` or `download` choosing the path after the headers, and a decision on how that interacts with `-c`.
- [ ] **D13 · Proxy-aware host policy** – a configured system proxy (`HTTP_PROXY`) does its own DNS, so the resolver filter cannot police what it connects to; only the literal-address and redirect checks apply. Either document it or refuse to combine a proxy with the default policy.
- [ ] **D14 · Digests for several URLs** – `--sha256` applies to a single URL. A `sha256sum`-style checksum file (`--checksum-file`) would verify a whole batch, and could reuse the verifier hook added in Cycle 6.
- [ ] **D15 · Renaming and segment files** – when numbering a name, an in-progress single-connection download (`name.part`) is respected but the segmented parts (`name.part0`, …) are not looked for. Only matters when two segmented downloads target the same name at once.
- [ ] **D11 · Opt-out for the secret-file list** – `.bashrc`, `.zshrc` and the other listed paths are refused even when downloading a dotfile from a dotfiles repository is exactly what is wanted. An explicit flag, alongside C2's `--allow-private`, would keep the default and allow the exception.
- [ ] **G · Project hygiene and release** – remove the unused dependencies (`anyhow` and the `digest` crate; nothing uses either), stop ignoring and commit `Cargo.lock` (this is a binary crate), run `cargo fmt` as its own commit, commit the end-to-end harness (`scripts/e2e/`) with a short README, bump the version to 1.1.0 and cut a dated release section in the changelog. The options-struct part is done (`DownloadOptions`), and `regex` was removed in Cycle 4. Cycle 5 added `tokio` (already in the dependency tree through `reqwest`) for the DNS filter. Existing code is not `rustfmt`-formatted; run `cargo fmt` as its own commit so blame stays readable. `scripts/check` (Cycle 8) deliberately leaves `cargo fmt --check` out until this lands.

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

1. **Cycle 9 – G, the 1.1.0 release**: remove the unused dependencies, track `Cargo.lock`, `cargo fmt`, commit the end-to-end harness, bump the version, and turn `[Unreleased]` into a dated `1.1.0` section. Everything in the high- and medium-priority groups is done, and Cycles 7–8 cleared D1–D8 and D10, so this is the natural point to release.
2. **Then**: the remaining low-priority items (D9, D11–D15), in whatever order is most useful.
