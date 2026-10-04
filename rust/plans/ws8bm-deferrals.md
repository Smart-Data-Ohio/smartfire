# WS8bm cutover browser deferrals

Branch `rust/ws8bm-deferrals`, from fresh main `78b9b1546`. The system inventory is **135 passed / 0 deferred / 0 blocked**. These are three new causal closures; the other 132 declarations retain their reviewed attribution. No Rust product code, Rails assets, vectors, masks, assertions or original deadlines changed.

## Causal diagnoses and closure bar

| Declaration and pin | Reproduced cause | Fix and discrimination |
| --- | --- | --- |
| Release click, `message_interactions_test.rb:55-86` | The translation used a **content viewport** of 390×844 and CDP touch. Holding the real actions response for 250 ms reproduced `DIV != menu` on **both** hosts: the cold menu starts near y351 while the unchanged message center is y307.34. | Execute the literal pinned body and helpers with Selenium's **window** resize and W3C touch actions, on both positive and negative hosts. The resulting content viewport is 390×701; the cold menu starts at y210.31 and covers the same y307.34 point, even with that response hold. The 700 ms hold, exact center hit, `menu` equality, menu-visible checks and hidden-all composer predicate are unchanged. Removing suppression delivers a trusted compatibility click at the press point, activates the menu and fails the original menu assertion at line 60. Attribution requires the broken guard, menu-visible click, original press point and native failure line; startup, line-80 geometry and unrelated failures earn no credit. |
| Scroll retention, `motion_test.rb:177-238` | Layout Cable connectivity does not establish lazy sidebar readiness. Opening during frame load lets `syncCurrentRoom` focus a link before the queued enter-focus callback. The harness blurs that first focus and sets smooth `scrollTop=400`; the second focus interrupts it. Holding the actual frame response 250 ms and delaying the existing enter-focus callback 400 ms reproduces the original five-second `drawer scroll lands` failure on both hosts. The callback arrives at offset **327 on Rails / 345 on Rust**, and neither reaches 400. | Wait for real `#user_sidebar[complete]`, its scroller and its synchronized current-room identity **before opening**, within the unchanged two-second default setup budget. With identical scheduling stress, first focus now follows frame completion, the scroll reaches 400, close retains 400 and reopen focuses a visible control at 400. The default `display:none` fault still fails specifically at the original **closed** offset assertion, with the destroyed boxes observed. |
| Reopen focus, `motion_test.rb:260-292` | Without that barrier, a 1,250 ms frame-response hold opens the drawer without its current link. The queued first-open callback runs too early; late frame synchronization invokes the reopen-only fault during initial setup. Both initial-focus failures are **invalid**, not intended negative rejections. | The same real frame/current-link readiness signal precedes opening. Original initial focus, five-second settled-scroll/current-in-view polling, Escape, reopen, two-second current focus and exact unchanged offset remain. Under frame/focus scheduling stress, the default fault now rejects on **both** hosts specifically at `reopenedCurrentFocus`; it cannot borrow initial-focus failure. |

The 15 real Scroll rooms, HQ, three original Designers messages and all IDs remain unchanged. Motion uses the original single signed-in browser; the previous unused second viewer is removed. Every complete positive pair compares persisted message/thread/user/room identities for motion, and exact message/history/reaction/thread rows for release. No message write is expected from these read-only original cases.

`behavior-browser-setup.mjs` records frame, focus and scroll order. `WS8BM_SETUP_DELAY` holds real response bytes; `WS8BM_OPEN_FOCUS_DELAY` delays the existing opening callback once without replacing its logic. These optional causal scheduling probes are **not acceptance evidence by themselves**, do not change a deadline, and are off during stock repetitions. Diagnostics cannot satisfy assertions. Native/outer teardown remains unconditional, including when readback diagnostics fail.

The inventory verifier now requires a reasoned closure note, independent fixtures, zero automatic retries, at least ten Rust successes, a Rails positive and paired **intended** negative rejections for these three formerly disputed entries. Its regressions reject missing notes/proofs, nine Rust passes, failed or retried runs, and invalid companion negatives. Ordinary reason-only deferrals remain valid and cannot claim closure evidence.

## Commands and receipts

Commands below ran from the worktree. All browser invocations use:

```sh
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1
export CARGO_TARGET_DIR="$PWD/rust/target"
export WS8BM_BROWSER_PORT_BASE=22020 WS8BM_DISCRIMINATION_RETRIES=1
```

The runner builds its seeds, binary and browser inputs. No previous target or untracked fixture is required. Raw logs live in `.scratch/ws8bm-deferrals/`.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --repeat 10 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --repeat 10 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view' --repeat 10 --keep-going
```

The successful independent-fixture batches are `release-ten.log`, `scroll-ten-final.log`, and `reopen-ten.log`, respectively. Each has the raw line:

```text
WS8bm behaviour repetition: 10 paired attempts; 1 named declaration; 0 failed
```

Thus **each fixed original case has at least 10/10 complete Rust passes**, with ten complete Rails pairs and real saved-row checks. Reopen's first batch spans removal of the unused viewer; its sidebar barrier and every original case predicate were unchanged throughout.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --negative --keep-going
WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400 python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --negative --keep-going
WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400 python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view' --negative --keep-going
```

`release-negative-final.log`, `scroll-negative-final.log`, `reopen-negative-final.log` each report:

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

The environment sets the attempt limit to one; **no automatic retry ran**. There are three valid paired intended rejections, not three generic assertion failures.

The causal before controls temporarily removed only the new `readySidebar` call, restoring it in `finally`. With `WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400`, the original scroll predicate fails on both hosts (`scroll-focus-before.log`); the same command with the barrier passes both (`scroll-focus-after.log`). With a 1,250 ms real response hold, the pre-barrier reopen negative fails both initial-focus assertions (`reopen-causal-before.log`) and gets zero rejection credit. The translated release control with a 250 ms real metadata hold fails both center-hit equalities (`release-delay-before.log`); the native unchanged-body control with that same hold passes (`release-native-delay.log`). No server response contents were rewritten in these latency controls.

Retained experimental failures are not overwritten: the initial scroll repetition reports **8 complete pairs / 2 failures**, and an additional reopen repetition reports **9 complete pairs / 1 failure**. All three are Rails `ERR_NETWORK_CHANGED` import/startup failures at the harness's pre-case composer startup check, before the HQ case; Rust completes its case. They are not asserted to be product differences, are not retry credit, and do not replace the complete passing receipts above. The initial native negative also remains recorded as invalid: the original matcher expected a later synthetic click and refused the real compatibility-click failure at line 60. The corrected native attribution requires that real mutated state and exact original failure line.

