# WS8bm2 picker diagnosis and browser continuation

**Coherent PR-ready partial slice; not owner-blocked-only.** Starting point was accepted `8e52b4291`. Five coherent implementation commits were pushed in order:

| Commit | Result |
| --- | --- |
| `1e9c0e3bc2ce34daa315627544e2448479fff632` | Composer readiness and deterministic picker-race regression |
| `cd0b71d8b77da34b36056a8784018fd7e03310e3` | All 45 browser behaviors, including the remaining 17 |
| `2b07fda12b9a193d7239e3db4d1f109527418c44` | Signed/expanded years and numeric offsets; Rails parser/HTTP differentials |
| `da8f44af273523d58e2cc1c071c223d9f2d5f21d` | Registered periodic poll closing through a real WebSocket |
| `352903747ababcd92e3adc298cb0f5884376fc05` | Fresh-checkout browser mountpoint initialization |

The fresh-clone workspace suite passed **3,688 tests, zero failures, 12 existing ignores**. All **28 Rails oracles** replay byte identically. Strict workspace/all-target clippy, locked metadata and the exact release-input build pass. Final browser results: **Rails 45/45, Rust 44/45**. The final Files timing observation remains open and was not retried. The deterministic picker regression passes on both apps at the unchanged 2,000 ms limit.

No main merge was performed this turn. The accepted main integration, #179 viewer-zone timestamps/cache keys and actual owner APIs are retained. No Rails source, application assets, dependencies, timing thresholds or test concurrency changed. No stash, rebase, force push, deployment or PR was used. The final report commit changes documentation only.

## Picker root cause and failing-first evidence

The original failure waited for the `/poll` suggestion before Enter; it never reached the poll dispatcher. Both apps render the same autocomplete textarea/controller/action wiring and serve the same pinned `markdown_autocomplete_controller-b12fb838.js`. The shared driver previously waited for cable subscriptions, then used atomic Playwright `fill`. Cable readiness does not imply that lazily loaded Stimulus input controllers have connected. Focus/input can occur first. Autocomplete's `connect` installs handlers for the already-focused input but does not search the already-filled value, leaving no autocomplete request and no suggestion.

The deterministic regression holds that module until the cable barrier completes, fills using the old ordering, then releases the module. Diagnostic traces on **both apps** show focus/input with `controller:false`, identical textarea attributes, subsequent module completion and no autocomplete request. Both fail at exactly 2,000 ms. This demonstrates a shared harness setup race under the same conditions. No Rust-specific wiring/rendering difference was found in this path. The historical one-off was not instrumented; its exact event trace is not claimed to have been captured.

`runtime.mjs` now waits under its existing two-second setup bound for composer, Markdown editor and autocomplete to connect before the first input. `picker-readiness.mjs` preserves the deterministic module gate and `--legacy-setup` negative variant. No application module or picker deadline changed. The negative variants were shown failing before the readiness fix; both variants were also rerun against final fresh-clone source and the final binary/reference.

