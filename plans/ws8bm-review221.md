# WS8bm #221: global events and preserved history identity

Review baseline `160ef0738478091736e0543eaf0d8ea4ec358c9b`; branch `rust/ws8bm-messages-http-2`. Rails remains pinned to `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift. This review changes tools, attribution documentation and the existing stale-instance DB regression. It changes no product implementation, Rails JavaScript, vector, mask, ignore, deadline or test concurrency.

## Changes by file

- `behavior_work_rows.py`, `behavior-work-controllers.mjs` and `behavior-check.py`: global final/per-PATCH event counts and original client-ID lookup/ID equality, using fresh read-only database paths.
- `behavior_work_rows_test.py`: five additional row regressions, including foreign events on refusal and displaced original message identity.
- `work-model-declarations.rb` and `crates/db/src/tests/work_mutations_test.rs`: original global stale-write delta restored; the native test includes a second thread.
- `work-producer-discrimination.py`, `work-producer-mutants.rb` and `work-global-model-discrimination.py`: reproducible real-producer before/after proofs, paired intended-assertion attribution and verified restored controls.
- `work_producer_discrimination_test.py`: four attribution regressions, including startup/companion failure and later-row-marker leakage.
- `ws8bm-controller-cases.md` and reports: corrected assertion scopes and exact audit/verification receipts. No owner-owned product file is edited.

## Fixes and assertion-scope audit

`behavior-work-controllers.mjs` reads synchronous, read-only SQLite snapshots around each real CSRF-bearing PATCH. Every successful work change adds exactly one **global** WorkThreadEvent; refused writes add zero. Separate per-request checks prevent an erroneous earlier delta from being cancelled by a later write. `behavior-check.py` supplies the fresh databases constructed by that invocation. `behavior_work_rows.py` retains the ordered target-thread history checks and now checks the total global delta too.

The history check finds the message through `(thread_id, client_message_id = "work-history")` and requires its ID to equal the originally posted message ID, as Rails :328 does. The original by-ID thread/source assertions remain. This catches both a rewritten client ID and moving the same client ID onto another message.

All ten newly attributed controller declarations were compared with the pinned originals. The additional omission was the global two-event assertion in the stale-instance model declaration; it is restored in both the real Rails runner and the Rust regression, retaining the unchanged JSON oracle and ordered history checks. A separate foreign thread makes the standalone Rust discrimination exercise the global scope.

All lines below are in pinned `test/controllers/channel_threads_controller_test.rb`:

| Declaration | Original scope / identity | Audit disposition |
| --- | --- | --- |
| Convert / assign / preserve discussion | :308 and :315 global +1 for each PATCH; :321–328 ordered target history and original `work-history` identity | Restore per-PATCH global count and exact lookup/ID; retain actor, owner, status, source and target-history assertions. |
| Eligible owner / revoked owner | :336 global zero for refused PATCH; :341 preserved owner; :348–349 exact revoked identity/status | Restore global refusal scope; preserve owner and revoked-user checks. Successful setup/clear writes also have separate global +1 checks. |
| Assigned owner changes status | :360 global +1; :366–372 forbidden reassignment/untracking and unchanged owner/status | Restore global successful delta and require zero on both refused writes; exact owner/status checks retained. |
| Manager removes work tracking | :379 global +1; :383–384 nil owner/status | Restore global delta; retain owner/status checks. |
| Omitted-owner direct-model refusal | :390–395 exception and unchanged status/owner; no global-count assertion in the original | Existing real-model exception and exact state oracle retained. No original identity/count lookup was dropped. |
| Separate stale instances | :404 global +2; :409–410 exact final status and ordered target statuses | **Additional audit fix:** restore global +2 in Rails runner and Rust regression; target history oracle unchanged. Foreign-event producer discrimination fails at this assertion. |
| Eligible agent assignment | :423 assigned agent's `work_assigned` event +1; :428–430 exact bot ID/agent flag/persisted owner | Keep per-agent scope; also snapshot its delta on each PATCH. Supplemental global work-event checks and original owner identity remain. |
| Owner options | :465 exact human name; :467–477 eligible ID/profile and excluded IDs | Existing exact JSON ID/profile checks retained; no global assertion or original-message lookup exists on these lines. |
| Forbidden agent assignment | :488 assigned agent's `work_assigned` event zero; :492–493 forbidden/nil owner | Keep per-agent scope and persisted owner/status; add global zero on the refused PATCH. |
| Ordinary thread JSON | :501–505 exact false/nil fields and **thread-local** empty work history | Keep the explicit thread-local emptiness assertion. Supplemental fixture-wide global zero and unchanged original message checks retained. |

The supplemental common-fixture identity check applies to all eight paired flows; the pinned identity requirement belongs to the conversion declaration. Agent-event counts remain scoped to the specified agent, not widened to unrelated agents. The ordinary-thread empty-history assertion remains thread-local, not falsely presented as a pinned global assertion.

## Failing first and real-producer discrimination

The replay mutates the real Rails and Rust work producers in isolated server/build inputs. It does not replace assertions, fetch responses or persisted fixtures. Each invocation first runs an ordinary paired control and constructs the seed, npm inputs, server image and binaries from tracked source. Readback requires both actual mutated database states. Browser failure attribution stops before later row receipts and requires the intended assertion independently for each application; startup or companion failures cannot earn rejection credit.

Against **unchanged 160ef0738 assertions**, both reviewer mutants escape on both apps:

| Producer | Rails baseline | Rust baseline | Fixed intended assertion on both apps |
| --- | --- | --- | --- |
| `extra-foreign-event` | PASS: global delta 4, target delta 2 | PASS: global delta 4, target delta 2 | First real PATCH fails `work-event-count:`: global delta 2 instead of 1. Readback confirms one target event plus one foreign event; second PATCH is never reached. |
| `rewrite-history-client-id` | PASS despite `corrupted-work-history` | PASS despite `corrupted-work-history` | Original-message lookup fails `work-history identity:`: the expected original ID is not found through `work-history`. Existing by-ID source/thread checks would still pass. |

The three earlier producer faults still reject independently on both apps: omitted work history at the first global event count, permission bypass at the 403 HTTP assertion, and missing agent notifications at the assigned agent's event delta. A supplemental Rust stale-instance foreign-event producer escapes the old target-only assertion and is rejected by the restored global +2 assertion. The replay rebuilds and verifies its restored unmutated control before returning, preventing its mutant test executable from contaminating later controls in the shared target.

The new row-check tests were also run against the old module: 3 pass / 4 fail; the four failures are refused foreign events, extra conversion events, rewritten client ID and displaced original identity. The fixed row and producer-attribution helper checks pass. These helper probes receive no extra declaration credit.

## Commands and raw receipts

Verification runs in the independent no-hardlinks clone at `.scratch/ws8bm-pr2-review/fresh`. Its prior scratch target was deleted at the preceding checkpoint; this review rebuilt it. Positive/discrimination browser assertions use `ebe929687`; the later replay attribution and foreign-thread setup changes do not alter those assertions or product inputs. Canonical app and strict clippy run at `0f887d5a9`; final replay, restored controls, DB regressions and Python helpers run at `d14ee2d16`. Their Rust source is identical: the intervening commit changes only replay tools and their unit tests.

The common environment is `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, an absolute own `CARGO_TARGET_DIR` of that clone's `rust/target`, `CAMPFIRE_REFERENCE` pointing at that clone, `TMPDIR` in its `.scratch`, `PARITY_OWNER=ws8bm`, `PARITY_NAMESPACE=ws8bm-review221`, `PARITY_CPUS=2` and `PARITY_IMAGE=triage-reference-d7c7de92`. Rustc keeps the configured machine-wide throttle.

