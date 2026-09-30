# WS8a messaging models — partial handoff, 2026-09-29

Branch: `rust/ws8-messaging-models`. Verified code commit: `1e9f62c7` (the following report commit changes documentation only). This is **partial WS8a**, not acceptance of the entire brief. Controllers, templates and rendered broadcasts remain WS8b. No PR was opened. The user explicitly requested pushing, overriding the common brief's no-push default.

## Recovery and committed slices

At takeover, Opus had committed threads (`62e417ff`), edits/forwarding (`5eca0d11`) and polls (`6cea6f45`). Its uncommitted work contained pins, saved items, a minimal activity writer, dependencies, tests and rollback/timestamp changes. That work was preserved first in `5b373ef2`, with validation explicitly still pending.

Subsequent coherent slices (including this continuation):

- `5c57359b`: pins/saved-item verification, Rails rollback validation and regenerated save-touch golden.
- `9ccb91e3`: keyword alerts and matcher.
- `276b785d`: scheduled-message models and transactional dispatcher.
- `722f3fc0`: categories and membership favorites.
- `8f3dd903`: merge the completed WS3/WS5 work and the Rails fixes at the decisions file's current reference pin, `fec615be`, preserving history.
- `47edc10b`: real WS3 queue rollback/retry tests for messaging writes and their discrimination checks.
- `ccd109a5`: quote reference validation/sync, edit enqueue, capped refresh and removal callbacks.
- `22da668a`: group DM mutations, name validation, literal notes and recipient-specific directory events.
- `c7d333c3`: Rails SearchQuery grammar, bound SQL filters and cursor windows.
- `218f84fe`: audit snapshots, redaction, URL summaries, action vocabulary and failure throttling.
- `1e9f62c7`: production Markdown adapter, three periodic tasks, quote worker and template-free broadcasts.

The continuation began with fetch and merge of origin/main. The merge reported `Already up to date.`: the earlier merge already contains the WS3 queue and reference pin `fec615be`. No additional Rails changes were made. This is a natural model/runtime boundary, **not completed WS8a**: room destruction, slash commands and retention still require their complete callback/dependency ports.

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
| `crates/db/src/rich_text.rs` | Model Markdown/canonicalization/plain-text seams and test stand-in. Production AppRichText now delegates to WS5, with Rails differential checks; fallible canonicalization/plain-text error propagation still needs the trait follow-up noted below. |
| `crates/db/src/broadcasts.rs`, `events.rs` | Typed Turbo/Cable event descriptions and accessors. The renderer/delivery contract still needs WS7/lead coordination. |
| `crates/db/src/models.rs`, `lib.rs`, `tests.rs` | Exports and test modules. |
| `crates/db/Cargo.toml`, `Cargo.lock` | Workspace regex dependency for keyword matching. |
| `crates/db/src/tests/channel_thread_test.rs`, `message_edit_test.rs`, `forwarder_test.rs`, `poll_test.rs`, `message_pin_test.rs`, `saved_item_test.rs`, `scheduled_message_test.rs`, `keyword_alert_test.rs`, `room_category_test.rs` | Domain tests, real SQLite claims, rollback, timestamp and typed-event assertions. |
| `crates/db/src/tests/fixtures_test.rs` | Export representative rows for Rails: threads/tags/memberships, forwards/edits/replies, polls/votes/options, pins/saves/activity, keywords/categories/favorites and scheduled pending/sent/dropped history. |
| `crates/db/ruby/rollback.rb` | Rails `valid?` for all exported changed rows plus domain read/write/delete checks. |
| `crates/db/ruby/save_touches.rb`, `crates/db/src/tests/save_touches_test.rs`, `message_save_touches.json` | Extend the Rails-produced timestamp golden to 64 cases, including pin/unpin/save/schedule. JSON was generated from Rails, not invented. |
| `crates/campfire/src/jobs.rs`, `jobs/periodic.rs`, `jobs/tests.rs` | Quote-refresh handler, reminder/scheduled/poll tasks, template-free Cable/remove sink; real runner and Rails payload/registration checks. Other partial delivery and notification workers remain outstanding. |
| `crates/jobs/src/tests.rs`, `tests/ws8_messaging_test.rs` | Four tests against WS3's actual `JobQueue`, injecting a trigger that rejects job inserts and proving rollback and successful retry. |
| `reference-tools/db/differential.sh` | Mount this checkout's test files, use cargo `-j 4`, and allow a workstream-specific reference image. |
| `reference-tools/db/ws8-discrimination.py` | Nineteen deliberate regressions; require real failing tests, reject compilation-only failures, restore sources after each mutation. |
| `crates/db/src/models/message_reference.rs`, `message_reference_test.rs`, Ruby/vector files | Required associations/self/uniqueness; reference extraction cap and HTML recovery; idempotent sync; atomic edit refresh job; capped/batched replacement events; deletion stamps without room touches. |
| `crates/db/src/models/direct_room.rs`, `direct_room_test.rs`, Ruby/vector files | Group administration cap/privacy guards, display names, rename throttling, member notes, leave and per-member directory events. |
| `crates/db/src/models/room.rs` | Direct-name validation on generic create/update; core last-member deleted marker. Complete begin_destroy validation, revocation, cleanup and enqueue remain WS8a follow-up. |
| `crates/db/src/models/search_query.rs`, `search_query_test.rs`, Ruby/vector files | Operator grammar/chips, Unicode words, bound literal LIKE values, date/attachment/thread predicates, accessible alive-room windows with a (created_at,id) cursor. Existing Message search APIs now delegate here. |
| `crates/db/src/models/audit_log.rs`, `audit_log_test.rs`, Ruby/vector files | Append-only API, context/label snapshots, action presence, secret filtering, user-agent truncation, origin digest and transactional sign-in failure collapse/cap. |
| `crates/campfire/src/rich_text.rs`, `build.rs`, `Cargo.toml`, runtime Ruby/vector files | WS5 renderer with unique active room-member SGIDs; reference-config brand/alias assets and current workspace icons; canonicalization and Markdown plain text. Build honors CAMPFIRE_REFERENCE. |
| `plans/ws8-wave2-report.md` | Tracked copy of this handoff report. |

