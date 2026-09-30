# WS8bm messaging HTTP — partial six-slice continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Started from accepted `15551848d9628eb1779653a0c1fc208782389a7f`.
Verified implementation input: `ee35680baaa83b4a0590690d2e4472f9ea990a19`.
The final report commit adds documentation only; its SHA is in the final reply.

**PARTIAL.** Six implementation slices and one verification slice were committed and pushed separately. The fresh seeded app suite has zero failures at default test concurrency. Full merged live thread HTML, the complete upload/state/recipient matrix and browser/system parity remain partial; none is represented as complete by the aggregate test counts below.

| Commit | Coherent slice |
| --- | --- |
| `b14759da` | Ordinary thread show full pinned template/layout goldens and named WS15g header seam |
| `e4538592` | Transactional forward processing, generated-file rollback, positive response/row/frame differentials |
| `f96ece70` | Modern human boost toggles, canonicalization and complete reaction replacements |
| `83700fe5` | Signed direct-upload capabilities in root/thread posts and edits; exact root response newline |
| `8ea2f96c` | 26 complete cold/warm message states and 78 delivered append/replace/remove frames |
| `a386724e` | Authorized WS12 501 on work/board panes; named request-context schedule seam |
| `ee35680b` | Fresh verification uses default test concurrency and preserves any failing exit while continuing available checks |

No new main merge was requested this turn. The accepted branch already contains main through `4278cb1e`. Read-only inspection found main at `2e20b24c`, with no Rust changes between those revisions. The shared brief now lists the post-pin status-popup/application-layout change as WS8b-r2-owned and the board-nudge tag change as WS17/WS12-owned. The reference used for this worker's messaging records/templates remains `d7c7de92`. The full-layout fixture below proves that pinned layout with explicit owner inputs; **it does not prove the new status-popup layout or a fully merged live page**. Those require WS8b-r2's updated layout/provider integration and post-change goldens.

## What changed, by file

Paths are relative to `rust/`.

