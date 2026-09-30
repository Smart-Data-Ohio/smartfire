# WS8a messaging models — partial handoff, 2026-09-29

Branch: `rust/ws8-messaging-models`. Verified implementation commits in this continuation: `f2d502e8` (room destruction/retention) and `41356cc24c0963ef2311dcb2044f6da943e3aee5` (runtime adapters and differential follow-up). The following report commit changes documentation only. **WS8a remains partial at the named integration boundaries below.** Controllers/templates remain WS8b. The user authorized committing and pushing; no PR was opened.

## Recovery and scope

Opus's committed threads, edits/forwarding and polls and its uncommitted pins/saves were preserved in the earlier takeover; the in-progress work was committed first as `5b373ef2`. Subsequent verified slices added keywords, scheduled messages, categories/favorites, durable queue checks, references/cards, group DMs, search, audit and initial runtime wiring. That handoff is preserved in report commit `fa404c9f`.

This continuation starts from `fa404c9f` and handles its remaining items 1, 3 and 4. **Slash commands were deliberately untouched:** a separate Sol worker owns `rust/ws8a-slash`, forked from `fa404c9f`. Existing WS3/WS5 merges and reference pin `fec615be` remain in the branch; no rebase or stash was used. No Astra findings arrived during this continuation; this report does not claim review acceptance.

## Complete in this continuation

- Room deletion's model/database path: validated marking; membership removal; huddle/agent revocation; stream ending; durable cleanup and destruction enqueue; bounded DestroyJob; dependency destruction; retry/resume/no-op behavior; claim refresh and stale reenqueue; alive directory scopes. The final thread cascade includes inbox items sourced by WorkThreadEvent and BoardSlaNudge.
- Retention's nine row kinds, calendar-year audit cutoff, strict/inclusive expiry boundaries, unread/pending/active exceptions, grant cleanup unlink/dependent inbox deletion and daily stuck-room backstop. Real app workers and periodic tasks are registered.
- Production Markdown/canonicalization/plain-text/mention failures now propagate through Message writes. Scheduled send, edit and forward run through the production adapter and match Rails-generated output.
- A real storage forward copier verifies source checksum, preserves metadata/content type and stages a new file. The transaction owns its file guard, including when a deferred durable enqueue fails after forwarding returns successfully.
- Runtime saved-reminder, scheduled-send and poll-close loops are checked against Rails with a failing first row and a successfully processed second row. Existing quote worker and template-free broadcasts remain registered. Rendered partials and notification delivery remain partial.

## Changes by file

Paths are relative to `rust/`. The first table describes this continuation; earlier delivered files follow so the overall brief status remains explicit.

