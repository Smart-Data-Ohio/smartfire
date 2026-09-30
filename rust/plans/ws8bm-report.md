# WS8bm review fixes and continuation — partial

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Verified implementation input: `562aeea2fb1b12dfc34eeca434336c0bf2f6e42b` (five implementation slices, integration followups, clippy cleanup and request-scoped snapshot correction).
The final report commit is documentation only. Main was not merged. Rails reference remains `d7c7de92`; post-pin layout/status work remains WS8b-r2-owned.

**PARTIAL.** The three assigned defects are fixed. Item 2 remains pending `rust/durable-attachment-analysis`, as explicitly assigned elsewhere. Wider retry coverage is complete for the recorded scalar matrix. Full upload/state/shell/system parity is still incomplete; a new pinned-Rails JPEG after-commit difference is recorded below.

## Review fixes and failing-first evidence

| Review item | Fix / commit | Exact `6375ba3f` failure | Fixed evidence |
| --- | --- | --- | --- |
| 1, P1 closed-thread missing signed image file | `4aca5f0b`: process thread media inside the enclosing write, reusing forwards' staged original/preview/variant guards | 500 but `(messages, joined, closed) = (1, true, false)` | 500 and `(0, false, true)`; byte-identical Rails response; all rows in nine request record tables and every retained storage file unchanged |
| 3, P2 boolean client-ID retries | `4929f9e4`: cast once with the Rails string-column cast, reuse for lookup and insert, preserve raw blank-value lookup semantics; audit/update all three retry paths | Two 201 responses, two rows with `"t"` | Two 201 responses, one `"t"` row; complete Rails response bytes match; 48-request matrix also passes |
| 4, P2 signed initial attachment | `a6f04901`: route initial top/nested attachment params through the existing human signed-capability verifier | Valid signed blob 13 returned 500 | 201, byte-identical full Rails response, one message with analyzed blob 13 and downloadable original |

All three regressions compiled and failed together against **exact** `6375ba3f5b996ad9bc27b5d9c0fd90ae2cc5bae2` before production edits. The committed reproducer regenerates seeds and adds only the regression module/vectors to that revision. A later final rerun verifies the same failures. The first fresh suite passed app/DB/views but clippy found a one-use parameter closure and an unnecessary post-write binding; `168a694c` removes those without changing behavior, and final fresh validation is reported below. A second fresh run caught an overbroad queue snapshot: the unrelated retention job completed during the request. `562aeea2` narrows the snapshot to the nine request record tables; the live runner/default concurrency remain intact and separate HTTP enqueue-failure trigger regressions remain in the suite. This is a corrected test invariant, not a timing threshold change or output mask. One intermediate rerun was killed by the host before tests executed; it was not counted as failing-first evidence.

**Correction to the earlier report:** the earlier claim of Rails-equivalent thread-upload transaction boundaries was wrong. Media processing followed commit and could leave a joined/reopened thread and orphan message on 500. The encompassing transaction now covers thread creation/lifecycle, membership, initial/reply message, blob metadata/touches, and representation rows; transaction guards remove generated files on rollback. Root creation keeps Rails' distinct post-commit processing boundary. This does not claim that every Rails after-commit failure rolls back: the JPEG case below demonstrates the distinction.

Item 2 is **not fixed here**. Human/bot attachment edits still rely on the shared in-memory analysis path. No shared `presenters/attachments.rs` change was made. After the lead authorizes merging the durable-analysis branch through main, add the HTTP message-edit trigger regression proving a failed durable enqueue rolls back the edit.

## Changed files and design

Paths are relative to `rust/`.