Final regression commands, from `.scratch/picker/fresh`:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior picker-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json --legacy-setup > .scratch/ws8bm2/picker-legacy-rust.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior picker-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json --legacy-setup > .scratch/ws8bm2/picker-legacy-rails.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior picker-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json > .scratch/ws8bm2/picker-fixed-rust.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior picker-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json > .scratch/ws8bm2/picker-fixed-rails.log 2>&1
```

Negatives exit 1 intentionally; positives exit 0. Raw summaries:

```text
WS8bm2 browser picker readiness (Rust): 0/1 passed; 1 failed
WS8bm2 browser picker readiness (Rails): 0/1 passed; 1 failed
WS8bm2 browser picker readiness (Rust): 1/1 passed; 0 failed
WS8bm2 browser picker readiness (Rails): 1/1 passed; 0 failed
```

Identical negative failure line:

```text
locator.waitFor: Timeout 2000ms exceeded.
```

## Changes by file, design and owner integration

- `parity/behavior/runtime.mjs`: composer readiness, unchanged 15-second cable barrier, optional read-only SQLite observation, user/zone/instant options. Existing two/ten-second assertions retain their bounds.
- `parity/behavior/slash.mjs`: all 26 cases, adding agent listing, insertion without execution, immediate no-argument execution, registration after page load, ephemeral dispatch with stored arguments and persisted custom status. Submissions use the real HTTP dispatchers.
- `parity/behavior/{polls.mjs,pins.mjs,pins-dst.mjs}`: all four poll and seven pin/save cases. HTTP fixture creates run callbacks. Fresh JZ-authored copies of source text keep live pin notes in the latest timeline window; old fixtures/closed polls use anchors. Saved completion checks row status, avoiding already-present filter navigation text. Unpin checks visible badge semantics, including hidden wrappers. Separate New York DST clocks prove tomorrow at 09:00 persists November 2 at 14:00 UTC.
- `parity/behavior/{fixtures.rb,fixture-server.mjs,fixture-client.mjs,register-live.rb}`: valid Rails model fixture setup on isolated storage; explicit Ruby clock even with `docker exec`. The Unix-only fixture bridge permits one validated command registration after page load. It creates no app endpoint and does not claim WS11-owned registration REST API parity. Its process-start guard does not change browser assertion limits.
- `parity/bin/behavior`, `parity/behavior/README.md`: pinned offline browser runner and full instructions. Fresh-clone verification showed both browser launches fail before reaching an app because Docker cannot create the ignored `node_modules` tmpfs mountpoint through the read-only checkout. The runner now creates that empty directory; no host npm installation is needed. Negative receipts remain in `browser-mount-before-{rust,rails}.log`; the corrected fresh runner reaches all cases.
- `crates/db/src/slash_commands/{calendar.rs,time_parser.rs}`: preserve signed and zero-expanded years, including named dates; support five/six-digit numeric offsets with seconds. Invalid offset hours/minutes/seconds leave the offset unset, matching Ruby `Date._parse` and Rails' viewer-zone fallback.
- `reference-tools/messaging/date_years.rb`, `vectors/messaging/date_years.json`, `crates/campfire/src/controllers/message_features/date_tests.rs`: 120 actual Rails parser probes across UTC, New York, Berlin and Apia, with fractions, DST edges, BC/year-zero and zero-expanded dates, seconds offsets and invalid offsets. Another 240 actual saved/scheduled HTTP requests compare statuses and **entire raw JSON bodies**. Reference-created fixture rows are committed, and queue assertions use `TestApp::without_job_runner()`.
- `reference-tools/messaging/verify_oracles.py`: new date oracle plus all previous 27; earlier oracle bytes unchanged.
- `crates/campfire/src/controllers/message_features/tests.rs`: explicit frozen-clock before/due/repeated ticks drive the **registered production periodic task**, with a real Axum/cable WebSocket. It receives the closed card once; `closed_at` stays stable, with no form/session values in the frame. No sleep establishes the interleaving. This proves task registration, state, idempotence and delivery, not wall-clock scheduling latency.

All paths above are under `rust/`. Accepted provider callbacks/private endpoints, WS11 human credential/command APIs and WS17 notification/WebPush delivery remain real integrations. Huddle readiness still consumes WS13's real config contract. No production stand-in was introduced or reinstated. Stored Drive fixture rows do not claim external Google network execution.

### Date failing-first

With only the new regression/vector added, calendar/time-parser production files were the baseline `1e9c0e3bc` bytes, also unchanged from accepted `8e52b4291`. It found **52/120 mismatches**: lost year signs, rejected zero-expanded years, seconds-offset handling and invalid-offset fallback. `.scratch/picker/date-years-before.log` retains 52 mismatch rows and this raw summary:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1833 filtered out; finished in 1.06s
```

All 120 parser cases and all 240 full HTTP bodies now match. The fresh feature run below includes both date regressions and periodic/socket proof.

## Fresh-clone verification

