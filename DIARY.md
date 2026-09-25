# 📘 DIARY.md

## Architectural Decision Log

*This document is a chronological record of architectural decisions, trade-offs, and reasoning for rget. Just as Git tracks code changes, this diary tracks the **why** behind the code.*

---

## 📋 Decision Index

| Date | Decision Area | Choice | Status |
|------|---------------|--------|--------|
| 2026-09-20 | Source layout | Feature-first: `app/` + `features/*` + `shared/` | ✅ Confirmed |
| 2026-09-20 | Restructure strategy | One feature at a time, green build after each | ✅ Confirmed |
| 2026-09-20 | Error types | Per-feature errors aggregated by `AppError` | ✅ Confirmed |
| 2026-09-20 | Proving "no behaviour change" | 65-case end-to-end baseline diff plus characterization tests | ✅ Confirmed |
| 2026-09-20 | Streaming the body and the meaning of `-t` | Explicit read loop; timeout becomes a stall limit | ✅ Confirmed |
| 2026-09-20 | `--limit-rate` with `--segments` | Limit applies to the whole download | ✅ Confirmed |
| 2026-09-20 | Partial downloads and resume validation | `name.part` + `.part.meta` sidecar, `If-Range` | ✅ Confirmed |
| 2026-09-20 | Retry policy | Classify errors; honour `Retry-After` (cap 60s) | ✅ Confirmed |
| 2026-09-20 | Segmented parts, validation and merge | Keep part files, sidecar layout check, `io::copy` merge | ✅ Confirmed |
| 2026-09-20 | URL validation | Judge the parsed URL: hostname, `..` segments, secret-file segments | ✅ Confirmed |
| 2026-09-20 | Host policy (SSRF) and `--allow-private` | Classify addresses; guard redirects; filter DNS answers | ✅ Confirmed |
| 2026-09-20 | Names taken from URLs | Decode, then sanitize; defer `Content-Disposition` | ✅ Confirmed |
| 2026-09-20 | Existing files (no-clobber) | `--if-exists overwrite\|skip\|rename`, default unchanged | ✅ Confirmed |
| 2026-09-20 | Verifying a checksum | Verify before putting in place, via a hook; delete on mismatch | ✅ Confirmed |
| 2026-09-25 | Handling Ctrl+C | `ctrlc` crate + one shared flag checked in `stream::copy` | ✅ Confirmed |

---

## 📝 Decision Entries

### Source Layout: Feature-First

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

rget 1.0.0 had a flat `src/`: `download.rs` (620 lines holding single-connection, segmented, retry, throttle and resume logic), `validator.rs`, `checksum.rs`, and a `main.rs` that also built output paths, read input files, and ran the worker pool. The next cycles (streaming, timeouts, resume, retry classification, validator fixes) all land in `download.rs`, so the file was about to get harder to change safely.

---

#### Options Considered

**Option A: Keep the flat layout**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No churn <br> • Nothing to learn |
| **Disadvantages** | • `download.rs` keeps growing <br> • Retry, resume and throttle logic stay tangled together <br> • Integration tests cannot reach a binary-only crate |
| **Implementation Difficulty** | None |
| **Fit with Constraints** | Poor for the planned bug-fix work |

**Option B: Feature folders directly under `src/`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Shallowest tree |
| **Disadvantages** | • `app`, `shared` and features sit at one level, so the dependency direction is not visible in the tree |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Acceptable |

**Option C: `app/` + `features/*` + `shared/`, with `lib.rs` and a thin `main.rs`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Dependencies point one way: `app` → `features` → `shared` <br> • Each feature passes the Delete Test <br> • Retry, resume, throttle and segmented code get their own files <br> • The library crate makes `tests/integration` possible |
| **Disadvantages** | • More files and modules for a ~1,500-line tool <br> • Many small `mod.rs` files |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best; follows `1architecture.md` and `rust.md` §10 |

---

#### Decision & Rationale

**Chosen Option:** C

**Reasoning:**

The planned fixes each touch one concern (streaming, retry, resume, validation), so isolating those concerns before changing them keeps each later diff small. Features never import each other; `app/run.rs` is the single place that wires input, validation, destination, download and integrity together. `shared/` holds only the progress bar and size parsing.

**Trade-offs accepted:**
- More files than a tool this size strictly needs.
- `mod.rs` style, as in the feature-first example in `rust.md` §10.1, rather than the `name.rs` + `name/` style.
- The scaffold also reserves `benches/throttle.rs`, which stays empty until the benchmark is written with `criterion` (Cycle 1).

---

#### Implementation Notes

- `download.rs` was split into `single`, `segmented`, `resume`, `retry`, `throttle`, `client`, `pool` and `options`. The 12-argument calls became a `DownloadOptions` struct.
- `validator.rs` became `validation/{sanitize,host}.rs`; `checksum.rs` became `integrity/sha256.rs`.
- Path building, file naming and input-file reading were carved out of `main.rs` into `destination/` and `input/`. The duplicate output-path computation used for checksum verification was removed; the check now uses the path the download wrote to.
- Internals are `pub(super)`; only entry points are exported (`download_file`, `run_pool`, `validate_url`, `verify_sha256`, …).
- Verified mechanically: no feature imports another feature, `shared/` imports nothing from `features/` or `app/`, and features never import `app/`.

---

#### References

- `$SUITE/1architecture.md`, `$SUITE/3language-etiquette/rust.md`
- `ROADMAP.md`, item R

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Restructure Strategy

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Moving ~1,500 lines into a new layout with no logic changes, in a repo with no tests, and with your existing GitHub history to preserve.

---

#### Options Considered

**Option A: Big bang**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Fewest steps <br> • One clean diff |
| **Disadvantages** | • About a dozen files change at once <br> • The error-type split ripples through the whole call graph <br> • Hard to debug if anything breaks |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Poor without a safety net |

**Option B: Feature by feature, build green after each step**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small reviewable steps <br> • A regression points at one feature |
| **Disadvantages** | • Old and new modules briefly coexist <br> • More steps |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option C: Facade first (`pub use` the old files, then move)**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Never a broken build |
| **Disadvantages** | • Double the churn <br> • Risk of leaving facade cruft behind |
| **Implementation Difficulty** | Medium, longest |
| **Fit with Constraints** | Acceptable but wasteful |

---

#### Decision & Rationale

**Chosen Option:** B

**Reasoning:**

With no existing tests, the only way to trust a large move is to keep each step small enough to verify. The order was dependency-first: `shared`, then `integrity`, `validation`, `input` and `destination`, then `download`, then `app` and the thin `main.rs`. Files are moved (not re-created) wherever the contents stay close to the original, so Git's rename detection keeps `git log --follow` and `git blame` useful. No new repository was created; the existing history is untouched.

