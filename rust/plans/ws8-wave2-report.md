# WS8a messaging models — partial handoff, 2026-09-29

Branch: `rust/ws8-messaging-models`. Verified code commit: `47edc10b` (the following report commit changes documentation only). This is **partial WS8a**, not acceptance of the entire brief. Controllers, templates and rendered broadcasts remain WS8b. No PR was opened. The user explicitly requested pushing, overriding the common brief's no-push default.

## Recovery and committed slices

At takeover, Opus had committed threads (`62e417ff`), edits/forwarding (`5eca0d11`) and polls (`6cea6f45`). Its uncommitted work contained pins, saved items, a minimal activity writer, dependencies, tests and rollback/timestamp changes. That work was preserved first in `5b373ef2`, with validation explicitly still pending.

Subsequent coherent slices:

- `5c57359b`: pins/saved-item verification, Rails rollback validation and regenerated save-touch golden.
- `9ccb91e3`: keyword alerts and matcher.
- `276b785d`: scheduled-message models and transactional dispatcher.
- `722f3fc0`: categories and membership favorites.
- `8f3dd903`: merge the completed WS3/WS5 work and the Rails fixes at the decisions file's current reference pin, `fec615be`, preserving history.
- `47edc10b`: real WS3 queue rollback/retry tests for messaging writes and their discrimination checks.

The recovered domain files were read against our Ruby, not treated as finished merely because they compiled. No stash, rebase, shared target directory, release build, PR, controller or template edits were used.

## Changes by file

Paths below are relative to `rust/`; entries describe WS8 changes relative to merged `origin/main`, rather than attributing inherited WS3/WS5 changes to WS8.

| File | Change |
|---|---|
| `crates/db/src/models/channel_thread.rs` | Core thread lifecycle, validations/defaults, tags, membership/posting, unread handling, counters/indicator data, stale closure, thread destruction dependencies, push arguments/payload. Persist thread push in the triggering message transaction. Board/work behavior remains partial. |
| `crates/db/src/models/thread_membership.rs` | Join/leave, involvement and unread state. |
| `crates/db/src/models/thread_tag.rs` | Normalization, required name, length and per-thread uniqueness. |
| `crates/db/src/models/message.rs` | Thread-aware creation/edit/deletion, reply tombstones/stream validation, edited timestamps, forwarding metadata and mention snapshots, dedup, flags, thread counters/indicators, dependencies on polls/pins/saves, and precommit conversation push enqueue. |
| `crates/db/src/models/forwarder.rs` | Multi-destination forwarding, destination guards, metadata, cloned blob/Drive attachment bookkeeping and failure cleanup through a copier interface. Real storage adapter remains partial. |
| `crates/db/src/models/poll.rs` | Poll/option/vote records, validation, ballot replacement, closing and typed replacement broadcast. |
| `crates/db/src/models/message_pin.rs` | Idempotent pin/unpin, cap 50, quiet pin-note rate limit, parent timestamp updates and three panel/badge broadcasts. |
| `crates/db/src/models/saved_item.rs` | Status, accessibility, reminder rearming, due selection/claim, activity dependency, reminder job and push-payload builder with policy callback. |
| `crates/db/src/models/activity_item.rs` | Minimal shared reminder/drop inbox writer, event vocabulary, unread refresh, source destruction and user activity event. Not the complete WS12 inbox model. |
| `crates/db/src/models/scheduled_message.rs` | Validation/history, draft edits, five-minute claim, access checks, thread-drop callback, transactional dispatch, root/thread events and legacy bot fanout; per-row Database dispatcher. |
| `crates/db/src/models/keyword_alert.rs` | User-scoped CRUD, normalization/cap/length/duplicate validation, independently matched literal phrases and Unicode boundaries. |
| `crates/db/src/models/room_category.rs` | Per-user ordered categories, name validation, collapse state, next position and dependent membership nullification. |
| `crates/db/src/models/membership.rs` | Category ownership validation, category assignment, favorite/unfavorite and clamped absolute reorder with compact positions. |
| `crates/db/src/models/push_subscription.rs` | Subscription enumeration needed by reminder payload building. |
| `crates/db/src/rich_text.rs` | Model Markdown/canonicalization/plain-text seams and test stand-in. The production adapter is still incomplete. |
| `crates/db/src/broadcasts.rs`, `events.rs` | Typed Turbo/Cable event descriptions and accessors. The renderer/delivery contract still needs WS7/lead coordination. |
| `crates/db/src/models.rs`, `lib.rs`, `tests.rs` | Exports and test modules. |
| `crates/db/Cargo.toml`, `Cargo.lock` | Workspace regex dependency for keyword matching. |
| `crates/db/src/tests/channel_thread_test.rs`, `message_edit_test.rs`, `forwarder_test.rs`, `poll_test.rs`, `message_pin_test.rs`, `saved_item_test.rs`, `scheduled_message_test.rs`, `keyword_alert_test.rs`, `room_category_test.rs` | Domain tests, real SQLite claims, rollback, timestamp and typed-event assertions. |
| `crates/db/src/tests/fixtures_test.rs` | Export representative rows for Rails: threads/tags/memberships, forwards/edits/replies, polls/votes/options, pins/saves/activity, keywords/categories/favorites and scheduled pending/sent/dropped history. |
| `crates/db/ruby/rollback.rb` | Rails `valid?` for all exported changed rows plus domain read/write/delete checks. |
| `crates/db/ruby/save_touches.rb`, `crates/db/src/tests/save_touches_test.rs`, `message_save_touches.json` | Extend the Rails-produced timestamp golden to 64 cases, including pin/unpin/save/schedule. JSON was generated from Rails, not invented. |
| `crates/campfire/src/jobs.rs` | Minimal exhaustive-match integration for `Event::Broadcast`; keep the merged WS3 durable job implementation. Broadcasts currently are not delivered by this sink. |
| `crates/jobs/src/tests.rs`, `tests/ws8_messaging_test.rs` | Three tests against WS3's actual `JobQueue`, injecting a trigger that rejects job inserts and proving rollback and successful retry. |
| `reference-tools/db/differential.sh` | Mount this checkout's test files, use cargo `-j 4`, and allow a workstream-specific reference image. |
| `reference-tools/db/ws8-discrimination.py` | Twelve deliberate regressions; require real failing tests, reject compilation-only failures, restore sources after each mutation. |
| `plans/ws8-wave2-report.md` | Tracked copy of this handoff report. |