Fresh source was created with `git clone --no-hardlinks --no-checkout . .scratch/picker/fresh`, then fetched/checked out the pushed branch. Its default and first-run seeds were built independently using the pinned reference image. Only a Cargo registry download cache was reused; no untracked fixture, source or build target supplied test inputs. Workspace source SHA: `da8f44af273523d58e2cc1c071c223d9f2d5f21d`. Final feature/browser source: `352903747ababcd92e3adc298cb0f5884376fc05`; the intervening commit touches only the browser shell runner/README, leaving Cargo inputs and the release-built production binary identical. Fresh tracked sources stayed clean.

Seed command, fresh clone root:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/seed build default first_run > .scratch/ws8bm2/seed.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Cargo commands use the canonical Rust 1.98.1/media CI image and the existing rustc throttle. The owned `.scratch/picker/ci-env.sh` sets `CI=1`, build jobs 2, container prefix `ws8bm2`, and the fresh scratch path. Its Docker adapter uses four CPUs, preserves `CARGO_INCREMENTAL=0`, `-j4` and four test threads, mounts the existing `/tmp/rust-port-rustc-slots` pool and reads its unchanged count from `/home/riels/.cache/rust-port/rustc-slots`. It assigns the owned 52500–52599 HTTP/cable/mail ranges. No machine throttle setting was changed. This is execution configuration, not an untracked application/test input.

Worktree root, exact executed commands:

```bash
source .scratch/picker/ci-env.sh
bash .scratch/picker/fresh/rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 > .scratch/picker/fresh/.scratch/ws8bm2/workspace-test.log 2>&1
bash .scratch/picker/fresh/rust/ci/cargo.sh test --locked -j4 -p campfire controllers::message_features -- --test-threads=4 --nocapture > .scratch/picker/fresh/.scratch/ws8bm2/features-test.log 2>&1
CARGO_HOME=/src/.scratch/picker/fresh/rust/.cargo-home CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings > .scratch/picker/clippy.log 2>&1
bash .scratch/picker/fresh/rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/picker/fresh/.scratch/ws8bm2/metadata.json 2> .scratch/picker/fresh/.scratch/ws8bm2/metadata.log
```

All exit 0. Metadata emits valid JSON (13 workspace members) and no summary; no lockfile change. Strict clippy covers all targets including html5ever, with no exclusions. All **60 raw workspace summaries**, followed by clippy (ANSI color removed only):

```text
test result: ok. 1833 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 518.37s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.55s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1144 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 146.92s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.58s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.69s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.46s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.52s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.36s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.65s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 16s
```

The 12 ignores are unchanged: three app/manual/gateway/latency cases, one cable capture, four DB Rails-export/differential/rollback cases, one mail export, one live ACME and two kit doctests. No ignore was added.

Raw final fresh feature receipts:

```text
WS8bm2 STI composer HTTP: 5/5 room types preserve Rails reply-control ids and labels
WS8bm2 broader dates: 202/202 Rails coercion/compact-width cases match
WS8bm2 review dates: 88/88 Rails compact/offset/DST cases match
WS8bm2 signed years: 120/120 Rails signed/expanded-year/offset/fraction/DST cases match
WS8bm2 date HTTP: 88 saved timestamps, 88 scheduled timestamps and 88 before/due dispatch pairs match Rails
WS8bm2 signed-year HTTP: 240/240 actual Rails reminder/scheduled response bodies match
WS8bm2 provider callbacks: 3 HTTP edits, 24/24 socket frames and reference sets, 3/3 scoped card endpoints match Rails
WS8bm2 provider edits: 5 HTTP edits, 40/40 real socket replacement frames byte-identical to Rails
WS8bm2 periodic poll runtime: registered task, before/due/idempotent ticks and real socket delivery passed
test result: ok. 166 passed; 0 failed; 0 ignored; 0 measured; 1670 filtered out; finished in 36.40s
```

Release guard, fresh clone root:

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/picker/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/release-inputs.log 2>&1
```

Exit 0; production binaries compile with only allowed release inputs:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 15s
```

All oracles and source checks, fresh clone `rust/`:

```bash
python3 reference-tools/messaging/verify_oracles.py > ../.scratch/ws8bm2/oracles.log 2>&1
python3 reference-tools/messaging/features-reference-check.py > ../.scratch/ws8bm2/reference-check.log 2>&1
```

