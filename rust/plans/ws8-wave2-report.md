# WS8a messaging models — WS10 merge, verification and partial handoff

Branch: `rust/ws8-messaging-models`. Current merged code: `4ddbbaf2` (parents `31080756`, `7d61c1f5`); main merge: `dda958c942ea757c570db82f1995eb1d2da0d06f` (parents `6cd91358`, `a2fbe296`). The lead merged the reviewed slash branch in `6cd91358`. Earlier continue2 and Astra fixes are retained. **WS8a remains partial at the named integration boundaries below.** This report is mirrored only under `rust/plans/`; `.claude/delegation/**` is not tracked. The user authorized committing and pushing; no PR was opened.

## WS10 / main merge (#156)

The lead left one conflict in `crates/db/src/models/message.rs`. Merge commit `4ddbbaf2` retains WS8's `Message::create`: rendered/canonicalized bodies, every existing column, transaction-local index/unread/reference bookkeeping, durable job enqueue and ordered post-commit broadcasts. WS10's `create_content` is removed. `create_markdown(tx, attributes, source)` only sets `attributes.markdown_source` and calls `create`; existing RoomMailbox and mail-crate callers keep their API. The shared validator supplies the same 50,000-character and blank-source errors, including Rails' streaming and attachment/kept-Drive exceptions; the narrower duplicated mail validator is removed.

Semantic follow-ups, by file:

- `campfire/src/app.rs`, `campfire/src/mail.rs`: install AppRichText into mail at boot before workers start. Mail preflight and Message's write-time rendering use the same fallible, room-aware renderer. A real durable RoutingJob compares its saved source, canonical body, plain text and creator with an actual Rails RoomMailbox post. Mail's preflight renderer remains for its readiness/retry contract; message creation always uses the shared WS8 path.
- `db/src/models/room.rs`: token rotation's `update!` also validates an existing Direct name. No icon/type change occurs, so their change-conditional validators and Open's type-change callback do not run. The incoming code had bypassed the Direct length check; a fresh Rails-derived invalid-name regression fixes that overlap. Existing creator foreign-key/core model behavior is retained.
- `mail/tests/inbound.rs`: replace the inherited indexing-after-commit fixture with the lead's transaction semantics. Dropping the index must roll back the mail message, attachment/blob rows and staged file while preserving the accepted raw email. The inbound is Failed, can retry after the index is restored, and posts one attachment successfully. A compiled mutation swallowing the index error fails the regression. This rollback policy follows the explicit round-2 lead decision; it is not claimed as Rails' failure behavior. Genuine post-commit failures still retain committed staged files through the existing hook; that broader failure coverage remains partial.
- `mail/tests/smtp.rs`: permit a worker's `MAIL_TEST_PORT_RANGE`; this worker uses 48000–48049. The default WS10 range stays intact.
- `db/ruby/ws8_mail_merge_vectors.rb`, `db/src/tests/mail_merge_test.rs`, `ws8_mail_merge_vectors.json`: six actual Message validations (normal, blank, streaming blank, Drive blank, 50,000 and 50,001 multibyte characters), a Direct token validation, and a real RoomMailbox post. Metadata expectations also come from Rails. `reference-tools/db/ws8-mail-merge-discrimination.py` proves source assignment, token validation, boot installation and mail rollback tests reject compiled regressions. `ws8-reference-check.sh` also checks six mailbox/mailer source files.
- `mail/README.md`: update the renderer integration status. The remaining Cargo/mail/controller/config/job files are inherited from main's reviewed WS10 commit. Cargo.lock merges its new mail dependencies without a manual lock repair; locked metadata passes, workspace dependencies have 75 unique keys, and no `.claude/delegation` path is tracked.

The new DB tests first failed by assertions and the production routing test first timed out before fixes. The old mail index probe first failed because the message correctly rolled back. Four reproducible compiled mutations cover the new checks, including the revised rollback assertions; they restore source before the final suite. No build failure is counted as test discrimination.

The incoming typed mail job registry is additive. Production job persistence remains inside the originating transaction, including routing acceptance, Message pushes, MessageCreated and lifecycle incineration. After-commit delivery uses WS7's existing conservative publisher/guard; no duplicate broadcast implementation was added. Email bot creation retains its skip-open-room-grant behavior. WS11 still owns installing BotWebhookFanout and broader bot callbacks; WS8b owns inbound-address controller/browser acceptance and complete message/attachment presentation. SMTP socket/golden tests prove the merged local adapter, not a live mail-provider deployment or fresh production MIME-depth calibration. The inherited WS10 MIME-depth profile remains historical.

## Round-2 review fixes

Astra's round-2 verdict was merge after fixes with two P2s; the five prior fixes, both merges (including `8fe8589a`), atomic enqueue, attachment rollback, deletion/retention resume probes, clippy and lock checks passed review. That round-2 continuation started at `a76d527e` and ports all four reviewer probes from `/home/riels/.cache/rust-port/ws8r/r2/reviewer_probe.rs` into the committed suite. No further merge or slash implementation was needed in that round.