| File | Change |
|---|---|
| `crates/db/src/models/room_delete.rs` | Room marking/revocation, cleanup snapshots and atomic jobs; ID-only batches of 500; per-record transactions; dependency cascades; final room destruction; retry/recovery claims. DTOs for DestroyJob, CleanupJob and Calendar::RemoteDeleteJob. |
| `crates/db/src/models/room.rs` | Delegates begin_destroy/destroy to the service; validates creator/direct name on marking; alive all/type/count/original directory scopes. |
| `crates/db/src/models/membership.rs` | Alive ordered/count directory queries; last group member uses complete begin_destroy. |
| `crates/db/src/models/channel_thread.rs` | Destroy inbox dependencies of work events and SLA nudges before deleting their sources; explicit work callback deferrals. |
| `crates/db/src/models/retention.rs` | PruneJob and bounded batches of 1,000; per-branch current-time cutoffs; calendar-year audit retention; per-grant destruction and one cleanup unlink per batch. |
| `crates/db/src/models/message.rs`, `rich_text.rs` | Fallible rendering/canonicalization/plain-text/mention seams; preflight before commit; create/edit/update_body/read paths propagate errors. Lightweight compatibility defaults remain available. |
| `crates/db/src/models/forwarder.rs` | Fallible canonicalization; copier receives mutable Tx so a staged file guard can belong to the transaction. |
| `crates/campfire/src/rich_text.rs` | Real WS5 fallible adapter; production scheduled/edit/forward differential test. Infallible compatibility entry points still log/fallback and remain a named integration boundary. |
| `crates/campfire/src/messaging.rs`, `main.rs` | Real ForwarderCopier and one additive module registration. WS8b must supply it to its action. |
| `crates/storage/src/storage.rs` | stage_copy: checksum-verified source, fresh key, identify:false, copied metadata/content type. Uses the existing Staged rollback guard. |
| `crates/campfire/src/jobs/messaging.rs`, `jobs.rs` | Two real runner handlers in one additive register call, avoiding a shared registry redesign. |
| `crates/campfire/src/jobs/periodic.rs` | Five-minute stuck-room sweep and configured daily retention enqueue; small pub(super) seams for testing the existing per-row loops. |
| `crates/campfire/src/jobs/tests.rs`, `ws8_runtime_vectors.json`, `ws8_loop_vectors.json` | Maintenance runner, real file copy/queue rollback and per-row fault tests; production Markdown and Rails runtime goldens. |
| `crates/db/src/tests/room_delete_test.rs`, `retention_test.rs`, `rich_text_failure_test.rs`, `ws8_deletion_vectors.json`, `ws8_retention_vectors.json` | Six room checks, one consolidated retention check and three renderer fault checks using real SQLite; Rails-generated cascade/survivor expectations. |
| `crates/jobs/src/tests/ws8_messaging_test.rs` | Two additional queue rejection/retry tests; room/cleanup writes, memberships and sweep claim roll back on enqueue failure. Six WS8 queue tests now exist. |
| `crates/db/src/tests/fixtures_test.rs`, `forwarder_test.rs`, `crates/db/ruby/rollback.rb` | Mutable copier seam and export/rollback validation of a marked room plus claimed cleanup. Rails validates 122 exported rows. |
| `crates/db/ruby/ws8_vector_helpers.rb`, `ws8_deletion_vectors.rb`, `ws8_retention_vectors.rb`, `ws8_runtime_vectors.rb`, `ws8_loop_vectors.rb` | Actual Rails writes/callbacks yield setup SQL deltas and expected results. No expected JSON was authored by hand. |
| `reference-tools/db/ws8-reference-check.sh`, `ws8-maintenance-discrimination.py`, `ws8-runtime-discrimination.py` | Reference identity check; ten maintenance and seven runtime regressions that must cause named tests to fail, with source restoration. |
| `crates/db/src/models.rs`, `tests.rs`, `crates/db/Cargo.toml`, `Cargo.lock` | Small additive exports/test modules and the existing workspace rails_compat dependency for opaque LiveKit room names. No schema changes. |
| `plans/ws8-wave2-report.md` | Tracked copy of this externally requested report. |

Earlier delivered domain files: `channel_thread.rs`, `thread_membership.rs`, `thread_tag.rs`, `message.rs`, `forwarder.rs`, `poll.rs`, `message_pin.rs`, `saved_item.rs`, minimal `activity_item.rs`, `scheduled_message.rs`, `keyword_alert.rs`, `room_category.rs`, `membership.rs`, `push_subscription.rs`, `message_reference.rs`, `direct_room.rs`, `search_query.rs`, `audit_log.rs` and their tests/Rails vectors. Typed events are in `broadcasts.rs`/`events.rs`; app Markdown icon wiring is in `rich_text.rs`/`build.rs`; existing queue, fixture/timestamp and differential tools remain intact. These are earlier WS8 deliveries, not newly attributed WS3/WS5 work.

## Design notes

Room marking and every emitted job share the triggering SQLite transaction. A rejected enqueue rolls back the deleted marker, memberships, cleanup and claim. Unlike Rails' Redis enqueue after commit, SQLite job insertion precedes commit while execution still observes committed data, as required by WS3's queue contract.

DestroyJob commits its claim and cleanup unlink before deleting children, then processes huddle grants, scheduled messages, messages, threads and events in that order. ID queries limit each batch to 500; each record gets its own write transaction so retries keep completed progress. Final dependencies include repository notifications/subscriptions, pins, board automation rows, agent slash rows and memberships; venue/agent-ledger references are nullified. Calendar deletion arguments are captured before removing entries and enqueued transactionally. A missing, live or already destroyed room is a no-op. A strict cutoff and conditional claim prevent competing sweep writers from enqueuing twice.