Both exit 0:

```text
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

### Final browser run

Startup, clock, model fixture and runtime commands are in `parity/behavior/README.md`. Both isolated databases originate from the fresh clone's independently built seed. Rust runs the binary from the fresh-clone release-input build on 52501/52503, Rails the pinned image on 52500. Normal apps/browser are frozen at March 2, 2026; DST uses independent copies at November 1, 2025. Browser images are pinned, external network disabled, same localhost:3999 origin, no screenshots/pixel checks.

Executed fresh-clone normal sequences:

```bash
bash .scratch/ws8bm2/browser-sequence.sh Rust > .scratch/ws8bm2/browser-rust.log 2>&1
bash .scratch/ws8bm2/browser-sequence.sh Rails > .scratch/ws8bm2/browser-rails.log 2>&1
```

The owned script runs `scheduled`, `slash`, `search-files`, `polls`, `pins` through `parity/bin/behavior`, with each app's URL/name and `.seed/default/labels.json`. Slash gets read-only `--database` and isolated `--fixture-storage` paths from the README. Each suite writes `browser-<suite>-<app>.log`. Rails exits 0; Rust exits 1 at Files. Its poll/pin suites then ran **once**, without repeating Files or the failed sequence:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser
for ws8bm2_suite in polls pins; do bash rust/parity/bin/behavior "$ws8bm2_suite" --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json > ".scratch/ws8bm2/browser-$ws8bm2_suite-rust.log" 2>&1 || exit "$?"; tail -n 1 ".scratch/ws8bm2/browser-$ws8bm2_suite-rust.log"; done
```

