# End-to-end run

Runs the real `rget` binary against a throw-away local HTTP server (`server.py`, which
answers `Range` requests so the segmented and resume paths are exercised for real).

```sh
cargo build --release
scripts/e2e/run.sh                 # or: scripts/e2e/run.sh path/to/rget
```

It needs `python3`, `sha256sum` and `cmp`, listens on `127.0.0.1` only, and works in a
temporary directory it removes afterwards. Exit status is `0` when every case passes.

This is a Tier 3 check: it is not part of `scripts/check`. Run it before a release.