**Trade-offs accepted:**
- The split of `download.rs` is not one-to-one, so blame on the split-out pieces will point at this restructure commit.

---

#### References

- `$SUITE/4iteration.md` (three courses of action before a non-trivial change)

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Error Types

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

`RgetError` was one enum shared by every module, which would make deleting a feature leave dead variants behind and would tie retry classification (Cycle 2) to unrelated errors.

---

#### Options Considered

**Option A: Keep one `RgetError`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No change |
| **Disadvantages** | • Breaks the Delete Test <br> • Every feature depends on a shared type that knows about all of them |
| **Implementation Difficulty** | None |
| **Fit with Constraints** | Poor |

**Option B: Per-feature errors plus an `AppError` that wraps them**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Each feature owns its failures <br> • Retry classification can match on `download::Error` directly |
| **Disadvantages** | • More types <br> • `main` prints the error with `{:?}`, so the variant names must be kept or its output changes |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option C: `anyhow` everywhere**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Least code |
| **Disadvantages** | • Loses the typed variants that retry and exit-code handling will need |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Poor for a library crate |

---

#### Decision & Rationale

**Chosen Option:** B

**Reasoning:**

Variant names and `#[error(...)]` messages were kept exactly as before, and `AppError` implements `Debug` by forwarding to the wrapped error (with an `Io(..)` special case). That keeps the `Error: InvalidUrl("…")` and `Error: ChecksumMismatch { … }` output byte-for-byte identical, which the end-to-end comparison confirmed. Roadmap item D3 will clean that output up deliberately, as its own change.

---

#### References

- `$SUITE/3language-etiquette/rust.md` §6

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Proving "No Behaviour Change"

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

A refactor is only safe if the behaviour is provably the same, and there were no tests to prove it. One obstacle: the URL validator blocks `localhost` and `127.0.0.1`, so a local test server would normally be rejected.

---

#### Options Considered

**Option A: Manual spot checks**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Quick |
| **Disadvantages** | • Misses edge cases <br> • Not repeatable |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Poor |

**Option B: Unit tests only**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Permanent value |
| **Disadvantages** | • Cannot see CLI output, exit codes, config merging, or the wiring in `main` |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Necessary but not sufficient |

**Option C: End-to-end baseline diff, plus characterization tests**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Runs the real binary, so it covers output text, exit codes, config, and file contents <br> • The old binary is the oracle <br> • The tests then keep protecting the behaviour |
| **Disadvantages** | • More setup <br> • The harness is Python plus a shell script, kept outside the repo |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

---

#### Decision & Rationale

**Chosen Option:** C

**Reasoning:**

A small Python HTTP server (with `Range`, `HEAD`, redirects, 404, a flaky endpoint, and header echo) is bound to `127.0.0.2`. The validator's loopback check is string-based (`127.0.0.1` only), so it lets `127.0.0.2` through, which is itself roadmap item C1. Sixty-five cases (argument errors, config, single, resume, segmented, retries, redirects, headers, jobs, SHA‑256) were recorded from the 1.0.0 binary, then re-run against the restructured one, with output lines sorted within each case because thread completion order varies. The two runs matched on every case. On top of that, characterization tests pin the same behaviour in the repo. Known defects are pinned by `known_defect_*` tests so that fixing them is a visible, deliberate flip.

**Bugs found while recording the baseline (kept as they were, now on the roadmap):**
- Resuming an already-complete file truncates it to 0 bytes and reports success (A5).
- Segmented resume advances a partial part's start twice and produces a short, corrupt file (A5).
- `plan_ranges` underflows when the file is smaller than the segment count (B).
- `-j 0` hangs (D7).

**Trade-offs accepted:**
- The end-to-end harness is not in the repo yet; it lives in the session scratchpad.

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Streaming the Body and the Meaning of `-t`

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

`response.bytes()` buffered the whole file in memory before the first byte reached disk, and its timeout was one deadline for the entire body, so any download longer than `-t` (default 30s) failed. Error responses were also saved as the file because the status was never checked. The roadmap items were A1 (streaming), A2 (timeout) and A4 (status), plus B7 and B10 in the same code.

---

#### Options Considered

**Option A: Explicit read loop with one reusable buffer, in a shared `stream` helper**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest change <br> • Read errors and write errors stay distinguishable, which the retry work needs <br> • Generic over `Read`/`Write`, so it is property-testable without a network <br> • Reads can be capped at the rate limit |
| **Disadvantages** | • A little more code than `io::copy` |
| **Implementation Difficulty** | Easy–Medium |
| **Fit with Constraints** | Best |

**Option B: `std::io::copy` into a writer that throttles and reports progress**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Least code |
| **Disadvantages** | • Read and write failures merge into one `io::Error`, so a network timeout cannot be told from a full disk <br> • Fixed 8 KiB buffer <br> • Throttling hidden inside `write()` |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Weak: it loses the error classification A6 needs |

**Option C: Async internals (tokio and the async `reqwest` client with `read_timeout`)**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • A real `read_timeout` setting <br> • Cancellation on drop |
| **Disadvantages** | • Rewrites `single`, `segmented`, `pool` and `client` <br> • Adds direct dependencies <br> • Buys nothing we lack (see below) |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** A

**Reasoning:**

The blocking `reqwest` client has `timeout` and `connect_timeout` but no `read_timeout` (only the async client does), so a "connect plus stall timeout" looked like it needed extra machinery. A throwaway probe against a slow local server showed otherwise: `bytes()` applies one deadline to the whole body (a 4.2s body with `-t 2` failed at 2.0s), while `Read::read()` computes a fresh deadline on every call (the same body finished in 3.5s, and a server that went silent failed at 2.0s). Streaming therefore turns the existing `-t` into a stall limit with no new code. That removed the main argument for option C.

**Results measured afterwards:** a 400 MiB download over loopback peaked at 437 MB of memory in 2.4s before and 24 MB in 0.7s after. The 73-case end-to-end comparison against the 1.0.0 binary changed only where intended: slow downloads succeed, error statuses are errors, header-guarded servers are segmented, and the DEBUG lines are gone.

**Trade-offs accepted:**
- `-t` no longer caps the total transfer time. A separate `--max-time` can be added if wanted.
- A stalled reader is reported by `reqwest` as an `io::Error` wrapping a `reqwest::Error` with the misleading text "error decoding response body". The helper unwraps it into `Error::Stalled` ("no data received for Ns") or `Error::Network`.
- Error statuses are a new `Error::HttpStatus`, checked before the output file is opened, so a 404 never replaces an existing file.
- Interim gap: a transfer interrupted mid-stream now leaves the bytes received so far under the final name (before it left an empty file). Roadmap item C5 (`name.part`, then rename) closes it, together with A5.
- A 416 on an already-complete file is now an error that leaves the file intact, instead of truncating it to 0 bytes and reporting success. A5 turns it into a success.

---