## Design and partial boundaries

- The schema remains Rails-owned. WS8 added no migrations. The merged WS3 `background_jobs` migration is included in schema/differential/replay verification.
- Writes and jobs share SQLite. Thread push enqueue moved out of the after-commit receive callback into Message's transaction. The real queue tests prove a rejected enqueue rolls back the post, thread state/membership, scheduled claim/history or reminder claim/activity as applicable. Full HTTP trigger tests remain **partial**, owned by WS8b with WS3.
- Scheduled claims use a conditional SQL update inside a real write transaction. A claim exactly five minutes old remains held; strictly older claims can be recovered. Four independent SQLite connections contest one row in the domain test. Failed SQL writes release the claim through rollback; locked threads explicitly release it for retry.
- Saved reminder dispatch rereads the row under the writer transaction. Concurrent dispatchers notify once. Its `dispatch_due(Tx, ...)` convenience helper batches rows in one transaction; runtime registration must instead use `due_reminder_ids` plus a separate `dispatch_reminder` write per row, rescuing/logging each error. The runtime now registers this per-row path. Registration is differentially checked; dedicated injected per-row failure/rescue tests for the app loops remain **partial**.
- Production AppRichText now delegates Markdown, canonicalization and Markdown plain text to WS5. Rails vectors cover eight sources (signed member mentions, missing members, brand aliases, workspace icons, tables and task lists) and four canonicalization inputs. Specific end-to-end scheduled/edit/forward flows, the real storage copier and legacy conversions remain **partial**. The existing infallible canonicalization/plain-text trait logs and falls back when WS5 raises; propagating those exceptions through model writes is deferred to WS8a with WS2/WS5, not claimed equivalent.
- Keyword tests cover every case in the two named Ruby files, including overlap, punctuation, flexible whitespace and Unicode boundaries. Ruby's untyped `Array`/`to_s` coercions and multi-character Unicode case folding are not proven; these remain **partial** at the future input boundary.
- No parity masks or allowlists were changed. The app-wide test suite and browser parity were not run; there is no production/cutover claim.

## Feature × Rails validations/callbacks

“Ported” describes the specified model helpers and tested cases, not complete end-to-end acceptance. Every unresolved integration or unproven parity edge is marked partial.