Two easily misread Rails behaviors were checked against the oracle: revoked AgentGrant rows retain their old room_id because Rails declares no dependent association/FK, and only Stage owns streams dependent:destroy. Marking ends all live streams; a Stage's final destruction deletes its stream history. Do not replace the retained AgentGrant behavior with a cleanup invented for Rust. The non-Stage imported-stream edge is source-matched but not separately differentially proven.

Huddle cleanup uses a stored grant room_name or WS1's opaque HMAC room name with configured signing credentials. A new delete_room cleanup receives a one-minute enqueue lease only when admin access is configured. Cleanup snapshot fields survive unlinking grants. The actual remote cleanup consumer is WS13, not a fake success handler here.

Retention uses Ruby's exact `<` or `<=` boundaries and updated_at age for seen activity, never unread activity. Audit uses a calendar year rather than 365 days, tested across a leap year. Pending cleanups and active/recent grants survive; expired 2FA rows use inclusive expiry. Ruby's delete_all branches intentionally bypass validations/callbacks. Revoked grants go through dependency destruction after one unlink per batch.

Raw `Room::find/find_by_id` remain explicitly unscoped for associations, validation and destruction. Directory/search/reachable-message/reminder/thread access scopes are alive-only. Existing HTTP room lookup and Cable subscription authorization use find_for_user; their secondary association lookups remain raw. Scheduled due selection deliberately includes inaccessible/deleted rooms so dispatch can record a dropped item, matching Ruby. Existing raw integration-job lookups are association cleanup paths, not directory readers. Controllers/channels were audited read-only, not changed or accepted end to end.

The real file copier installs Staged's guard on Tx's after-commit path. Returning successfully from forward() does not keep the file prematurely. A rejected durable enqueue rolls back rows and drops the guard, deleting the new file while preserving the source. HTTP wiring and attachment processing remain WS8b/WS10 integration work.

## Feature × Rails validations/callbacks

“Ported” means these model helpers and tested cases. Every named integration below remains partial.

| Feature | Ported validations/scopes/callbacks | Partial/deferred and owner |
|---|---|---|
| Threads | Required room/creator/name, name limits/defaults, parent/board constraints, tags/membership/unread/recount/stale rules, parent stamp/indicator, root timeline, atomic thread push request, dependent work/SLA inbox deletion. | Deleted-work snapshot, work_unassigned ledger/webhook and their atomic jobs: WS11/WS12; board row removal: WS12/WS8b. Hard user removal/finalization: WS11. Push execution: WS17. |
| Replies/edits/forwards | Conversation/metadata/destination guards, tombstones, edit/source/body timestamp rules, mention snapshots, dedup/flags, quote resync, real staged copy and rollback, production rendering/error propagation. | Legacy conversions and attachment processing: WS8b/WS10/WS5; non-message reference resync WS14/WS15; HTTP/auth/rendered append/replace WS8b. |
| Polls | Message/options/votes validations, ballot replacement, close claims, parent touch/dependencies and typed replacement; per-row scheduler plus failure continuation. | External coercion edges unproven; rendered poll delivery WS7/WS8b. |
| Pins/saves | Presence/ownership/caps, quiet note rules, timestamps/dependencies, reminder claim/activity/transactional job and payload; per-row failure continuation. | Hard user deletion WS11; complete inbox accessibility/source policy WS12; real DND/quiet-hours/push handler WS17. |
| Scheduled | Source/time/conversation validation, access recheck, claim/history/drop/dependencies, atomic send, production Markdown and row-failure continuation. | Complete ActivityItem policy WS12; HTTP/rendered delivery/push WS8b/WS7/WS17. |
| Keywords | Required user/phrase, normalization/80-character limit/create cap 20, SQL LOWER duplicate semantics, independent literal matching/Unicode boundaries. | Recorder integration WS17; broader Unicode case folding and input coercions WS8a/WS8b, partial. |
| References/cards | Required associations, self/uniqueness, normalized capped extraction/HTML code exclusion, nonstreaming sync, transactional refresh enqueue, capped/batched replacement, removal stamps. | Fragment cache/rendered placeholders WS8b/WS7; finalize WS11; import suppression WS16; other reference types WS14/WS15. |
| Categories/favorites | Required user/name/50-character cap, positions/order/collapse, dependent nullification, same-user membership category, favorite append/unfavorite/reorder. | Current-user HTTP scopes/authorization WS8b. No model broadcasts applicable. |
| Group DMs | Cap 10/private-pair/admin guards, direct-name generic validation, keyed/unkeyed lookup, rename/add/leave notes and per-user directory/header events; last-member begin_destroy now complete at database layer. | Huddle participant locals WS13; rendered sidebar/header and HTTP WS8b; broader Ruby scenarios/query instrumentation partial. |
| Room deletion | Room creator/direct-name validation; marker/direct key/timestamps; membership delete_all; grant presence/identity uniqueness and revocation; quality validation/stream end; cleanup enum/presence/snapshot/lease/job; atomic destroy job; retries/recovery/dependencies/alive directories. | Huddle in-call ended/leave/voice callbacks and synchronous Stage last-grant stream behavior WS13; thread work callbacks WS11/WS12; calendar consumer WS14; HTTP switches to begin_destroy and rendered delivery WS8b. |
| Search | Operator grammar/chips, invalid-token preservation, Unicode FTS words, bound literal LIKE values, dates/attachment/thread predicates, alive/access scopes and tuple cursor. | Board/work/event sections WS12/WS14; timezone/DST/coercion and HTTP/preloads partial WS8a/WS8b. |
| Non-agent slash | No new implementation in this branch. Existing action flag support is available to the handler. | **Owned by parallel `rust/ws8a-slash` worker**, including dispatcher/time parser/handlers and WS13/WS14 stubs. |
| Retention/audit | All nine retention row kinds/backstop and audit append-only/action presence, snapshots/redaction/user-agent cap/digest/failure collapse. Retention alone deletes old audit rows, as in Ruby. | Current/audit foreign labels/UI WS4/WS11/WS14/WS17/WS8b; broader nil/IP/coercion parity partial. HuddleGrant destroy inbox/unlink ported; revocation broadcasts WS13. |