## Design and partial boundaries

- The schema remains Rails-owned. WS8 added no migrations. The merged WS3 `background_jobs` migration is included in schema/differential/replay verification.
- Writes and jobs share SQLite. Thread push enqueue moved out of the after-commit receive callback into Message's transaction. The real queue tests prove a rejected enqueue rolls back the post, thread state/membership, scheduled claim/history or reminder claim/activity as applicable. Full HTTP trigger tests remain **partial**, owned by WS8b with WS3.
- Scheduled claims use a conditional SQL update inside a real write transaction. A claim exactly five minutes old remains held; strictly older claims can be recovered. Four independent SQLite connections contest one row in the domain test. Failed SQL writes release the claim through rollback; locked threads explicitly release it for retry.
- Saved reminder dispatch rereads the row under the writer transaction. Concurrent dispatchers notify once. Its `dispatch_due(Tx, ...)` convenience helper batches rows in one transaction; runtime registration must instead use `due_reminder_ids` plus a separate `dispatch_reminder` write per row, rescuing/logging each error. That runtime error-isolation path is **partial**.
- The model calls the RichText trait for Markdown. **Production `AppRichText` still inherits the BasicRichText Markdown/canonicalization defaults**; merging WS5 alone did not wire its renderer. Scheduled-message rendering and forwarding storage integration are therefore **partial**.
- Keyword tests cover every case in the two named Ruby files, including overlap, punctuation, flexible whitespace and Unicode boundaries. Ruby's untyped `Array`/`to_s` coercions and multi-character Unicode case folding are not proven; these remain **partial** at the future input boundary.
- No parity masks or allowlists were changed. The app-wide test suite and browser parity were not run; there is no production/cutover claim.

## Feature × Rails validations/callbacks

“Ported” describes the specified model helpers and tested cases, not complete end-to-end acceptance. Every unresolved integration or unproven parity edge is marked partial.