Baseline tools run from the current worktree with `--root` pointing at that clone checked out at `160ef0738`. No fixed assertion is copied into the baseline. The standalone stale test gets only the same foreign-thread setup used after the fix:

```sh
python3 rust/reference-tools/messaging/work-producer-discrimination.py --root /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh --mutant extra-foreign-event --mutant rewrite-history-client-id --expect-escapes
python3 rust/reference-tools/messaging/work-global-model-discrimination.py --root /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh --expect-escape
```

In the verification clone, with the environment above:

```sh
python3 rust/reference-tools/messaging/behavior-check.py channel_threads_controller --keep-going
python3 rust/reference-tools/messaging/behavior-check.py channel_threads_controller --negative --keep-going
python3 rust/reference-tools/messaging/work-producer-discrimination.py
python3 rust/reference-tools/messaging/work-global-model-discrimination.py
bash rust/parity/bin/seed build default first_run agents_ui
bash rust/parity/bin/reference runner --seed default rust/reference-tools/messaging/work-model-declarations.rb
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db message_controller_ -- --test-threads=8
node --test rust/reference-tools/messaging/behavior-*.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
python3 rust/reference-tools/messaging/controller-case-inventory.py
```

The Rails runner's JSON is compared exactly with unchanged `rust/vectors/messaging/work-model-declarations.json`; its informational stderr is excluded from that comparison. All three seed families are rebuilt.

