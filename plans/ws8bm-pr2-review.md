# WS8bm -2: continuation PR and phase-attribution fixes

Branch `rust/ws8bm-messages-http-2`. Main `2fd002490865e8f5ec04466cd4ea526e8eeb8f08` (#189) merged cleanly in `45ea50271` with a merge commit; no stash or test weakening. Main later advanced to `24255712fe336f6173c79c4fc1695167432f6af2` (#218), merged cleanly in checkpoint `f16c1ed89`. The full positive set uses `f16c1ed89`; the full discrimination set and final helper/Rust checks use `2e8508818`, which changes only attribution/tests, with identical positive assertions, fixtures and product source to that checkpoint. Verification uses the checkpoint in the same cold, no-hardlinks clone under the authorized `.scratch/ws8bm-pr2-review/fresh`. That clone started at the first merge with no target or seed; the generated target is removed after verification. Rails pin remains `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift.

## What the PR adds beyond main

- Eight paired controller flows attribute the ten formerly WS12-blocked thread/work declarations: authenticated/unauthenticated reads, permissions, ownership, work status, validation, stale updates and preservation of the discussion. Direct Rails model differentials and two Rust database regressions cover omitted-owner untracking and distinct stale-instance changes/history.
- Sixteen additional system declarations cover work tracking/assignment/activity/guidance, phone thread navigation, anchored/read/unread thread history, live mobile chrome/overflow/navigation, coarse-input field sizing, and narrow/theme-aware thread code. These use original visibility predicates and deadlines, real browser writes and persisted-row checks.
- Held-job upload/provider fixtures use Rails' nonperforming TestAdapter and Rust's `TestApp::without_job_runner()`. The generated cold browser-host copy includes its release/public/provider callback inputs; it needs no pre-existing scratch files or target. Main's newly included agents-UI cast/replay input files are also copied; the cold build caught their omission and a failing-first copier regression covers all four. List/composer shell seams remain documented in `ws8bm-integration.md`.
- Initial-phase rejection attribution and pinned native phone execution close this review's four P3s. Reports retain the seventeen concrete deferrals, distinguish injected browser Turbo duplicate delivery from server-originated redelivery, and keep the release-click case deferred.

The new P3 changes touch reference tools only. The continuation's Rust delta consists of test support and database regression tests; no Rust product behavior, Rails JavaScript, message goldens, masks or deadline changes are included.

## Failing-first attribution and phone audit

The reviewer's actual asset-installer/synthetic-stack reproduction was rerun against the merged baseline. All seven stacks got credit, including four wrong-phase stacks. The initial seven regression checks use the baseline source call sites and fail against it: **3 pass / 4 fail**. These are controlled stack inputs at real source call sites, not a claim of browser failures. The committed fixed tests retain those four wrong-phase inputs at their current source call sites and add a distinct-message state-witness rejection plus a pre-right-click lookup failure. The latter was accepted at `f16c1ed89` and rejected after `2e8508818`; all nine phase tests pass.

| Finding | Before | After | Pinned assertion/phase |
| --- | --- | --- | --- |
| onContextMenu no-op | Shared menu wait accepted later Shift+F10 and long-press failures. | Require both the post-click `await assertMenuOpen(page)` frame and its initial `await openMenu(page)` caller; the unaffected handlers and pre-click lookup fail closed. | `message_interactions_test.rb:18` initial right-click; later keyboard/press checkpoints stay separate. |
| First bold message hidden | Shared text wait accepted failure on the newer plain draft, using an earlier bold body's hidden-state witness. | Target the first submitted-text call and require the hidden witness's normalized text to be `First message stays exact.`; the later draft or a different seeded body cannot supply credit. | `workspace_markdown_test.rb:232,237` first send versus second draft/send. |
| Initial conversation hidden once | Shared conversation locator accepted reopened-conversation failures after the sessionStorage one-shot guard stopped acting. | A named creation-only call wraps the same conversation assertion and original ten-second deadline; a real inline-hidden conversation witness is required. Reopen assertions stay separate. | `composer_test.rb:255,269,281` initial creation versus later room navigation/reopens. |
| Phone Escape focus race | Translated original phone control: 1 paired pass / 2 failed pairs, with Rust failing the two-second post-Escape panel assertion. Reviewer independently traced the same behavior on both apps. | Positive parity runs the pinned body and private/system helpers through pinned Capybara 3.40.0 / Selenium 4.35.0. Only the excluded screenshot is removed. Three consecutive fresh-fixture pairs pass, 25 assertions per app/pair; recorded Escape targets are inside the menu. No diagnostic focus wait earns credit. | `threads_test.rb:293-340`, especially menu assertions :317, Escape :328, panel :329; `system_test_helper.rb:128-135`. |

The native helper extracts both Ruby source files from the exact git pin at runtime. A regression reconstructs the original body byte-for-byte after restoring only its method header and screenshot, and verifies the exact private helper text. The native control uses host Chromium/ChromeDriver 153, with the pinned Ruby gems in the reference image; it is a targeted body execution, not the full Rails system suite. It refuses to reuse an occupied ChromeDriver port, owns and removes a short cache directory for Chrome sockets, and uses the same session, room and production-host test-motion input as the paired runner. Served negative phone controls still execute their translated initial creation checkpoint; they do not get native positive credit.

## Possible shared Rails client bug

`message_actions_controller.js:643` schedules menu focus on requestAnimationFrame. `thread_panel_controller.js:103` exempts Escape only if the event starts inside the menu/dialog/details. If Escape starts on the message before the focus frame arrives, it closes the drawer as well as the menu on **both** apps. Both client files exactly match the pinned Rails sources; no client fix is attempted here.

Reproduction: at a 390 × 844 phone viewport with pinned `data-test-motion=off`, open the thread drawer, create the second thread, right-click its first message, and deliver Escape immediately while the message still owns focus (`menuContainsActive=false`). Observe `#thread-panel[aria-hidden=true]`; the original post-Escape assertion expects false within two seconds. Delivering Escape after menu focus leaves the drawer open. Review evidence, read only: `/home/riels/.cache/rust-port/ws8bmbr/smartfire/.scratch/rereview-ad0fb6aa/phone-summary.json` and `phone-playwright-motion.log`. Own baseline and canonical-native logs are retained in `.scratch/ws8bm-pr2-review/phone-before.log` and `phone-after.log`. The native traces show menu focus preceding Escape on each of the six passing application runs. This is a possible Rails client bug for the lead, not a Rust-only defect or a new credited assertion.

## Commands and raw receipts

All current commands below run from the cold no-hardlinks clone `.scratch/ws8bm-pr2-review/fresh` with `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, `CARGO_TARGET_DIR=$PWD/rust/target`, `CAMPFIRE_REFERENCE=$PWD`, `TMPDIR=$PWD/.scratch`, `PARITY_OWNER=ws8bm`, `PARITY_NAMESPACE=ws8bm-pr2-verify`, `PARITY_CPUS=2`, and `PARITY_IMAGE=triage-reference-d7c7de92`. The browser driver derives its own namespaced image/container names. Commands run sequentially. No pre-existing seed/target/scratch input is required; source-dependent test fixtures are tracked or extracted from the exact git pin. Complete logs stay under `.scratch/ws8bm-pr2-review/`.

### Failing-first and actual served-state controls

Baseline `45ea50271` preserves the merged #189 classifier. The read-only reviewer reproduction executes the actual mutation installer on in-memory source and the actual classifier on seven controlled stacks. Four wrong-phase stacks incorrectly receive rejection credit; its output is `phase-before.log`. The read-only original is `/home/riels/.cache/rust-port/ws8bmbr/smartfire/.scratch/rereview-ad0fb6aa/phase-collision-repro.mjs`; only its imports are relocated to the baseline clone for execution. The initial seven regression checks reproduce those failures (`phase-tests-before.log`, expected exit 1):

```text
ℹ tests 7
ℹ pass 3
ℹ fail 4
```

The corrected suite keeps those cases and adds the unrelated-message witness and pre-right-click lookup failures. A synthetic pre-action source stack is accepted by `f16c1ed89` but refused by `2e8508818` (`menu-pre-action-before-after.log`):

```text
WS8bm menu pre-action attribution before f16c1ed89: true
WS8bm menu pre-action attribution after 2e8508818: false
```

The full browser discrimination invocation below runs the real served variants. Their affected controls fail at the initial right-click, first submitted body and initial conversation assertions, respectively; the logged expected/actual stacks and state witnesses are valid. These raw application receipts are from that full run, not synthetic browser claims:

```text
WS8bm discrimination: workspace_markdown: sending preserves the submitted source and a newer draft: hidden-submitted-body-visible-strong: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: workspace_markdown: sending preserves the submitted source and a newer draft: hidden-submitted-body-visible-strong: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: composer: thread drafts persist per thread without touching the channel draft: hidden-initial-composer-conversation: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: composer: thread drafts persist per thread without touching the channel draft: hidden-initial-composer-conversation: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: default: Rust served mutant REJECTED (TimeoutError)
```

The baseline translated phone control (`--repeat 3`) retains one paired pass and two failed pairs. The unchanged client sequence fails at the two-second post-Escape assertion; reviewer evidence independently shows the same unfocused-Escape failure on Rails. The native pinned sequence passed all three pairs after the harness fix, with 25 assertions per app:

```text
WS8bm behaviour repetition: 1 paired attempts; 1 named declaration; 2 failed
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
```

The merged cold browser-host compile also revealed four missing agent UI test includes. The new copier regression fails before the copier fix (`host-inputs-before.log`) and passes after (`host-inputs-after.log`); the full current Python suite below includes it. The compile failure is retained separately in `full-behaviour-before-host-fix.log` and receives no browser parity credit.

```text
Ran 2 tests in 0.002s
FAILED (errors=1)
Ran 2 tests in 0.002s
OK
```

### Fresh fixtures, helper regressions and inventory

```sh
bash rust/parity/bin/seed build default first_run agents_ui
node --test rust/reference-tools/messaging/behavior-*.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
python3 rust/reference-tools/messaging/controller-case-inventory.py
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
ℹ tests 36
ℹ pass 36
ℹ fail 0
ℹ skipped 0
Ran 15 tests in 0.036s
OK
WS8bm controller inventory: 156 named declarations; 156 scoped attributions; 0 owner-blocked
```

The phase suite contributes nine tests to the 36 helper tests; native-body extraction and source reconstruction are included. Grouped controller counts are 56 root, 19 root Drive, 4 cached CSRF, 2 legacy cache, 17 boosts, 24 threads, 12 thread messages, 13 thread Drive, 7 forwards and 2 forward-source declarations. This is scoped attribution, not a claim that all original Ruby files execute unchanged.

### Full behaviour and discrimination

```sh
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
```

```text
WS8bm behaviour check: 125 named cases passed on Rails and Rust; 3 failed; no pixel checks
WS8bm invalid discrimination attempts: 6; bounded fresh-fixture retries only
WS8bm discrimination check: 185 served mutants rejected on Rails and Rust across 120 named checks; 0 invalid or escaped
```

The three positive failures are all at the unchanged 15-second Stimulus startup gate, before the affected assertions: opaque external headers (Rust), external-thread deep-link refusal (Rails), and picker Custom icon (Rails). Asset requests record `net::ERR_NETWORK_CHANGED`. The raw full summary remains failed even if isolations recover; no deadline or test concurrency is loosened.

The six retained invalid application attempts in the negative run were all Rails: opaque-header startup/network once, attachment filename twice at the earlier preview assertion, attach-options network once, boosting startup/network once, and reply/tombstone startup/network once. Each eventually has a same-attempt Rails/Rust rejection. In particular, the filename variant finally rejects on both apps at its intended filename assertion; its earlier preview failures receive no rejection credit. No escape is discarded by retrying.

## Final isolated controls and native Rust checks

All final controls, main-baseline and cleanup receipts follow.

### Isolated startup cases, pinned phone sequence and hidden-allowed probes

```sh
python3 rust/reference-tools/messaging/behavior-check.py mobile_layout --case 'headers outside the workspace shell stay opaque over scrolled content' --repeat 3 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'rejects an external thread deep link before fetching it' --repeat 3 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar --case 'the picker Custom tab reacts with a workspace icon' --repeat 3 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'keeps the thread drawer usable on a phone and preserves the channel' --repeat 3 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py --mutant-set hidden-scopes --keep-going
```

Raw repetition lines, in command order; all exit 0. Original full-run startup failures remain recorded above.

```text
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

Each of the six merged-head native phone application runs prints the following raw Minitest line. All six recorded Escape events originate inside the focused menu; observing those events does not add an assertion or focus wait.

```text
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
```

### Locked metadata and inventory

Locked `cargo metadata --format-version 1` passes with no stdout; Python's strict `tomllib.loads` of the workspace Cargo manifests rejects duplicate keys and passes. Current inventory is re-counted directly from the tracked JSON:

```sh
mise exec rust@1.98.1 -- cargo metadata --manifest-path rust/Cargo.toml --locked --format-version 1 >/dev/null
```

```text
WS8bm merge metadata: locked metadata PASS; duplicate TOML keys absent
WS8bm system inventory: 118 passed; 17 deferred; 0 owner-blocked; 135 named declarations
```



### Complete fresh-clone workspace, strict clippy and release-input build

```sh
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

The complete workspace command exits 101; every target is run with `--no-fail-fast`. The sole failing target is the app binary, with the six native media cases listed below. Both new DB regressions pass. The external held-job browser host remains explicitly ignored in the ordinary test suite because the browser driver invokes it explicitly; there is no silent missing-seed skip. All three required seed families were generated above. `html5ever` is excluded under the workspace's existing instructions/CI scope.

Every raw libtest summary from `workspace.log` follows:

```text
test result: FAILED. 2649 passed; 6 failed; 8 ignored; 0 measured; 0 filtered out; finished in 491.40s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.45s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1319 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 110.63s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.36s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.97s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
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
error: 1 target failed:
    `-p campfire --bin campfire`
```

Strict clippy and release-input builds both exit 0. Their raw final lines, respectively:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.05s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 35s
```

The release guard compiles the binaries from Cargo.toml, Cargo.lock and crates-only Rust source inputs, with the guard's explicit reference asset inputs. Neither newer Rust change introduces a non-test include outside crates: the database oracle include lives in the test-only module, and the browser host lives in presenter test support.


### Main baseline for the six native media failures

After the current-head checks, the same fresh clone is checked out at unmodified main `24255712fe336f6173c79c4fc1695167432f6af2`. It uses the same toolchain, target cache, seeds and environment. A new main test binary is compiled; the generated compiler-artifact JSON supplies `WS8BM_MAIN_TEST_BINARY`. Only these six exact test filters are executed, with the same eight-thread maximum:

```sh
git switch --detach 24255712fe336f6173c79c4fc1695167432f6af2
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire --bin campfire --no-run --message-format=json
"$WS8BM_MAIN_TEST_BINARY" --exact --test-threads=8 \
  controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers \
  controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files \
  controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy \
  controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect \
  controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy \
  controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect
git switch --detach 2e8508818
```

```text
running 6 tests
test controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect ... FAILED
test controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files ... FAILED
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 2656 filtered out; finished in 0.98s
```

Both branch and unmodified-main runs fail the same six named tests: the logo response bytes, fresh-video preview/variant state, and four missing-video preview/variant response cases. This establishes inheritance in the tested environment; it does not establish a codec/tool version as the root cause. No product, golden, mask, ignore, wait or concurrency changes are made to hide these failures. The full workspace result remains failed. Strict clippy and release-input builds pass independently.

## Cleanup and checkpoint

The sole own scratch target is removed after all checks. Browser listeners on 52020, 52021, 52022 and 52023 are absent. A final `find .scratch -type d -name target -prune` prints no target directories. Raw logs are retained; the Python model server and other workers' resources are untouched.

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml --target-dir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh/rust/target
```

```text
     Removed 28073 files, 34.1GiB total
```

Verification source is `2e8508818` (full positive assertions/product source identical to `f16c1ed89`). The final report commit changes documentation only. The continuation is a coherent PR-ready slice with the full positive startup failures and main-inherited media failures disclosed; seventeen own system declarations stay deferred and none is owner-blocked. No further scope is attempted at this checkpoint.
