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