#### Implementation Notes

- One 64 KiB buffer is allocated per download and reused; memory no longer depends on the file size.
- The segmented probe now sends `-H` and the User-Agent and follows `--follow-redirects`; a non-2xx probe falls back to the single-connection path, which reports the real error.
- `🔍 DEBUG:` prints became `log::debug!` (`RUST_LOG=debug`). `log` is a direct dependency now.
- Benchmark: `benches/throttle.rs` (`criterion`), unthrottled versus a generous limit, over loopback to `/dev/null`. About 400–470 MiB/s, and the throttle bookkeeping is within noise. To gate on regressions, record `--save-baseline before` and fail on more than 5%.

---

#### References

- `$SUITE/2engineering.md` (pillars 2, 3 and 5), `$SUITE/4iteration.md`
- `ROADMAP.md`, items A1, A2, A4, B7, B10

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### `--limit-rate` with `--segments`

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Each segment thread ran its own throttle with the full limit, so `--limit-rate 2M --segments 4` allowed up to 8 MB/s. Before streaming, throttling only slowed disk writes and the effect was hidden; once the network read is throttled, the excess became visible.

---

#### Options Considered

**Option A: Keep the limit per segment**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No change |
| **Disadvantages** | • The limit is not honoured; the real total is N times larger |
| **Implementation Difficulty** | None |
| **Fit with Constraints** | Poor |

**Option B: Divide the limit by the number of segments**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small and deterministic <br> • The total stays at or below the limit |
| **Disadvantages** | • An idle or finished segment does not lend its share to the others <br> • Rounds down |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Good |

**Option C: One shared token bucket across segments**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Exact total, with unused share redistributed |
| **Disadvantages** | • Shared mutable state between threads, so a lock on the hot path <br> • Needs a concurrency model first (`2engineering.md` pillar 4) |
| **Implementation Difficulty** | Medium–Hard |
| **Fit with Constraints** | More than this needs |

---

#### Decision & Rationale

**Chosen Option:** B

**Reasoning:**

It fixes the real problem with no new shared state. `0` (unlimited) stays unlimited, and no segment is ever divided down to zero (each gets at least 1 byte per second). A shared bucket can replace it later if uneven segments make the split visibly wasteful.

**Trade-offs accepted:**
- When one segment finishes early, the others stay at their share instead of speeding up.

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Partial Downloads and Resume Validation

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Three roadmap items were tangled: A5 (resume correctness: a complete file was truncated, a changed remote file produced a corrupt result, retries restarted from zero, segmented resume skipped bytes), C5 (partial data lived under the final name), and the need for a *validator* (ETag or Last-Modified) remembered from the earlier response. Where the partial state is stored decides where that memory can live, so they were designed together.

---

#### Options Considered

**Option A: `name.part` plus a sidecar `name.part.meta`, validated with `If-Range`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Exact change detection with standard HTTP <br> • Works across runs <br> • Falls back to Last-Modified when there is no ETag <br> • A sidecar for another URL is ignored safely <br> • The `toml` and `serde` dependencies already existed |
| **Disadvantages** | • An extra file next to the download <br> • A moved `.part` loses its sidecar and resumes unvalidated |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: `name.part` only, with a stateless overlap check (re-fetch the last 4 KiB and compare)**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No extra file |
| **Disadvantages** | • Only a heuristic: misses changes outside the window <br> • Costs an extra request <br> • Awkward to splice into the stream |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Weak |

**Option C: Validators in memory only, partial data stays under the final name**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest change |
| **Disadvantages** | • Does not fix C5 <br> • A changed file across runs still corrupts |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** A

**Reasoning:**

Retries within a run and resumes across runs then use the same mechanism: the state on disk. There is no in-memory state to lose, and a retry is simply another attempt that finds a `.part` file and its sidecar. Writing to `name.part` and renaming at the end also means an existing `name` is only replaced by a finished download.

**The resume state machine** (`resume.rs`, pure functions with a decision table in the module docs, exhaustive unit tests, and property tests), per `2engineering.md` pillar 4:

| Request | Response | Meaning |
|---------|----------|---------|
| full (no usable partial) | 2xx | normal download |
| `Range: from-` | 206 starting at `from` | append |
| `Range: from-` | 206 elsewhere, or without `Content-Range` | error (wrong offset) |
| `Range: from-` + `If-Range` | 200 | remote file changed: restart |
| `Range: from-` | 200 | server ignores ranges: restart |
| `Range: from-` | 416, complete length equals `from` | already complete: success |
| `Range: from-` | 416, any other length | partial is stale: drop it and refetch |
| any | other status | reported by the caller |

Before the request, `plan` decides between `Fresh` and `Continue`: no partial data, a sidecar for another URL, or more partial bytes than the file has all mean `Fresh`. A strong ETag is the validator; a weak ETag is never sent in `If-Range`, so Last-Modified is used instead. With no sidecar at all (data from 1.0.0 or another tool) the resume goes ahead unvalidated, as wget and curl do.

**Behaviour rules:**
- Without `-c`, a leftover `name.part` is discarded. Within a run, retries always continue from what the previous attempt wrote; a server that ignores `Range` just answers `200`, which restarts.
- `-c` also adopts a partial `name` (renamed to `name.part` only once the server confirms it will resume), so partials from 1.0.0 still work.
- A target that exists but is not a regular file (`/dev/null`, a FIFO) is written directly.
- A symlinked output is resolved first, so the download writes through it. The first version of the staging replaced the link with a regular file; a manual check caught this and a test now pins it. A dangling link cannot be resolved and is replaced.

**Trade-offs accepted:**
- One more file beside an in-progress download.
- Segmented downloads keep their `name.partN` files without validators; only the double-offset bug (a partial part resumed twice as far in as it should have been) was fixed here. Validation and cleanup for segments is roadmap item B11. Their merge now goes through `name.part` and a rename.
- A server that answers `206` to an `If-Range` it should have rejected cannot be detected.

---

#### Implementation Notes

- New `partial.rs` (paths, the `.part` lifecycle, the sidecar) and a rewritten `resume.rs`; `single.rs` orchestrates.
- The benchmark now writes to a real file so the staging and rename are measured: about 340–360 MiB/s over loopback, against 400–470 MiB/s to `/dev/null` before, which is the cost of actually writing 32 MiB.
- Mutation-checked: removing `If-Range`, retrying everything, ignoring `Retry-After`, and restarting instead of continuing each made a specific integration test fail.

---

#### References

- `$SUITE/2engineering.md` (pillars 1, 2 and 4), RFC 9110 sections 13.1.5 (`If-Range`) and 14 (ranges)
- `ROADMAP.md`, items A5, C5

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Retry Policy

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Every error was retried, including ones that cannot succeed (a 404, a blocked URL), and a server's `Retry-After` was ignored. The backoff exponent also overflowed for a very large `-r`.