| Brief feature | Ported validations, scopes and callbacks | Partial/deferred and owner |
|---|---|---|
| 1. Threads | Required room/creator/name; name limits/defaults; parent-room and board constraints; archive/stale/lifecycle rules; tags and thread memberships; unread updates; recount excludes system notes/unfinished streams; parent stamp/indicator; root timeline excludes replies; atomic thread push request. | Board/work/assignment/handoff behavior: WS12 with WS11 agents. Hard-user-destroy callback path: WS11. Broadcast-failure isolation scenarios and push delivery: WS8a follow-up/WS7/WS17. |
| 2. Replies/edits/forwarding | Reply conversation checks and tombstones; required body/source and existing Message limits; source/body edited timestamp rules; forward metadata pairs; destination membership/board/locked guards; mention snapshots; client-id dedup; system_note/action/embeds flags; transactional attachment bookkeeping. | Legacy conversions; real blob copier/storage cleanup: WS8a with WS5/storage owner. Non-message reference resync remains with WS14/WS15; message quote resync is ported. HTTP decoding/auth and append/replace rendering: WS8b. |
| 3. Polls | Message/option/vote validations, ballots, due claims, parent touch/dependents and typed replacement event retained. Per-row poll closing is now registered with the scheduler. | Injected app-loop failure isolation and odd external coercions still unproven; rendering WS7/WS8b. |
| 4. Pins/saves | Pin/saved validations, caps, notes, stamps, dependency handling, reminder claims/activity/job/payload retained. Per-row reminders are now scheduler-registered. | Hard user deletion WS11; full inbox access policy WS12; real DND/quiet-hours policy and the unregistered reminder push handler WS17. App-loop failure isolation needs WS8a follow-up. |
| 5. Scheduled | Source/time/conversation validations, sendability/history, claim/recheck/drop/dependency handling retained; WS5 runtime rendering and per-row scheduler registration now wired. | Full scheduled-send differential with the production adapter and injected loop failures remain WS8a follow-up; inbox accessibility WS12; rendering/push WS8b/WS7/WS17. |
| 6. Keywords | Required user/phrase; normalization, 80 characters, SQL LOWER duplicate semantics, cap 20 on create; literal independent matcher passes, insertion order and user dedup. | Recorder integration: WS17. Untyped coercions and broader Unicode case-fold parity: WS8a/WS8b follow-up, partial. No model broadcasts applicable. |
| 7. References/cards | Required message/source, self and uniqueness validations; normalized capped extraction; HTML code exclusion/hrefs; nonstreaming create/edit sync; atomic quote job; capped/batched refresh and deletion stamp/removal events. Real job enqueue rollback and handler execution tested. | Viral-card fragment-cache behavior and actual rendered placeholders/endpoints WS8b/WS7; finalize integration WS11; import broadcast suppression WS16. Job runner test proves execution on a missing-source no-op; cross-room card logic is proven separately at the model layer. |
| 8. Categories/favorites | Category user/name presence, 50 characters, per-user position/id ordering, collapse, next position; destruction nullifies category assignments without touching memberships; membership required user/room and same-user category; favorite append/idempotence/unfavorite/reorder. | HTTP current-user scopes, channel restrictions and authorization: WS8b. No model broadcast or parent touch applicable. |
| 9. Group DMs | Existing member-key/unkeyed lookup foundation retained; cap 10 and private-pair guards, direct-name validation on every generic write path, rename/add/leave notes, rename window and directory/header events now ported. | Last-member deletion only marks deleted/removes memberships: complete validation, grant revocation, cleanup and transactional destroy enqueue remain WS8a. Huddle participant/sidebar locals WS13/WS8b. Broader Ruby scenarios are partial; no HTTP or rendered-sidebar acceptance claimed. |
| 10. Soft delete | Existing alive scopes retained; new search, reminders and scheduled-send guards account for deleted rooms; minimal last-member deleted marker exists. | **Partial**: complete begin_destroy validations/callbacks, grants/streams/cleanup, destroy job batches 500, recovery claims, retention backstop and exhaustive reader audit remain WS8a with WS3/WS11/WS13/WS14. Do not invoke the minimal marker as a finished room-deletion workflow. |
| 11. Search | from/in/has/before/after/on/is:thread, invalid-token preservation, chips/removal queries, Unicode FTS terms, literal bound LIKE filters, alive/access scopes and bounded tuple cursor windows. Existing dedup and nonstreaming/system-note index rules retained. | Board/work/events side-section query seams are not implemented (WS12/WS14). Time-zone/DST and broader coercion differentials, HTTP and query preloads remain WS8a/WS8b follow-up. |
| 12. Slash commands | No new registry/dispatcher/time parser or handlers. | **Not ported**: shrug, me, remind, status, dnd, ooo, poll; explicit huddle/event/play stubs still needed for WS13/WS14. WS8a follow-up. |
| 13. Retention/audit | Audit action presence (unknown actions allowed), 63-action vocabulary, actor/target snapshots, nested secret filter, scalar wrapping, user-agent cap, origin digest, append-only methods and transactional failure throttling. | **Retention not ported** (WS8a/WS3, HuddleGrant callbacks WS13). Current-context integration and actor/target lookup/admin UI WS4/WS17/WS8b; foreign-domain labels WS11/WS14; broader nil/IP/coercion cases remain partial. |