Additional row audit: Event destruction removes attendance, calendar entries, event references and source inbox items; EventCalendarEntry's remote-delete snapshot is enqueued in the same transaction (consumer WS14). RepositorySubscription dependent notifications and bot-membership removal are satisfied by the room membership delete_all; these rows have no additional destroy hooks. WorkThreadEvent, BoardSlaNudge, WorkThreadLink, WorkHandoff, AgentStep and PR mapping dependent rows are removed with their source inbox dependencies where applicable; their create/update models remain their owners' work. Venue/agent references and cleanup unlink use Ruby's update_all semantics, intentionally skipping validation/touches. Active Stream update! checks quality; unchanged association validation follows Rails' configured required-FK behavior. The deleted room suppresses normal Stage stream/presence rendering lookups; remaining in-call notifications are explicitly WS13.

ActivityItem remains a minimal reminder/drop writer, not the complete WS12 inbox model. Existing after-commit rendering/indexing compatibility paths outside Message's new fallible calls are still partial under WS2/WS5/WS8b.

## Broadcast contracts

Existing typed events remain: parent thread indicator replacement and user unread-thread Cable; poll replacement; pin badge/count/list replacements and quiet-note append; active-human activity Cable; scheduled conversation append/root unread Cable; quote-card replacement/removal; group room timeline notes and recipient-specific sidebar/header replacement/prepend/removal. Membership removal disconnects after its sidebar removal. QuoteCards, DirectSidebar, RoomHeader, Poll, pin, thread and message partial descriptions remain available to WS8b.

Only plain Cable payloads and template-free Turbo remove are delivered by the existing app sink. Partials remain explicitly unregistered; no HTML/subscriber acceptance is claimed. Marking uses membership delete_all, so it intentionally skips individual Membership destroy broadcasts/callbacks, like Ruby. No new Huddle broadcast delivery was fabricated for the deferred WS13 callbacks.

## Ruby scenario counts versus Rust proof

Ruby Minitest files were read/count-checked, **not executed as whole suites**. Rails runners were executed to produce differential expectations. The counts below describe modeled coverage, not one-to-one full acceptance.

