# WS8bm messaging HTTP — partial six-slice continuation

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm`.
Started from accepted `6375ba3f5b996ad9bc27b5d9c0fd90ae2cc5bae2`.
Final verified implementation input: `94b5e8a062904f1f7c9e4558843c16f12183051e`.
The report/integration-document commit changes documentation only; its pushed SHA is in the final reply.

**PARTIAL.** Six coherent slices were committed and pushed. The final fresh worker-branch seeded suite has zero failures at default test concurrency. The isolated owner integration passes all 11 targeted tests and 8 of 9 exact room components, but its strict byte checker still fails the Designers list's populated card fragments. Full live shell/layout, the remaining state/controller matrix and system signoff are not claimed complete.

No main or owner branch was merged into WS8bm. Main's asset-fingerprint issue was left to its owner. The isolated scratch merge uses published WS8b-r `27990da2851f4c056db71c6b430c894307bc6bfe`, including its M2/WS11/WS17 inputs. The lead still owns the published branch merges. Messaging reference remains `d7c7de92`; post-pin application layout/status popup `2e20b24c` remains WS8b-r2-owned. No timing threshold or test-concurrency reduction was introduced. No timing test flaked in the final runs.

| Commit | Slice |
| --- | --- |
| `014233a8` | Signed upload expiry/purpose/missing-blob boundaries and compiled security discriminators |
| `ea8d4e16` | Every eligible forward recipient, six private-stream refusals and compiled membership discriminator |
| `6c107af6` | Complete boost index/new/actions goldens, autocomplete/profile trigger, distinct counts and hostile tooltip names |
| `973dc215` | Actual room container/footer/pending-template whitespace, stable additive footer API and isolated owner integration |
| `71073f32` | Signed edit rejection with retained attachments/captions; precise boost case attribution |
| `94b5e8a0` | 46-request/44-frame boost matrix, paired Unicode/plain-text cases and two custom-icon reactors |

## Changes by file and design notes

Paths below are relative to `rust/`.

- `crates/campfire/src/controllers/messages.rs`: root create's explicit room-not-found render now raises the Rails MissingTemplate 500 for a requested non-HTML format, rather than returning UnknownFormat 406. This fixes the valid signed ID for a nonexistent blob in a Turbo Stream root create. HTML's existing template behavior is retained. Missing-blob root/thread **edits** remain 404, as their actual Rails requests prove; invalid, expired and wrong-purpose edit capabilities return 500 and leave rows intact.
- `reference-tools/messaging/signed-attachments.rb`, `vectors/messaging/signed-attachments.json`, `controllers/messages/upload_tests.rs`: expand from 9 to 27 real root/thread requests. Complete bodies/headers and message/blob snapshots cover initial file posts, image replacement, nil/empty removal, forged unpermitted blob columns, tampering, wrong purpose, nonexistent blob, expiry at the exact boundary, expired-capability retries, successful file reattachment and eight rejected edits. Rejected edits retain nonempty captions and blob 13. `now` is a clock input captured by Rails, not output normalization. Existing authorization/enqueue rollback test remains in the fresh suite. `upload-discriminate.py` compiles bypasses of expiry clock and signature verification, requires runtime failures and restores sources in `finally`.
- `reference-tools/messaging/forward-success.rb`, `vectors/messaging/forward-success.json`, `channels/tests/hub_test/message_parity.rs`: compare Rails's real `RoomMessagesChannel.subscribable_target` decisions for David, Jason, Kevin and Bender across All Talk, Quiet Corner, a direct room and a thread. Rust's actual WS7 sockets check 16 decisions, six private refusals, all 29 eligible deliveries and all 20 unique owned publisher payloads. One captured Rails activity payload remains the explicit WS12 seam, not a pass. `recipient-discriminate.py` temporarily compiles a WS7 membership bypass and requires the real subscription-rejection test to fail; no production WS7 source change remains.
- `controllers/messages/boosts_tests.rs`, `views/templates/messages/boosts/index.html`, `new.html`, `reference-tools/messaging/boost-pages.rb`, `vectors/messaging/boost-pages.json`: seven actual index/new/actions requests with whole fixed-input components and whole live actions JSON. Fix the missing index newline, reuse the existing avatar/profile-card helper and supply Rails's exact autocomplete/combobox attributes. Duplicate boosts from one reactor count once; viewer-specific active flags match. Interactive-looking reactor names are escaped as text. Actual form requests retain and validate their session's real CSRF token; fixed renderer secrets are explicit inputs to the complete component comparison, never removed/replaced output text.
- `controllers/presenters/room_list.rs`, `views/src/messages/composer.rs`, `templates/messages/_composer.html`, new `_composer_markup.html` and `_footer_composer.html`, `_pending.html`, `templates/channel_threads/_conversation.html`, `channel_threads/content_tests.rs`, `reference-tools/messaging/room-list.rb`, `thread-content.rb`, their vectors: room-list reference capture now uses the actual rendered messages container, including the empty invitation-expression line missed by the old branch-only capture. Keep `room_message_list` and inline `Composer` signatures stable. Add `FooterComposer` sharing the same markup but reproducing `content_for :footer` indentation. Preserve the pending partial's newline and compensate its pane call-site whitespace, keeping complete existing pane bytes unchanged. Whole room footer and pending-template fixtures were added, not inferred from substring checks.
- `reference-tools/messaging/owner-integration-check.py`, `owner-schedule-integration.patch`: reproducible isolated owner merge, generated seeds, locked metadata and TOML duplicate-key check. The small patch wires the already named thread schedule call site to M2's real `ComposerButton`, uses the additive room `FooterComposer` and makes complete pane comparisons invoke that real provider. It is a reviewable patch for the lead after owner merge, not an implementation of M2's feature or a hidden runtime provider. The normal worker branch's schedule call site remains explicitly empty until that merge/application. The strict checker keeps the remaining card difference as a failure and continues the available pane/show checks.
- `reference-tools/messaging/modern-boosts.rb`, `vectors/messaging/modern-boosts.json`, `controllers/messages/boosts_tests.rs`, `channels/tests/hub_test/message_parity.rs`: 46 actual writes compare full status/header/body and stored-row snapshots; all 44 rendered replacements are delivered through WS7 and its session-bound-value guard. Add paired flags, keycaps, family sequences, VS16, skin modifiers, non-quick emoji, trailing-space fire/brand autocomplete, repeated digits/letters and two custom-icon reactors toggling in changed insertion order. Existing failure rollback, concurrent toggle, bot/CSRF/member/other-reactor refusal tests also pass. Modern bot endpoints remain WS11-owned.
- `reference-tools/messaging/reference-check.py`: add the two boost page sources, validating 63 pinned controller/model/helper/template/icon files. `check-goldens.py` now runs 19 Rails oracles and compares 20 committed files. `plans/ws8bm-integration.md`, `ws8bm-controller-cases.md` and this report document exact entry points, case evidence and incomplete owner signoff.

## Exact shell/list/composer seam

`Presenter::messages(&records)` and `messages::Index { ctx, messages }` remain unchanged. WS8b-r calls `presenter.room_message_list(&selected_roots, divider.message_id, divider.count)?` for `ShellComponents.message_list`. Set `Presenter.cache_base_url` to the verified request origin and use the application fragment cache. The returned string includes the empty invitation line; mount it verbatim. WS8b-r owns root/around selection, invitation content, membership cursors, read effects, divider count/ID, scroll and jump facts. The adapter locates the divider by record ID outside the shared message fragments. Nonempty invitation/full-parent combinations still require the room owner's broader live-shell signoff.

The merged shell must pass the live `ViewContext` and CSRF provider, `Presenter::composer_facts(room, viewer, thread, drive)`, viewer `UserView`, ordered built-in/agent command names, and Google's resolved availability via `composer_drive_flow(viewer, share_picker_available)`. Use `thread: None` and **`FooterComposer`** in the room's footer; keep **`Composer`** for inline/thread panes. Supply M2's actual `scheduled_messages::ComposerButton { ctx, room_id, thread_id }` as trusted `scheduled_control`. Render it under the same request context, without caching the composer. Mount `channel_threads::PendingTemplate { ctx, user }` for optimistic messages.

Panes additionally need scoped message items, optional same-thread anchor, room `updated_at`, ordered thread steps, viewer, composer facts and schedule child. Apply `owner-schedule-integration.patch` after the owner merge to wire this at `render_thread_schedule_control`; no owner module is copied into WS8bm. Standalone pinned thread show has no composer/schedule child: `/content` owns those. Its ordinary HTML remains live. `render_thread_pull_request_header` stays the clearly named WS15g header call site. WS12 activity and work/board panes remain flagged seams, with authorized work/board HTML show/content 501.

## Commands re-run this session and raw summaries

### Final fresh clone of the committed implementation

```sh
python3 rust/reference-tools/messaging/fresh-check.py > .scratch/fresh-final.log 2>&1
```

The helper executed these inside the new clone with `CI=1`, its own new target and scratch, default test concurrency and generated seeds:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
bash rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_db
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_views --test core
mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

```text
WS8bm fresh checkout: 94b5e8a062904f1f7c9e4558843c16f12183051e; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-yma_uqgt
WS8bm fresh concurrency: default test threads; four build jobs; no timing threshold changes
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 41s
test result: ok. 473 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 42.06s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 19.09s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 39.46s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `test` profile [unoptimized + debuginfo] target(s) in 18.17s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.41s
WS8bm fresh-check: committed inputs only; generated default/first_run seeds; app/db/views/clippy passed
```

No seed skips or new ignores occurred. Three existing app ignores are Rails recording, WS11's `manages_bots` and push-latency measurement. Four existing DB ignores are the external Rails scenario/export/fixture/rollback hooks. The zero-test DB line is zero discovered doctests. Tests read committed vectors and newly generated seeds/storage, not `.scratch` fixtures or a pre-existing Cargo target. Removed only two older owned clone targets to make space, after checking no active clone processes; their source clones and logs remain.

### Locked metadata and dependency keys

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
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

Metadata exited zero; the parser rejects duplicate TOML keys. No dependency/lockfile changes on the worker branch.

### Pinned source identity and complete golden rerun

```sh
python3 rust/reference-tools/messaging/reference-check.py > .scratch/reference-current.log 2>&1
python3 rust/reference-tools/messaging/check-goldens.py > .scratch/goldens-final.log 2>&1
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
WS8bm golden check: 19 Rails oracles re-run; 20 golden files byte-identical
```

### Isolated owner integration: explicit failure retained

```sh
python3 rust/reference-tools/messaging/owner-integration-check.py --target-dir rust/target > .scratch/owner-integration-current.log 2>&1
```

This ran against `973dc215` plus the pinned owner branch, with freshly generated seeds. Its optional target is reusable build output; this is not the final fresh-build claim above. It exits **1**, rather than accepting missing provider output.

```text
WS8bm owner integration: worker 973dc215c7ffd5508716e0a59de4b092a607b852; shell 27990da2851f4c056db71c6b430c894307bc6bfe; isolated merge /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-owner-1t5dzm9n; main unmerged
WS8bm owner manifests: 13 parsed; zero duplicate workspace dependency keys
native:     Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 03s
native: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 976 filtered out; finished in 1.57s
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
pane:     Finished `test` profile [unoptimized + debuginfo] target(s) in 22.19s
pane: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 977 filtered out; finished in 1.52s
show:     Finished `test` profile [unoptimized + debuginfo] target(s) in 12.53s
show: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 976 filtered out; finished in 1.20s
AssertionError: Owner integration failures: ['native component bytes']; all available checks attempted
```

Before the owned whitespace fixes, the same strict component comparison compiled and rejected all nine components despite four passing presence/selection/token tests. After the fixes, all composer/pending slots and two complete lists match; the Designers list's 2,532-byte difference is populated GitHub PR, Fizzy lazy frame, link-embed and LinkedIn output. WS15g/WS15e own those providers. Full live layout/status-popup facts and whole-page bytes are not established by the 11 targeted tests or these component comparisons.

### Failing-first security/render evidence and targeted checks

Authorization tests already passed before successful upload/boost behavior was credited. Earlier this session, the expanded root upload test compiled and found 406 instead of Rails's 500 for a valid missing-blob capability; the boost page test compiled and found the missing index newline. These raw failed summaries precede their fixes:

```text
upload-capability-before:
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 473 filtered out; finished in 0.60s
boost-pages-before:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 475 filtered out; finished in 0.52s
```

The following commands were executed this session before the later vector-only widening; their compiled security regressions were rejected and source restoration was verified:

```sh
python3 rust/reference-tools/messaging/upload-discriminate.py > .scratch/upload-discrimination-current.log 2>&1
python3 rust/reference-tools/messaging/recipient-discriminate.py > .scratch/recipient-discrimination-current.log 2>&1
```

```text
expiry-clock: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.76s
signature-verification: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.91s
WS8bm upload discriminator: 2 compiled expiry/signature regressions rejected; source restored and differential passed
private-stream-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 474 filtered out; finished in 0.48s
WS8bm forward recipients: 16 subscription decisions; 6 private-stream refusals; 29 delivered frames; 20 unique owned frames; 1 WS12 activity frame deferred
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 474 filtered out; finished in 5.82s
WS8bm recipient discriminator: compiled private-stream regression rejected; source restored and all eligible deliveries passed
```

Final expanded boost checks also ran under `CI=1`, owned `TMPDIR`/`CARGO_TARGET_DIR`, `CABLE_TEST_PORT_RANGE=52000-52049`, `MAIL_TEST_PORT_RANGE=52000-52049` and default test concurrency:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::messages::boosts_tests -- --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire channels::tests::hub_test::message_parity::modern_reaction_replacements_match_rails_bytes_through_ws7 -- --exact --nocapture
```

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 472 filtered out; finished in 1.22s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 475 filtered out; finished in 14.28s
```

## Grouped controller evidence and deferred signoff

Final fresh app output has **83 passing owned Rust test functions**, zero failures/ignores in these groups. This is not a one-to-one Rails case count.

```text
channels::tests::hub_test::message_parity: 7 ok
controllers::channel_thread_messages::tests: 3 ok
controllers::channel_thread_messages::write_tests: 3 ok
controllers::channel_threads::content_tests: 3 ok
controllers::channel_threads::page_tests: 4 ok
controllers::channel_threads::tests: 5 ok
controllers::channel_threads::write_tests: 5 ok
controllers::message_forwards_tests: 7 ok
controllers::messages::boosts_tests: 4 ok
controllers::messages::collection_tests: 2 ok
controllers::messages::http_tests: 15 ok
controllers::messages::paging_tests: 4 ok
controllers::messages::room_list_tests: 1 ok
controllers::messages::root_tests: 9 ok
controllers::messages::state_tests: 1 ok
controllers::messages::tests: 8 ok
controllers::messages::upload_tests: 2 ok
WS8bm owned test groups: 83 passed; 0 failed; 0 ignored
```

The exact 156 named reference declarations and their owners are in `plans/ws8bm-controller-cases.md`. Three boost cases now have explicit component/action attribution; room/browser signoff remains separately marked. Expanded modern/upload oracle counts are real request counts, not additional named Rust test-function counts.

The pinned Rails files were independently re-run. These are **Rails reference counts only**, not 156 completed Rust ports:

```sh
python3 rust/reference-tools/messaging/check-controller-files.py > .scratch/controllers-current.log 2>&1
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