ActivityItem's limited writer validates the recipient and event vocabulary and only broadcasts to active humans. Its full inbox scopes, source policy, huddle payload and other event-specific behavior are deferred to WS12/WS13; it must not be mistaken for a complete ActivityItem port.

## Broadcasts emitted

All are typed Event::Broadcast. The app delivers plain Cable payloads and template-free Turbo removes through WS7; broadcasts requiring partials remain explicitly unregistered for WS8b. Payload shapes are verified; subscriber-level delivery was not re-tested in this slice.

| Trigger | Stream / action / target / data |
|---|---|
| Thread reply count, finalization or destruction | Parent room `:messages`; replace with maintain_scroll; `thread_indicator_message_<client_id>`; parent id and committed reply count. |
| Thread receive | `user_<id>_unread_threads`; Cable `{threadId, roomId}`. |
| Poll vote/close | Message conversation `:messages`; replace with maintain_scroll; `poll_<id>`; poll id. |
| Pin/unpin | Room `:messages`; three replacements with maintain_scroll: message pin badge, room pins count, room pins list; message/room ids. |
| Pin note | Room `:messages`; append to the STI room messages target; note message id. System notes cause no push/unread fanout. |
| Activity create or unread refresh | `user_<id>_activity`; Cable `{activityItemId}` to active human only. |
| Scheduled send | Thread or room `:messages`; append to that conversation messages target; message id. Root sends also emit `user_<id>_unread_rooms` / `{roomId}` according to involvement/mention rules. |

Additional events: source edit refresh/deletion emits conversation-scoped replacements with maintain_scroll at `message_link_cards_message_<client_id>` and QuoteCards(message_id). Group notes append to the STI room timeline; remaining members receive replacements of `list_<STI-room>` and `header_<STI-room>`, and newcomers receive prepends to direct_rooms. Membership destroy removes the leaver's sidebar row before disconnecting. DirectSidebar carries membership/member ids; RoomHeader carries room/recipient ids. Huddle participant locals and partial HTML remain WS13/WS8b.

Quote refresh now emits an implemented model callback and has a registered worker. The app sink still cannot render QuoteCards, DirectSidebar, RoomHeader, Poll, pin/thread or message partials; those events log the explicit WS8b integration boundary. End-to-end HTML delivery remains partial.

## Ruby test counts versus Rust coverage

Counts are named Ruby test scenarios, not a claim that the Ruby Minitest files were executed here. Rails runners were executed for the differential/rollback checks below. Consolidated scenarios and added Rust checks make function totals different.

