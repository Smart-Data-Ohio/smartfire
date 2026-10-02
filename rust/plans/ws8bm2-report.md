# WS8bm2 Files search/Images correction

**Requested Files slice complete and PR-ready. Stopping as requested.** Fix source is pushed `46b07672dfb460fc4235d8ed70003033d1b86e46`, based on accepted `d6d38afb23804a16eb858ca3088d9a377c50b36c`. The final report commit changes documentation only. Fresh-clone results: **Rails 45/45 and Rust 45/45 browser cases; 3,688 workspace tests passed, zero failures, 12 existing ignores; 28/28 Rails oracles byte-identical; strict all-target workspace clippy clean; locked metadata and release-input build pass.** No remaining browser failure or unimplemented WS8bm2 browser case remains.

The broader workstream remains partial: further coercion/provider/query proofs and owner integrations remain listed below. This is not an owner-blocked-only claim. No main merge, stash, rebase, force push, production deployment or PR was performed.

## Root cause and deterministic failing-first evidence

This was a shared browser-driver defect, not a Rust filename-filter defect in the sampled path. The old sequence submitted filename search, then asserted that `system-cover.png` was visible. That upload was already visible before submission. The assertion therefore did not establish that Turbo rendered the search response. The following Images click could use the old link, which had no `filename` query parameter. The app correctly returned all images instead of the expected one.

The committed `files-readiness` regression fetches and holds **the actual app's search response**, without changing its bytes. Both apps return HTTP 200, one upload and an Images link carrying the filename. While that response is held, the old filename assertion succeeds and the old DOM's Images link has no filename. With the legacy sequence, both browsers send an Images request without the filename and fail the unchanged ten-second count assertion. Initial clean-seed reproductions on both show **expected 1, received 4**, matching the observed Rust failure.

With the new shared helper still containing only the original three search steps, the stronger regression also failed on both apps because search completed against the pre-submit DOM (`true !== false`). After the fix, the helper stays pending while the response is held. Releasing the response renders the filtered link, the Images request carries the exact filename and the one-row assertion passes. There is no sleep, timing retry, synthetic response, modified app asset or timeout change. The independent app responses and matching Rails failure demonstrate the shared harness cause; no Rust controller/template change was required.

Initial failing-first commands, worktree root (both modes shown failing before the helper correction):

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json --legacy-setup > .scratch/files/legacy-rust.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json --legacy-setup > .scratch/files/legacy-rails.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json > .scratch/files/before-rust.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json > .scratch/files/before-rails.log 2>&1
```

All four exit 1 intentionally. Initial legacy traces, identical on both apps:

```text
TRACE search response: status=200 uploads=1 Images carries filename=true
TRACE search held: old filename visible=true Images carries filename=false
TRACE Images request: filename present=false
Expected: 1
Received: 4
Timeout:  10000ms
WS8bm2 browser Files search readiness (Rust): 0/1 passed; 1 failed
WS8bm2 browser Files search readiness (Rails): 0/1 passed; 1 failed
```

The ordinary regression's before-fix failure on both is `search must not complete against the pre-submit DOM`, with `true !== false`. The first authored readiness check assumed query-parameter insertion order; both apps actually sort filename before type. That intermediate assertion error is retained in `.scratch/files/intermediate-order-{rust,rails}.log`; the final helper checks the exact filename parameter independently of key order. This was a test-authoring correction, not an app change or a timing retry.

## Changes by file

- `rust/parity/behavior/file-search.mjs`: shared `searchUploads` uses the original input/submission/filename assertion and waits for the rendered Images link to carry the submitted filename. It uses the existing ten-second assertion limit, a query-parameter boundary and regex-escaped encoding. Images count and Videos-empty assertions remain intact.
- `rust/parity/behavior/search-files.mjs`: the Files case calls that shared helper, so the regression exercises its actual readiness logic.
- `rust/parity/behavior/files-readiness.mjs`: valid real HTTP attachment fixture, unique non-secret filename, actual response gate, one-row/filtered-link inspection, request capture, pending-state check and final Images count. `--legacy-setup` retains the exact old sequence as an explicit negative control. The gate is event-driven and forwards the app response unchanged.
- `rust/parity/bin/behavior`: exposes the regression through the existing pinned offline browser runner.
- `rust/parity/behavior/README.md`: documents the readiness condition and both gate modes.

No production Rust/Rails code, model validation, callback, golden bytes, allowlist, mask, asset, dependency, viewer-zone/cache behavior, test concurrency or timing threshold changed. Accepted WS11/WS13/WS17 APIs stay integrated. No other worker's production files were edited.

## Final browser verification from a fresh clone

Source was cloned at the fix with `git clone --no-hardlinks . .scratch/files/fresh`. Both seeds were built independently there; no earlier app database was copied into the final run. The owned normal Cargo target/registry caches were reused as build caches, not test fixtures. Browser assertions run the fresh clone's committed scripts against isolated app storage copies. Production crate/asset bytes are unchanged from the accepted branch; Rust serves its actual canonical-toolchain binary. Both apps use the same pinned Playwright image, localhost:3999 origin, frozen clock and viewport, with external browser network disabled. No screenshots or pixel checks.

From `.scratch/files/fresh`:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash rust/parity/bin/seed build default first_run > .scratch/ws8bm2/seed.log 2>&1
bash .scratch/ws8bm2/browser-sequence.sh Rails > .scratch/ws8bm2/browser-rails.log 2>&1
bash .scratch/ws8bm2/browser-sequence.sh Rust > .scratch/ws8bm2/browser-rust.log 2>&1
```