DST commands, fresh clone root; both exit 0:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52502 --name Rust --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/.scratch/browser-dst-rust/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rust.log 2>&1
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52505 --name Rails --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/rust/parity/.seed/.instances/52505/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rails.log 2>&1
```

Raw final per-file browser summaries:

```text
WS8bm2 browser scheduled (Rails): 4/4 passed; 0 failed
WS8bm2 browser slash (Rails): 26/26 passed; 0 failed
WS8bm2 browser search-files (Rails): 4/4 passed; 0 failed
WS8bm2 browser polls (Rails): 4/4 passed; 0 failed
WS8bm2 browser pins/saves (Rails): 6/6 passed; 0 failed
WS8bm2 browser pins/saves DST (Rails): 1/1 passed; 0 failed
WS8bm2 browser scheduled (Rust): 4/4 passed; 0 failed
WS8bm2 browser slash (Rust): 26/26 passed; 0 failed
WS8bm2 browser search-files (Rust): 3/4 passed; 1 failed
WS8bm2 browser polls (Rust): 4/4 passed; 0 failed
WS8bm2 browser pins/saves (Rust): 6/6 passed; 0 failed
WS8bm2 browser pins/saves DST (Rust): 1/1 passed; 0 failed
```

**Retained Rust Files failure:** after filename search, Images expected 1 upload, received 4, at the unchanged 10,000 ms limit. The filename assertion can already match the pre-submit upload, possibly allowing an old Images link to be clicked before Turbo's search response. This is a hypothesis from the driver/template, not an instrumented root-cause finding. Rails passed the final run; matching Rails failure is not claimed. Earlier development runs passed Files on both, but **44/45 remains the authoritative final result**. No retry, longer timeout, reduced concurrency or weaker assertion was applied. Full failure: `.scratch/picker/fresh/.scratch/ws8bm2/browser-search-files-rust.log`.

## Inventories and grouped pass counts

These are named behavior ports, not claims that Rails test files ran against Rust. All 140 inventoried controller behaviors remain represented; Rails independently supplies the replayed oracles. No controller behavior was newly deferred.

| Rails controller file beneath `test/controllers/` | Covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16 / 16 |
| `messages/pins_controller_test.rb` | 6 / 6 |
| `rooms/pins_controller_test.rb` | 3 / 3 |
| `saved_items_controller_test.rb` | 12 / 12 |
| `scheduled_messages_controller_test.rb` | 19 / 19 |
| `searches_controller_test.rb` | 36 / 36 |
| `rooms/slash_commands_controller_test.rb` | 10 / 10 |
| `autocompletable/icons_controller_test.rb` | 6 / 6 |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 |
| `autocompletable/users_controller_test.rb` | 5 / 5 |
| `rooms/message_links_controller_test.rb` | 12 / 12 |
| `rooms/files_controller_test.rb` | 8 / 8 |
| **Total named behavior ports** | **140 / 140** |

| Rails system file under `test/system/` | Implemented / total | Final Rails | Final Rust |
| --- | --- | --- | --- |
| `polls_test.rb` | 4 / 4 | 4 pass | 4 pass |
| `pins_saved_test.rb` | 7 / 7 | 7 pass | 7 pass |
| `slash_commands_test.rb` | 26 / 26 | 26 pass | 26 pass |
| `search_files_test.rb` | 4 / 4 | 4 pass | 3 pass, 1 failure |
| `scheduled_messages_test.rb` | 4 / 4 | 4 pass | 4 pass |
| **Total** | **45 / 45** | **45 pass** | **44 pass, 1 failure** |

No WS8bm2 browser case remains unimplemented. The one final failure remains open. Passing feature tests grouped by module in the workspace run: slash 37; scheduled 26; saved 18; links/files 17; poll/pin/runtime 16; date 11; quote integration 9; provider 8; reminder 7; root cache 6; composer 4; panel 4; user coercion 3: **166**. The separate feature summary confirms that count. Search model/query tests and other app tests run in the full suite.

## Precisely remaining and owner dependencies

1. **Owned:** investigate the final Files search/Images observation. The 17 previously missing browser cases pass on both apps.
2. **Owned coercion/date:** exceptional grammar, years outside Jiff's ±9999 range, further expanded ISO shapes; structured pager/link/option/file parameters and unprobed lookup/user shapes. New signed-year/offset evidence does not establish universal Ruby parser/coercion parity. Existing builder, compact/offset/DST and HTTP/dispatch samples stay green.
3. **Owned provider/query proof:** more populated complete search/older-window samples, uncached callback combinations, constant-query poll/pin-list measurements. Accepted actual callbacks/private endpoints pass. Registered poll-close socket delivery is now proven; wall-clock scheduling latency is not claimed.
4. **WS12 #187:** completed board/work pane code requires its reviewed merge/integration into this accepted snapshot. Remaining agent work/board-post/handoff/presence services are still listed incomplete in the owner report. Consume those services when available. WS12 itself still has unblocked work; this is a consumer dependency, not a claim that all its work is blocked.
5. **WS11-API:** broader REST/MCP integration is absent from this snapshot. Human command auth/invocation is already real; direct model browser registration is fixture setup, not REST registration API proof. The owner report retains WS12-dependent board/work/result/handoff writes and broader API validation. Wire reviewed owner APIs rather than duplicate them here.
6. **WS13/WS14g:** configured public huddle join/show/header adapter and enhanced Google OAuth/Picker/share network execution remain owner integration boundaries in this snapshot. Real huddle readiness and stored Drive/composer/Files behavior are wired. Passing stored-row browser cases do not claim external network execution.
7. **WS17:** production physical reminder transport is already integrated. Deterministic transport adapters are test-only; no physical-send production stand-in remains.

No approval or product decision is pending. This stops at a **coherent PR-ready partial slice with owned work remaining**, explicitly **not owner-blocked-only**.

## Cleanup

Owned app servers, Rails/browser containers and parity forwarders are stopped. No listener remains in the six exercised HTTP/cable ports. The extra fresh-clone Cargo target was measured at **4.6G** and deleted after verification; ordinary `rust/target`, registry caches, seeds and raw logs remain. The release guard removed its temporary input tree. Machine rustc throttle settings stayed unchanged; no other worker's files/processes or the Python model server were touched. Scratch receipts remain under `.scratch/picker/`, mainly `.scratch/picker/fresh/.scratch/ws8bm2/`. The tracked report and authorized external `wave4/ws8bm2-report.md` are byte-identical.