1. **Synchronous dependencies:** `Room#destroy!` now follows Room's declarations separately from DestroyJob's preliminary deletes. Memberships use delete_all; messages, threads and events run their dependent destroys. Room's final callbacks revoke grants without destroying them or unlinking cleanups. Scheduled rows use delete_all, preserving the dropped-message inbox item. DestroyJob retains its preliminary grant unlink/destroy and scheduled dependent destroys. Both paths are checked against fresh Rails-generated rows, including the linked cleanup and retained/deleted inbox item. Existing dependency callbacks and their named WS11/WS12/WS13/WS14 deferrals below remain in force.
2. **Atomic message bookkeeping:** create now performs index insertion, room/thread unread updates, message-reference sync and the stale-thread sweep in its SQLite transaction. Edits and touches reindex transactionally; reference edits are atomic too. Destroy removes its index row and stamps quote sources transactionally. Index/renderer, unread or reference failure rolls the whole write back. This intentionally strengthens Rails' after-commit failure behavior under the lead's decision; the successful rows match Rails. Broadcast delivery and job wakeups remain after commit; every job is still persisted in the triggering transaction. The actual Rails success oracle also exposed an existing reversed thread push/indicator order, now corrected to unread broadcast, push wake, indicator. The indicator callback reads the final counter after commit and performs no bookkeeping writes. No bookkeeping piece needed a new durable fallback job.

`ws8_round2_vectors.rb` calls our actual room destroy/DestroyJob/PruneJob and message create callbacks. Ordered events come from ActionCable's emitted frames and ActiveJob's enqueue notifications; the thread indicator count comes from the real rendered frame, with no replacement of its callback. The six success checks cover messages/timestamps, FTS, room unread/read pointers, thread unread, references and reply counts. SQLite trigger failures test unread/reference rollback; renderer faults pass preflight before failing indexing, including edits and touches. These are failure invariants required by the lead, not a claim that Ruby rolls back its after-commit error.

All ten new regressions were exercised against the unfixed baseline: three passed and seven failed by assertions (`.scratch/round2-before.log`). Eight committed discrimination mutations make every new test fail against a compiled broken implementation, including the two otherwise-passing resume probes; sources are restored afterward. The 501-message and 1001-grant probes compare Rails-generated progress and final retry states. They prove SQL-abort retry progress across batch boundaries; process-kill/materialization/query-count instrumentation remains partial.

New files: `crates/db/src/tests/round2_test.rs`, `ws8_round2_vectors.json`, `crates/db/ruby/ws8_round2_vectors.rb`, `reference-tools/db/ws8-round2-discrimination.py`. Existing Message, Room and room_delete implementation/comments and the additive test module are the only production/test registration changes for these fixes. The regenerated prior vectors retain their actual Rails expectations; only random encrypted fixture bytes or SQLite-default wall-clock setup values may differ.

## Recovery and scope

Opus's committed threads, edits/forwarding and polls and its uncommitted pins/saves were preserved in the earlier takeover; the in-progress work was committed first as `5b373ef2`. Subsequent verified slices added keywords, scheduled messages, categories/favorites, durable queue checks, references/cards, group DMs, search, audit and initial runtime wiring. That handoff is preserved in report commit `fa404c9f`.

Continue2 started from `fa404c9f` and handled its remaining items 1, 3 and 4, pushed as `6811fcd7` including its report. The earlier Astra review follow-up started there. **Slash implementation was delivered by the separate Sol worker** on `rust/ws8a-slash`, forked from `fa404c9f`, and is now merged by the lead. This continuation only adapts its broadcast constructors to WS7's envelope; it implements no slash commands. Existing WS3/WS5 merges and model reference `fec615be` remain in the branch; no rebase or stash was used. The newer decisions pin `d7c7de92` only changes the PWA install partial, outside this model work; its UI acceptance is WS6/WS8b and is not claimed here.

Astra's review of `fa404c9f` was merge after fixes, with no P1s. Its original five vectors regenerated identically and its atomic enqueue rejection passed. The four reproductions under `/home/riels/.cache/rust-port/ws8r/` were re-created as committed Rails-generated checks here. Round-2 reviewed those earlier fixes and continue2; the two round-2 fixes above are preserved in `f46cfb35`.

## Astra fixes and regression proof