---

#### Options Considered

**Option A: Classify the error in the retry loop; carry `Retry-After` on the error**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small and testable <br> • The typed errors added in Cycle 1 make the rule a single `match` |
| **Disadvantages** | • The policy is fixed in code |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Best |

**Option B: A configurable `RetryPolicy` struct (flags such as `--retry-all-errors`, `--retry-max-time`)**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Extensible |
| **Disadvantages** | • New CLI surface nobody has asked for yet |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Premature |

**Option C: Keep retrying everything**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No change |
| **Disadvantages** | • A permanent failure costs the whole backoff schedule |
| **Implementation Difficulty** | None |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** A

**Reasoning:**

Retried: a stalled transfer, transport errors, and HTTP 408, 425, 429, 500, 502, 503 and 504 (the same set as `curl --retry`). Not retried: other 4xx, 501/505, a disabled redirect, protocol errors, and I/O errors (a full disk will not clear by itself). A `Retry-After` (seconds or an HTTP-date) replaces the computed backoff when it is longer; the wait is capped at 60s, and a server that asks for more makes us give up immediately and say so, rather than sleep. The backoff itself is capped and saturating.

**Trade-offs accepted:**
- Every transport error is retried except a malformed request or a redirect loop. `reqwest` does not distinguish a refused connection from an invalid certificate or a missing DNS name, so those are retried too. That is harmless but slow, and it is on the roadmap as D8.
- `-r` still defaults to 0, so none of this matters until `-r N` is used.

---

#### References

- `ROADMAP.md`, item A6

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Segmented Downloads: Parts, Validation and Merge

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Three roadmap items touched the same code: B8 (a failed segment failed the whole download and the Cycle 2 retry policy was not used), B9 (the merge read each part fully into memory) and B11 (parts were trusted by size alone, and leftovers piled up). Re-reading the worker turned up more defects in the same place: a `200` answer to a ranged request was accepted, the `Content-Range` offset and final length were never checked, "ranges unsupported" was detected by searching an error *message*, the layout of the parts was not recorded (so a different `--segments` misaligned every part), oversized parts were trusted, and a file smaller than the segment count panicked.

---

#### Options Considered

**Option A: Keep the part files, stream the merge, extend the sidecar with the layout**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small and safe <br> • Fixes every listed defect <br> • `io::copy` between files is done in the kernel on Linux <br> • Disk use stays at about the file size plus one part, because each part is deleted once merged |
| **Disadvantages** | • One extra pass over the data at the end |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: One preallocated `name.part`, workers `write_at` their ranges, progress checkpoints in the sidecar**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No merge pass and no second write of the data |
| **Disadvantages** | • Checkpointing and fsync ordering <br> • Unix-only `write_at` <br> • A sparse, full-length `.part` looks *complete* to a single-connection resume or another tool, which is a corruption hazard |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Poor for the saving it buys |

**Option C: Only retries and `io::copy`, no sidecar**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest change |
| **Disadvantages** | • A changed file across runs still corrupts <br> • A different `--segments` still misaligns the parts |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Leaves B11 undone |

---

#### Decision & Rationale

**Chosen Option:** A

**Reasoning:**

Before choosing, the two merges were measured on 4 parts of 128 MiB (tmpfs): reading each part into a `Vec` peaked at 133 MB and took 1.3s; `io::copy` peaked at 2.4 MB and took 0.57s. The cost of A is one pass the measurement shows to be cheap, whereas B adds real hazards for a saving that matters only on very large files. It can be revisited if the merge ever shows up as a bottleneck.

**Results measured afterwards:** a real 400 MiB download with 4 segments over loopback went from 127 MB peak memory and 1.4s to 34 MB and 0.9s, with an identical 419,430,400-byte result. The 90-case end-to-end comparison against the previous build changed only in the eight new cases; all 82 existing ones were byte-identical. The results in the changed cases were checked against independently computed hashes.

**Rules implemented:**
- The sidecar gains a `segments` field and is written once, after the probe and before the workers start. A resume is accepted only when the URL, total size and segment count match and the file is unchanged (same strong ETag, else same Last-Modified). Weak ETags cannot prove byte identity and are ignored. Otherwise every part is discarded and the download starts over.
- With no sidecar (parts from 1.0.0, say) the parts are accepted as unvalidated prefixes, the same rule as a single-connection resume. A single-connection sidecar never validates segmented parts, and a segmented sidecar never validates a single `name.part`.
- A `200` to a ranged request, a `416`, or a `Content-Range` at the wrong offset becomes a typed `Error::RangesUnsupported`, which is never retried and makes the download fall back to one connection. This replaces matching on an error message.
- Each segment retries on its own with the Cycle 2 policy and continues from whatever its part file holds. The bar counts each byte once.
- Every part is length-checked before any is merged; a wrong-sized part is deleted so the next run fetches it again.
- Parts beyond the current segment count and parts longer than their range are deleted.
- The download returns the failing segment's own error, and keeps the parts and sidecar so `-c` can continue.
- There are never more segments than bytes (`plan_ranges` clamps), which removes the underflow.

**Trade-offs accepted:**
- One extra pass over the data at the end.
- When a segment fails for good the other segments keep downloading until they finish. Their parts stay resumable, so nothing is lost, but bandwidth is wasted; a shared cancel flag is on the roadmap as D10.
- A server that answers `206` to an `If-Range` it should have rejected cannot be detected.

---

#### Implementation Notes

- New `parts.rs` holds the pure and file-level logic (planning, discovery, cleanup, reconciliation, resume validation, merge) with unit and property tests; `segmented.rs` is orchestration only.
- The test server gained `/dropseg` (the first range request in the second half dies mid-body) and `/lying` (advertises ranges, answers `200`).
- Mutation-checked: allowing every resume, keeping leftover parts, accepting a `200`, disabling per-segment retries, keeping oversized parts, and skipping the length check before merging each made a specific test fail. Two of my first mutation runs tested nothing (an unsupported `a|b` filter, and a redundant defence in the worker that let one mutation survive at integration level), so the runs were repeated with correct filters and a unit-level check.

---

#### References

- `$SUITE/2engineering.md` (pillars 1, 2 and 4), `$SUITE/4iteration.md`
- `ROADMAP.md`, items B8, B9, B11

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### URL Validation: Judge What the URL Means

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

The validator refused any URL whose *text* contained `; | & $ ( ) < > ` ``, refused a decoded `; | &`, refused `../` anywhere, and matched ten "sensitive" regular expressions as substrings of the whole URL. In practice that rejected ordinary URLs: anything with a query string like `?a=1&b=2`, `file(1).zip`, `Rust_(programming_language)`, `;jsessionid=…`, an encoded `%26`, hosts such as `foo.environment.com`, and query text such as `?next=../home`. It was also weaker than it looked: `%2e%2e` and `%2eenv` passed straight through.