### Paired cases, discrimination and helper receipts

Baseline commands exit 0 **because escape is the expected failing-first result**. The fixed producer and served-mutant commands exit 0 only when both applications reject at the required assertion. The affected positive command passes all eight paired cases. Raw baseline/fixed summary lines:

```text
WS8bm real producer discrimination: extra-foreign-event: ESCAPED as expected at baseline
WS8bm real producer discrimination: rewrite-history-client-id: ESCAPED as expected at baseline
WS8bm producer discrimination check: 2 paired proofs; 0 invalid or unexpected; baseline escapes expected
```

```text
WS8bm real producer discrimination: missing-history: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: allow-reassignment: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: missing-agent-events: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: extra-foreign-event: REJECTED on Rails and Rust at intended assertion
WS8bm real producer discrimination: rewrite-history-client-id: REJECTED on Rails and Rust at intended assertion
WS8bm producer discrimination check: 5 paired proofs; 0 invalid or unexpected; no escapes accepted
```

```text
WS8bm behaviour check: 8 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm discrimination check: 11 served mutants rejected on Rails and Rust across 8 named checks; 0 invalid or escaped
```

The fixed extra-foreign-event producer fails the first, intended PATCH assertion on each app:

```text
WS8bm positive application FAILED: Rails: converts a thread to work, assigns an eligible owner, and keeps an audit trail: AssertionError [ERR_ASSERTION]: work-event-count: converts a thread to work, assigns an eligible owner, and keeps an audit trail: PATCH {"work_status":"planned"}
WS8bm positive application FAILED: Rust: converts a thread to work, assigns an eligible owner, and keeps an audit trail: AssertionError [ERR_ASSERTION]: work-event-count: converts a thread to work, assigns an eligible owner, and keeps an audit trail: PATCH {"work_status":"planned"}
```

The additional stale-instance audit probe prints the mutant result followed by its restored control. Baseline (old assertion accepts the foreign events):

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1322 filtered out; finished in 0.08s
WS8bm restored model control:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1322 filtered out; finished in 0.08s
WS8bm global model discrimination: foreign-event producer ESCAPED at baseline
```

Fixed assertion (mutant deliberately fails; restored control passes):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1322 filtered out; finished in 0.21s
WS8bm restored model control:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1322 filtered out; finished in 0.76s
WS8bm global model discrimination: foreign-event producer REJECTED at global event delta
```

