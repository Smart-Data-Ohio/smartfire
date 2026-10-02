# WS8bm2 main merge verification

**Requested merge slice complete and PR-ready. Stopping as requested.** Both source parents are retained: accepted feature head `3d0dcac092a3ccc6d1994a51d01be14cf1c5e8e8` and fetched main `3ab3a4db58e094cea0f41b90e63f54d59ce9309e` (121 newly incorporated main commits). Source merge `a672fdcaa46f97916c5f4e712fb6d338ec379851` was checked out in an independent fresh clone for verification. The final merge adds this report only; production/test source bytes remain those verified in the fresh clone. The final pushed merge SHA is in the delivery reply.

Fresh-clone results: **3933 workspace tests passed, zero failed, 12 existing ignores; strict workspace/all-target clippy passed; locked metadata and release-input build passed; 28/28 Rails oracles replay byte identically; Rails 45/45 and Rust 45/45 browser cases.** No seed-dependent test silently skipped. The full workspace includes vendored html5ever and doctests. No ignore, mask, allowlist, timing threshold, test-thread count, or automatic retry was added or changed.

## Merge resolution and cross-workstream boundaries

The sole Git conflict was `rust/crates/views/src/messages.rs`, where the feature branch's `composite_fragment_key`/`cached_composite_fragment` helper block met main's corrected boost documentation. The resolution retains both helpers and main's documentation. It preserves main's Rails collection template digest, uncached HTML boost rendering, and the complete reactions replacement partial from the reviewed rendering work (#182/#184). No side of the conflict was discarded.

The automatic presenter/composer merges were inspected as well: main's expanded rendering dependencies, Markdown plain-text behavior and provider fetch intent remain, together with feature-side composer facts/deserialization and STI-specific room parameter keys. Polls, pins, saved/reminders, scheduled sends, search preloads/operators/cursors, slash/autocomplete/play, files/links, provider cards/callbacks and the accepted viewer-zone behavior remain. No new domain or rendering behavior was invented for this merge. The existing browser readiness fixes are retained.

No new failing-first test was necessary for this merge-only change. The earlier accepted picker/Files negative controls remain committed; their previous evidence is archived in the parent report rather than claimed as rerun here. The requested current 45-case browser suites and full existing seeded regression suite were rerun.

## Build environment and exact commands

All commands below were executed this turn. Source checkout is `.scratch/merge-main/fresh`, created with `git clone --no-hardlinks . .scratch/merge-main/fresh` after the source merge. Only an owned Cargo registry cache was copied in; default and first-run seeds were built independently using the pinned Rails image. The ordinary owned `rust/target` cache was mounted as `/native-target`; it supplies build artifacts, not fixtures or pre-existing test databases. The checkout has no untracked source/test input beyond explicitly generated seeds and scratch output.

The owned `.scratch/merge-main/ci-env.sh` adapter sets `CI=1`, `CARGO_BUILD_JOBS=2`, `RUST_CI_IMAGE=campfire-toolchain-ci-rust-speedups`, the `ws8bm2` container prefix, and a disk-backed `RUNNER_TEMP`. Its Docker function keeps the existing machine-wide rustc throttle/slot configuration, mounts the owned target as `/native-target`, limits each Cargo container to four CPUs, and selects this worker's cable/integration/mail port ranges. Explicit Cargo `-j4` and four test threads remain unchanged. There are no release-profile builds. The release-input guard, clippy and full workspace commands ran sequentially against that target, avoiding overlapping generated assets.

Post-merge locked metadata, worktree root:

```bash
source .scratch/merge-main/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/merge-main/metadata.json 2> .scratch/merge-main/metadata.log
```

Exit 0; Cargo.lock unchanged. The same check was then executed from the fresh clone via its script:

```bash
source .scratch/merge-main/ci-env.sh
CARGO_TARGET_DIR=/native-target bash .scratch/merge-main/fresh/rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/merge-main/fresh/.scratch/ws8bm2/metadata.json 2> .scratch/merge-main/fresh/.scratch/ws8bm2/metadata.log
```

Exit 0, valid JSON with 13 workspace members. Cargo metadata emits JSON and has no textual result summary.

Release-input guard, fresh clone root (environment already sourced):

```bash
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/release-inputs.log 2>&1
```

Exit 0; raw summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 06s
```

Strict clippy and full workspace, worktree root:

```bash
source .scratch/merge-main/ci-env.sh
CARGO_TARGET_DIR=/native-target bash .scratch/merge-main/fresh/rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings > .scratch/merge-main/fresh/.scratch/ws8bm2/clippy.log 2>&1
CARGO_TARGET_DIR=/native-target bash .scratch/merge-main/fresh/rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 > .scratch/merge-main/fresh/.scratch/ws8bm2/workspace.log 2>&1
```

Both exit 0. Clippy raw summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 38s
```

The provider-capacity interruption detached the host Docker log client while the workspace container continued the same Cargo process. The original partial client receipt remains `workspace.log`; the complete receipt was recovered from that exact running container, with its actual exit status captured independently. No test was restarted, skipped or retried. Recovery commands, worktree root (each held in its own foreground session):

```bash
docker logs --follow ws8bm2-cargo-2961742 > .scratch/merge-main/fresh/.scratch/ws8bm2/workspace-complete.log 2>&1
docker wait ws8bm2-cargo-2961742 > .scratch/merge-main/fresh/.scratch/ws8bm2/workspace-exit.txt
```

Both recovery clients exit 0; `workspace-exit.txt` contains `0`, the Cargo container's exit status. The full raw summaries below are extracted from the recovered complete receipt, not the interrupted partial receipt.

All 60 raw workspace unit/integration/doctest summaries (3933 passed, zero failed, 12 existing ignores; 166 owned feature tests pass):

```text
test result: ok. 2022 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1074.92s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.95s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1195 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 223.93s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.78s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.18s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.84s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Reference setup, fresh clone root:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92
bash rust/parity/bin/seed build default first_run > .scratch/ws8bm2/seeds.log 2>&1
```

Exit 0; both seeds generated by Rails:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Oracle commands, fresh clone `rust/`:

```bash
python3 reference-tools/messaging/verify_oracles.py > ../.scratch/ws8bm2/oracles.log 2>&1
python3 reference-tools/messaging/features-reference-check.py > ../.scratch/ws8bm2/reference-check.log 2>&1
```

Both exit 0; raw summaries:

```text
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

## Browser verification

Fresh clone scripts drive isolated Rails and Rust copies of the rebuilt default seed. Rust serves the actual binary built by the release-input check. Both apps use the pinned offline Playwright image, the same origin, viewport and frozen clock. Tests use real HTTP, Turbo and cable behavior and read-only database proof where specified. No pixel checks.

Setup, fresh clone root:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser
bash rust/parity/bin/reference up --seed default --port 52500 --time 2026-03-02T16:00:00Z --freeze > .scratch/ws8bm2/rails-start.log 2>&1
bash rust/parity/bin/reference runner --port 52500 rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json > .scratch/ws8bm2/fixtures-rails.log 2>&1
cp -a rust/parity/.seed/default .scratch/browser-rust
bash rust/parity/bin/reference runner --storage .scratch/browser-rust --time 2026-03-02T16:00:00Z --freeze rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json > .scratch/ws8bm2/fixtures-rust.log 2>&1
set -a
source rust/parity/.env.reference
set +a
CAMPFIRE_STORAGE_PATH="$PWD/.scratch/browser-rust" CAMPFIRE_FILES_PATH="$PWD/.scratch/browser-rust/storage" CAMPFIRE_FROZEN_TIME=2026-03-02T16:00:00Z HTTP_PORT=52501 TARGET_PORT=52503 /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/rust/target/debug/campfire > .scratch/ws8bm2/rust-server.log 2>&1
```

The long-running binary was kept in its own foreground session. Fixtures exit 0 and report `WS8bm2 browser fixtures: 2 validated agent commands; 1 expired poll; agent membership granted` on each app. The Rails launcher starts a background forwarder; this execution environment terminated that child after the launching command returned. The first scheduled Rails group therefore failed HTTP setup (`502 !== 200`, 0/4), before exercising any browser interaction. That receipt is retained as `invalid-forwarder-scheduled-rails.log`, excluded from the valid-suite aggregate. Rails itself stayed healthy. A foreground session fixed the missing transport:

```bash
ws8bm2_ip=$(docker inspect ws8bm2-reference-52500 --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')
python3 rust/parity/bin/forward-port 52500 "$ws8bm2_ip" > .scratch/ws8bm2/rails-forwarder.log 2>&1
```

With that setup fixed, the five ordinary groups ran once per app; no failed interaction/timing case was retried. The owned `browser-sequence.sh` receipt executes `scheduled`, `slash`, `search-files`, `polls`, `pins` in order, passing `--database` and `--fixture-storage` only to slash. Rust database/storage are `/work/.scratch/browser-rust/db/production.sqlite3` and `.scratch/browser-rust`; Rails uses `/work/rust/parity/.seed/.instances/52500/db/production.sqlite3` and `rust/parity/.seed/.instances/52500`. Both use `--labels .seed/default/labels.json`, target port 52501/52500 and name Rust/Rails.

```bash
bash .scratch/ws8bm2/browser-sequence.sh Rust > .scratch/ws8bm2/browser-rust.log 2>&1
bash .scratch/ws8bm2/browser-sequence.sh Rails > .scratch/ws8bm2/browser-rails.log 2>&1
```

Both valid sequences exit 0. The DST case has separate fresh storage and servers at `2025-11-01T16:00:00Z`, Rust HTTP/target ports 52502/52504 and Rails port 52505. Its model fixtures explicitly receive that instant as the second Ruby argument (the runner's frozen process clock alone is insufficient because fixtures.rb calls travel_to). The Rails forwarder for 52505 was kept in a foreground session too. DST setup/case commands, fresh clone root:

```bash
cp -a rust/parity/.seed/default .scratch/browser-dst-rust
bash rust/parity/bin/reference runner --storage .scratch/browser-dst-rust --time 2025-11-01T16:00:00Z --freeze rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json 2025-11-01T16:00:00Z > .scratch/ws8bm2/fixtures-dst-rust.log 2>&1
bash rust/parity/bin/reference up --seed default --port 52505 --time 2025-11-01T16:00:00Z --freeze > .scratch/ws8bm2/rails-dst-start.log 2>&1
bash rust/parity/bin/reference runner --port 52505 rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json 2025-11-01T16:00:00Z > .scratch/ws8bm2/fixtures-dst-rails.log 2>&1
CAMPFIRE_STORAGE_PATH="$PWD/.scratch/browser-dst-rust" CAMPFIRE_FILES_PATH="$PWD/.scratch/browser-dst-rust/storage" CAMPFIRE_FROZEN_TIME=2025-11-01T16:00:00Z HTTP_PORT=52502 TARGET_PORT=52504 /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/rust/target/debug/campfire > .scratch/ws8bm2/rust-dst-server.log 2>&1
ws8bm2_ip=$(docker inspect ws8bm2-reference-52505 --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')
python3 rust/parity/bin/forward-port 52505 "$ws8bm2_ip" > .scratch/ws8bm2/rails-dst-forwarder.log 2>&1
bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52502 --name Rust --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/.scratch/browser-dst-rust/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rust.log 2>&1
bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52505 --name Rails --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/rust/parity/.seed/.instances/52505/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rails.log 2>&1
```

Both DST runs exit 0; readonly DB assertions prove tomorrow 09:00 New York persists as `2025-11-02T14:00:00Z`. Raw group summaries below; the two `browser total` lines are transparent sums of the six groups (4+26+4+4+6+1), not extra test cases:

```text
WS8bm2 browser scheduled (Rails): 4/4 passed; 0 failed
WS8bm2 browser slash (Rails): 26/26 passed; 0 failed
WS8bm2 browser search-files (Rails): 4/4 passed; 0 failed
WS8bm2 browser polls (Rails): 4/4 passed; 0 failed
WS8bm2 browser pins/saves (Rails): 6/6 passed; 0 failed
WS8bm2 browser pins/saves DST (Rails): 1/1 passed; 0 failed
WS8bm2 browser total (Rails): 45/45 passed; 0 failed
WS8bm2 browser scheduled (Rust): 4/4 passed; 0 failed
WS8bm2 browser slash (Rust): 26/26 passed; 0 failed
WS8bm2 browser search-files (Rust): 4/4 passed; 0 failed
WS8bm2 browser polls (Rust): 4/4 passed; 0 failed
WS8bm2 browser pins/saves (Rust): 6/6 passed; 0 failed
WS8bm2 browser pins/saves DST (Rust): 1/1 passed; 0 failed
WS8bm2 browser total (Rust): 45/45 passed; 0 failed
```

| Rails system file under `test/system/` | Rails | Rust |
| --- | --- | --- |
| `polls_test.rb` | 4/4 | 4/4 |
| `pins_saved_test.rb` | 7/7 | 7/7 |
| `slash_commands_test.rb` | 26/26 | 26/26 |
| `search_files_test.rb` | 4/4 | 4/4 |
| `scheduled_messages_test.rb` | 4/4 | 4/4 |
| **Total** | **45/45** | **45/45** |

## Remaining scope and cleanup

Nothing remains in this requested main-merge verification slice. All 140 inventoried controller behavior ports remain represented by the feature tests and Rails oracles; this is behavior-port coverage, not a claim that Rails test files execute directly against Rust. No browser case remains failed or deferred.

The broader workstream remains partial, as in the accepted parent report: exceptional date/coercion/lookup matrices, additional populated search/older-window/provider callback permutations and poll/pin query-count measurement remain owned work. WS12 #187 and WS11-API consumer integration remain flagged owner dependencies; configured public huddle and enhanced Google/Picker/share network boundaries retain their prior owners. The accepted real human agent command invocation, huddle readiness and WS17 physical transport integrations are retained. This is not an owner-blocked-only completion claim. No new standalone seam was added.

All owned app servers, browser/reference containers, fixture processes and forwarders were stopped before the full workspace runtime tests. After the suite, any test-created fresh-clone target directory was measured and deleted; the ordinary owned build cache, independent seeds and raw receipts remain. The rustc throttle and model server were never changed or bypassed. No other worker's files/processes were touched, and no stash, rebase, force push, PR or production deployment was performed. Raw receipts are in `.scratch/merge-main/fresh/.scratch/ws8bm2/`, with merge and initial metadata receipts under `.scratch/merge-main/`. The tracked and authorized external `wave4/ws8bm2-report.md` reports are identical. **Stopping for the lead's PR.**