All exit 0. Startup and valid Rails model fixture commands are documented in `parity/behavior/README.md`. The owned sequence script executes `scheduled`, `slash`, `search-files`, `polls`, `pins` once each against freshly prepared apps, using common seed labels. Normal Rust uses 52501/52503, Rails 52500, both at March 2, 2026. Slash observes the isolated DB read-only and uses the one-operation direct-model fixture bridge, not a fake production registration endpoint. Seed builder raw lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Separate fresh DST app copies and browser clocks use November 1, 2025. Rust 52502/52504, Rails 52505. From the fresh clone root:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser
bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52502 --name Rust --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/.scratch/browser-dst-rust/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rust.log 2>&1
bash rust/parity/bin/behavior pins-dst --target http://127.0.0.1:52505 --name Rails --labels .seed/default/labels.json --instant 2025-11-01T16:00:00Z --database /work/rust/parity/.seed/.instances/52505/db/production.sqlite3 > .scratch/ws8bm2/browser-dst-rails.log 2>&1
```

Both exit 0. The total lines below are computed by summing the six raw suite logs and asserting their expected case counts (4+26+4+4+6+1=45), not by counting the additional readiness regression. All cases ran; none was retried or skipped. Raw per-file and aggregate lines:

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

The committed negative control and positive regression were also rerun from the fresh clone:

```bash
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json --legacy-setup > .scratch/ws8bm2/legacy-current-rust.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json --legacy-setup > .scratch/ws8bm2/legacy-current-rails.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52501 --name Rust --labels .seed/default/labels.json > .scratch/ws8bm2/readiness-rust.log 2>&1
bash rust/parity/bin/behavior files-readiness --target http://127.0.0.1:52500 --name Rails --labels .seed/default/labels.json > .scratch/ws8bm2/readiness-rails.log 2>&1
```

Negatives exit 1 intentionally, positives 0. After the canonical cases populated more images, both negative controls receive six rows instead of one, with the same missing-filename request. Positive traces and raw summaries:

```text
TRACE search response: status=200 uploads=1 Images carries filename=true
TRACE search held: old filename visible=true Images carries filename=false
TRACE Images request: filename present=true
WS8bm2 browser Files search readiness (Rust): 0/1 passed; 1 failed
WS8bm2 browser Files search readiness (Rails): 0/1 passed; 1 failed
WS8bm2 browser Files search readiness (Rust): 1/1 passed; 0 failed
WS8bm2 browser Files search readiness (Rails): 1/1 passed; 0 failed
```

## Fresh workspace, clippy, metadata and reference receipts

All commands below ran on committed fix source `46b07672`, with independently built default/first_run seeds and CI enabled, so no seed-dependent test silently skipped. The existing owned `.scratch/files/ci-env.sh` configures the canonical Rust 1.98.1/media image, the ws8bm2 container prefix, fresh disk scratch, ordinary owned `/native-target` build cache and unchanged rustc slot pool. Build jobs remain 2; Cargo `-j4` and four test threads are unchanged (below the user's eight-thread limit). No machine-wide throttle setting was edited.

**Retained verification orchestration error:** the first workspace run's 3,688 tests passed, but its asset doctest compile failed because the release-input guard ran concurrently using that same target. It replaced generated `include_bytes!` paths with `/src/.scratch/release-inputs.VxBNlK/...`, then removed that temporary tree while rustdoc was still using the generated file. This was my build/check scheduling error, not an app/test timing defect. `.scratch/files/fresh/.scratch/ws8bm2/workspace-test.log` retains the initial command's exit 101 and `error: doctest failed, to rerun pass -p campfire_assets --doc`. After all builds finished, the affected doctest was rerun successfully, then the entire workspace was rerun serially. No timing test failed, no test threshold/concurrency was changed and no failure was hidden.

Worktree root, exact final test/repair/check commands:

```bash
source .scratch/files/ci-env.sh
CARGO_TARGET_DIR=/native-target bash .scratch/files/fresh/rust/ci/cargo.sh test --locked -j4 -p campfire_assets --doc -- --test-threads=4 > .scratch/files/fresh/.scratch/ws8bm2/assets-doctest.log 2>&1
CARGO_TARGET_DIR=/native-target bash .scratch/files/fresh/rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 > .scratch/files/fresh/.scratch/ws8bm2/workspace-final.log 2>&1
CARGO_TARGET_DIR=/native-target bash .scratch/files/fresh/rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings > .scratch/files/fresh/.scratch/ws8bm2/clippy.log 2>&1
CARGO_TARGET_DIR=/native-target bash .scratch/files/fresh/rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/files/fresh/.scratch/ws8bm2/final-metadata.json 2> .scratch/files/fresh/.scratch/ws8bm2/final-metadata.log
```

All four exit 0. Clippy covers every target with no exclusions; metadata is valid JSON with 13 workspace members, no lockfile change and no textual summary. The final workspace invocation has **all 60 raw summary lines**, followed by strict clippy (ANSI color removed only):

```text
test result: ok. 1833 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 794.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.61s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1144 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 276.32s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.72s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.06s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 8.36s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.49s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.45s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.05s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 18s
```

The affected doctest's raw summary:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Existing 12 ignores remain unchanged (three app/manual/gateway/latency, one cable capture, four DB reference/export/rollback, one mail export, one live ACME and two kit doctests). No ignore was added. All 166 existing owned feature tests pass within the full workspace run; no new Rust tests were added by this browser-only fix.

Release guard, fresh clone root:

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/files/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/release-inputs.log 2>&1
```

