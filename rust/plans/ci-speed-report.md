# Rust CI wall time

Goal: a pull request's slowest required Rust check in about 10 minutes, the full matrix
(push to main, nightly, dispatch) in about 15, with every test still running in the same place.

## Where the time went (before)

Measured on run 37350631485 (dispatch, full scope, 74.5 min) and recent PR runs
(43–70 min). Hosted runners have 4 vCPUs; larger runners aren't available to this repo.

| Job / step | Time |
| --- | --- |
| `Rust port` (the only PR job) | 43–70 min |
| ↳ setup: delete preinstalled SDKs | 50 s (up to 130 s) |
| ↳ setup: parity prepare, image load, seed validation | ~51 s |
| ↳ setup: toolchain image, Cargo cache restore (2.4 GB) | ~70 s |
| ↳ clippy / production binaries | 109 s / 93 s |
| ↳ compile every test harness (`verify-ignored.sh`) | 307 s, ~4 min of it the `campfire` test target |
| ↳ `campfire` tests | **3117 s** |
| ↳ `campfire_db` tests + doctests / other workspace tests | 164 s / 111 s |
| `Rust correctness (messaging)` | 74 min: app build 3m42s and test-host build 6m39s (both `-j2`), **145 paired cases 2920 s** in 108 sequential batches, 53 WS14/WS15 originals 309 s |
| `Rust correctness (browsers)` | 23 min: 16 serial browser tests 681 s (huddle 208 s) |
| `Rust correctness (livekit)` | 16 min |
| database / acme / agents-ui | 6–9 min |

The test time is CPU in dependency code, not the app: a `TestApp` boot spends most of
its ~1 s in PBKDF2/HMAC/SHA (monomorphized into `rails_compat`), bcrypt, SQLite and
regex compilation, all at opt-level 0. Profiling one boot loop:

| Profile | CPU per boot |
| --- | --- |
| opt-level 0 everywhere (before) | ~1.0 s |
| dependencies at opt-level 1 | no compile saving; ~10 % slower at run time than 2 |
| dependencies `"*"` and `rails_compat` at opt-level 2 | **~0.25 s** |

## What changed

1. **CI profile** (`rust/ci/cargo-config.toml`, installed as the CI `CARGO_HOME` config by
   `cargo.sh` and `exec.sh`): registry dependencies and `rails_compat` at opt-level 2. The
   app crates stay at opt-level 0, so their per-PR compile is unchanged; debug assertions
   and overflow checks stay on everywhere. Developer builds don't read it. `cargo.sh` now
   mounts the repository at its host path, like `exec.sh`, so both containers share one
   target directory's fingerprints.
2. **`Rust port` split into parallel jobs**, with `Rust port` kept as the aggregator that
   fails unless every one succeeded (failed, skipped and cancelled all fail it):
   - `Rust seeds`: the parity prepare/image/build/validate path and its negative tests,
     unchanged, once per run. Other jobs restore that exact seed cache entry.
   - `Rust tests (K/12)`: `nextest run --workspace --partition slice:K/12`. Shard 1 also
     runs `verify-ignored.sh` against the harnesses it already compiled.
   - `Rust clippy, binaries and doctests`.
3. **Correctness shards** behind a new `Rust correctness` aggregator, which also checks the
   receipts add up (`rust/ci/correctness_gate.py`):
   - browsers: 4 shards, split by each ignored test's recorded seconds (the four ledger
     tests main added meanwhile take 85–653 s each).
   - messaging behaviour: 16 shards of whole case batches (`behavior-check.py --shard`),
     each writing the cases it ran; `--verify-receipts` requires all 145 exactly once.
     The app server and the paused-jobs test host are built once by two
     `Rust messaging host` jobs with `behavior-check.py`'s own commands, then shared.
   - messaging originals: 2 shards of the 53 WS14/WS15 declarations.
   - database, acme, livekit, agents-ui unchanged.
4. **Waste removed**: the SDK deletion runs in the background; the Cargo caches hold only
   registry dependencies (`rust/ci/prune-target.py`: workspace crates rebuild after every
   checkout anyway), split into a `tests` and a `build` cache.

## Levers rejected

- **nextest archive** (build once, run shards from the archive): every shard's setup would
  wait behind one build job, so the critical path is longer than each shard compiling.
- **opt-level 1 dependencies**: no compile saving over 2, slower tests.
- **Restoring source mtimes** so cached workspace crates count as fresh: risks running stale
  artifacts.
- **sccache**: the app crate, the long compile, changes in every PR, and PRs can't write caches.
- **Larger runners**: not available (API 403).
- **Saving caches from PRs**: against the cache-poisoning policy; only main writes.
- **Retries, longer timeouts, fewer tests**: out of scope by design.