| Ruby file under `test/` | Ruby scenarios | Covered scenarios / Rust tests | Remaining |
|---|---:|---|---|
| `models/channel_thread_messages_count_test.rb` | 19 | 16/19; 16 Rust functions in the count group, including an extra finalize-claim test and a consolidated finalize scenario | Two broadcast-failure isolation scenarios; hard-user-delete scenario. |
| `models/message_conversations_test.rb` | 5 | 3/5; 3 Rust | Two legacy conversion scenarios. |
| `models/thread_tag_test.rb` | 2 | 2/2; 2 Rust | None in this file; all three groups plus 9 core-thread checks total 30 Rust tests. |
| `services/messages/forwarder_test.rb` | 8 | 8/8 modeled scenarios; 9 Rust | Real storage and full forwarding differential with production Markdown remain partial despite modeled coverage. |
| `models/poll_test.rb` | 15 | 15/15; 19 Rust | Scheduler is registered; rendered delivery and runtime fault-isolation tests remain partial. |
| `models/message_pin_test.rb` | 16 | 15/16; 17 Rust | Hard-user-delete broadcast scenario, WS11. |
| `models/saved_item_test.rb` | 12 | 11/12; 13 Rust in model group | ActivityItem accessible_to after room access loss, WS12. |
| `models/saved_item/reminder_dispatcher_test.rb` | 7 | 7/7; 9 Rust | Adapter now registered; app-loop fault-injection proof remains partial. |
| `models/saved_item/reminder_pusher_test.rb` | 4 | 4/4 through payload/policy seam; 5 Rust | Actual policy/delivery partial, WS17. All saved groups total 27 Rust tests. |
| `models/scheduled_message_test.rb` | 8 | 8/8; 8 Rust | Full ActivityItem query privacy and renderer integration remain partial. |
| `models/scheduled_message/dispatcher_test.rb` | 13 | 13/13 modeled scenarios; 13 Rust plus 4 SQLite/state checks | Registration and renderer seam now wired; full app send differential remains partial. Combined scheduled tests: 25. |
| `models/keyword_alert_test.rb` | 4 | 4/4; 4 Rust | Broader input/coercion parity remains partial. |
| `models/notifications/keyword_matcher_test.rb` | 10 | 10/10; 10 Rust | Broader Unicode folding remains partial. Combined keyword tests: 14. |
| `models/channel_thread_board_test.rb` | 16 | 0/16 | WS12. |
| `models/channel_thread_auto_assign_test.rb` | 10 | 0/10 | WS12/WS11. |
| `models/channel_thread_agent_assignment_test.rb` | 29 | 0/29 | WS12/WS11. |
| `models/channel_thread_handoff_test.rb` | 11 | 0/11 | WS12/WS11. |

Additional suites: 11 message edit/reply/flag/dedup checks, 8 category/favorite model checks derived from controller scenarios (**zero HTTP controller tests claimed ported**), and 4 WS3 durable queue integration tests. These do not imply entire message/controller Ruby suites are ported.


### Coverage added in this continuation

| Ruby file | Ruby scenarios | New Rust tests / differential cases | Remaining proof |
|---|---:|---|---|
| models/message/reference_sync_test.rb | 10 | Six shared reference tests; 11 extraction and 8 HTML oracle cases cover core sync scenarios and extra validation/edit/delete checks. | Finalize/import integrations; no full per-scenario one-to-one mapping claimed. |
| jobs/message/quote_cards_refresh_job_test.rb | 5 | Shared capped/conversation/deletion tests plus real app missing-source worker test and queue atomicity test. | Fragment-cache refresh beyond cap and rendered-card acceptance WS8b. |
| models/rooms/direct_test.rb | 28 | Six new consolidated tests, existing WS2 key tests and seven display vectors; Rails reads back a renamed/widened/left group. | Complete last-member destroy, query-count instrumentation, huddle/rendering/agent-specific scenarios remain partial. |
| models/search_query_test.rb | 24 | Three tests compare 25 grammar cases, 21 SQLite queries and two cursor windows against Rails. | Three side-section domains WS12/WS14; broader timezone and performance instrumentation are unproven. |
| models/audit_log_test.rb | 17 | Eight tests compare 63 actions, four redaction inputs, 15 labels, 15 origins, three snapshots and 27 failure writes. | Actor/target lookup and deleted foreign targets, additional IP/no-IP and adversarial inputs are partial. |
| Runtime adapters | not a direct model-test file mapping | Three Markdown tests plus periodic registration, quote-worker execution and broadcast-payload tests; all six execute without the external parity seed. | Per-row loop fault injection, subscriber delivery and full scheduled/edit/forward app differentials remain partial. |

New core write validations/callbacks: direct name validation now runs through both generic Room create/update; MessageReference validates required associations, self and uniqueness; AuditLog validates action presence and has no parent touch/broadcast/enqueue. Audit labels for User/Room/Account are built from the records; other owners must provide a base-class Target snapshot and their Rails label (WS11 agents, WS14 workspace/integrations). Existing membership writes retain WS2 validation; huddle/grant/stream side effects on membership destruction remain WS11/WS13. Search is read-only; no model callbacks apply. Reference-sync rows, group notes and audit rows are included in Rails valid? export checks. The existing Rails-generated 64-case save-touch table passes; new exhaustive save/touch and app-path equivalence are not claimed.

## Final verification — rerun commands and raw summaries

Root cwd: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8`, except the differential command uses its `rust/` subdirectory. Every command below was rerun after the final code changes. Scratch is under `.scratch/`, Cargo uses -j 4 and the worktree target, and Docker names start ws8-. No published ports were used.

### DB, real queue and crash recovery

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -- --test-threads=4 > .scratch/tests-final.log 2>&1
```