Exit 0; raw line:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
```

Reference commands, fresh clone `rust/`:

```bash
python3 reference-tools/messaging/verify_oracles.py > ../.scratch/ws8bm2/oracles.log 2>&1
python3 reference-tools/messaging/features-reference-check.py > ../.scratch/ws8bm2/reference-check.log 2>&1
```

Both exit 0. Raw summaries:

```text
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

## Inventory, remaining scope and cleanup

| Rails system file under `test/system/` | Final Rails | Final Rust |
| --- | --- | --- |
| `polls_test.rb` | 4 / 4 | 4 / 4 |
| `pins_saved_test.rb` | 7 / 7 | 7 / 7 |
| `slash_commands_test.rb` | 26 / 26 | 26 / 26 |
| `search_files_test.rb` | 4 / 4 | 4 / 4 |
| `scheduled_messages_test.rb` | 4 / 4 | 4 / 4 |
| **Total** | **45 / 45** | **45 / 45** |

All 140 inventoried controller behavior ports remain represented by the unchanged Rust tests and replayed Rails oracles. These are behavior ports, not a claim that Rails test files execute against Rust. No controller or browser case is newly deferred. The original 24 Rails oracles plus four accepted continuation fixtures all replay byte identically.

**Outside this requested stopping slice:** broader exceptional date/coercion/lookup matrices; additional populated search/older-window/provider callback permutations and poll/pin query-count measurement remain owned work. WS12 #187 and WS11-API consumer integration remain flagged owner dependencies, along with the accepted configured public huddle (WS13) and enhanced Google/Picker/share network (WS14g) boundaries. Human agent command auth/invocation, real huddle config readiness and WS17 physical notification transport remain integrated; no new production stand-in was added. The direct-model browser registration fixture still does not claim WS11 REST registration API parity. No new product decision is required.

All owned browser app servers, Rails/browser containers and parity forwarders are stopped. Any small test-created fresh-clone target directory is measured and deleted after verification; the ordinary owned Rust build cache, registry cache, isolated seeds and raw receipts remain. No other worker's files/processes or the Python model server were touched. Raw logs are under `.scratch/files/`, mostly `.scratch/files/fresh/.scratch/ws8bm2/`. The tracked report and authorized external `wave4/ws8bm2-report.md` are byte-identical. **Stopping here for the lead's PR, as requested.**