- `crates/campfire/src/controllers/channel_threads.rs`, `channel_threads/page_tests.rs`, `crates/views/src/channel_threads.rs`, `templates/channel_threads/show.html`, `reference-tools/messaging/thread-pages.rb`, `vectors/messaging/thread-pages.json`: ordinary nested HTML show is routed and returns 200. Seven full fixed-token template/layout comparisons cover ordinary, empty, stale, closed, locked, deleted-starter and Turbo-frame pages; the reference script makes 28 real Rails requests. The live HTTP test checks the owned body, scopes, statuses, headers and real tokens separately. Chrome/brand names/Google/quiet-hours facts are explicit inputs to the full-render comparison, not response masks. The pinned standalone show has no composer or schedule child; `/content` owns the conversation/composer. `Show.pull_request_header` is an additive slot directly after its header and before its starter, preserving Rails whitespace. The clearly named `render_thread_pull_request_header` is explicitly empty pending WS15g, which is absent from the inspected main. After its owner merge, resolve the PR and invoke `campfire_views::github::thread_header(ctx, room_id, thread_id, card)` there with request context. Work/board HTML show and content are authorized 501 seams for WS12; JSON show retains the existing read API. `render_thread_schedule_control(ctx, room_id, thread_id)` is the named M2 request-context slot; it is currently empty. No owner branch was merged or copied.
- `crates/db/src/models/forwarder.rs`, `crates/campfire/src/messaging.rs`, `controllers/message_forwards.rs`, `controllers/message_forwards_tests.rs`, `controllers/messages.rs`: additive `BlobCopier::process(tx, message)` makes each copied attachment's analysis, metadata touches, preview and thumb generation participate in the same transaction as every destination. Commit guards cover originals and generated media; failures at a later destination, media stage or durable enqueue remove all newly generated files and roll back message/blob/thread-membership rows. The controller reloads committed records before broadcasting. `ForwarderCopier` is still the runtime adapter. The additive `forward_with_client_ids` generator seam retains random production UUIDs; a `cfg(test)` header supplies the same deterministic UUID sequence as the Rails oracle. Production does not recognize that header. File/media processing runs synchronously on the writer in this transactional adapter; staging it off the writer while retaining the same rollback contract is a performance follow-up, not a parity claim.
- `reference-tools/messaging/forward-success.rb`, `vectors/messaging/forward-success.json`, `channels/tests/hub_test/message_parity.rs`: five positive actual Rails JSON/HTML requests create seven forwards. Entire response bodies/headers and snapshots compare without UUID substitution or masks: Markdown, legacy, forward-of-forward, private copied file, reopened thread, direct-room destination and Drive metadata. Four recipient sockets check all seven message frames and thirteen unread markers in this oracle. Rails also captures one activity frame: **20 of the 21 frames are delivered/compared here**. Message activity-item callbacks are the explicit WS12 seam in WS8a, so that captured frame is retained and reported, not counted as passed. Wider recipient memberships/races remain deferred.
- `controllers/presenters.rs`: stable `room_display_name` now follows Rails's direct/group display rules: explicit group name first, otherwise ordered other participants' first names separated by commas, with the multi-member suffix and full single-participant name. This fixed the positive direct-forward golden. No WS8b-r room helper/shell file was edited.
- `crates/db/src/models/boost.rs`, `controllers/messages/boosts.rs`, `controllers/messages/boosts_tests.rs`, `controllers.rs`, `crates/views/src/messages/reactions.rs`, `messages.rs`, `reference-tools/messaging/modern-boosts.rb`, `vectors/messaging/modern-boosts.json`, `channels/tests/hub_test/message_parity.rs`: human create/destroy use canonical emoji/brand/custom-icon content, Ruby ASCII/NUL stripping, atomic toggles that remove every old duplicate for this reactor, and complete reaction-container replacement through WS7's publisher/guard. Free text stays one row per submission. Twenty-two actual Rails requests compare statuses, headers, whole bodies and persisted boosts; twenty complete delivered replacements compare byte for byte. Auth/CSRF/nonmember/other-reactor rejection, failed touch rollback and two concurrent toggles are covered. Existing WS11 bot helpers retain their API and policy; this does not claim modern bot endpoint parity or every boost picker/action case.
- `controllers/messages.rs`, new `messages/upload_tests.rs`, `crates/views/templates/messages/create.turbo_stream.html`, `reference-tools/messaging/signed-attachments.rb`, `vectors/messaging/signed-attachments.json`: human root/thread attachment parameters accept verified Active Storage signed blob IDs without changing shared avatar/bot assignment APIs. Nine actual Rails requests compare complete responses and blob/message rows for file posts, image replacement, nil/empty removal, tampered signatures and a forged unpermitted blob column. A scope/enqueue failure test confirms existing blob storage survives and no message is created. The root Turbo Stream response preserves Rails's final newline. Existing ordinary multipart PNG/attachment processing tests also pass in the fresh suite. This is not a full direct-upload/expiry/corrupt-media/inline-ActionText matrix.
- `controllers/messages/state_tests.rs`, `reference-tools/messaging/message-states.rb`, `vectors/messaging/message-states.json`: extend the accepted matrix from 19 to 26 complete states, adding bodyless legacy records, empty streams, edited text and image captions, file replies, forwarded files and mixed modern/legacy reaction-plus-image combinations. Existing media/sound/Drive/action/emoji/bot/system/reply/deleted-reply/steps/forward cases remain. Both cold and warm render bytes and all 78 actual append/replace/remove payloads pass through the WS7 socket path and session-bound-value guard. Fixture Time inputs serialize as ISO8601; output bytes are not normalized.
- `reference-tools/messaging/check-goldens.py`: reruns all 18 owned Rails oracles; optional named regeneration writes only real Rails output. `reference-check.py` now validates the 61 pinned source files read by these slices. `fresh-check.py` removes the previously committed four-test-thread override, uses default test concurrency, creates a new clone/target/seeds, and runs all available app/DB/views/clippy checks while retaining a failure exit if any check fails. No timing test/assertion/threshold/ignore was changed. No timing failure occurred in this run.
- `plans/ws8bm-integration.md`, `plans/ws8bm-controller-cases.md`, this report: stable owner seams, exact named Rails case attribution backlog, current verification and explicit remaining work. The external report is identical to this tracked mirror.

## Stable shell/list/composer contract

`Presenter::messages(&records)` and `campfire_views::messages::Index { ctx, messages }` are unchanged. The message-owned room-slot adapter remains `presenter.room_message_list(&messages, divider.message_id, divider.count)?`; WS8b-r passes this to `ShellComponents::message_list`. The shell supplies correctly scoped root/around records and owns unread cursor/count/read side effects. Set `Presenter.cache_base_url` to the verified origin and render under the app fragment-cache context. Divider markup stays outside shared message fragments.