| Ruby test file under `test/` | Ruby scenarios | Rust proof / remaining |
|---|---:|---|
| `jobs/room/destroy_job_test.rb` | 13 | Six consolidated room tests plus two real queue tests and app worker execution: marking, cascades/scheduled inbox, repeat/live/missing no-op, competing claims/expiry, refreshed claim and transient resume. Exact maximum materialization/query-count instrumentation remains **partial**. |
| `jobs/retention/prune_job_test.rb` | 10 | One consolidated Rails survivor differential covers all nine row kinds and stuck backstop, including one-year leap cutoff; app worker test. One unlink per batch is implemented but its SQL query count and multi-batch/crash instrumentation remain **partial**. |
| `models/channel_thread_messages_count_test.rb` | 19 | Earlier 16/19 modeled; two broadcast-failure scenarios and hard-user-delete remain partial. |
| `models/message_conversations_test.rb` | 5 | Earlier 3/5; two legacy conversions remain partial. |
| `models/thread_tag_test.rb` | 2 | Earlier 2/2; core thread groups total 30 Rust checks. |
| `services/messages/forwarder_test.rb` | 8 | Earlier eight modeled scenarios/nine tests; this continuation adds real file copy/rollback and production forward differential. Full HTTP/attachment processing still partial. |
| `models/poll_test.rb` | 15 | Earlier fifteen modeled scenarios/nineteen checks; runtime row rescue now proven, rendering partial. |
| `models/message_pin_test.rb` | 16 | Earlier 15/16, seventeen checks; hard-user-delete broadcast remains WS11. |
| `models/saved_item_test.rb` | 12 | Earlier 11/12, thirteen checks; accessible_to after access loss remains WS12. |
| `models/saved_item/reminder_dispatcher_test.rb` | 7 | Earlier 7/7, nine checks; runtime row rescue now proven. |
| `models/saved_item/reminder_pusher_test.rb` | 4 | Earlier four payload/policy-seam scenarios/five checks; actual policy/delivery WS17. |
| `models/scheduled_message_test.rb` | 8 | Earlier 8/8, eight checks; complete inbox privacy/HTTP still partial. |
| `models/scheduled_message/dispatcher_test.rb` | 13 | Earlier thirteen modeled scenarios plus SQLite/state checks, combined scheduled twenty-five; production send and row rescue now proven. |
| `models/keyword_alert_test.rb`, `models/notifications/keyword_matcher_test.rb` | 4 + 10 | Earlier fourteen matching Rust tests; broader coercions/folding partial. |
| `models/message/reference_sync_test.rb`, `jobs/message/quote_cards_refresh_job_test.rb` | 10 + 5 | Six consolidated model tests, extraction/HTML vectors, real missing-source worker and queue rejection; no complete per-scenario mapping or rendered-cache proof. |
| `models/rooms/direct_test.rb` | 28 | Six consolidated tests, WS2 key checks, display vectors and Rails rollback group reads; complete last-member database deletion added; broader agent/huddle/rendering scenarios partial. |
| `models/search_query_test.rb` | 24 | Three consolidated grammar/filter/cursor tests; timezone/DST and side sections partial. |
| `models/audit_log_test.rb` | 17 | Eight consolidated audit tests; external-context/foreign-label and broader coercion parity partial. |
| `models/channel_thread_board_test.rb`, `channel_thread_auto_assign_test.rb`, `channel_thread_agent_assignment_test.rb`, `channel_thread_handoff_test.rb` | 16 + 10 + 29 + 11 | 0 ported by WS8; WS12/WS11 ownership. |

Extra Rust tests have no separate Ruby scenario count: three renderer error propagation checks, production Markdown/canonicalization checks, staged-file/durable-enqueue fault check, periodic row-failure check and queue atomicity properties. The fixtures and timestamps are produced by Rails, while transaction/file safety assertions test the required queue adaptation.

## Commands rerun and raw evidence

All commands below were rerun in this worktree during this continuation. Successful suites were rerun with restored sources after discrimination. Logs/scratch are below `.scratch/`; nothing was written to /tmp. No Docker ports were exposed. The isolated `ws8-reference-models` image is based on image id `83d1ae45158680dc665489d1e1cb7c053ac3a860c0e2841d6f19b29f731329f5` with this checkout's Rails db overlaid; relevant source identity is checked explicitly, not inferred from the image name. The shared reference image was not changed.

