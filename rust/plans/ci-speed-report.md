# Rust CI wall time

Goal: a pull request's slowest required Rust check in about 10 minutes, the full matrix
(push to main, nightly, dispatch) in about 15, with every test still running in the same place.

## Result

| | Before | After |
| --- | --- | --- |
| Pull request (`Rust port`) | 43–70 min | **8.0 min** (slowest shard 7.8 min) |
| Full matrix | 69–84 min (77 min on this base) | **22 min** |

The pull-request target is met. The full matrix is not: its floor is one test (below).

Before: main push [37379298592](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37379298592)
on this branch's base (76.9 min: `Rust port` 71 min, messaging 77 min, browsers 64 min),
dispatch 37350631485 (74.5 min), main push 37360757177 (84.2 min), and PR runs
37350629501 (43.5 min) and 37362400997 (62.6 min). After: see the pull request for the
measured runs on its head.

## Where the time went (before)

Hosted runners have 4 vCPUs; larger runners aren't available to this repository.

| Job / step | Time |
| --- | --- |
| `Rust port` (the only PR job) | 43–71 min |
| ↳ setup: delete preinstalled SDKs | 50 s (up to 130 s) |
| ↳ setup: parity prepare, image load, seed validation | ~51 s |
| ↳ setup: toolchain image, Cargo cache restore (2.4 GB) | ~70 s |
| ↳ clippy / production binaries | 109 s / 93 s |
| ↳ compile every test harness (`verify-ignored.sh`) | 307 s, ~4 min of it the `campfire` test target |
| ↳ `campfire` tests | **3108 s** |
| ↳ `campfire_db` tests + doctests / other workspace tests | 152 s / 111 s |
| `Rust correctness (messaging)` | 77 min: app build 3m42s and test-host build 6m39s (both `-j2`), **145 paired cases ~2900 s** in 108 sequential batches, 53 WS14/WS15 originals ~310 s |
| `Rust correctness (browsers)` | 64 min: 20 serial browser tests, four of them the new ledger tests (85–771 s each) |
| `Rust correctness (livekit)` | 20 min |
| database / acme / agents-ui | 9–12 min |

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
   - browsers: 4 shards, split by each ignored test's recorded `seconds`. Each shard
     compiles the server binary and campfire's test harnesses under the toolchain image
     while its prerequisite image builds; `correctness.sh`'s builds then find them fresh.
   - messaging behaviour: 16 shards of whole case batches (`behavior-check.py --shard`),
     each writing the cases it ran; `--verify-receipts` requires all 145 exactly once.
     The app server and the paused-jobs test host are built once by two
     `Rust messaging host` jobs with `behavior-check.py`'s own commands, and the `app`
     job exports the prerequisite image the shards load.
   - messaging originals: 2 shards of the 53 WS14/WS15 declarations.
   - database, acme, livekit, agents-ui unchanged.
4. **Waste removed**: the SDK deletion runs in the background; the Cargo caches hold only
   registry dependencies (`rust/ci/prune-target.py`: workspace crates rebuild after every
   checkout anyway), split into a `tests` and a `build` cache.
5. **One test fix** (outside CI, see below): the stage join browser test now holds the
   fake LiveKit server's validate request as well as its socket.

## Test counts

Twelve test shards report 441 × 7 + 440 × 5 = 5287 passed nextest tests and 85 distinct
ignored ones, plus 1 passed and 2 ignored doctests: 5288 passed, 87 ignored, exactly the
base run's 1453 + 3077 + 757 nextest tests (4 + 78 + 3 skipped) and its doctests. The
same `verify-ignored.sh` guard lines appear (78 correctness tests, 7 utilities; 8; 1).
`Rust correctness` requires the browser shards' receipts to hold exactly the 20 registered
browser tests, the originals' exactly the 53 declarations, and the behaviour shards' all
145 cases once.

## What blocks the 15-minute full matrix

The critical path is browsers shard 1/4, which holds a single test,
`ledger_browser_tests::original_ledger_navigation_assertions` (631–771 s across runs),
after setup (~80 s) and a serial compile of the `campfire` server binary and test
harness (~7 min, overlapping the prerequisite image). One test can't be split across
shards without changing it, so the full matrix stays around 20–22 min until that test or
the `campfire` compile gets faster.

## The stage join test

`huddle_system_cases_in_real_browser` failed in three of four full runs on this branch
(never on main): "join stage dispatches huddle:join and toggles while connected" saw two
`huddle:join` events. An instrumented run showed why. The test holds the fake LiveKit
socket so the panel stays "connecting" under a synthetic connected event, but when the
socket fails livekit-client fetches `/rtc/v1/validate`, and once that answers the panel
reports "failed". With optimized dependencies the Rust server returns the credentials and
`livekit-client` ~50 ms after the synthetic event, so "failed" landed between the
`Leave stage` assertion and the click (3 of 4 instrumented runs), and the click joined
again. The test now holds the validate request too, as its comment intends. The
assertions are unchanged.

## Flaky downloads

Two runs each lost one messaging behaviour shard to `curl: (16) Error in the HTTP2
framing layer` while fetching pinned Debian packages for the prerequisite image, when
sixteen shards fetched them at once. The behaviour shards now load that image from the
`app` host job instead, so a run builds it 11 times rather than 27 (6 before). Those two
failures were re-run, not worked around.

## Levers rejected

- **nextest archive** (build once, run shards from the archive): every shard would wait
  behind one build job, so the critical path is longer than each shard compiling.
- **opt-level 1 dependencies**: no compile saving over 2, slower tests.
- **Restoring source mtimes** so cached workspace crates count as fresh: risks running stale
  artifacts.
- **sccache**: the app crate, the long compile, changes in every PR, and PRs can't write caches.
- **Larger runners**: not available (API 403).
- **Saving caches from PRs**: against the cache-poisoning policy; only main writes.
- **A toolchain image tarball cache**: ~20 s per job over the Buildx cache.
- **Building the server binary beside the test harness** in a second target directory:
  ~2 min off the browser shards, but it needs a second copy of the dependency cache and
  a prebuilt-binary path through `correctness.sh`.
- **Retries, longer timeouts, fewer tests**: out of scope by design.