## Local helper verification

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
python3 -m unittest discover -s rust/reference-tools/messaging -p '*test.py'
node --test --test-concurrency=4 rust/reference-tools/messaging/*.test.mjs
```

```text
WS8bm system inventory: 135 named declarations; 135 mapped behaviour passes; 0 deferred; 0 WS12 blocked; no pixel checks
Ran 41 tests in 2.624s
OK
ℹ tests 69
ℹ pass 69
ℹ fail 0
ℹ skipped 0
```

## Fresh-clone verification

A shared-object clone at `.scratch/ws8bm-deferrals/fresh` checked out committed source `69ea6c0a6`, with a clean tracked tree. It built its own `default`, `first_run` and `agents_ui` seeds and installed its browser dependencies. Compiler output used the single worktree `rust/target` cache; the clone had no scratch target or copied fixtures.

The fresh-clone helper commands above report:

```text
WS8bm system inventory: 135 named declarations; 135 mapped behaviour passes; 0 deferred; 0 WS12 blocked; no pixel checks
Ran 41 tests in 6.934s
OK
ℹ tests 69
ℹ pass 69
ℹ fail 0
ℹ skipped 0
```

The additional fresh-clone browser pass selected exactly these three declarations from `motion` and `message_interactions`, excluding their registered sibling cases. Its raw results are:

```text
WS8bm behaviour check: 2 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 3 served mutants rejected on Rails and Rust across 3 named checks; 0 invalid or escaped
```

The failed positive pair is reopen focus: **Rails fails before the case**, while loading Designers at `behavior.mjs:115`. Stylesheets and JavaScript modules return `net::ERR_NETWORK_CHANGED`; the recorded page has `stimulus: false`, and the unchanged 15-second composer startup check times out. Rust completes the original reopen sequence. This failure is retained in `fresh-positive.log`, earns no closure credit, and was not retried. The subsequent fresh negatives all reach their intended assertions on both hosts (`fresh-negative.log`). The ten complete passing pairs per declaration remain the closure receipts above; this extra run is not represented as green.

## Workspace preparation and checks

The first workspace attempt incorrectly rebuilt seeds directly from the base pinned image. It exits 101, with **2,842 passed / 2,106 failed / 22 ignored** across the outer targets. Campfire's raw summary is `test result: FAILED. 731 passed; 2106 failed; 13 ignored; 0 measured; 0 filtered out; finished in 360.97s`. The failures report `missing migrations 20261003180000`; three subprocess-wrapper failures also print nested libtest summaries, which are not three additional cases. The seed lacks `active_storage_blobs.message_processing_token` and `message_processing_expires_at`. This is a recorded preparation error, not a Rust difference or a green run (`fresh-workspace.log`).

The browser image uses `browser.Dockerfile`'s current-schema overlay, matching `parity/bin/ci-seed`. Before rebuilding, its `/rails/db/schema.rb` hash was checked against the fresh checkout: both are `68f02627974758d3944e968fa61d4112986db229c938bc82fbfbedd6eaa7fc88`. The immutable image `sha256:e527f7b2399480d8f657a06ac8ae29cd0280c7670aaef55c83ea54580f156dd8` was tagged `ws8bm-deferrals-reference-d7c7de92` for these commands. Its Rails application remains pinned; the required migration and both columns are now present in all three newly built seeds.

From the fresh clone's `rust/`:

```sh
export PARITY_IMAGE=ws8bm-deferrals-reference-d7c7de92
parity/bin/seed build default first_run agents_ui
parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb default
parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb first_run
parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb agents_ui
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1
mise exec rust@1.98.1 -- cargo test --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=4
mise exec rust@1.98.1 -- cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

`CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=4`, `CI=1` and the worktree's absolute `CARGO_TARGET_DIR` apply throughout. `CAMPFIRE_REFERENCE` points at the fresh checkout. Nextest is not installed on this host; libtest runs each target with at most four test threads. The machine-wide rustc wrapper remains unchanged. Tests use `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=.scratch/ws8bm-deferrals/canonical-runner.sh` (absolute worktree path), which selects the reference-extracted media binaries and libraries from `rust/target/ws8bm-media` for the child executable. The host environment and libraries are unchanged.

The media runtime was freshly generated from the worktree root, using the tracked extraction tool:

```sh
env PARITY_IMAGE=triage-reference-d7c7de92 PARITY_NAMESPACE=ws8bm-deferrals PARITY_OWNER=ws8bm WS8BR2_MEDIA_DIR="$PWD/rust/target/ws8bm-media" bash rust/reference-tools/users/media_runtime.sh
```

The temporary runner is execution configuration, not a fixture input; its complete contents are:

```sh
#!/usr/bin/env bash
set -euo pipefail
owner_root=$(git -C "$(dirname -- "$0")" rev-parse --show-toplevel)
media="$owner_root/rust/target/ws8bm-media"
export LD_LIBRARY_PATH="$media/native-libs"
export PATH="$media/usr/bin:$PATH"
exec "$@"
```

Raw media versions:

```text
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
libvips 8.16.1
```

Rails validator summary fields, in seed order:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
  "passed": 40,
  "failed": 0
```

Metadata exits 0. Strict clippy and the release-input-only binary build both exit 0, with raw terminal lines:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 53s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 32s
```

Corrected full workspace run (`fresh-workspace-final.log`, exit 0):

```text
WS8bm fresh workspace: 4948 passed; 0 failed; 22 ignored; exit 0
```

Every raw libtest target summary from that successful run:

```text
test result: ok. 2837 passed; 0 failed; 13 ignored; 0 measured; 0 filtered out; finished in 1050.37s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.23s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1379 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 74.63s
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.42s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.34s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s
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
```

## Final resource audit

The generated fresh clone (420,033,585 bytes) was removed after verification. No target was created under scratch; compiler output and the canonical media runtime remain in the single allowed `rust/target`. Pre-existing scratch history was preserved. The final ownership-scoped audit checked test processes, browser working directories, ports 22020/22021/22022/52023, and containers whose mounts belong to this worktree. Protected model processes and the pre-existing Codex CUA service are outside that ownership set and were not stopped.

```text
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "created_big_outputs": []}
```