### Reference and Rails generation

Run from the worktree root:

```sh
bash rust/reference-tools/db/ws8-reference-check.sh > .scratch/reference-check.log 2>&1
docker run --rm --cpus 2 --name ws8-deletion-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_deletion_vectors.rb /out/ws8_deletion_vectors.json' > .scratch/deletion-vectors-final.log 2>&1
docker run --rm --cpus 2 --name ws8-retention-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2024-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_retention_vectors.rb /out/ws8_retention_vectors.json' > .scratch/retention-vectors-final.log 2>&1
docker run --rm --cpus 2 --name ws8-runtime-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/campfire/src:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_runtime_vectors.rb /out/ws8_runtime_vectors.json' > .scratch/runtime-vectors-final.log 2>&1
docker run --rm --cpus 2 --name ws8-loop-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/campfire/src:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_loop_vectors.rb /out/ws8_loop_vectors.json' > .scratch/loop-vectors-final.log 2>&1
```

```text
WS8 reference: 48 implementation files, icon catalog and Rails db match worktree
WS8 deletion vectors: 6 begin checks, 22 cascade checks, 1 calendar jobs, 2 recovery claims
WS8 retention vectors: 9 row kinds, 11 survivor checks, 1 stuck room jobs
WS8 runtime vectors: 8 Markdown, 4 canonicalization, 5 periodic tasks, 2 template-free broadcasts, 3 write flows, 1 attachment copy
WS8 loop vectors: 3 first-row SQL failures, 4 continuation checks
```

The loop generator deliberately injects SQL errors on the first row and invokes the real Ruby dispatchers, allowing their rescue behavior to generate the expected continuation results. Deletion setup includes real associations/callbacks; retention covers ±1 microsecond and exact boundaries. Setup SQL is a delta of actual Rails writes. Opaque encrypted fixture bytes can change when regenerated; they are not hand-authored expectations.