| Brief feature | Ported validations, scopes and callbacks | Partial/deferred and owner |
|---|---|---|
| 1. Threads | Required room/creator/name; name limits/defaults; parent-room and board constraints; archive/stale/lifecycle rules; tags and thread memberships; unread updates; recount excludes system notes/unfinished streams; parent stamp/indicator; root timeline excludes replies; atomic thread push request. | Board/work/assignment/handoff behavior: WS12 with WS11 agents. Hard-user-destroy callback path: WS11. Broadcast-failure isolation scenarios and push delivery: WS8a follow-up/WS7/WS17. |
| 2. Replies/edits/forwarding | Reply conversation checks and tombstones; required body/source and existing Message limits; source/body edited timestamp rules; forward metadata pairs; destination membership/board/locked guards; mention snapshots; client-id dedup; system_note/action/embeds flags; transactional attachment bookkeeping. | Legacy conversions/real WS5 adapter; real blob copier/storage cleanup: WS8a with WS5/storage owner. Cross-domain reference resync is missing. HTTP decoding/auth and append/replace rendering: WS8b. |
| 3. Polls | Message presence/uniqueness; no streaming poll; 2–10 normalized labels; option required/length/position; future close time on create; vote user/poll/option association and uniqueness; ballot replacement/closed guards; message touch, dependency destruction and poll replacement event. | `close_due` exists but is not scheduler-registered: WS8a/WS3. Ruby odd-shaped external coercions and close-loop error isolation need follow-up; rendering WS7/WS8b. |
| 4. Pins/saves | Pin required message/room and uniqueness, cap/idempotence/ordering; quiet system-note throttling; pins_changed_at/message stamp, not room ordering; pin/saved message dependencies; saved required user/message, uniqueness/status/future changed reminder; accessibility/due scopes; rearm; reminder activity refresh; claim/job/payload. | Hard user deletion: WS11. Full ActivityItem accessibility/source resolution: WS12. Per-row periodic registration: WS8a/WS3. Real DND/quiet-hour policy and delivery: WS17. |
| 5. Scheduled | Required user/room/source; 50,000-character limit; changed future send_at; thread/reply same conversation; active-human/live-room/membership sendability; history ordering; claim/recheck; locked-thread retry; missing-access drops; thread deletion drops pending only; sent/reply FK nullification; nonstreaming post and notifications. | Actual WS5 renderer and scheduler registration: WS8a with WS5/WS3. Full ActivityItem accessible scope: WS12. Render/delivery: WS7/WS8b/WS17. |
| 6. Keywords | Required user/phrase; normalization, 80 characters, SQL LOWER duplicate semantics, cap 20 on create; literal independent matcher passes, insertion order and user dedup. | Recorder integration: WS17. Untyped coercions and broader Unicode case-fold parity: WS8a/WS8b follow-up, partial. No model broadcasts applicable. |
| 7. References/cards | Existing Message deletion cleanup and a typed `QuoteCards` partial placeholder only. | **Not ported**: MessageReference validation/model, ReferenceSync, refresh job and edit/removal/resync callbacks. WS8a follow-up; rendering WS7/WS8b. |
| 8. Categories/favorites | Category user/name presence, 50 characters, per-user position/id ordering, collapse, next position; destruction nullifies category assignments without touching memberships; membership required user/room and same-user category; favorite append/idempotence/unfavorite/reorder. | HTTP current-user scopes, channel restrictions and authorization: WS8b. No model broadcast or parent touch applicable. |
| 9. Group DMs | WS2's existing member key and unkeyed lookup retained. | **Not ported by WS8**: MAX_MEMBERS 10, group mutations, rename/add-member system notes. WS8a follow-up. |
| 10. Soft delete | Existing WS2 alive filters retained; new saved/scheduled sendability/due queries account for deleted rooms. | **Not ported**: begin_destroy, destroy job batches 500, stuck reenqueue, complete room-reader audit. WS8a follow-up with WS3. |
| 11. Search | WS2 dedup/index baseline retained; thread-aware Message edit/delete paths extended. | **Not ported**: SearchQuery operators, cursor pagination, complete skip/update audit. Board/work/events sections require explicit WS12/WS14 seams. WS8a follow-up. |
| 12. Slash commands | No new registry/dispatcher/time parser or handlers. | **Not ported**: shrug, me, remind, status, dnd, ooo, poll; explicit huddle/event/play stubs still needed for WS13/WS14. WS8a follow-up. |
| 13. Retention/audit | No new retention or audit implementation. | **Not ported**: prune jobs/registration, append-only AuditLog, redaction, sign-in throttle and action vocabulary. WS8a follow-up with WS3/WS4 as needed. |

ActivityItem's limited writer validates the recipient and event vocabulary and only broadcasts to active humans. Its full inbox scopes, source policy, huddle payload and other event-specific behavior are deferred to WS12/WS13; it must not be mistaken for a complete ActivityItem port.