This is a security feature that the README advertises, so the decision started from the threat model instead of from the false positives.

**What was established before choosing** (a throwaway probe against the `url` crate, plus a search of the code):
- rget never spawns a process, so URL text never reaches a shell. The "command injection" character list could only matter if a shell had already seen the URL, and by then rget has not run yet (which is why URLs with `&` must be quoted).
- `& ; $ ( ) |` are legal sub-delimiters in a path or query. The parser keeps them, percent-encodes `< > `` ` and space, strips CR and LF, and resolves `..` segments in every spelling (`../`, `%2e%2e`, `.%2E`, `\..\`).
- The parser does *not* handle two things: it accepts hosts such as `exa;mple.com`, `exa&mple.com` and `exa$mple.com`, and it leaves `%2eenv` and `..%2f` encoded.

---

#### Options Considered

**Option A: Structural checks on the parsed URL**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Fixes every false positive <br> • Keeps the protections the README advertises <br> • Catches encoded spellings the old code missed <br> • Removes the `regex` dependency |
| **Disadvantages** | • What counts as "sensitive" is a judgement call <br> • More logic than the alternatives |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: Keep every check, but only on the path, and match `.env` as a segment**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest diff |
| **Disadvantages** | • Still refuses `file(1).zip`, `_(disambiguation)`, `;jsessionid=` and `%26` in paths <br> • Fixes two of the three known defects |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Not enough |

**Option C: Scheme and host checks only**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No false positives <br> • Honest about the threat model: asking a *remote* server for `/etc/passwd` cannot hurt the local machine, and the real risks are SSRF (host checks) and writing a bad local file name |
| **Disadvantages** | • Removes protections the README advertises |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Defensible, but a bigger product change |

---

#### Decision & Rationale

**Chosen Option:** A, with the sensitive-file list kept as policy.

**Rules implemented** (`sanitize.rs`, pure functions on `&str` and `&Url`):
- **Scheme:** `http` and `https` only (unchanged).
- **Host:** letters, digits, `.`, `-` and `_` (an internationalised name is already ASCII). IP literals are not checked here.
- **Traversal:** judged on the *raw* text, because the parsed path has already been resolved. The raw path is what follows the authority, cut at the first `?` or `#`. Each segment is percent-decoded and split again on `/` and `\`, so `%2e%2e`, `.%2E`, `..%2f` and `..%5c` all count; a segment must equal `..` exactly, so `a..b`, `dir../x` and `...` do not. The query and fragment are never examined. The error message is unchanged.
- **Secret files:** the parsed path is decoded, lower-cased and flattened into pieces. A piece equal to `.env`, `.bashrc` or `.zshrc`, or two consecutive pieces equal to `etc/passwd`, `etc/shadow`, `etc/sudoers`, `.git/config`, `.aws/credentials`, `.ssh/id_rsa` or `.ssh/authorized_keys`, is refused. Host, query and fragment are never examined, and matching is exact, so `.env.example`, `environment` and `etc/passwd.bak` pass.
- **Everything else legal in a URL is allowed.**

**Things caught along the way:**
- My first `raw_path` searched for the first slash *before* cutting at `?`, so for `http://example.com?x=/../` the query text was mistaken for a path and the false positive I was removing came straight back. A unit test caught it.
- One mutation survived: checking traversal on the whole URL instead of only the path. My example used `?next=../home`, where the segment is `x?next=..` and never equals `..` even on the whole string. The distinguishing case has a slash first (`?next=/../home`); it is now an example test and part of a property test, and the mutation is caught.
- The end-to-end comparison showed the old validator waving `http://example.com/a/%2e%2e/b` and `http://example.com/%2eenv` through to the server. Both are now refused.

**Verification:** 187 tests pass. Mutating each rule (checking the whole URL, skipping the percent-decoding, matching secret names anywhere in the text, skipping the host check) makes a specific test fail. The properties were also run with 10,000 cases each. The 99-case end-to-end comparison against the previous build changed exactly the 12 intended cases; every other case was identical once file listings were ignored. The four validator cases that used `example.com` now hit the local server, so the harness no longer depends on the internet.

**Trade-offs accepted:**
- The secret-file list is a policy choice, kept as it was. It still refuses `.bashrc` from a dotfiles repository. An opt-out flag next to C2's `--allow-private` is on the roadmap (D11).
- Percent-decoding is applied once. A double-encoded `%252e%252e` is not treated as traversal, because nothing in a client decodes it twice.
- **A consequence to watch:** the output file name is taken from the URL as written, and URLs may now contain `;`, `$`, `(`, `)` and `|`. Such a name is legal on Linux but awkward in a shell. Roadmap item C3 (safe file names) matters more because of this change, and must also make sure a decoded name cannot leave the target directory.

---

#### References

- RFC 3986 section 2.2 (reserved characters), the WHATWG URL Standard (host and path parsing)
- `ROADMAP.md`, item A3

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Host Policy: Refusing Local and Private Destinations

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

The validator refused hosts by matching the start of the host *text* (`192.168.`, `10.`, `172.16.` … `127.0.0.1`). Probing it showed how thin that was: **9 of 15** loopback and private spellings passed, including `127.0.0.2`, `0.0.0.0`, `169.254.169.254` (the cloud metadata address), carrier-grade NAT, `[::ffff:127.0.0.1]`, `[::]`, `foo.localhost` and `localhost.`. The URL parser already normalises number spellings (`2130706433`, `0x7f.1`), so those were caught.

A literal-address check can also never see the two routes that matter most for a downloader. With a throwaway pair of local servers I confirmed both: a direct request to an internal service was refused, but **the same service was fetched through a redirect** from a host the validator allowed, and **through a hostname that resolves to loopback**; in both cases its content was written to disk. Redirects are followed by default and were never re-checked, and hostnames were never checked at all.

---

#### Options Considered

**Option A: Classify addresses, and re-check every redirect hop**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small <br> • Fixes every literal-address miss and the redirect hole |
| **Disadvantages** | • A DNS name that leads to a private address (`localtest.me`, `*.nip.io`) still gets through |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Not enough |

**Option B: A, plus a filtering DNS resolver installed on every client**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Closes every hole demonstrated <br> • The connection is made to exactly the addresses the resolver returns, so there is no gap between "checked" and "connected" (DNS rebinding) <br> • Covers every redirect hop's hostnames for free |
| **Disadvantages** | • `tokio` becomes a direct dependency (already in the tree through `reqwest`) <br> • Some async plumbing |
| **Implementation Difficulty** | Medium–Hard |
| **Fit with Constraints** | Best |

**Option C: B, plus a pre-flight lookup in validation for a friendlier error**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • A nicer early message |
| **Disadvantages** | • A second lookup and a check-then-connect gap <br> • No added protection over B |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** B, with `--allow-private` added in the same cycle.

**Why C2 came along:** fixing the classifier makes `127.0.0.2` unreachable, and the end-to-end harness, development servers and machines on a home network all depend on reaching such addresses. Without an opt-out the fix would have been unusable for them; the flag was small once the plumbing existed.

**How it is built:**
- **`shared/address.rs`** holds the one pure function, `is_public(ip)`, and the `HostPolicy` enum. It lives in `shared` so that `validation` and `download` can both use it without importing each other (the Delete Test). IPv4: `0/8`, `10/8`, `100.64/10`, `127/8`, `169.254/16`, `172.16/12`, `192.0.0/24`, `192.168/16`, `198.18/15`, the documentation ranges, `224/4` and `240/4`. IPv6: `::`, `::1`, `fc00::/7`, `fe80::/10`, `fec0::/10`, `ff00::/8`, `2001:db8::/32`, and addresses that wrap an IPv4 one (IPv4-mapped, the deprecated IPv4-compatible form, NAT64 `64:ff9b::/96`, 6to4 `2002::/16`), which are judged by the address inside. `localhost` and `*.localhost` are refused by name.
- **Validation** checks the initial URL (an IP literal, or a local name). **Download** checks what validation cannot: redirect hops (a custom redirect policy that refuses a destination before connecting) and hostnames (a resolver that drops non-public answers and refuses a name that has only such answers). Because the download does not re-check the initial URL, a loopback test server can stand in for "a host that was let through".
- **Errors:** a refusal travels inside `reqwest`'s error chain and is brought back out as `Error::BlockedAddress`, which is never retried.
- **Mixed DNS answers:** private addresses are filtered out and the public ones used; the name is refused only if none remain.

**Things caught along the way:**
- My first name test used `localhost`, which the *name rule* refuses before the resolver ever runs, so nothing yet proved that the resolver was installed and that a refusal survived `reqwest`'s error chain. I made the lookup injectable and added unit tests that drive an invented name that resolves to loopback through the real client.
- One of those tests had a wrong premise: with `--allow-private` no resolver is installed at all, so the injected lookup is never used. The correct assertion is that nothing is filtered.
- The classifier is checked against an independently written CIDR table (`(network, prefix)` pairs) over 20,000 random IPv4 addresses, plus properties that a wrapped IPv6 address is judged like the IPv4 address inside it.
- Ten mutations (forgetting CGNAT, ignoring wrapped IPv6, following every redirect, not installing the resolver, keeping private answers, ignoring local names, and four for file names) each made a specific test fail.

**Trade-offs and limits accepted:**
- A configured system proxy does its own DNS, so the resolver cannot police it. Only the literal-address and redirect checks apply (roadmap D13).
- An address that is globally routable but belongs to this machine cannot be recognised as local. A hostname on the development machine resolved to its own global IPv6 address, which is correctly "public".
- The end-to-end harness cannot exercise redirect and DNS blocking, because that needs a first hop that passes validation and no such address exists locally. The integration and unit tests cover them.
- `--allow-private` turns off every part of the policy (validation, redirects and the resolver filter) together.

---

#### References

- RFC 6761 (special-use names), RFC 6890 (special-purpose address registries), the OWASP SSRF prevention cheat sheet
- `ROADMAP.md`, items C1 and C2

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Names Taken from URLs

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

The output name came straight from the last path segment of the URL, still percent-encoded, so `a%20b.txt` was saved as `a%20b.txt` (pinned by a `known_defect_*` test). Simply decoding it would have been *worse*: `..%2f..%2fetc%2fcron.d%2fx` decodes to `../../etc/cron.d/x`, and joining that to the download directory would leave it. Decoding therefore had to come with sanitization. The A3 change made this more pressing, because URLs may now contain `;`, `$`, `(`, `)` and `|`.

---

#### Options Considered

**Option A: Decode, then sanitize URL-derived names**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small and safe <br> • Property-testable: the result is always exactly one path component |
| **Disadvantages** | • A few names now differ from the URL text |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Best |

**Option B: A, plus `Content-Disposition`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Matches `curl -J` and `wget --content-disposition` |
| **Disadvantages** | • The name must be known before the request: resume looks for `name.part` by name, and no-clobber needs it too <br> • Needs an extra `HEAD`, or `download` choosing the path after the headers |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Conflicts with resume |

**Option C: Refuse suspicious names**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Simple rule |
| **Disadvantages** | • Hostile to legitimate names |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Poor |

---

#### Decision & Rationale

**Chosen Option:** A. `Content-Disposition` is deferred as roadmap item D12.

**Rules** (`filename.rs`): a name given with `-O` is the user's choice and is used as written. Otherwise the last path segment is percent-decoded (if the bytes are not valid UTF-8 the text is left as written rather than guessed at), then `/`, `\`, control characters and invisible text-direction characters (used to disguise an extension) become `_`; the result is trimmed; `.`, `..` and a blank fall back to `downloaded`; a leading `-` becomes `_`, so `rm *` cannot read a file called `-rf` as an option; and a name longer than 240 bytes is cut on a character boundary, keeping a short extension (`.gz`, or `.tar.gz`). The 240 leaves room for the `.part` and `.part.meta` suffixes the download adds. Legal characters such as `; $ ( ) |` and spaces are kept: `file(1).zip` stays `file(1).zip`. The real hazard is a leading dash, not shell metacharacters in a name.

**What the tests establish:** for any string, encoded fully or pushed through the URL library's segment encoder, the result is exactly one ordinary path component, is not `.` or `..`, is at most 240 bytes, has no control characters and no leading dash, and `dir.join(name)` has `dir` as its parent. Removing the slash replacement makes that property fail, which is the very hazard decoding could have introduced.

**Things caught along the way:**
- Two of my first test expectations were wrong (`--help.txt` becomes `_-help.txt`, not `__help.txt`, since only the first character matters; and a tab becomes `_`, not the fallback).
- One *design* point improved: my first version kept only the last extension, turning a long `….tar.gz` into `….gz`. Keeping a short compound extension is nicer, so the code changed, not the test.
- At 50,000 cases, a round-trip property aborted with "too many global rejects", because its generator produced stems ending in a space that a `prop_assume!` then discarded. The fault was in the test's generator, not the code; it now cannot generate such stems.

**Trade-offs accepted:**
- `-O` names are not sanitized, on purpose.
- A name made only of replaced characters becomes `_` and is kept.
- `Content-Disposition` is not honoured yet (D12).

---

#### References

- `ROADMAP.md`, item C3

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Existing Files: `--if-exists`

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

Probing the tool showed three things. A download over an existing, hand-edited file replaced it without a word. Two URLs that map to the same file name (`/x/file.bin`, `/y/file.bin`) silently lost one download: with `-j 1` the second replaced the first, and with `-j 2` the winner was random, with both reporting success. And since Cycle 2 the file is put in place by a rename at the very end, so any "don't clobber" rule has to hold at that moment, not at the start.

---

#### Options Considered

**Option A: `--if-exists overwrite|skip|rename`, default unchanged**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Explicit and discoverable <br> • No silent change for existing scripts <br> • `-n/--no-clobber` is wget muscle memory |
| **Disadvantages** | • The default stays "overwrite", as `curl -O` does |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: Make `rename` the default**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Safe by default, in keeping with the rest of the tool |
| **Disadvantages** | • **Breaking**: a script that re-runs `rget url` to refresh a file would start producing `file (1).zip`, which deserves a major version |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | A product decision, not a bug fix |

**Option C: Only `--no-clobber`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest |
| **Disadvantages** | • No way to keep both copies |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Too little |

---

#### Decision & Rationale

**Chosen Option:** A.

**Rules:**
- **Numbering:** `file (1).zip`, `file (2).zip`, and `archive (1).tar.gz`. A candidate is taken if its final path, its `.part` file or its sidecar exists, so someone else's partial download is never trampled. Only `.tar.<ext>` counts as a compound extension: my first rule accepted any short middle segment and would have numbered `my.file.v2.zip` as `my.file (1).v2.zip`; a unit test caught it.
- **Which paths:** the policy applies to whatever will be written, `-O` included. A device or FIFO (`-O /dev/null`) is never "in the way".
- **`skip`:** prints `already exists, skipping`, counts as success, and if `--sha256` is given the existing file is verified.
- **Duplicates within a run:** names are claimed one URL at a time *before* any download starts (a `Claims` set), so `-j` cannot make two downloads pick the same name and the result is deterministic. A later duplicate is numbered under every policy (skipped under `skip`), because overwriting a file fetched moments ago in the same command is never what was meant.
- **`-c`:** resume continues an existing file, which `skip` and `rename` would refuse to touch, so they cannot be combined on the command line. An explicit `-n` wins over `resume = true` in the config file; a `skip` or `rename` default in the config file does not apply to a run that resumes.
- **Atomic at the last step:** under `skip` and `rename` the finished file is claimed with a hard link, which fails if the name exists. On failure `rename` asks the caller for the next free name and `skip` discards the download, so a file that appears while the download runs is never overwritten. A test creates the file a second into a three-second transfer and checks both policies. Overwrite still overwrites.
- **Layering:** `download` cannot import `destination`, so the last-moment decision is injected as an `OnOccupied` policy (`Replace`, `Skip`, or `Relocate` with a callback the app builds from the `Claims`). `download_file` now returns an `Outcome` saying where the file actually went.

**Trade-offs and limits accepted:**
- The default is still "overwrite". Changing it is a semver-major decision (option B), not made here.
- On a filesystem without hard links the last step falls back to a check followed by a rename, which leaves a small window.
- Numbering respects an in-progress single-connection download (`name.part`) but does not look for segment files (`name.part0` …). Recorded as roadmap item D15.

---

#### References

- `ROADMAP.md`, item C4

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |

---

### Verifying a Checksum

**Date:** 2026-09-20
**Status:** Confirmed

---

#### Context / Background

`--sha256` was checked after the download had already been renamed into place. Probing it showed the consequences: a well-formed but wrong digest replaced an existing good file and then *stayed*, with an error printed afterwards; an upper-case digest was reported as a mismatch (a `known_defect_*` test); and a malformed digest such as `deadbeef` was only noticed after the whole file had downloaded.

---

#### Options Considered

**Option A: Verify before putting in place, through a hook**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • A bad download never replaces anything <br> • The check and the no-replace claim are one step <br> • `download` stays generic: the app hands it a callback built from `integrity` |
| **Disadvantages** | • A small new callback type |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Best |

**Option B: Keep verifying after the rename**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Smallest |
| **Disadvantages** | • The good file is already gone by the time we find out |
| **Implementation Difficulty** | Easy |
| **Fit with Constraints** | Poor |

**Option C: Hash while streaming**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • One fewer read |
| **Disadvantages** | • Only works for plain single-connection downloads; resume and segments still need a read pass |
| **Implementation Difficulty** | Hard |
| **Fit with Constraints** | Poor for the saving |

---

#### Decision & Rationale

**Chosen Option:** A. A mismatch **deletes** the download (the alternative was moving it aside as `name.corrupt`).

**How it works:**
- **`Sha256Digest`** is a value type (`try_new`, per `2engineering.md` pillar 1) holding 32 bytes. It accepts either case and only the first whitespace-separated word, so a whole `sha256sum` line can be pasted; anything that is not exactly 64 hexadecimal digits is refused. Clap parses `--sha256` with it, so a bad digest is a usage error (exit 2) before any request.
- **The hook:** `download` accepts an optional `Verifier` (a callback `&Path -> Result<(), String>`). It runs on the staged `name.part` after the merge (for segmented downloads) and before the rename. A refusal deletes the partial data and its sidecar and returns `Error::Verification`, which is never retried. The app builds the verifier from `integrity::verify_file`.
- **Deleting rather than keeping** matters: if the corrupt `.part` were left behind, a later `-c` would find it complete (the server answers 416 for a full-length range) and put it in place as if it were good.
- **Resume paths:** a complete `.part` left by a killed run is verified before it is accepted, and deleted on failure. A complete existing *final* file that fails verification is reported but never deleted, because this run did not produce it.
- A device or FIFO target cannot be read back, so it is not verified.

**Things caught along the way:**
- Three of my own test digests were wrong: an old test used `deadbeef` as a "wrong digest", which is now correctly a malformed one; one end-to-end case used a full digest whose tail I had typed from memory (only the first 16 characters had ever been shown), so I compute it from the same deterministic data instead.
- One mutation I wrote to prove the case-insensitivity tests was too clever to compile, so it tested nothing; I replaced it with a simple one (reject upper-case hex), and four tests then failed as they should.
- The end-to-end comparison: 140 cases, 24 changed (all intended, none unexplained), 115 identical.

**Trade-offs accepted:**
- A checksum mismatch is not retried: retrying would help a corrupted transfer but waste bandwidth on a wrong digest, and the two cannot be told apart.
- `--sha256` still applies to a single URL (roadmap D14 sketches a checksum file for a batch).
- The delete-on-mismatch choice means a mistyped but well-formed digest costs a re-download.

---

#### References

- `$SUITE/2engineering.md` (pillars 1 and 2), `ROADMAP.md`, item C6

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-20 | Initial entry | deltaog-117 |


---

### Cycle 7: Small Correctness and Polish Fixes (D1–D5)

**Date:** 2026-09-25
**Status:** ✅ Confirmed

---

#### Context / Background

With the high- and medium-priority groups clear (Cycles 0–6), the roadmap's low-priority
list (`D1`–`D15`) held several small, independent defects and rough edges rather than one
feature. Five were picked for this cycle: `D1` (`--init` throttles new users to 1 MB/s
without saying so), `D2` (a broken `config.toml` is silently ignored), `D3` (`main`
prints Rust's raw `Debug` output on failure, and one invalid URL in a batch used to abort
every other URL in the same run), `D4` (the `User-Agent` is hardcoded to `rget/0.1.0`
while the crate is 1.0.0), and `D5` (the progress bar has no transfer speed, a poor fit
for an unknown-size download, and no handling for Ctrl+C).

Four of the five (`D1`, `D2`, `D4`, and the speed/spinner half of `D5`) were mechanical,
single-file fixes with one obvious approach, so they went in without a COA table per
`4iteration.md`'s "only when the approach isn't obvious" rule. Ctrl+C handling was the one
genuine design point: it needed a new dependency and touches the one loop every download
path streams through.

---

#### Options Considered

**Option A: `ctrlc` crate + one shared atomic flag checked in `stream::copy`**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Small, well-audited crate (used by ripgrep, fd) <br> • One flag checked in the single loop every download path already streams through (single-connection, segmented, `-j`) covers all of them without per-path duplication <br> • The handler runs on its own thread (per the crate's own contract), so printing and setting the flag from it is safe |
| **Disadvantages** | • One new dependency |
| **Implementation Difficulty** | Small |
| **Fit with Constraints** | Best |

**Option B: Raw `libc`/`signal-hook` signal handling**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • No mid-level crate abstraction to trust |
| **Disadvantages** | • Async-signal-safety pitfalls to get right by hand <br> • `signal-hook` is a comparably sized dependency anyway, so there is no real saving |
| **Implementation Difficulty** | Medium |
| **Fit with Constraints** | Worse for the same result |

**Option C: No handler; ship only the progress-bar polish**

| Aspect | Assessment |
|--------|------------|
| **Advantages** | • Zero new dependency, zero design risk |
| **Disadvantages** | • Leaves `D5` half-done <br> • Ctrl+C still just kills the process outright, with no "resume with -c" hint and no guarantee the terminal is left in a clean state |
| **Implementation Difficulty** | Trivial |
| **Fit with Constraints** | Poor — does not answer what `D5` actually asks for |

---

#### Decision & Rationale

**Chosen Option:** A, picked by the user from the table above.

**How it works:**
- **`D1`:** the `limit_rate = 1048576` line `--init` writes is now commented out (`# limit_rate = 1048576  # bytes per second; unset means no limit`), matching how every other optional key in the generated file is already presented.
- **`D2`:** `Config::load` now takes `quiet: bool` and reports a read or parse failure on stderr (`⚠️  Could not parse …`) before falling back to defaults, instead of swallowing the error. The call site passes `args.quiet` directly, since a broken config file cannot itself supply that flag.
- **`D3`:** the per-URL loop in `run()` now matches on `validate_url_with`'s result instead of using `?`; a failure is reported and the loop `continue`s, so the rest of the batch still runs. `main` no longer relies on the standard library's default `Result`-returning-`main` behaviour (which prints `Error: {:?}`); it matches `rget::app::run()` itself and prints the error with `{}` (`Display`, which `thiserror`'s `#[error(...)]` already derives per variant), returning `ExitCode::FAILURE`. `AppError`'s manual `Debug` impl stays, since `std::error::Error` still requires it, but its doc comment now says why.
- **`D4`:** a `DEFAULT_USER_AGENT` constant (`concat!("rget/", env!("CARGO_PKG_VERSION"))`) replaces the literal `"rget/0.1.0"`.
- **`D5` (progress):** the bar's template gained `{binary_bytes_per_sec}`, and an unknown-size download now builds a `ProgressStyle::default_spinner()` with its own template (`{spinner} [{elapsed}] {bytes} ({rate})`) instead of reusing the determinate-bar template, which had a `{bar}` with no length and an `{eta}` that could never be computed.
- **`D5` (Ctrl+C):** `shared::interrupt` holds one `AtomicBool`. `install()` (called once, from `run()`) registers a `ctrlc` handler that sets the flag and prints the interrupt notice the first time it fires. `stream::copy` — the one loop every download path (single-connection, every segment, every `-j` worker) streams bytes through — checks the flag once per chunk and returns the new `download::Error::Interrupted`, which is not retried (`retry::is_retryable`) and is not printed a second time by the retry loop or by the segmented per-part error path, since the handler already announced it once. `run()` reports exit code `130` (the conventional code for a process that stopped on `SIGINT`) when the flag is set, ahead of the ordinary `error_count > 0` check.
- Whatever was already written to `name.part` (or `name.part<i>` for segments) and its `.part.meta` sidecar are untouched by an interrupt: it is exactly the same "stopped partway through" state a stalled connection or a killed process already leaves, and `-c` already knows how to continue from it.

**Trade-offs accepted:**
- The interrupt flag is process-wide, so a second Ctrl+C before the first is noticed does nothing new — the process can only unblock a stalled read once its own read times out (bounded by `-t`, default 30s). This was accepted as a reasonable bound rather than added complexity to interrupt a blocking read directly.
- No unit test flips the real `INTERRUPTED` flag: `cargo test` runs every test in one process, and `stream::copy`'s own tests share that process, so setting the flag true in a test would race any `stream::copy` call running concurrently in another test thread. The flag's read/write mechanics are trivial enough (one `AtomicBool`) that this was judged not worth the flakiness risk; `is_retryable(&Error::Interrupted)` is covered instead.

---

#### Implementation Notes

> - `shared/interrupt.rs` (new): `install()`, `requested()`, backed by one `static AtomicBool`.
> - `features/download/error.rs`: new `Error::Interrupted` variant (`#[error("Interrupted")]`).
> - `features/download/stream.rs`: `interrupt::requested()` checked at the top of the copy loop.
> - `features/download/retry.rs`: `Interrupted` added to the non-retryable arm and to the "already announced, don't print again" exclusion alongside `RangesUnsupported`.
> - `features/download/segmented.rs`: a part returning `Interrupted` is recorded as `first_error` without its own `❌ Part N: …` line.
> - `app/run.rs`: `interrupt::install()` at the top of `run()`; the `run_pool` callback skips the per-URL `❌` line for `Interrupted`; exit code `130` when `interrupt::requested()`, checked before the ordinary `error_count` exit.
> - `Cargo.toml`: `ctrlc = "3.4"`.

---

#### References

- `ROADMAP.md`, items D1–D5
- `$SUITE/4iteration.md` (COA-table rule), `$SUITE/2engineering.md` (no silent failures, no comments for "what")

---

#### Review / Update Log

| Date | Update | Author |
|------|--------|--------|
| 2026-09-25 | Initial entry | deltaog-117 |