The composer entry point remains `messages::composer::Composer { ctx, facts, scheduled_control }`. The merged shell must supply its live `ViewContext` (viewer, verified URL, assets, signed stream signer and CSRF provider), `Facts { room_id, room_kind, room_name, thread, slash_commands, drive, ... }` via `Presenter::composer_facts`, `thread: None` for room or `Some(Thread { id, name })` for pane, and the Google owner's availability fact via `composer_drive_flow(viewer, share_picker_available)`. Supply M2's trusted rendered `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as `scheduled_control`; thread content has the explicitly named `render_thread_schedule_control` call site. For panes also supply selected messages, optional same-thread anchor, room update timestamp, viewer, ordered thread steps and `PendingTemplate`. The real Rails schedule child is an explicit input to the current complete component comparison; runtime schedule wiring is pending the lead's owner merge.

Current `Layout::load` does not supply all owner chrome facts used by the full-layout oracle. Merge/wire WS8b-r/r2's shell/chrome, M2's schedule provider and WS15g's PR provider, then compare complete live HTTP bytes against the applicable updated Rails layout. Do not count the current empty schedule/PR slots or fixed-token component comparison as that acceptance.

## Commands re-run this session and raw results

### Fresh clone, committed test inputs only

```sh
python3 rust/reference-tools/messaging/fresh-check.py >.scratch/fresh-current.log 2>&1
```

The helper's exact test/check commands, executed inside the new clone with `CI=1`, generated seeds and its own target, are:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
bash rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_db
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_views --test core
mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

```text
WS8bm fresh checkout: ee35680baaa83b4a0590690d2e4472f9ea990a19; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-aayyek94
WS8bm fresh concurrency: default test threads; four build jobs; no timing threshold changes
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 22s
test result: ok. 472 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 37.25s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 20.74s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 83.38s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 22.44s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 27s
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/db/views/clippy passed
```

App ignores are explicit existing hooks: `channels::tests::golden::record_reference` (needs running Rails), `controllers::presenters::accounts::tests::manages_bots` (WS11) and `jobs::tests::push_latency` (measurement). DB ignores are `scenario_matches_ruby`, `export_database_for_rails`, `fixtures_match_ruby_row_for_row` and `read_rails_rollback_changes`, each requiring the external reference hook described in its annotation. No missing-seed skips, new ignores or silent test skips occurred. The DB doctest line is zero discovered doctests, not a hidden failure.

The clone began with no `.scratch` or `rust/target`; tests read committed vectors and freshly generated seed storage. Scratch is only output/log space. Seven previous clone targets were removed to make room, preserving their source clones and logs. No other worker's output was touched.

### Locked workspace metadata and duplicate dependency keys

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'CHECK'
from pathlib import Path
import tomllib
files=[Path('rust/Cargo.toml'),*sorted(Path('rust/crates').glob('*/Cargo.toml'))]
for path in files:
    tomllib.loads(path.read_text())
print(f'WS8bm dependency-key check: {len(files)} Cargo manifests parsed; zero duplicate workspace dependency keys')
CHECK
```

```text
WS8bm dependency-key check: 13 Cargo manifests parsed; zero duplicate workspace dependency keys
```

Metadata exited zero; TOML parsing rejects duplicate keys. No dependencies or lockfile changed.

### Pinned source identity and every golden regenerated from Rails

```sh
python3 rust/reference-tools/messaging/reference-check.py >.scratch/reference-current.log 2>&1
python3 rust/reference-tools/messaging/check-goldens.py >.scratch/goldens-current.log 2>&1
```

```text
WS8bm reference source check: 61 controller, model, helper, template and icon files match d7c7de92
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
WS8bm modern-boosts oracle: 22 actual toggle/alias/legacy/duplicate/delete/coercion requests; 20 rendered reaction replacements
WS8bm signed-attachments oracle: 9 actual root/thread attach/replace/delete/tamper/forged-column requests
WS8bm golden check: 18 Rails oracles re-run; 19 golden files byte-identical
```

These are reference records/response/render counts, not counts of all possible ported Rails tests. Session secrets are controlled renderer inputs for full HTML fixtures, not removed/replaced output text. The forward response test uses an identical UUID input sequence in both implementations; real frame tests separately check the publisher/guard and absence of request-bound values. Capture of 21 forward reference frames does not imply execution of the WS12 activity frame.

### Meaningful failing-first observations

The following ran before the relevant implementation changes this session, compiled successfully, and failed on observable behavior. Original logs remain in owned scratch; the final fresh run executes their restored passing versions. No compiler/setup error is counted as a regression detection.