| File under `test/controllers/` | Precisely remaining |
| --- | --- |
| `messages_controller_test.rb` | WS8bm: remaining timezone/edited-time/bodyless/identical-save, quote/source rename/delete, uncommon formats/shapes, measured query/cache and recipient race cases. WS15g/e: populated/add/remove/re-sync card cases. M2: pins/validators. WS11: webhooks/agents. WS8b-r/r2: full live shell/layout. |
| `messages_drive_attachments_test.rb` | WS8bm: direct-upload/storage integration, edit/removal and processing/media failures; WS14g: Google consent/Drive flows; WS5: inline ActionText SGIDs. Browser flows remain end-to-end work. |
| `messages/cached_fragment_csrf_test.rb` | Full live merged cached pages/refresh/thread forms with all viewer/cache/race permutations. Current complete fixed-input component and real-token HTTP checks remain separate evidence. |
| `messages/legacy_presentation_cache_test.rb` | Explicit named rollout invalidation and broader warm/cold/off-page source permutations; accepted key/ETag tests remain passing. |
| `messages/boosts_controller_test.rb` | Named one-to-one file signoff and live room/index parent integration; broader concurrent cache/viewer/thread recipients and modern WS11 bot endpoints. New 46 writes/44 frames plus seven full picker/action components cover paired Unicode/plain text, two custom reactors, distinct counts and escaped tooltips; they are not browser interaction signoff. |
| `channel_threads_controller_test.rb` | Complete live shell/chrome and post-pin layout, M2 runtime schedule patch application, WS15g populated PR header, membership/lock/staleness races and measured preload/query assertions. WS12 conversion/status/ownership/history and WS11 agent pickers remain with owners. |
| `channel_thread_messages_controller_test.rb` | Remaining joined-recipient/preferences, uncommon shapes/formats, lock/membership races and whole merged parent redirects/panes; populated add/remove cards with WS15g/e. |
| `channel_thread_messages_drive_attachments_test.rb` | Wider storage/Drive/deletion/corrupt-media/inline combinations and processing failures; signed create/edit expiry/purpose/missing/tamper preservation is now in the 27-request matrix. |
| `message_forwards_controller_test.rb` | Malformed/nil inputs, source/destination/member races, measured constant-query assertion, wider media/preview/variant failures and membership/preference/cache permutations. All four current recipient roles and their private refusals now execute; WS12 activity remains unimplemented. |
| `message_forward_sources_controller_test.rb` | Unusual coercions/formats and deletion/membership races; existing canonical source URL/null privacy/no-store bytes pass. |