```text
test result: ok. 355 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 11.72s
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
The three ignored DB tests are opt-in fixtures/scenario/export checks; all three execute in the differential command below. Four test threads avoid the highly concurrent default run, which was killed during an earlier attempt; that killed run is not claimed as a pass.

### App runtime checks

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire rich_text::tests::runtime_ > .scratch/runtime-markdown-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 178 filtered out; finished in 0.53s
```

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test -j 4 --manifest-path rust/Cargo.toml -p campfire jobs::tests::ws8_ > .scratch/runtime-jobs-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 178 filtered out; finished in 0.25s
```
All six selected app tests run their assertions without the external parity seed. Other app tests are filtered, not claimed run. The full app/workspace test suite and browser parity were not run.

### Workspace clippy

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.21s
```
Exit 0.

### Reference identity and Rails-generated vectors

The isolated ws8-reference-models image overlays this checkout's Rails db tree on the reference image pinned by digest `83d1ae45158680dc665489d1e1cb7c053ac3a860c0e2841d6f19b29f731329f5`. The shared image is unchanged. Identity includes 28 implementation files, the icon catalog and the full schema/migrations tree.

```sh
docker run --rm --cpus 2 --name ws8-reference-source-check --entrypoint "" -v "$PWD/app:/oracle:ro" -v "$PWD/db:/oracle-db:ro" -v "$PWD/config/icons.yml:/oracle-icons.yml:ro" ws8-reference-models sh -ec 'for file in models/channel_thread.rb models/thread_membership.rb models/thread_tag.rb models/message.rb services/messages/forwarder.rb models/poll.rb models/poll_option.rb models/poll_vote.rb models/message_pin.rb models/saved_item.rb models/saved_item/reminder_dispatcher.rb models/saved_item/reminder_pusher.rb models/scheduled_message.rb models/scheduled_message/dispatcher.rb models/keyword_alert.rb models/notifications/keyword_matcher.rb models/room_category.rb models/membership.rb models/room.rb models/message_reference.rb models/message/reference_sync.rb models/rooms/direct.rb models/search_query.rb models/audit_log.rb models/message/markdown.rb models/icons.rb jobs/message/quote_cards_refresh_job.rb services/periodic/runner.rb; do cmp "/rails/app/$file" "/oracle/$file"; done; cmp /rails/config/icons.yml /oracle-icons.yml; diff -r /rails/db /oracle-db; echo "WS8 reference: 28 implementation files, icon catalog and Rails db match worktree"' > .scratch/reference-check.log 2>&1
```

```text
WS8 reference: 28 implementation files, icon catalog and Rails db match worktree
```

```sh
docker run --rm --cpus 2 --name ws8-reference-vectors --entrypoint "" --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails runner /tools/ws8_reference_vectors.rb /out/ws8_reference_vectors.json' > .scratch/reference-vectors.log 2>&1
```

```text
WS8 reference vectors: 11 extraction, 8 HTML cases
```

```sh
docker run --rm --cpus 2 --name ws8-direct-vectors --entrypoint "" --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_direct_vectors.rb /out/ws8_direct_vectors.json' > .scratch/direct-vectors.log 2>&1
```

```text
WS8 direct vectors: 7 display cases
```

```sh
docker run --rm --cpus 2 --name ws8-search-vectors --entrypoint "" --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_search_vectors.rb /out/ws8_search_vectors.json' > .scratch/search-vectors.log 2>&1
```

```text
WS8 search vectors: 25 grammar, 21 SQLite filters, 2 cursor windows
```

```sh
docker run --rm --cpus 2 --name ws8-audit-vectors --entrypoint "" --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/db/src/tests:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_audit_vectors.rb /out/ws8_audit_vectors.json' > .scratch/audit-vectors.log 2>&1
```

```text
WS8 audit vectors: 63 actions, 4 redactions, 15 labels, 15 origins, 3 snapshots, 27 throttled writes
```

```sh
docker run --rm --cpus 2 --name ws8-runtime-vectors --entrypoint "" --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e 'CAMPFIRE_FIXTURES_NOW=2026-03-10 12:00:00' -v "$PWD/test:/rails/test:ro" -v "$PWD/rust/crates/db/ruby:/tools:ro" -v "$PWD/rust/crates/campfire/src:/out" ws8-reference-models sh -ec 'bin/rails db:prepare >/dev/null; bin/rails runner /tools/ws8_runtime_vectors.rb /out/ws8_runtime_vectors.json' > .scratch/runtime-vectors.log 2>&1
```