1. Audit parameter matching uses full Unicode folding via the [caseless implementation](https://github.com/unicode-rs/rust-caseless), preserving original character boundaries. `paßword` and `paẞword` are filtered; matching cannot begin halfway through the folded `ß` in `ßecret` or `ßession`. Three generated nested/plain input cases are checked through AuditLog::record and a database reload.
2. Search uses the Unicode word class, including connector punctuation and join controls, so `a＿b` remains one quoted FTS phrase. Seventeen Rails queries also cover the other connector characters, zero-width joiners, ordinary whitespace/hyphens, circled letters and numeric characters. Actual SQLite results distinguish adjacent `a b` from separated `a x b`.
3. Continue2 already canonicalized create/edit bodies and propagated adapter errors. The review's `# hi` case plus an edit are now explicit Rails-generated regression checks. Separate deliberate create and edit bypasses both fail the test; the existing three adapter-error rollback checks remain intact. No redundant production change was made for this finding.
4. Unloaded DM members use SQL ORDER BY LOWER(users.name), while supplied members retain Ruby-style Unicode lowercase sorting. Three generated ordering cases cover `Ø/á`, `é/É`, Greek capitals, and both viewer-excluding paths. Custom names return before issuing a member query.
5. The Cable golden no longer contains handwritten `{threadId:4}`. A real Rails threaded Message create triggers ChannelThread#receive; the generator captures ActionCable.server.broadcast and verifies one recipient-stream callback. A Rust Message create generates the corresponding typed event, then passes it through the actual template-free adapter. Expected stream/payload come only from Rails and include roomId. Removing roomId fails the new check. The merge continuation also replaces the old helper-only remove vector with a real Membership#destroy callback capture, including its encoded stream name.

The three new DB tests initially failed together (`.scratch/review-db-before.log`). Seven reproducible review regressions cover full folding, fold boundaries, phrase splitting, default ordering, canonicalization on create, canonicalization on edit and Cable roomId. Raw results appear below. The previous 43 domain/runtime regressions and the merged slash branch's nine regressions are rerun, alongside six migration comparator mutations.

## Complete in this continuation

- Room deletion's model/database path: validated marking; membership removal; huddle/agent revocation; stream ending; durable cleanup and destruction enqueue; bounded DestroyJob; dependency destruction; retry/resume/no-op behavior; claim refresh and stale reenqueue; alive directory scopes. The final thread cascade includes inbox items sourced by WorkThreadEvent and BoardSlaNudge.
- Retention's nine row kinds, calendar-year audit cutoff, strict/inclusive expiry boundaries, unread/pending/active exceptions, grant cleanup unlink/dependent inbox deletion and daily stuck-room backstop. Real app workers and periodic tasks are registered.
- Production Markdown/canonicalization/plain-text/mention failures now propagate through Message writes. Scheduled send, edit and forward run through the production adapter and match Rails-generated output.
- A real storage forward copier verifies source checksum, preserves metadata/content type and stages a new file. The transaction owns its file guard, including when a deferred durable enqueue fails after forwarding returns successfully.
- Reviewed non-agent slash registry, parser, dispatcher and seven command handlers are merged; huddle/event/play execution adapters retain their named deferrals. The real AppRichText slash row check runs after the merge.
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

Review follow-up file changes: `models/audit_log.rs` adds full-fold/boundary matching; `models/search_query.rs` fixes the word class; `models/direct_room.rs` separates SQL/default ordering from supplied-member sorting; their three test files add the review checks. `crates/db/ruby/ws8_review_vectors.rb` and `crates/db/src/tests/ws8_review_vectors.json` hold the fresh Rails oracle. `crates/campfire/src/rich_text.rs` adds the canonicalized create/edit test; `jobs/tests.rs` replaces the circular Cable assertion with a real domain callback check; `ws8_runtime_vectors.rb`/JSON capture Rails callbacks. Workspace/db Cargo files expose the already-locked caseless dependency. `ws8-review-discrimination.py` adds seven failing-test regressions; the identity script now checks User's SQL ordering and UnreadThreadsChannel too (50 implementation files).

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
| Non-agent slash | Reviewed registry, permissions, dispatcher, timezone parser; shrug/me/remind/status/dnd/ooo/poll; existing-user validations and cached calendar/OOO rules; atomic root/thread push and legacy webhook enqueue. | HTTP authorization, JSON/pickers and presentation WS8b; agent invocation WS11; huddle launch WS13; event form/save WS14; play presentation/client execution WS8b/WS5. Full Date._parse grammar and coercions remain partial. |
| Retention/audit | All nine retention row kinds/backstop and audit append-only/action presence, snapshots/redaction/user-agent cap/digest/failure collapse. Retention alone deletes old audit rows, as in Ruby. | Current/audit foreign labels/UI WS4/WS11/WS14/WS17/WS8b; broader nil/IP/coercion parity partial. HuddleGrant destroy inbox/unlink ported; revocation broadcasts WS13. |

Additional row audit: Event destruction removes attendance, calendar entries, event references and source inbox items; EventCalendarEntry's remote-delete snapshot is enqueued in the same transaction (consumer WS14). RepositorySubscription dependent notifications and bot-membership removal are satisfied by the room membership delete_all; these rows have no additional destroy hooks. WorkThreadEvent, BoardSlaNudge, WorkThreadLink, WorkHandoff, AgentStep and PR mapping dependent rows are removed with their source inbox dependencies where applicable; their create/update models remain their owners' work. Venue/agent references and cleanup unlink use Ruby's update_all semantics, intentionally skipping validation/touches. Active Stream update! checks quality; unchanged association validation follows Rails' configured required-FK behavior. The deleted room suppresses normal Stage stream/presence rendering lookups; remaining in-call notifications are explicitly WS13.

ActivityItem remains a minimal reminder/drop writer, not the complete WS12 inbox model. Message bookkeeping is now transactional; infallible renderer compatibility callers elsewhere remain partial under WS2/WS5/WS8b.

## Broadcast contracts

Existing typed events remain: parent thread indicator replacement and user unread-thread Cable; poll replacement; pin badge/count/list replacements and quiet-note append; active-human activity Cable; scheduled conversation append/root unread Cable; quote-card replacement/removal; group room timeline notes and recipient-specific sidebar/header replacement/prepend/removal. Membership removal disconnects after its sidebar removal. QuoteCards, DirectSidebar, RoomHeader, Poll, pin, thread and message partial descriptions remain available to WS8b.

Plain Cable payloads and template-free Turbo remove now go through WS7's single app sink; Turbo frames use its conservative session-bound guard. The unread-thread payload is now differentially checked via real callbacks in both models, including roomId; the earlier handwritten/pass-through Cable case was circular and did not prove that contract. Partial descriptions remain explicitly unregistered for WS8b. The new Rails-generated remove and unread-thread frames are proven through actual WebSocket subscribers; full rendered domain-partial acceptance remains partial. Marking uses membership delete_all, so it intentionally skips individual Membership destroy broadcasts/callbacks, like Ruby. No new Huddle broadcast delivery was fabricated for the deferred WS13 callbacks.

## Ruby scenario counts versus Rust proof

The WS8 Ruby Minitest files listed below were read/count-checked, **not executed as whole suites**. The separate WS10 Rails suites were executed as documented below. Rails runners were executed to produce differential expectations. The counts below describe modeled coverage, not one-to-one full acceptance.

| Ruby test file under `test/` | Ruby scenarios | Rust proof / remaining |
|---|---:|---|
| `jobs/room/destroy_job_test.rb` | 13 | Six earlier consolidated room tests, separate synchronous/asynchronous differentials, a 501-message progress/retry differential, two real queue tests and app worker execution: marking, cascades/scheduled inbox, repeat/live/missing no-op, competing claims/expiry, refreshed claim and transient resume. Exact maximum materialization/query-count instrumentation remains **partial**. |
| `jobs/retention/prune_job_test.rb` | 10 | One consolidated Rails survivor differential covers all nine row kinds and stuck backstop, including one-year leap cutoff; app worker test. The 1001-grant SQL-abort probe proves cross-batch unlink and retry states against Rails. One unlink per batch is implemented; SQL query-count and process-kill instrumentation remain **partial**. |
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
| `services/slash_commands/time_parser_test.rb` | 10 | Merged parser tests cover 3,328 base, 2,610 review and 2,444 supplemental Rails-generated inputs; exhaustive Date._parse/civil-range parity remains partial. |
| `services/slash_commands/dispatcher_test.rb` | 40 | Merged registry/recognition, dispatch/row/callback and user/calendar differentials plus three queue rejection tests and real renderer row checks; agent cases WS11 and HTTP/UI cases WS8b/WS13/WS14. No complete one-to-one Ruby scenario mapping is claimed. |
| `models/channel_thread_board_test.rb`, `channel_thread_auto_assign_test.rb`, `channel_thread_agent_assignment_test.rb`, `channel_thread_handoff_test.rb` | 16 + 10 + 29 + 11 | 0 ported by WS8; WS12/WS11 ownership. |

Extra Rust tests have no separate Ruby scenario count: three renderer error propagation checks, production Markdown/canonicalization checks, staged-file/durable-enqueue fault check, periodic row-failure check and queue atomicity properties. The fixtures and timestamps are produced by Rails, while transaction/file safety assertions test the required queue adaptation.

## Main merge

The lead's uncommitted merge of `origin/main` at `a2fbe296f0675b1a657cf81c1f537b6687451403` is resolved and committed. The reviewed slash merge remains intact, including fallible `prepare_body` for Markdown. No test functions were dropped: both parents have 16 membership tests and 29 room tests, all retained. The shared callback assertions now check one broadcast followed by one reconnect event, rather than filtering away extra events.

| Conflict | Resolution |
|---|---|
| `Cargo.toml` | Keep both caseless and pinned unicode-segmentation dependencies. TOML has 72 unique workspace dependency keys. |
| `crates/campfire/src/main.rs` | Keep both messaging and security module registrations. |
| `crates/db/src/events.rs` | Keep WS7's open BroadcastRequest/trait envelope; serialize WS8 descriptions into it; retain WS8 job/broadcast decoding helpers. |
| `crates/db/src/models.rs` | Union domain modules/exports, retaining MessageChanges, RoomRemovalBroadcast and WorkspacePresenceLease. |
| `crates/db/src/models/membership.rs` | Emit WS7 RoomRemovalBroadcast once, then reconnect, remove thread membership, and refresh direct keys in Rails order. Preserve WS8 alive readers/favorites/categories. |
| `crates/db/src/tests/membership_test.rs` | Keep every test and main's exact ordered broadcast-plus-disconnect assertion; WS8 thread cleanup coverage remains. |
| `crates/db/src/tests/room_test.rs` | Keep every test, exact revocation event assertions, and WS8 deletion/domain behavior. |

Additional integration edits: `broadcasts.rs` derives serialization, stores symbol names as owned strings, and uses WS1's encoded GlobalID params for actual wire stream names. Raw record identities remain available for the slash oracle's callback-argument comparison. Every WS8 and slash broadcaster uses Event::broadcast; there is one delivery arm in Jobs and one WS7 channel sink. The moved template-free adapter publishes Turbo through broadcast_stream_to (including the conservative guard) and Cable JSON through the same server. It logs unregistered partial descriptions with their WS8b owner instead of inventing rendering.

`ws8_runtime_vectors.rb` captures both real Ruby threaded-message and membership-destroy callbacks. Its old helper-only, URI-named remove case is gone. The new subscriber test first failed by timeout (`.scratch/merge-sink-before.log`), failed again with the old URI wire names, then passed after the envelope bridge and encoded naming fix. Existing auth/ban tests also failed with the newer WS19 seed: it starts with fresh verified sessions and loopback IPs. Only their setup changes in `app/tests.rs` and `controllers/presenters/accounts/tests.rs`: explicitly stale/unverified sessions and a separate public ban target. Their assertions remain intact; no production authentication/ban behavior was changed.

Clippy required moving the new trait implementation/helper before test modules. The slash discrimination and generation tools now use four Cargo jobs, `/home/riels/.cache/rust-port/ws8/slash/` and `ws8-` Docker names. Freshly generated setup SQL can contain changed encrypted fixture bytes and SQLite-default wall-clock timestamps; no hand-authored normalization was applied.

The merge inherited main's Rails/UI/test/CI changes, including the Edge PWA fix and background_jobs migration; this worker makes no additional Rails source change. Shared Cargo.lock already retained both registry and vendored html5ever 0.35 entries: locked metadata succeeds without any lock repair. No rebase, stash, parity allowlist/mask change or production/deployment action.

## Commands rerun and raw evidence

All commands below were rerun in this worktree after the WS10 merge. Cargo builds use four jobs and this worktree's target. Scratch stays outside `/tmp`. Docker names start `ws8-`; socket tests use only `48000-48049`. Logs are in `.scratch/`. Tests that require seeds have both reference-built default and first_run seeds; no seed-based skips were observed. No whole Ruby Minitest/browser/media acceptance is implied.

### Locked dependency checks

Run from `rust/`:

```sh
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
python3 - <<'CHECK'
import tomllib
with open("Cargo.toml", "rb") as f: doc = tomllib.load(f)
print(f"workspace dependencies: {len(doc['workspace']['dependencies'])} unique keys; TOML parsed successfully")
CHECK
```

```text
workspace dependencies: 75 unique keys; TOML parsed successfully
```

Metadata exited 0 with no output. Cargo.lock contains the incoming WS10 merge; no manual repair was needed. Zero .claude/delegation paths are tracked.

### Rails reference, seeds and vectors

Run from the worktree root:

```sh
bash rust/reference-tools/db/ws8-reference-check.sh > .scratch/reference-check.log 2>&1
```

```text
WS8 reference: 56 implementation files, icon catalog and Rails db match worktree
```

```sh
docker run --rm --cpus 2 --name ws8-deletion-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_deletion_vectors.rb /out/ws8_deletion_vectors.json' > .scratch/deletion-vectors-final.log 2>&1
```

```text
WS8 deletion vectors: 6 begin checks, 22 cascade checks, 1 calendar jobs, 2 recovery claims
```

```sh
docker run --rm --cpus 2 --name ws8-retention-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2024-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_retention_vectors.rb /out/ws8_retention_vectors.json' > .scratch/retention-vectors-final.log 2>&1
```

```text
WS8 retention vectors: 9 row kinds, 11 survivor checks, 1 stuck room jobs
```

```sh
docker run --rm --cpus 2 --name ws8-loop-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/campfire/src:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_loop_vectors.rb /out/ws8_loop_vectors.json' > .scratch/loop-vectors-final.log 2>&1
```

```text
WS8 loop vectors: 3 first-row SQL failures, 4 continuation checks
```

```sh
docker run --rm --cpus 2 --name ws8-review-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_review_vectors.rb /out/ws8_review_vectors.json' > .scratch/review-vectors-final.log 2>&1
```

```text
WS8 review vectors: 3 redaction, 17 phrase queries, 3 direct ordering, 2 canonicalized writes
```

```sh
docker run --rm --cpus 2 --name ws8-runtime-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/campfire/src:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_runtime_vectors.rb /out/ws8_runtime_vectors.json' > .scratch/runtime-vectors-final.log 2>&1
```

```text
WS8 runtime vectors: 8 Markdown, 4 canonicalization, 5 periodic tasks, 2 template-free broadcasts, 3 write flows, 1 attachment copy
```

```sh
docker run --rm --cpus 2 --name ws8-round2-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_round2_vectors.rb /out/ws8_round2_vectors.json' > .scratch/round2-vectors-final.log 2>&1
```

```text
WS8 round-2 vectors: 2 destruction paths, 501-message resume, 1001-grant resume, 6 bookkeeping checks, 4 ordered callback events
```

```sh
docker run --rm --cpus 2 --name ws8-mail_merge-vectors --entrypoint '' --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_mail_merge_vectors.rb /out/ws8_mail_merge_vectors.json' > .scratch/mail_merge-vectors-final.log 2>&1
```

```text
WS8 mail merge vectors: 6 Rails validations, 1 token validation, 1 real RoomMailbox post
```

```sh
PARITY_NAMESPACE=ws8 PARITY_OWNER=ws8 PARITY_CPUS=2 PARITY_IMAGE=ws19-reference-fec615be TMPDIR="$PWD/.scratch" bash rust/parity/bin/seed build default first_run > .scratch/merge-seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The model image identity check covers 56 implementation files, icons and Rails db. Seed construction uses the pinned WS19 reference image and current checked-in seed scripts. Seven WS8 vector generators call our real Ruby callbacks/dispatchers; generated setup SQL/checks may contain encrypted random bytes, generated client UUIDs or SQLite-default wall-clock timestamps. Expectations were not hand-edited. The reviewed slash corpora are inherited from the merged branch; this continuation reruns their Rust comparisons and discrimination, not their oracle generator or 56-scenario slash export validator. Their prior results in `plans/ws8-slash-report.md` are historical. The general Rails rollback validator below is rerun here.

### Rust checks, including the requested binary and Cable suites

Run from the worktree root:

DB, queue and process-crash tests:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -- --test-threads=4 > .scratch/test-final.log 2>&1
```

```text
test result: ok. 387 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 36.72s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Full richtext renderer:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_richtext -- --test-threads=4 > .scratch/renderer-final.log 2>&1
```

```text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.35s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.16s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

WS8 production renderer adapter:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire rich_text::tests::runtime_ -- --test-threads=4 > .scratch/app-rich-text-final.log 2>&1
```

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.29s
```

WS8 real job/broadcast/storage/loop runtime:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire jobs::tests::ws8_ -- --test-threads=4 > .scratch/app-jobs-final.log 2>&1
```

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 289 filtered out; finished in 0.24s
```

Merged slash production adapter:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire slash_runtime_ -- --test-threads=4 --nocapture > .scratch/app-slash-final.log 2>&1
```

```text
WS8 slash runtime: 19 Rails-generated rich-text/FTS row comparisons
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 295 filtered out; finished in 2.03s
```

Full WS10 mail crate:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_mail -- --test-threads=4 > .scratch/mail-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.71s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Production mail/relay adapter:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire mail::tests:: -- --test-threads=4 > .scratch/app-mail-final.log 2>&1
```

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.51s
```

Cable package (actual Cargo name campfire_cable):

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_cable -- --test-threads=4 > .scratch/cable-final.log 2>&1
```

```text
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test replays_reference_frames ... ok
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Full campfire binary:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/campfire-bin-final.log 2>&1
```

```text
WS8 slash runtime: 19 Rails-generated rich-text/FTS row comparisons
test channels::tests::golden::replays_reference_frames ... ok
test result: FAILED. 293 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 24.11s
```

Full workspace clippy, including vendored html5ever:

```sh
MAIL_TEST_PORT_RANGE=48000-48049 CABLE_TEST_PORT_RANGE=48000-48049 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.18s
```

The normal DB run ignores three environment-dependent tests, all executed by the differential command below. Cable ignores only its Rails recorder; the binary ignores the Rails recorder and optional push-latency measurement. **Binary acceptance is partial:** exit 101, only `controllers::presenters::accounts::tests::manages_bots` fails (the inherited bot-key assertion, WS11). All other listed Rust test commands and workspace clippy exit 0. Neither replay is skipped. The separate adapter selections deliberately filter other binary tests; the full binary command exercises them. Broader workspace unit suites and browser/media-vector acceptance are not claimed.

### Rails rollback, timestamp and migration differential

Run the first command from `rust/`:

```sh
OUT="$PWD/../.scratch/differential" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models CARGO_TARGET_DIR="$PWD/target" TMPDIR="$PWD/../.scratch" CARGO_BUILD_JOBS=4 mise exec rust@1.98.1 -- bash reference-tools/db/differential.sh > ../.scratch/differential-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 387 filtered out; finished in 4.06s
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

Rails reads/validates all 122 representative exported rows and exercises rollback reads/edits/deletes. This is not exhaustive validation of every possible row combination.

### WS10 Rails suites and Rust-to-Rails mail rollback

```sh
docker run --rm --cpus 2 --name ws8-mail-rails-final --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -v "$PWD/app:/rails/app:ro" -v "$PWD/config:/rails/config:ro" -v "$PWD/db:/rails/db:ro" -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/reference-tools/mail:/tools:ro" -v "$PWD/rust/vectors/mail:/out" ws8-reference-models -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails test test/mailboxes/room_mailbox_test.rb test/mailboxes/bounce_mailbox_test.rb test/mailboxes/relay_ingress_test.rb test/mailers/security_mailer_test.rb test/mailers/two_factor_mailer_test.rb test/controllers/rooms/inbound_email_addresses_controller_test.rb' > .scratch/mail-rails-final.log 2>&1
```

```text
46 runs, 136 assertions, 0 failures, 0 errors, 0 skips
```

```sh
docker run --rm --cpus 2 --name ws8-mail-review-rails-final --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -v "$PWD/app:/rails/app:ro" -v "$PWD/config:/rails/config:ro" -v "$PWD/db:/rails/db:ro" -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/reference-tools/mail:/tools:ro" -v "$PWD/rust/vectors/mail:/out" ws8-reference-models -ec 'redis-server --daemonize yes; bin/rails db:prepare >/dev/null; bin/rails test /tools/review_test.rb' > .scratch/mail-review-rails-final.log 2>&1
```

```text
4 runs, 37 assertions, 0 failures, 0 errors, 0 skips
```

```sh
CAMPFIRE_MAIL_EXPORT_DIR="$PWD/.scratch/mail-rollback-merge" TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_mail --test inbound export_for_rails -- --ignored --exact --nocapture > .scratch/mail-export-final.log 2>&1
docker run --rm --cpus 2 --name ws8-mail-rollback --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -v "$PWD/app:/rails/app:ro" -v "$PWD/config:/rails/config:ro" -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/reference-tools/mail:/tools:ro" -v "$PWD/.scratch/mail-rollback-merge:/rails/storage" ws8-reference-models -ec 'bin/rails runner /tools/rollback.rb' > .scratch/mail-rollback-final.log 2>&1
```

```text
mail rollback artifact: 2 posted messages, 3 inbound emails, raw email and attachment files
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.17s
Rails rollback: Email bot, room, 2 messages, 3 inbound emails and attachment bytes validated
```

```sh
mkdir -p .scratch/mail-oracle-regenerated
docker run --rm --cpus 2 --name ws8-mail-goldens --entrypoint sh --env-file rust/parity/.env.reference -e RAILS_ENV=test -v "$PWD/app:/rails/app:ro" -v "$PWD/config:/rails/config:ro" -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/reference-tools/mail:/tools:ro" -v "$PWD/.scratch/mail-oracle-regenerated:/out" ws8-reference-models -ec 'bin/rails runner /tools/generate.rb' > .scratch/mail-goldens-final.log 2>&1
cmp .scratch/mail-oracle-regenerated/reference.json rust/vectors/mail/reference.json
```

```text
mail reference: 579 authentication headers, 93 HTML cases, 34 MIME messages
```

The fresh mail oracle compares byte-for-byte with the committed reference.json (cmp exit 0, no output). The multipart oracle regenerated in the Rails reviewer suite also remains unchanged.

Mail normally ignores only its artifact exporter; it is executed above. Production MIME calibration and live SMTP/provider/browser acceptance are historical/unproven here, not silently included in these results.

### Failing-first and discrimination evidence

The round-2 tests and mutations above add fresh failure-first proof. The earlier shared-sink subscriber test failed by an actual frame timeout before its bridge/stream naming fixes; the real callback oracle also participates in the rerun shared-sink regression. The three existing seed-sensitive tests failed in the initial binary run before their setup changes; their assertions were retained. Earlier room/retention/renderer/copy/loop and Astra failing-first logs remain in `.scratch/*-before.log`. The selected reproducible checks below reject compile-only failures, require named test assertions/timeouts, and restore source in finally. All seven WS8 scripts and the mail script run sequentially without other source edits/builds. Afterward the implementation diff was checked for deliberate mutation remnants, and the final clean Rust pass above reran. Older recovered tests beyond these selections remain individually unproven; no complete TDD history for Opus's work is invented.

Initial compiled merge baselines (before source assignment, token validation and boot installation):

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.15s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 5.11s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.11s
```

```sh
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-discrimination.py > .scratch/discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-maintenance-discrimination.py > .scratch/maintenance-discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-runtime-discrimination.py > .scratch/runtime-discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-review-discrimination.py > .scratch/review-discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-slash-discrimination.py > .scratch/slash-discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-round2-discrimination.py > .scratch/round2-discrimination-final.log 2>&1
CABLE_TEST_PORT_RANGE=48000-48049 python3 rust/reference-tools/db/ws8-mail-merge-discrimination.py > .scratch/mail-merge-discrimination-final.log 2>&1
```

```text
keyword-rules: detected (14 failing tests)
test result: FAILED. 0 passed; 14 failed; 0 ignored; 0 measured; 376 filtered out; finished in 0.14s
scheduled-claim: detected (4 failing tests)
test result: FAILED. 21 passed; 4 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.60s
scheduled-validation: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.62s
scheduled-access: detected (5 failing tests)
test result: FAILED. 20 passed; 5 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.61s
scheduled-thread-drop: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.61s
category-validation: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.21s
category-nullify: detected (1 failing tests)
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.22s
favorite-writes: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.32s
transaction-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.08s
save-touch-golden: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.16s
thread-job-atomicity: detected (2 failing tests)
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.16s
reminder-job-atomicity: detected (1 failing tests)
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.27s
quote-references: detected (6 failing tests)
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 384 filtered out; finished in 0.12s
group-dms: detected (7 failing tests)
test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 383 filtered out; finished in 0.25s
search-grammar: detected (4 failing tests)
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 386 filtered out; finished in 0.11s
audit-contract: detected (9 failing tests)
test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 381 filtered out; finished in 0.31s
runtime-markdown: detected (5 failing tests)
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 291 filtered out; finished in 0.29s
runtime-jobs-broadcasts: detected (5 failing tests)
test result: FAILED. 4 passed; 5 failed; 0 ignored; 0 measured; 287 filtered out; finished in 5.18s
quote-job-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.07s
WS8 discrimination: 19 mutations detected; sources restored
room-start: detected (4 failing tests)
test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 384 filtered out; finished in 0.26s
room-worker: detected (3 failing tests)
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 384 filtered out; finished in 0.23s
room-live-guard: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.13s
room-sweep: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.08s
room-readers: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.09s
retention: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.09s
room-enqueue-atomicity: detected (2 failing tests)
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.22s
maintenance-registry: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.14s
cleanup-enqueue-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.11s
sla-dependent-inbox: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.14s
WS8 maintenance discrimination: 10 mutations detected; sources restored
renderer-failure-propagation: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 387 filtered out; finished in 0.26s
write-flow-markdown: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.22s
real-file-copier: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.12s
file-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.16s
loop-saved: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.12s
loop-scheduled: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.13s
loop-poll: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.12s
WS8 runtime discrimination: 7 mutations detected; sources restored
review-audit-fold: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.08s
review-audit-boundaries: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.08s
review-search-phrase: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.15s
review-direct-order: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.12s
review-create-canonical: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.12s
review-edit-canonical: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.14s
review-cable-room: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.13s
WS8 review discrimination: 7 mutations detected; sources restored
registry: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.03s
parser: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.04s
dispatch: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.12s
callbacks: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 4.47s
validation: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 1.09s
push-atomicity: detected (3 named tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 49 filtered out; finished in 0.18s
webhook-atomicity: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.11s
runtime-richtext: detected (1 named tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.25s
review-fold: detected (2 named tests)
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 388 filtered out; finished in 8.23s
WS8 slash discrimination: 9 mutations detected; all 11 new tests failed; sources restored
round2-sync-dependencies: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.10s
round2-async-dependencies: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.11s
round2-room-resume: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.26s
round2-retention-resume: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.20s
round2-create-postcommit: detected (3 failing tests)
test result: FAILED. 7 passed; 3 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.59s
round2-edit-postcommit: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.16s
round2-touch-postcommit: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.11s
round2-callback-order: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.12s
WS8 round-2 discrimination: 8 mutations detected; sources restored
mail-source: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.07s
mail-token-validation: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.07s
mail-renderer-boot: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 5.11s
mail-index-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.11s
WS8 mail merge discrimination: 4 compiled regressions detected; implementation restored
```

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=48000-48049 MAIL_TEST_PORT_RANGE=48000-48049 CARGO_BUILD_JOBS=4 python3 rust/reference-tools/mail/verify-mutations.py > .scratch/mail-discrimination-final.log 2>&1
```

```text
throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.15s
attachment: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.11s
room-filter: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 53 filtered out; finished in 0.10s
auth-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
html-removal: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.02s
atomic-relay: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 295 filtered out; finished in 0.48s
```

Historical shared-sink baseline (actual compiled timeout before the earlier merge fixes; its mutation was rerun above):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 290 filtered out; finished in 5.13s
```

64 WS8 model/runtime/slash/mail-merge mutations, six WS10 mail mutations and six migration comparator mutations detected; no deliberate mutation remains committed.

## Cross-workstream touches

- WS2: retain Room/Membership/Message extensions, alive scopes, fallible RichText seams, destruction dependencies and export. RoomType serialization supports the shared envelope; no schema redesign.
- WS3: keep atomic job persistence, additive messaging registry and periodic tasks. Jobs delivers broadcasts through WS7 once. Cleanup/calendar/push DTOs still need their named consumers.
- WS5/WS10: retain production fallible rendering and staged forward-copy rollback. Mail uses the shared creation/renderer path; local routing, relay and staged attachment rollback are proven. Browser/complete presentation and webhook fanout remain partial.
- WS7: use its open broadcast envelope, RoomRemovalBroadcast, ordering, guarded publisher and both replay suites. Broadcast description changes propagate through models/tests and the reviewed slash constructors, without implementing slash logic.
- WS6/WS19: inherited main's UI/parity work; update three integration-test setups for the newer seed. Production controllers/templates were not changed by this worker.
- WS11/WS12/WS13/WS14/WS17: deletion dependencies remain implemented; their domain consumers/callbacks and the known binary failure remain named below.
- Lead's slash merge: reviewed handlers/parser are retained. Resource limits and discrimination anchors follow the merged interface. Its separate tracked report records historical branch verification.
- Main's Rails/UI/test/CI edits are inherited from the authorized merge. No additional Rails source edits, PR, deployment or production-data actions. The explicitly requested external report is identical to its Rust-only tracked copy; zero `.claude/delegation/**` paths are tracked.

## Precise remaining work / restart point

1. **Push runtime (WS17 with WS8a integration):** implement/register SavedItem::ReminderPushJob and ChannelThread::PushMessageJob with the actual notification/DND/quiet-hours policy and Web Push services. Durable DTOs currently lack these handlers; unknown-class is not masked by no-op consumers.
2. **Huddle/Stage (WS13):** real Huddle::CleanupJob/reconciler/network retries; in-call ended, leave and voice effects on revocation; synchronous alive-Stage last-grant stream behavior and broader membership host-loss paths. Validated revocation/cleanup rows and atomic enqueue are complete here; remote consumption is not.
3. **Calendar/Event (WS14):** Calendar::RemoteDeleteJob API consumer; Event create/update/reference/audit-label model and slash event form/save. Delete snapshots and durable enqueue before entry deletion are implemented.
4. **Thread work/agents (WS11/WS12):** deleted-work snapshot, work_unassigned ledger/webhooks and their atomic jobs; board removal; hard-user-destroy/stream finalization. Agent slash invocation/capability/rate/event/webhook behavior remains WS11; import suppression WS16 and other reference sync WS14/WS15.
5. **HTTP/rendering (WS8b with WS7/WS10/WS3):** begin_destroy route switch; ForwarderCopier and attachment processing; rendering the emitted QuoteCards/group/pin/poll/thread/message/UserStatus/OooNotice partials; slash/autocomplete endpoints, active-human/room/thread authorization and JSON/picker results; route-level enqueue rejection and browser acceptance. Slash huddle launching WS13, event execution WS14, play presentation/client sound WS8b/WS5. Template-free subscriber delivery and the production mail routing/relay adapter are proven here; broader template/browser acceptance remains partial.
6. **Parity/instrumentation still partial:** 500-message materialization/query-count proof, one retention unlink per batch, process-kill worker-resume instrumentation beyond the proven 501-message/1001-grant SQL-abort retries, non-Stage imported stream history, broadcast-failure/hard-user/legacy conversion cases, complete ActivityItem accessibility, broader Unicode/coercion/nil/IP and search timezone/DST, foreign audit context/labels, exhaustive Ruby Date._parse and civil-range behavior. Old infallible RichText callers and storage crash/orphan recovery remain WS2/WS5/WS8b/WS10.
7. **Known binary failure (WS11):** manages_bots bot-key assertion. Full binary acceptance remains partial until its owner fixes it; no suppression/skip/expectation change is made here.

No external blocker remains for this merge and the committed WS8 database/runtime slices. This report is a named integration handoff, not complete WS8a or production/cutover acceptance.