## Broadcasts emitted

All are typed `Event::Broadcast`; none asserts WS7 has approved or delivered the shape.

| Trigger | Stream / action / target / data |
|---|---|
| Thread reply count, finalization or destruction | Parent room `:messages`; replace with maintain_scroll; `thread_indicator_message_<client_id>`; parent id and committed reply count. |
| Thread receive | `user_<id>_unread_threads`; Cable `{threadId, roomId}`. |
| Poll vote/close | Message conversation `:messages`; replace with maintain_scroll; `poll_<id>`; poll id. |
| Pin/unpin | Room `:messages`; three replacements with maintain_scroll: message pin badge, room pins count, room pins list; message/room ids. |
| Pin note | Room `:messages`; append to the STI room messages target; note message id. System notes cause no push/unread fanout. |
| Activity create or unread refresh | `user_<id>_activity`; Cable `{activityItemId}` to active human only. |
| Scheduled send | Thread or room `:messages`; append to that conversation messages target; message id. Root sends also emit `user_<id>_unread_rooms` / `{roomId}` according to involvement/mention rules. |

The current app jobs sink does not render/deliver these broadcasts. Message edit/remove HTTP broadcasts, room removal, and quote-card refresh remain partial. `QuoteCards` is a type placeholder, not an emitted implemented callback.

## Ruby test counts versus Rust coverage

Counts are named Ruby test scenarios, not a claim that the Ruby Minitest files were executed here. Rails runners were executed for the differential/rollback checks below. Consolidated scenarios and added Rust checks make function totals different.

| Ruby file under `test/` | Ruby scenarios | Covered scenarios / Rust tests | Remaining |
|---|---:|---|---|
| `models/channel_thread_messages_count_test.rb` | 19 | 16/19; 16 Rust functions in the count group, including an extra finalize-claim test and a consolidated finalize scenario | Two broadcast-failure isolation scenarios; hard-user-delete scenario. |
| `models/message_conversations_test.rb` | 5 | 3/5; 3 Rust | Two legacy conversion scenarios. |
| `models/thread_tag_test.rb` | 2 | 2/2; 2 Rust | None in this file; all three groups plus 9 core-thread checks total 30 Rust tests. |
| `services/messages/forwarder_test.rb` | 8 | 8/8 modeled scenarios; 9 Rust | Real storage/Markdown adapter remains partial despite modeled coverage. |
| `models/poll_test.rb` | 15 | 15/15; 19 Rust | Scheduler/delivery outside this file remain partial. |
| `models/message_pin_test.rb` | 16 | 15/16; 17 Rust | Hard-user-delete broadcast scenario, WS11. |
| `models/saved_item_test.rb` | 12 | 11/12; 13 Rust in model group | ActivityItem accessible_to after room access loss, WS12. |
| `models/saved_item/reminder_dispatcher_test.rb` | 7 | 7/7; 9 Rust | Runtime per-row scheduler adapter is partial. |
| `models/saved_item/reminder_pusher_test.rb` | 4 | 4/4 through payload/policy seam; 5 Rust | Actual policy/delivery partial, WS17. All saved groups total 27 Rust tests. |
| `models/scheduled_message_test.rb` | 8 | 8/8; 8 Rust | Full ActivityItem query privacy and renderer integration remain partial. |
| `models/scheduled_message/dispatcher_test.rb` | 13 | 13/13 modeled scenarios; 13 Rust plus 4 SQLite/state checks | Runtime registration/real renderer partial. Combined scheduled tests: 25. |
| `models/keyword_alert_test.rb` | 4 | 4/4; 4 Rust | Broader input/coercion parity remains partial. |
| `models/notifications/keyword_matcher_test.rb` | 10 | 10/10; 10 Rust | Broader Unicode folding remains partial. Combined keyword tests: 14. |
| `models/channel_thread_board_test.rb` | 16 | 0/16 | WS12. |
| `models/channel_thread_auto_assign_test.rb` | 10 | 0/10 | WS12/WS11. |
| `models/channel_thread_agent_assignment_test.rb` | 29 | 0/29 | WS12/WS11. |
| `models/channel_thread_handoff_test.rb` | 11 | 0/11 | WS12/WS11. |

Additional suites: 11 message edit/reply/flag/dedup checks, 8 category/favorite model checks derived from controller scenarios (**zero HTTP controller tests claimed ported**), and 3 WS3 durable queue integration tests. These do not imply entire message/controller Ruby suites are ported.