```text
WS8 runtime vectors: 8 Markdown, 4 canonicalization, 3 periodic tasks, 2 template-free broadcasts
```
The committed JSON expectations are regenerated by these runners, not handwritten. Pure helper vectors are supplemented by Rails SQL scopes, fixture comparisons and rollback row validation; they are not exhaustive proof for every domain input.

### Differential fixtures/scenario/export and timestamps

Cwd: worktree rust/.

```sh
OUT="$PWD/../.scratch/differential" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models CARGO_TARGET_DIR="$PWD/target" TMPDIR="$PWD/../.scratch" mise exec rust@1.98.1 -- bash reference-tools/db/differential.sh > ../.scratch/differential-final.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 355 filtered out; finished in 10.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
message_save_touches.json matches the reference
schema.sql, reference db:prepare and Rust prepare agree
validated 120 rows Rust wrote: {"accounts" => 1, "action_text_rich_texts" => 23, "activity_items" => 2, "audit_logs" => 1, "boosts" => 1, "channel_threads" => 2, "drive_attachments" => 3, "keyword_alerts" => 1, "memberships" => 25, "message_pins" => 1, "message_references" => 3, "messages" => 25, "poll_options" => 4, "poll_votes" => 3, "polls" => 2, "room_categories" => 1, "rooms" => 8, "saved_items" => 2, "scheduled_messages" => 3, "searches" => 1, "sessions" => 2, "thread_memberships" => 2, "thread_tags" => 2, "users" => 2}
rollback ok
```
The export uses a recording sink, so durable job persistence is proven separately by the real queue suite.

### Migration replay and comparator checks

```sh
TMPDIR="$PWD/.scratch" CONTAINER_PREFIX=ws8 PARITY_IMAGE=ws8-reference-models bash rust/reference-tools/db/check-migration-replay.sh > .scratch/migration-final.log 2>&1
```

```text
self-test: all 6 mutations caught
migration replay matches schema.sql: 1279 facts, 87 tables with an id, 248 indexes, 128 versions
```

### Failing-first evidence and reproducible regressions

This continuation showed all six reference, six group, three search and eight audit tests failing against compiled scaffolds before their implementation passed. The three Markdown adapter tests failed while the real app still inherited BasicRichText defaults. Periodic registration and the quote worker failed while unregistered; the template-free broadcast test failed against its None scaffold. The added real-queue quote test failed with the refresh enqueue deliberately omitted, then passed after restoration. These are actual assertion failures, not compilation errors. Initial scaffolding/generator compilation errors were corrected and do not count.

The following rerun reproduces those regressions plus the earlier twelve fault checks. It requires every new named test to fail, rejects compile-only failures, and restores each source in finally. This repeat is mutation evidence; it does not rewrite the earlier implementation chronology. The older recovered tests not named in the harness remain individually unproven.

```sh
python3 rust/reference-tools/db/ws8-discrimination.py > .scratch/discrimination-final.log 2>&1
```

```text
keyword-rules: detected (14 failing tests)
test result: FAILED. 0 passed; 14 failed; 0 ignored; 0 measured; 344 filtered out; finished in 0.22s
scheduled-claim: detected (4 failing tests)
test result: FAILED. 21 passed; 4 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.82s
scheduled-validation: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.68s
scheduled-access: detected (5 failing tests)
test result: FAILED. 20 passed; 5 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.69s
scheduled-thread-drop: detected (2 failing tests)
test result: FAILED. 23 passed; 2 failed; 0 ignored; 0 measured; 333 filtered out; finished in 0.79s
category-validation: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 350 filtered out; finished in 0.23s
category-nullify: detected (1 failing tests)
test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 350 filtered out; finished in 0.25s
favorite-writes: detected (2 failing tests)
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 350 filtered out; finished in 0.29s
transaction-rollback: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.19s
save-touch-golden: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.30s
thread-job-atomicity: detected (2 failing tests)
test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.22s
reminder-job-atomicity: detected (1 failing tests)
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.20s
quote-references: detected (6 failing tests)
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 352 filtered out; finished in 0.41s
group-dms: detected (6 failing tests)
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 352 filtered out; finished in 0.26s
search-grammar: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 355 filtered out; finished in 0.15s
audit-contract: detected (8 failing tests)
test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 350 filtered out; finished in 0.18s
runtime-markdown: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 178 filtered out; finished in 0.27s
runtime-jobs-broadcasts: detected (3 failing tests)
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 178 filtered out; finished in 0.13s
quote-job-atomicity: detected (1 failing tests)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.13s
WS8 discrimination: 19 mutations detected; sources restored
```

