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