## Final verification — exact commands and raw summaries

Commands ran after the final code changes. Root cwd is `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8`, except the differential command explicitly uses its `rust/` subdirectory. Logs/scratch are ignored under `.scratch/`; every Cargo invocation uses `-j 4` and this worktree's target. No published ports were used.

### Database and real durable queue suites

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs > .scratch/tests-final.log 2>&1
```

Raw lines in order: DB unit tests, jobs unit tests, jobs crash integration tests, DB doc tests, jobs doc tests.

```text
test result: ok. 332 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 6.02s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The 3 ignored DB tests are the opt-in reference fixture/scenario/export tests; all three were separately executed next. DB and jobs suites do not silently depend on the app parity seed. App HTTP/browser and the full workspace test suite were **not run**.

### Workspace clippy

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

Exit 0; raw summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.74s
```

### Rails reference identity

The isolated image `ws8-reference-models` overlays this checkout's `db/` on the existing reference image pinned by digest `83d1ae45158680dc665489d1e1cb7c053ac3a860c0e2841d6f19b29f731329f5`. It does not replace the shared reference image. This check verifies the actual 18 model/service sources used and the entire schema/migrations tree.

```sh
docker run --rm --name ws8-reference-source-check --entrypoint "" -v "$PWD/app:/oracle:ro" -v "$PWD/db:/oracle-db:ro" ws8-reference-models sh -ec 'for file in models/channel_thread.rb models/thread_membership.rb models/thread_tag.rb models/message.rb services/messages/forwarder.rb models/poll.rb models/poll_option.rb models/poll_vote.rb models/message_pin.rb models/saved_item.rb models/saved_item/reminder_dispatcher.rb models/saved_item/reminder_pusher.rb models/scheduled_message.rb models/scheduled_message/dispatcher.rb models/keyword_alert.rb models/notifications/keyword_matcher.rb models/room_category.rb models/membership.rb; do cmp "/rails/app/$file" "/oracle/$file"; done; diff -r /rails/db /oracle-db; echo "WS8 reference: 18 implementation files and Rails db match worktree"' > .scratch/reference-check.log 2>&1
```

```text
WS8 reference: 18 implementation files and Rails db match worktree
```

### Differential fixtures/scenario/export, timestamps and Rails rollback

Cwd: the worktree's `rust/` directory.

```sh
OUT="$PWD/../.scratch/differential" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models CARGO_TARGET_DIR="$PWD/target" TMPDIR="$PWD/../.scratch" mise exec rust@1.98.1 -- bash reference-tools/db/differential.sh > ../.scratch/differential-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 332 filtered out; finished in 5.21s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
message_save_touches.json matches the reference
schema.sql, reference db:prepare and Rust prepare agree
validated 104 rows Rust wrote: {"accounts" => 1, "action_text_rich_texts" => 19, "activity_items" => 2, "boosts" => 1, "channel_threads" => 2, "drive_attachments" => 3, "keyword_alerts" => 1, "memberships" => 22, "message_pins" => 1, "messages" => 21, "poll_options" => 4, "poll_votes" => 3, "polls" => 2, "room_categories" => 1, "rooms" => 7, "saved_items" => 2, "scheduled_messages" => 3, "searches" => 1, "sessions" => 2, "thread_memberships" => 2, "thread_tags" => 2, "users" => 2}
rollback ok
```

This is representative row validation, not exhaustive proof for every possible domain input. The exported DB uses the model test sink, so durable job rows are proven by the separate real-queue tests instead.

### Migration replay and comparator discrimination

Root cwd:

```sh
TMPDIR="$PWD/.scratch" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models bash rust/reference-tools/db/check-migration-replay.sh > .scratch/migration-final.log 2>&1
```

```text
self-test: all 6 mutations caught
migration replay matches schema.sql: 1279 facts, 87 tables with an id, 248 indexes, 128 versions
```

### Tests shown failing against deliberately broken implementations

These are actual test failures, not compiler errors. They were injected after implementation and restored, so they are discrimination evidence rather than invented chronological “failing first” claims.

```sh
python3 rust/reference-tools/db/ws8-discrimination.py > .scratch/discrimination-final.log 2>&1
```

```text
keyword-rules: detected (14 failing tests)
test result: FAILED. 0 passed; 14 failed; 0 ignored; 0 measured; 321 filtered out; finished in 0.17s
scheduled-claim: detected (4 failing tests)
test result: FAILED. 21 passed; 4 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.69s
scheduled-validation: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.65s
scheduled-access: detected (5 failing tests)
test result: FAILED. 20 passed; 5 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.66s
scheduled-thread-drop: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.66s
category-validation: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 327 filtered out; finished in 0.23s
category-nullify: detected (1 failing tests)
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 327 filtered out; finished in 0.19s
favorite-writes: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 327 filtered out; finished in 0.31s
transaction-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 334 filtered out; finished in 0.08s
save-touch-golden: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 334 filtered out; finished in 0.14s
thread-job-atomicity: detected (2 failing tests)
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.09s
reminder-job-atomicity: detected (1 failing tests)
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.14s
WS8 discrimination: 12 mutations detected; sources restored
```

The harness asserts every keyword test failed under broken normalization/validation/matching; other mutations assert named claim, access, dependency, organization, rollback and enqueue failures. Other recovered/new tests were rerun but have not each been independently shown failing; their **individual discrimination proof remains partial**. No blanket proof is claimed for the 160 WS8 database tests or the entire 332-test DB suite.

## Cross-workstream touches

- WS2: `message.rs`, `membership.rs`, `push_subscription.rs`, shared RichText/events/exports, fixture/rollback/timestamp tooling. Extensions are listed above; existing core/schema decisions were retained. `room.rs`, `user.rs` and `session.rs` have no WS8 diff against merged main.
- WS3: test module plus three integration tests in `campfire_jobs`; minimal app jobs exhaustive match; resolve the merge by retaining durable enqueue hooks and moving thread push into the triggering write. No queue redesign.
- WS5: trait seam only; no renderer changes. Real app integration is explicitly outstanding.
- WS7: new broadcast description types; coordination/renderer/delivery outstanding. No cable or template implementation changes.
- WS12: minimal ActivityItem writer required by reminders/drops; complete inbox behavior deferred.
- Shared Cargo registry/lock and DB reference script edits are small and necessary for this workstream.
- The merged Rails fixes and background_jobs migration are inherited from `origin/main`, not authored by WS8. No Rails source changes were made in this workstream.
- The report path outside the worktree is the user-requested exception. A tracked copy is included on the branch.

## Precise remaining work / restart point

The remaining scope exceeds a safe single handoff session; the committed slices above are useful, but the brief is not complete.

1. Wire WS5 into `AppRichText` (`render_markdown`, canonicalization and Markdown plain text), using real room-member SGIDs and the workspace/brand icon resolver; test scheduled/forwarded/edit rendering against Rails. Connect Forwarder's copier to real storage and prove filesystem cleanup/rollback.
2. Register polls, saved reminders and scheduled dispatcher in WS3's periodic host. Saved reminders and poll closing need per-row writes and rescue isolation. Register thread/reminder push job handlers with WS17's actual policy/pusher. The unregistered domain job classes can be persisted today, but the runner cannot execute them successfully without those handlers.
3. Implement MessageReference, ReferenceSync and quote-card refresh/removal/resync jobs/callbacks, including edit/deletion interactions and real job-insert rollback tests.
4. Finish group DM cap/mutations/rename/add-member notes on the existing WS2 member-key/lookup foundation.
5. Implement room soft delete, batches of 500, stuck sweeps and a complete alive-room reader audit; include job atomicity and recovery tests.
6. Implement SearchQuery grammar and cursor pagination; audit dedup/FTS updates/skips; expose explicit board/work/events extensions for WS12/WS14.
7. Implement non-agent slash registry, dispatcher and timezone-aware time parser; implement listed domains and explicit huddle/event/play stubs for WS13/WS14.
8. Implement retention jobs/registration and append-only audit log, redaction, throttling and action vocabulary.
9. Complete missing model scenarios: broadcast-failure isolation, hard-user-deletion dependency events with WS11, ActivityItem accessible scope with WS12, legacy conversion, broader coercion/Unicode cases and individual discrimination evidence.
10. Coordinate the Event shape through the lead/WS7. WS8b then adds HTTP/controllers/templates, rendered frames and full HTTP rejecting-job-trigger tests. No WS8b acceptance is implied here.

Open integration questions are the final WS7 event shape, the real storage copier adapter, and WS5's icon/member resolver wiring. There are no unanswered questions blocking the already committed model slices. No deployment or production data was changed.