**System execution: zero this run.** Per instruction, system/browser/pixel signoff waits for the end-to-end phase and owner merges; controller counts are not a substitute. No new system passes/skips are claimed.

## Remaining priority work and cross-workstream boundaries

1. Finish whole **live merged** shell/layout bytes after lead owner merges. Apply the checked schedule/footer patch; wire WS15g's named PR header and WS15g/e's four populated message-card providers, rerun the strict checker to zero differences, then the post-pin WS8b-r2 application/status-popup layout and nonempty invitation/full-parent cases. The current ordinary standalone thread show is live; the worker schedule call site is still explicitly empty before owner integration.
2. Complete upload/state/recipient combinations beyond the measured 27-upload/46-boost/26-state matrices: corrupt/missing/inline media metadata, processing failure/retry/retention, quote/source rename/delete, suppressed/populated cards, work/PR/agent and locked-thread combinations, all cache/viewer/member/preference races and direct-upload endpoint/browser integration. Do not infer these from the passing aggregates.
3. Finish the grouped named-controller backlog above, including measured query/preload assertions. M2 still owns polls, pins, saved/reminder/scheduled features, search, slash commands, autocomplete, message links and room files; WS8bm added no implementation of those features.
4. WS12 activity frame and authorized work/board HTML panes remain flagged seams/501. System signoff stays in the end-to-end phase. Modern bots remain WS11-owned; Google flows WS14g-owned; populated cards WS15g/e-owned; room chrome/layout WS8b-r/r2-owned.

No Rails source, schema/migration, dependency/lockfile, response mask, allowlist or timing threshold changed on WS8bm. No stash, rebase, PR, production action or external message. Co-author trailers are present. Only owned scratch output remains untracked. The external report and tracked mirror are identical. No independent-review findings had been relayed when this report was written.