| Test/log | Observed failure before fix |
| --- | --- |
| `attachment_processing_failure_rolls_back_every_forward_and_thread_side_effect`, `.scratch/forward-atomic-before.log` | Two messages/two blobs and a thread membership remained committed after processing failed |
| `modern_boosts_match_rails_toggle_coercion_duplicate_and_destroy_rows`, `.scratch/boosts-before.log` | `:thumbsup: ` created a second literal row instead of toggling the existing canonical emoji; the authorization-first test already passed |
| `signed_root_and_thread_attachments_match_rails_response_and_blob_rows`, `.scratch/signed-before.log` | A valid signed file post returned 500 instead of Rails's 200 |
| `work_and_board_html_remain_authorized_ws12_seams`, `.scratch/ws12-content-before.log` | Authorized work pane returned an incomplete 200 instead of the specified flagged 501; unauthorized scope remained checked first |

```text
forward-atomic-before:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.87s
boosts-before:
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 469 filtered out; finished in 0.75s
signed-before:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.41s
ws12-content-before:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.97s
```

Existing negative authorization tests remain exercised in the fresh suite: nonmember/removed member, nested room/thread/message mismatch, deleted rooms, another author/booster, administrator edit restrictions, system-note immutability, bot tokens and forged CSRF. Existing negative tests were not made to fail by introducing an unrelated defect. New boost/upload scopes pass before their corresponding successful behavior is credited.

## Grouped controller evidence and deferred cases

Fresh app output groups **82 passing Rust tests** across owned HTTP/message and message-publisher files, zero failures/ignores in these groups. Counts are Rust test functions, not a one-to-one Rails case count. Twelve new test functions were added this run; the state/socket matrix functions were also broadened.

```text
channels::tests::hub_test::message_parity: 7 ok
controllers::channel_thread_messages::tests: 3 ok
controllers::channel_thread_messages::write_tests: 3 ok
controllers::channel_threads::content_tests: 3 ok
controllers::channel_threads::page_tests: 4 ok
controllers::channel_threads::tests: 5 ok
controllers::channel_threads::write_tests: 5 ok
controllers::message_forwards_tests: 7 ok
controllers::messages::boosts_tests: 3 ok
controllers::messages::collection_tests: 2 ok
controllers::messages::http_tests: 15 ok
controllers::messages::paging_tests: 4 ok
controllers::messages::room_list_tests: 1 ok
controllers::messages::root_tests: 9 ok
controllers::messages::state_tests: 1 ok
controllers::messages::tests: 8 ok
controllers::messages::upload_tests: 2 ok
```

The complete exact 156 named Rails declarations and case owners are in [ws8bm-controller-cases.md](ws8bm-controller-cases.md). That file is explicitly a case-level attribution/signoff backlog, not a claim that every underlying behavior is absent. The aggregate goldens above establish the stated behaviors; full named-case port accounting is still partial. Wider coercion/format/race/cache/query and browser integration cases remain to be proved rather than inferred from those counts.

Per owned Rails file, remaining verification/integration is:

| File under `test/controllers/` | Remaining owner and cases |
| --- | --- |
| `messages_controller_test.rb` | WS8bm: complete edited-time/time-zone, attachment/bodyless/identical-save, rename/delete/quote, recipient and uncommon-format/shape matrix. WS15g/e: populated card fetch/re-sync and edit-add/remove cases. M2: pins and their validators. WS11: legacy/agent webhook delivery permutations. WS8b-r/r2: full live shell/list/layout integration. |
| `messages_drive_attachments_test.rb` | WS8bm/WS14g: complete consent/edit/direct-upload/file removal and processing-failure combinations; actual browser attachment menu/download behavior. Accepted Drive set/JSON/chip/guard fixtures remain. |
| `messages/cached_fragment_csrf_test.rb` | WS8bm with WS8b-r/r2/WS4: every live cached page/refresh/thread form with actual per-viewer tokens, full merged shell and concurrent cache/viewer permutations. Current fixed-token renderer and real HTTP token checks are separate evidence. |
| `messages/legacy_presentation_cache_test.rb` | WS8bm: explicit named rollout invalidation cases and wider warm/cold legacy/off-page-reference combinations. Existing cache-key/ETag/vector tests pass. |
| `messages/boosts_controller_test.rb` | WS8bm: named picker/new/action soft-keyboard cases, two custom-icon reactors, tooltip hostile reactor names, distinct-reactor action metadata and wider custom-icon/cache/thread recipient permutations. Human toggle/create/delete differential and delivered replacements are complete for the 22-request oracle; WS11 owns bot boosts. |
| `channel_threads_controller_test.rb` | WS8bm: full merged live show/content bytes, schedule/PR wiring, additional membership/lock/staleness races and measured constant-query/preload assertions. WS12: conversion, status, owner eligibility/reassignment/removal/audit; WS11: agent eligibility/notification and picker integration. |
| `channel_thread_messages_controller_test.rb` | WS8bm: all joined recipients regardless of preference, uncommon request formats/shapes, lock/membership races and whole merged shell redirects/panes. WS15g/e: populated post/PR add/remove card cases. Current 15-write/18-read oracle and scoped/atomic tests pass. |
| `channel_thread_messages_drive_attachments_test.rb` | WS8bm/WS14g: wider Drive/storage processing and deletion combinations, direct-upload signatures/expiry/media, attachment-only edits and browser flows. Current accepted Drive writes and new signed root/thread responses pass. |
| `message_forwards_controller_test.rb` | WS8bm: malformed/nil inputs and source/destination/member races, measured constant-query assertion, wider thread/direct recipient coverage and media matrix. Processing atomicity and five positive whole-response/row goldens now pass. WS12: activity callback frame. |
| `message_forward_sources_controller_test.rb` | WS8bm: complete unusual coercions/formats and deletion/membership races. Canonical URL/null privacy and no-store bytes are already covered by the refusal/picker/source oracle. |