- `crates/campfire/src/controllers/messages.rs`: thread processing inside the writer; root processing boundary preserved; shared human verifier factored into `resolve_human_attachment`; create callers supply the same cast client ID used for retry lookup.
- `controllers/channel_thread_messages.rs`, `controllers/channel_threads/writes.rs`: consistent cast/lookup/insert and verified initial signed attachment; initial thread media processing stays inside the creation transaction. No bot/work/board policy was redesigned.
- `crates/campfire/src/messaging.rs`: small `process_message_attachment` adapter reuses `ForwarderCopier`'s transaction-aware processor. No DB schema, dependencies, lockfile or Rails app changes.
- `controllers/messages/review_tests.rs`: the three compiled review regressions, complete HTTP golden comparisons, nine-table/file rollback snapshots, multipart reply/initial variant-insert failures, and 24 scenarios/48 complete retry responses across root/reply/initial paths. Scalars: true, false, zero, decimal, string, empty string, ASCII whitespace and nonbreaking space. False still persists as `"f"` and does not deduplicate.
- `reference-tools/messaging/{thread-review,client-retries,thread-upload-coverage}.rb`, `oracle-database.rb`, corresponding vectors: real Rails requests. Restore the private DB between scenarios; no outer test transaction changes request rollback semantics. The broader upload oracle is reference evidence, not full Rust signoff.
- `review-failing-first.py`: exact reviewed-revision compilation/failure proof. `initial-image-gap.py`: isolated strict reproducer of the explicitly unresolved JPEG difference; not a new suite ignore or response mask.
- `crates/richtext/src/markdown.rs`, `owner-schedule-integration.patch`: restore the room owner exact-key icon reader without changing Markdown lookup, and wrap the in-transaction thread result in the owner merged `PostingOutcome::Created`. The isolated merge previously failed compilation at these two concrete seams; the existing schedule patch now includes that typed wrapper. The seam comment is byte-identical to the owner definition to avoid a comment-only merge conflict.
- `plans/ws8bm-controller-cases.md`: attribute the three named retry cases to the complete request matrix. Other named cases retain their explicit owner/backlog status.

## New unresolved JPEG case

The wider standalone Rails probe finds `top_image` and `nested_image` return 500 **after committing** one thread/message. Its unfiltered trace runs through `ActiveStorage::VariantRecord#_run_commit_callbacks!`, `CreateOne#upload`, and `DiskService#upload` (`IO.copy_stream`: `IOError: closed stream`). This occurs when creating a new JPEG thumbnail; valid blob 13/file is unaffected. Rust currently returns 201 with the committed thread/message. The strict isolated probe below reproduces that mismatch. The lead was asked whether to defer for a Rails fix or port this distinct after-commit failure exactly; no answer was assumed. JPEG parity remains open. The broader upload oracle's 66 responses must not be counted as 66 passing Rust comparisons.

## Stable shell/list/composer seam

No presenter/list/composer entry point changed in this continuation. The merged shell must pass:

- `Presenter::room_message_list(&selected_roots, divider.message_id, divider.count)?` for the complete message-list slot. Set `Presenter.cache_base_url` to the verified request origin and use the application fragment cache. Mount the returned bytes verbatim; they include the invitation-expression blank line. WS8b-r supplies root/around selection, last-read/unread divider facts, invitation content and scrolling/jump facts.
- The real `ViewContext`/CSRF provider, viewer `UserView`, `Presenter::composer_facts(room, viewer, thread, drive)`, ordered command names, and `composer_drive_flow(viewer, share_picker_available)` from Google's availability. Use `FooterComposer` in room footers and `Composer` inline/in thread panes. Pass M2's actual `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as `scheduled_control`; do not cache the composer.
- Scoped thread items, optional same-thread anchor, room `updated_at`, ordered thread steps, viewer, composer facts and real schedule child to `Conversation`. Mount `PendingTemplate { ctx, user }` for optimistic messages.

Apply the existing `owner-schedule-integration.patch` after merging the owner branches: it wires `render_thread_schedule_control` to M2's real provider and the room footer to `FooterComposer`. The worker baseline still has the flagged empty schedule seam because the owner module is absent. Standalone pinned thread show has no schedule/composer child; `/content` owns those. `render_thread_pull_request_header` remains WS15g's named fragment call site. Work/board HTML and activity remain WS12's flagged seams. No actual main/owner branch merge was made here; the checker performs an isolated scratch merge only.

## Commands re-run and raw summaries

All commands were executed from the worktree root. Fresh validation uses its private clone/target/scratch, CI=1, four build jobs, default test threads, ports 52050–52099 and newly generated seeds. Earlier proof/owner tools use ports 52000–52049. The optional owner target is compiler cache only, not a fixture input.

```sh
python3 rust/reference-tools/messaging/fresh-check.py > .scratch/fresh-review-final.log 2>&1
```
The helper executes locked metadata, seed build, campfire binary tests, DB tests/doctests, the views core test, and workspace/all-target clippy with `-D warnings`.

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
bash rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_db
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_views --test core
mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

Locked metadata exited zero. Final fresh app/DB/views/clippy all passed; no timing test failed in that run.

```text
WS8bm fresh checkout: 562aeea2fb1b12dfc34eeca434336c0bf2f6e42b; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-iagkbbd5
WS8bm fresh concurrency: default test threads; four build jobs; no timing threshold changes
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 16s
test result: ok. 478 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 44.19s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3m 04s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 53.40s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 48.36s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 04s
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/db/views/clippy passed
```

Three existing app ignores: reference cable recording, WS11 manages_bots and push-latency measurement. Four existing DB ignores: external Rails scenario/export/fixture/rollback hooks. Zero discovered DB doctests account for the zero-test line. No seed skips or new ignores.

```sh
python3 rust/reference-tools/messaging/review-failing-first.py > .scratch/review-failing-first-final.log 2>&1
```
```text
WS8bm failing-first production revision: 6375ba3f5b996ad9bc27b5d9c0fd90ae2cc5bae2; only regression module/vectors added; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-review-6375-0sd5_tla
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 01s
WS8bmr Rust signed initial: 500 Internal Server Error
test controllers::messages::review_tests::review_signed_initial_thread_attachment_matches_rails ... FAILED
WS8bmr Rust boolean retry: statuses 201 Created 201 Created; rows [(935962058, "t"), (935962059, "t")]
test controllers::messages::review_tests::review_boolean_client_retry_matches_rails_one_row ... FAILED
WS8bmr Rust missing-file: status 500 Internal Server Error; (messages, joined, closed) (1, true, false)
test controllers::messages::review_tests::review_failed_thread_upload_rolls_back_like_rails ... FAILED
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 477 filtered out; finished in 0.75s
WS8bm failing-first: all 3 assigned regressions compiled and failed against 6375ba3f
```
```sh
python3 rust/reference-tools/messaging/initial-image-gap.py > .scratch/initial-image-gap-current.log 2>&1
```
```text
WS8bm image-gap checkout: 1589e20b569f1786905ea3e361a2ca55e8b65b4f; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-image-gap-sh3e7erd
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 57s
WS8bm initial JPEG gap: Rust status 201 Created; new messages 1; new threads 1; Rails status 500 / messages 1 / threads 1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 481 filtered out; finished in 2.67s
WS8bm initial JPEG: unresolved mismatch reproduced; explicit deferred case, not an application-suite ignore or parity mask
```
This separate probe requires the known mismatch to fail; it is not an application-suite pass.

```sh
python3 rust/reference-tools/messaging/owner-integration-check.py --target-dir rust/target > .scratch/owner-integration-review.log 2>&1
```
```text
WS8bm owner integration: worker 3080f8504cc476390ed9caabb5840e9eeb1a12ee; shell 27990da2851f4c056db71c6b430c894307bc6bfe; isolated merge /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-owner-_w_vuns8; main unmerged
WS8bm owner manifests: 13 parsed; zero duplicate workspace dependency keys
native:     Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 10s
native: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 981 filtered out; finished in 1.09s
FAIL room 654632876 message_list: Rust 286016 bytes, Rails 288548 bytes
PASS room 654632876 composer: 10371 exact bytes
PASS room 654632876 pending_template: 1449 exact bytes
PASS room 186869642 message_list: 19538 exact bytes
PASS room 186869642 composer: 10367 exact bytes
PASS room 186869642 pending_template: 1449 exact bytes
PASS room 699448329 message_list: 6552 exact bytes
PASS room 699448329 composer: 8594 exact bytes
PASS room 699448329 pending_template: 1462 exact bytes
Native room component acceptance: 8 exact matches; 1 differences; no masks
pane:     Finished `test` profile [unoptimized + debuginfo] target(s) in 24.26s
pane: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 982 filtered out; finished in 1.31s
show:     Finished `test` profile [unoptimized + debuginfo] target(s) in 19.69s
show: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 981 filtered out; finished in 1.62s
AssertionError: Owner integration failures: ['native component bytes']; all available checks attempted
```
Owner checker exit 1 is retained: Designers populated card fragments differ. All 11 targeted tests pass; 8/9 components match. Earlier compile failures at the owned return/icon seams were fixed in the published integration patch/additive reader. The final owner check used implementation input 3080f850; subsequent changes were lint-only expressions and test snapshot scope, with no presenter/template or patch changes.

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-review-current.log 2>&1
python3 rust/reference-tools/messaging/check-goldens.py > .scratch/goldens-review-final.log 2>&1
```
```text
WS8bm reference source check: 63 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm preview oracle: 8 real Rails HTTP responses; 0 messages written
WS8bm invalid-create oracle: 6 real Rails HTTP responses; 0 messages written
WS8bm scalar-cast oracle: 8 actual Rails model assignments
WS8bm fragment oracle: 5 real Rails messages; 2 viewers through one fragment cache; 0 session-bound values
WS8bm root oracle: 10 real Rails actions responses; 6 updates and saved rows; 2 rejected updates; 9 edit forms and 1 actions menu; 4 standalone messages
WS8bm paging oracle: 16 real Rails page requests; 12 format requests; template digest 8c84e9c3391ab09f136a472ad8ab8b69
WS8bm broadcast oracle: 6 real Rails writes; 25 rendered/channel publisher frames; request port 3443
WS8bm thread-membership oracle: 11 real Rails requests; membership rows and JSON bytes captured
WS8bm collection oracle: 10 real Rails states; keys and cache-hit bytes; 0 session-bound values
WS8bm room-list oracle: 9 real Rails room requests; selected roots/unread facts and show list-slot bytes; 0 session-bound values
WS8bm message-states oracle: 26 real Rails states rendered cold/warm; 78 actual append/replace/remove frames; 0 session-bound values
WS8bm thread-message read oracle: 18 actual Rails requests; scoped pages, empty formats, raw JSON/actions/HTML and locked reads
WS8bm thread-message write oracle: 15 actual Rails writes; 50 publisher frames; retries, rows, Drive sets, locks and tombstones
WS8bm thread-pages oracle: 28 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
WS8bm thread-lifecycle oracle: 27 actual Rails actions; creation retries, metadata/tags, lifecycle permissions, rollback rows and delete frames
WS8bm thread-content oracle: 9 actual requests; anchor scope and fixed-secret conversation/composer bytes
WS8bm forwards oracle: 18 real picker/refusal/source-privacy requests; exact JSON bytes
WS8bm forward-success oracle: 5 positive actual requests; 7 forwards; complete bodies/rows and 21 rendered frames
WS8bm modern-boosts oracle: 46 actual toggle/alias/legacy/duplicate/delete/coercion requests; 44 rendered reaction replacements
WS8bm signed-attachments oracle: 27 actual root/thread requests; attach/replace/delete, expiry/purpose/missing-blob rejection, expired retries and failed-edit preservation
WS8bm boost-pages oracle: 7 actual index/new/actions requests; complete fixed-token forms, distinct reactors and hostile tooltip names
WS8bm thread-review oracle: 4 actual Rails requests; boolean retry, failed closed-thread media rollback, signed initial attachment
WS8bm thread-upload-coverage oracle: 42 scenarios; 66 actual Rails requests; 3 client-id paths, 4 media types, top/nested initial capabilities and rollback
WS8bm client-retries oracle: 24 scenarios; 48 actual Rails requests; root/reply/initial scalar IDs and raw blank-value semantics
WS8bm golden check: 22 Rails oracles re-run; 23 golden files byte-identical
```
These are source identity / Rails regeneration checks, not a claim that all 66 exploratory upload requests pass on Rust.

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_richtext > .scratch/richtext-compat.log 2>&1
```
```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 8.54s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 32.18s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.81s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
```
```sh
python3 - <<'CHECK'
from pathlib import Path
import tomllib
files=[Path('rust/Cargo.toml'),*sorted(Path('rust/crates').glob('*/Cargo.toml'))]
for path in files: tomllib.loads(path.read_text())
print(f'WS8bm dependency-key check: {len(files)} Cargo manifests parsed; zero duplicate workspace dependency keys')
CHECK
```
```text
WS8bm dependency-key check: 13 Cargo manifests parsed; zero duplicate workspace dependency keys
```
```sh
python3 rust/reference-tools/messaging/check-controller-files.py > .scratch/controllers-review-current.log 2>&1
python3 rust/reference-tools/messaging/deferred-system-inventory.py > .scratch/systems-review-current.log 2>&1
```


## Controller and system accounting

The Rails source suite below is independently executed reference evidence, not 156 one-to-one Rust ports. There are 156 named declarations across ten files. Six have explicit case attribution in the inventory (three prior boost cases and the three retry cases); remaining attribution/signoff is tracked there by name/owner. Aggregate Rust counts are separate.

```text
test/controllers/messages_controller_test.rb
56 runs, 265 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages_drive_attachments_test.rb
19 runs, 83 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/cached_fragment_csrf_test.rb
4 runs, 58 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/legacy_presentation_cache_test.rb
2 runs, 13 assertions, 0 failures, 0 errors, 0 skips
test/controllers/messages/boosts_controller_test.rb
17 runs, 155 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_threads_controller_test.rb
24 runs, 206 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_thread_messages_controller_test.rb
12 runs, 65 assertions, 0 failures, 0 errors, 0 skips
test/controllers/channel_thread_messages_drive_attachments_test.rb
13 runs, 57 assertions, 0 failures, 0 errors, 0 skips
test/controllers/message_forwards_controller_test.rb
7 runs, 35 assertions, 0 failures, 0 errors, 0 skips
test/controllers/message_forward_sources_controller_test.rb
2 runs, 12 assertions, 0 failures, 0 errors, 0 skips
WS8bm Rails controller reference: 10 files passed; reference counts only
```

The pinned system inventory records 19 files / 156 literal declarations, **0 executed**. End-to-end/pixel acceptance remains the end-to-end phase; no system signoff is claimed.

```text
test/system/boosting_messages_test.rb: 4 literal test declarations; 0 executed; deferred
test/system/code_highlighting_test.rb: 6 literal test declarations; 0 executed; deferred
test/system/sending_messages_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/threads_test.rb: 15 literal test declarations; 0 executed; deferred
test/system/workspace_markdown_test.rb: 8 literal test declarations; 0 executed; deferred
test/system/composer_test.rb: 11 literal test declarations; 0 executed; deferred
test/system/composer_attach_menu_test.rb: 9 literal test declarations; 0 executed; deferred
test/system/message_interactions_test.rb: 10 literal test declarations; 0 executed; deferred
test/system/message_actions_mobile_test.rb: 2 literal test declarations; 0 executed; deferred
test/system/message_toolbar_test.rb: 13 literal test declarations; 0 executed; deferred
test/system/message_list_a11y_test.rb: 29 literal test declarations; 0 executed; deferred
test/system/drive_attachments_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/unread_divider_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/search_forward_edit_test.rb: 3 literal test declarations; 0 executed; deferred
test/system/keyboard_shortcuts_test.rb: 14 literal test declarations; 0 executed; deferred
test/system/content_security_policy_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/motion_test.rb: 9 literal test declarations; 0 executed; deferred
test/system/mobile_layout_test.rb: 5 literal test declarations; 0 executed; deferred
test/system/timezone_detection_test.rb: 2 literal test declarations; 0 executed; deferred
WS8bm system inventory: 19 pinned files; declarations only; no browser or pixel pass claim
```

## Precisely remaining

1. Durable attachment analysis from its owner branch; authorized main merge followed by atomic message-edit enqueue regression.
2. Resolve the new JPEG variant after-commit mismatch; wider upload/state/recipient coverage beyond the committed matrices, including later cases of the 66-response upload probe not yet verified against Rust.
3. Published owner merges plus schedule patch; full live shell/application layout/status popup acceptance, populated WS15g/WS15e card and PR fragments, invitation/owner chrome inputs. WS12 activity/work/board seams stay flagged.
4. Complete the remaining named controller attribution/signoff in `ws8bm-controller-cases.md`, grouped by its ten files. The source tests pass; that does not close their Rust parity backlog.
5. The 19 listed system files: all browser/pixel execution/signoff deferred to the end-to-end phase. M2's polls/pins/saves/schedules/search/slash/autocomplete/message-links/files, WS11 bot/agent endpoints, WS14 Google integrations, and WS15 card providers remain their owners' work.

No test concurrency override, timing-threshold change, new ignore, or parity mask/allowlist change was introduced. Any failing timing result from final validation is reported in evidence instead of being tuned.