## Cross-workstream touches

- WS2: Message lifecycle/search, generic Room direct-name validation/deleted marker, Membership sidebar removal/organization, RichText seams, exports and callback tests. Minimal edits are described above; no schema redesign.
- WS3: four real queue tests, app quote job wrapper/registration and three periodic tasks; no queue/store redesign. Every emitted job in these slices persists before commit through EventSink.
- WS5: app renderer/canonicalization/plain-text adapter and reference icon catalog wiring. The richtext crate itself is unchanged. Fallible error propagation remains the named WS8a/WS2/WS5 follow-up.
- WS7/WS8b: additional typed partial data and template-free sink; no controllers or templates edited. Partial renderers, event-shape coordination and subscriber acceptance remain outstanding.
- WS11/WS13: grant/huddle/stream cleanup and agent finalize/user-destroy callbacks are still deferred; the minimal last-member deleted marker is not a substitute.
- WS12: minimal ActivityItem reminder/drop writer retained; full inbox policy and board/work behavior/search sections remain deferred.
- WS14/WS15/WS16/WS17: integration reference sync, event/foreign audit label adapters, import suppression, notification policy/pushers and slash-owned user/event domains remain with their owners as listed.
- Shared Cargo dependency/lock and a small app build script are necessary for the reference-config icon catalog. The merged Rails background_jobs migration is inherited from main. No Rails source changes, PR, deployment or production data edits were made.
- The external report is the explicitly authorized exception to rust-only writes; this tracked copy matches it.

## Precise remaining work / restart point

1. **Room deletion (WS8a):** replace the partial marker with full validated begin_destroy; revoke agent/huddle grants, end streams and persist cleanup/destroy jobs in the triggering transaction. Implement DestroyJob batches of 500 with scheduled items destroyed first, all message/thread/event and cross-domain dependencies, claim timestamps, retry/resume/idempotence and stale reenqueue claims. Audit every Rust room reader for deleted_at; retain an explicit unscoped lookup only where cleanup needs it. Add Rails-generated cascade/recovery cases and real queue rejection tests. The last group member currently only marks deleted/removes memberships and does not enqueue destruction.
2. **Non-agent slash commands (WS8a):** registry, permissions, dispatcher and timezone-aware leading/trailing time parser; shrug/me/remind/status/dnd/ooo/poll handlers with every domain validation/callback, plus explicit huddle/event/play stubs for WS13/WS14. No slash implementation is delivered in this continuation.
3. **Retention (WS8a):** all branches of PruneJob, batching and cutoff boundaries, expired 2FA rows, read-only old activity, revoked huddle grant dependency/unlink behavior and stuck-room backstop; register the daily task and worker with atomic enqueue. AuditLog is ported, but retention is not.
4. **Runtime completion:** register reminder/thread push handlers with WS17's real policy/Web Push pool. These classes can currently be durably persisted but have no app handlers and fail in the runner. Add app-loop row-failure/rescue checks; full scheduled/edit/forward rendering differentials; implement the real storage copier and filesystem rollback. Make canonicalization/plain-text errors fail model writes instead of the current infallible trait fallback (WS8a/WS2/WS5).
5. **Remaining named-owner integration:** rendered QuoteCards, group headers/sidebar, pins, polls, threads and message event handling and HTTP routes/authorization are WS8b with WS7; huddle participant locals WS13; board/work/events search sections WS12/WS14; audit Current/context and foreign target lookup/labels WS4/WS11/WS14/WS17/WS8b; streaming finalize reference sync WS11; import suppression WS16; other integration reference sync WS14/WS15.
6. **Unproven parity:** earlier broadcast-failure, hard-user-destroy and legacy-conversion scenarios; full ActivityItem accessible scope; broader Unicode/coercion, IP/no-IP, timezone/DST and adversarial inputs; exact per-Ruby-scenario test mapping and remaining individual discrimination proof. The rows exported are valid to Rails, but this is representative, not exhaustive end-to-end acceptance.

There are no external blockers to the committed slices. The remaining domains are substantial and need their own coherent, failing-first slices. This report explicitly marks the workstream partial; no cutover or WS8b acceptance is implied.