The following command was re-run against the pinned Rails archive. These are **Rails reference pass counts only**, not 156 Rust ports:

```sh
python3 rust/reference-tools/messaging/check-controller-files.py >.scratch/controllers-current.log 2>&1
```

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

M2 retains polls, pins, saved items/reminders, scheduled messages, search, slash commands, autocomplete, message links and room files. No new implementation or port counts for those controllers are claimed here.

### System files — zero executed, explicitly deferred

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py >.scratch/systems-current.log 2>&1
```

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

These are literal pinned declarations, not executed case counts. Every file has zero browser/system passes claimed. Message/controller interaction belongs to WS8bm; merged room/composer/unread/layout/keyboard/mobile cases need WS8b-r/r2, Google flows WS14g, agent cases WS11, work/board WS12, CSP WS4 and populated cards WS15g/e. Search portions belong to M2. Controller vectors do not establish browser or pixel parity.

## Precise remaining work in priority order

1. **Thread show/pane integration:** ordinary show returns 200 and complete pinned components/layout fixtures pass. Finish complete live HTTP shell/chrome bytes after the lead's WS8b-r/r2 merge, including the post-pin status-popup layout. Wire M2's real schedule child at `render_thread_schedule_control`; standalone pinned show itself has no schedule composer. Wire WS15g's populated PR header at `render_thread_pull_request_header` after its owner merge and prove the populated cases. Complete live pane formats/coercions and measured preload/query/race cases. Work/board show/content remain the intentional authorized WS12 501 seam.
2. **Forward coverage:** the processing atomicity gap is fixed and positive complete responses/rows plus owned message/unread frames pass. Finish malformed/nil/destination/source/member races, constant-query measurement, wider image/video/preview/variant/failure combinations and every eligible message-stream recipient permutation. Rails's one captured activity frame remains WS12-owned and unimplemented here.
3. **Upload/state/recipient completion:** signed direct-upload IDs and existing multipart PNG cases pass, but finish signature expiry/missing blob, direct-upload creation endpoint integration, multipart coercions, corrupt/missing metadata/media, retention and processing failure/retry behavior, inline ActionText SGIDs with WS5, and edit/removal combinations. Root/thread attachment processing has Rails's own save/processing boundaries; its complete failure differential is still pending. The 26 states/78 frames are a measured matrix, not the full one: add quote placeholders and source rename/delete, missing/corrupt inline/media metadata, populated/suppressed cards, work/PR/agent lifecycle and locked-thread combinations, M2 poll/pin facts, custom-icon/action/tooltip cases and all viewer/recipient/membership/cache permutations. Named-controller attribution/signoff remains partial as catalogued above.
4. **Deferred controllers/system:** finish the case-level backlog grouped above and run the 19 system files with merged owners; full browser/pixel parity has not been run. Keep WS12 work/board 501 until its provider/domain is merged. No timing test failed this run; no concurrency reduction or threshold widening was used.

No Rails source, room-shell/sidebar, M2 feature implementation, schema/migration/dependency/lockfile, response mask or allowlist changed. No stash, rebase, PR, production action or external message. Co-author trailers are present. Only owned `.scratch/` remains untracked. The report is partial and the lead must merge the owner branches before the named live integration acceptance.