Final two native DB regressions, current Python helpers and unchanged browser helpers:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1321 filtered out; finished in 0.38s
Ran 24 tests in 0.055s
OK
ℹ tests 36
ℹ pass 36
ℹ fail 0
ℹ skipped 0
```

Old module plus the seven current row tests (intentional failing-first result), and unchanged Rails model oracle:

```text
Ran 7 tests in 0.005s
FAILED (failures=4)
WS8bm baseline row regressions: 3 pass; 4 fail as expected at 160ef0738
WS8bm Rails direct-model declarations: 2 cases; omitted-owner refusal and independent stale writes; real callbacks
WS8bm Rails direct-model oracle: exact existing JSON; global two-event assertion PASS
```

Raw seed and inventory receipts:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
test/controllers/messages_controller_test.rb: 56 named declarations; 56 scoped attributions; 0 blocked
test/controllers/messages_drive_attachments_test.rb: 19 named declarations; 19 scoped attributions; 0 blocked
test/controllers/messages/cached_fragment_csrf_test.rb: 4 named declarations; 4 scoped attributions; 0 blocked
test/controllers/messages/legacy_presentation_cache_test.rb: 2 named declarations; 2 scoped attributions; 0 blocked
test/controllers/messages/boosts_controller_test.rb: 17 named declarations; 17 scoped attributions; 0 blocked
test/controllers/channel_threads_controller_test.rb: 24 named declarations; 24 scoped attributions; 0 blocked
test/controllers/channel_thread_messages_controller_test.rb: 12 named declarations; 12 scoped attributions; 0 blocked
test/controllers/channel_thread_messages_drive_attachments_test.rb: 13 named declarations; 13 scoped attributions; 0 blocked
test/controllers/message_forwards_controller_test.rb: 7 named declarations; 7 scoped attributions; 0 blocked
test/controllers/message_forward_sources_controller_test.rb: 2 named declarations; 2 scoped attributions; 0 blocked
WS8bm controller inventory: 156 named declarations; 156 scoped attributions; 0 owner-blocked
WS8bm system inventory: 118 passed; 17 deferred; 0 owner-blocked; 135 named declarations
```


## Canonical media and app verification

Canonical media is readily available in `triage-reference-d7c7de92`. The existing tracked extraction tool is used with `WS8BR2_MEDIA_DIR` set to the own review's `media` scratch directory:

```sh
bash rust/reference-tools/users/media_runtime.sh
```

Only private extracted libraries and binaries are used; no host library is changed. A private Cargo runner sets `LD_LIBRARY_PATH` to `media/native-libs` and prepends `media/usr/bin` to PATH before executing a test binary. Rustc/linking keep host libraries. The app command runs once with `CI=1`, canonical binaries on PATH and `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER` set to that runner. This preserves all native assertions and deadlines.

```sh
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire -- --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

`html5ever` stays excluded under the existing workspace check scope. The canonical app command exits 0: all **2,655 executed tests pass**, with the eight pre-existing ignores unchanged. The six previously failing media cases all pass in this command with canonical tools. This confirms the review's native-version diagnosis without changing expectations. Strict clippy also exits 0. Raw extracted-runtime versions, six named media results, complete app summary and clippy finish line:

```text
ws8bm pinned media runtime: image triage-reference-d7c7de92; libvips, FFmpeg tools and libraries extracted; no host libraries changed
WS8bm canonical libvips: 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
ffprobe version 7.1.5-0+deb13u1 Copyright (c) 2007-2026 the FFmpeg developers
```

```text
test controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers ... ok
test controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files ... ok
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect ... ok
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy ... ok
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy ... ok
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect ... ok
test result: ok. 2655 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out; finished in 544.93s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 42s
```


## Retained invalid attempts and remaining scope

Earlier local replay setup attempts had a generated ROOT path error, a missing tracked WS17 cfg(test) input, an inert foreign-event model producer without a second thread, an informational-stderr/JSON parser failure, and a cached mutant DB executable in the next control. These are retained in the own review logs, receive no parity/rejection credit, and are followed by corrected controls. No app failure, wait, golden or test concurrency is changed to hide them.

Inventory stays **156/156 controller declarations; 118 passed / 17 deferred / 0 owner-blocked systems**. This review promotes no declaration. The exact seventeen deferred cases remain in [the primary report](ws8bm-report.md#exact-remaining-system-work). The continuation remains PR-ready with those concrete deferrals. This review stops after the requested fixes and verification. Raw logs are retained under `.scratch/ws8bm-review221` and the verification clone's `.scratch/ws8bm-work-producers`. The first baseline producer logs are also preserved in `.scratch/ws8bm-review221/before`. The audit uses the reviewer's receipts read-only. The old cached-mutant control failure is retained as `db-mutant-cache-invalid.log`; the final same-command control passes above. No invalid setup attempt gets rejection credit.

The sole own scratch target is removed after all checks; all own test processes have completed. Final cleanup command and raw receipts:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml --target-dir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-pr2-review/fresh/rust/target
```

```text
     Removed 17376 files, 21.9GiB total
WS8bm cleanup: 0 scratch targets; 0 own listeners on 52020-52023
```

The final report commit changes documentation only. No further scope is attempted after this checkpoint.