### Successful Rust checks

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -- --test-threads=4 > .scratch/test-final.log 2>&1
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire rich_text::tests::runtime_ -- --test-threads=4 > .scratch/app-rich-text-final.log 2>&1
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire jobs::tests::ws8_ -- --test-threads=4 > .scratch/app-jobs-final.log 2>&1
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
test result: ok. 365 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 8.71s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 181 filtered out; finished in 0.19s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 179 filtered out; finished in 0.15s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.49s
```

The normal DB run ignores three environment-dependent differential/export tests; the next command runs all three. Jobs' two crash-process tests also ran. The app selections ran four runtime renderer tests and six WS8 job/broadcast/storage/loop tests; 181 and 179 filtered tests respectively were **not run**. No fixture-based app acceptance was silently claimed. Other app/workspace test suites, media-vector parity and browser parity were not executed; clippy compiled all workspace targets.

### Rails rollback, fixture/timestamp and migration checks

Run the first command from `rust/`:

```sh
OUT="$PWD/../.scratch/differential" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models CARGO_TARGET_DIR="$PWD/target" TMPDIR="$PWD/../.scratch" mise exec rust@1.98.1 -- bash reference-tools/db/differential.sh > ../.scratch/differential-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 365 filtered out; finished in 3.96s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
message_save_touches.json matches the reference
schema.sql, reference db:prepare and Rust prepare agree
validated 122 rows Rust wrote: {"accounts" => 1, "action_text_rich_texts" => 23, "activity_items" => 2, "audit_logs" => 1, "boosts" => 1, "channel_threads" => 2, "drive_attachments" => 3, "huddle_cleanups" => 1, "keyword_alerts" => 1, "memberships" => 25, "message_pins" => 1, "message_references" => 3, "messages" => 25, "poll_options" => 4, "poll_votes" => 3, "polls" => 2, "room_categories" => 1, "rooms" => 9, "saved_items" => 2, "scheduled_messages" => 3, "searches" => 1, "sessions" => 2, "thread_memberships" => 2, "thread_tags" => 2, "users" => 2}
rollback ok
```

Run from the worktree root:

```sh
TMPDIR="$PWD/.scratch" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models bash rust/reference-tools/db/check-migration-replay.sh > .scratch/migration-final.log 2>&1
```

```text
self-test: all 6 mutations caught
migration replay matches schema.sql: 1279 facts, 87 tables with an id, 248 indexes, 128 versions
```

The Rails boot validates all 122 exported rows, including the newly marked Room and claimed HuddleCleanup, then performs its read/write/delete rollback checks. This is representative rollback compatibility, not exhaustive validation of every possible row combination.

### Failing-first and discrimination evidence

All new test functions were shown failing against scaffolds or deliberate regressions before accepting their implementation: room begin/cascade/recovery/readers/retry/no-op, retention survivors, room/sweep queue atomicity, real maintenance registry, production write flows, canonical/plain-text/mention failures, real copier/file rollback and loop rescue. The final SLA dependency case was added to the Rails oracle, failed against the missing dependency, then passed after the fix (`.scratch/sla-cascade-before.log`). Initial scaffold failures are also retained in `.scratch/*-before.log`; the reproducible evidence below rejects compile-only failures and asserts the names of the tests that must fail.

The three commands run sequentially; no other source editing/build runs during mutation. Each restores the original source in finally. Older recovered tests outside these selected checks remain individually unproven; mutation evidence is not rewritten as a complete TDD history for Opus's work.

```sh
python3 rust/reference-tools/db/ws8-discrimination.py > .scratch/discrimination-final.log 2>&1
python3 rust/reference-tools/db/ws8-maintenance-discrimination.py > .scratch/maintenance-discrimination-final.log 2>&1
python3 rust/reference-tools/db/ws8-runtime-discrimination.py > .scratch/runtime-discrimination-final.log 2>&1
```

```text
keyword-rules: detected (14 failing tests)
test result: FAILED. 0 passed; 14 failed; 0 ignored; 0 measured; 354 filtered out; finished in 0.11s
scheduled-claim: detected (4 failing tests)
test result: FAILED. 21 passed; 4 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.59s
scheduled-validation: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.60s
scheduled-access: detected (5 failing tests)
test result: FAILED. 20 passed; 5 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.62s
scheduled-thread-drop: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.61s
category-validation: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 360 filtered out; finished in 0.23s
category-nullify: detected (1 failing tests)
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 360 filtered out; finished in 0.24s
favorite-writes: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 360 filtered out; finished in 0.23s
transaction-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.08s
save-touch-golden: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.14s
thread-job-atomicity: detected (2 failing tests)
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.17s
reminder-job-atomicity: detected (1 failing tests)
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.15s
quote-references: detected (6 failing tests)
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 362 filtered out; finished in 0.15s
group-dms: detected (6 failing tests)
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 362 filtered out; finished in 0.21s
search-grammar: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.10s
audit-contract: detected (8 failing tests)
test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 360 filtered out; finished in 0.15s
runtime-markdown: detected (4 failing tests)
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 181 filtered out; finished in 0.19s
runtime-jobs-broadcasts: detected (3 failing tests)
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 179 filtered out; finished in 0.17s
quote-job-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.07s
WS8 discrimination: 19 mutations detected; sources restored
room-start: detected (4 failing tests)
test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 362 filtered out; finished in 0.19s
room-worker: detected (3 failing tests)
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 362 filtered out; finished in 0.21s
room-live-guard: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.08s
room-sweep: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.08s
room-readers: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.07s
retention: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.07s
room-enqueue-atomicity: detected (2 failing tests)
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.15s
maintenance-registry: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.09s
cleanup-enqueue-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 48 filtered out; finished in 0.07s
sla-dependent-inbox: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 367 filtered out; finished in 0.09s
WS8 maintenance discrimination: 10 mutations detected; sources restored
renderer-failure-propagation: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.15s
write-flow-markdown: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.15s
real-file-copier: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.09s
file-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.09s
loop-saved: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.10s
loop-scheduled: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.18s
loop-poll: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 184 filtered out; finished in 0.10s
WS8 runtime discrimination: 7 mutations detected; sources restored
```

## Cross-workstream touches

- WS2: minimal Room/Membership/Message extensions, alive directories, fallible RichText seams, destruction dependencies and rollback export. No core schema redesign.
- WS3: additive messaging job module/register call and two periodic entries; six queue tests. Queue/store internals are unchanged. Cleanup/calendar DTOs are durably written but need their domain consumers.
- WS5: production adapter calls and exception propagation; richtext implementation unchanged. Compatibility callers outside the fallible Message paths remain partial.
- WS10/storage: small stage_copy helper and existing Staged guard reuse. WS8b must supply the new copier; after-commit attachment processing remains its owner integration.
- WS11/WS12/WS13/WS14: dependency rows required for room deletion/retention are handled here; domain-specific deferred callbacks/consumers are listed precisely below. These deletes do not claim complete domain model ports.
- WS7/WS8b: existing typed partial events retained; no controllers/templates changed and no rendered-delivery acceptance.
- Parallel slash branch: no slash implementation touched; module/registry additions are small and additive. The mutable BlobCopier seam requires any new implementer to accept mutable Tx, but registration points were not reorganized.
- The merged Rails background_jobs migration is inherited from main. No Rails source edits, PR, deployment or production data changes were made. The external report is the explicitly requested exception to rust-only writes; its tracked copy is identical.

## Precise remaining work / restart point

1. **Parallel slash worker:** registry/dispatcher/permissions/timezone-aware time parsing and shrug/me/remind/status/dnd/ooo/poll, with WS13/WS14 huddle/event/play stubs, belong to `rust/ws8a-slash`. Merge/review that separate slice; nothing from it is claimed here.
2. **Push runtime (WS17 with WS8a integration):** implement/register real SavedItem::ReminderPusher and ChannelThread::PushJob using the actual notification/DND/quiet-hours policy and Web Push pool. The durable DTOs currently have no handlers and fail unknown-class in the runner. Payload/policy seams and enqueue atomicity are proven; delivery is not. Do not register no-op/always-allow handlers.
3. **Huddle/Stage callbacks and remote cleanup (WS13):** in-call call-ended banners, leave notice/voice presence on grant revocation; synchronous alive-Stage last-grant stream ending/broadcasts and broader membership host-loss paths; real Huddle::CleanupJob/reconciler/network retry consumer. This branch writes validated revoke/cleanup state and atomic jobs, but does not consume that remote job. Marked-deleted Stage lookups suppress the normal stream/presence rendering, while in-call effects remain partial.
4. **Calendar/Event integration (WS14):** consume Calendar::RemoteDeleteJob with real remote API behavior; full Event create/update/reference/audit-label model. Room destruction captures the exact delete snapshot and durably enqueues it before deleting entries; the consumer is not registered here.
5. **Thread work deletion callbacks (WS11/WS12):** capture the deleted work item, write work_unassigned ledger and webhooks and enqueue their jobs in the triggering transaction; board row removal WS12/WS8b. Dependency inbox items are already removed. Hard-user-destroy and stream-finalization integration remain WS11; import suppression WS16; other reference sync WS14/WS15.
6. **HTTP/rendering acceptance (WS8b with WS7/WS10/WS3):** switch room delete routes to begin_destroy (the existing handler still calls synchronous destroy), supply ForwarderCopier, coordinate attachment processing and render typed QuoteCards/group/pin/poll/thread/message events; execute route authorization, subscriber and HTTP-trigger enqueue rejection checks. No controllers/templates/browser parity were changed or tested here.
7. **Unproven parity/instrumentation:** materialization/query-count proof for 500-message destruction and one unlink per retention batch; large multi-batch and retention crash-resume cases; non-Stage imported stream history; earlier broadcast-failure/hard-user/legacy-conversion cases; full ActivityItem accessibility; broader Unicode/coercion, nil/IP and timezone/DST inputs; foreign audit context/labels and every individual Ruby scenario/discrimination mapping. Old infallible RichText compatibility callers and storage crash/orphan recovery remain partial with WS2/WS5/WS8b/WS10.

There are no external blockers to these committed database/runtime adapter slices. This is a natural boundary at named domain consumers and WS8b delivery work, **not complete WS8a or a production/cutover acceptance**.
