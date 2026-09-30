# WS17 Wave 4 — partial, 332 of 347 selected scenarios delivered

Reference Rails `d7c7de92`, with the shared brief's approved board tag from `a6f10a25`; status-popup target `2e20b24c` remains WS8b-r2-owned. Merged main `2e20b24c` at `093ca4ea`, retaining auth/board merge `68cd54bf` and the earlier `21a7332f` merge. Branch `rust/ws17-push-presence`. The full Rust workspace suite, strict clippy and all 11 Chromium scenarios ran in the fresh clone at `8dee44c8e8b57e61f62da51cd0b67663a15d1b2c`. The final report-only commit changes no runtime or test code. The delivery reply gives the final pushed SHA. The tracked report exactly mirrors the requested external report.

This continuation closes **43 of the former 58 deferred exact titles**, leaving **332/347 passed equivalent and 15 source-owner cases deferred**. Pushed slices: `ee85dc13` (DM/profile), `809333f1` (10 browser cases), `7527a335` (readers/matcher/DNS/proxy), `98b2b3aa` (WS13 wire bridge), `79fae2b1` (push gating/forwarding), `68cd54bf` (main auth integration), `2b64dade` (reproducible source fixtures and exact remaining ownership), `87452458` (worker test-queue isolation), `41dbe4bd` (configured/disconnected Calendar browser link), `bb4571a1` (bounded exact icon-style assertions), `e4091d9c` (unsaved profile/error-layout metadata), `4f8297ad` (wrong explicit-zone ordering injection), `5efbc46f` (exact captured metadata assertions), `f21f1108` (approved #162 board tag and actual constructor payloads), `093ca4ea` (main reference merge), `edbb3042` (post-change global layout/assets and complete-page reference integration), and `8dee44c8` (post-change compiled/static asset reference exports).

## Delivered

Earlier pushed slices (`7b9267a3`, `32b49aa4`, `76c34826`, `0adcae95`, `12c448d7`) provide typed notification policy, tagged Web Push with Smartfire subject and IP pinning, durable room/thread/saved/test push transport, presence leases/HTTP/pruner, validated dirty-tracked settings writes, status and notification PATCH, DND allowances, keyword-list replacement, cache reconciliation, manual OOO claims, and complete badge/OOO broadcast HTML. PWA worker/offline bytes are pinned. Existing Rails oracle vectors remain exercised by the full suites below.

Profile/subscription/allowance slice pushed at `ba916992`; keyword recording pushed at `fd411985`; calendar dispatch pushed at `9dff8776`; notification push adapters pushed at `7eaf269f`; named policy replay pushed at `e90bb4c5`; named OOO/meeting replay pushed at `1021be6a`; continuation slices:

| Files | Change and verification |
| --- | --- |
| `campfire/controllers/users/profiles.rs`, `profiles/ws17_tests.rs` | Persist theme, text size and time zone through the existing validated settings writer, mark an explicitly submitted zone (including blank) as chosen, atomically roll back other submitted fields on invalid appearance. Preserve unsaved form values. Profile errors populate the profile path and do not inherit the settings-controller missing-device failure. Eleven seeded HTTP tests cover writes, invalid rollback, IANA/legacy selection, actual layout sound markers, the two WS9 authentication/appearance interactions, and exact unsaved error-layout metadata. Six raw metadata snapshots and the invalid-notification sound string come from the actual pinned layout/helper. The request timezone remains the saved one while metadata reads the unsaved user; early password rejection leaves time_zone_explicit unchanged. |
| `views/users/settings.rs`, `profile_zones.json`, `templates/users/profiles/_appearance.html`, profile show; `presenters/status_settings.rs` | Mechanical appearance form port, error wrappers and escaping. Six complete form strings and six raw root/color-scheme/time-zone/sound metadata snapshots match actual Rails. All 135 friendly zone choices retain TZInfo identifiers and current base-offset labels using pinned TZInfo transitions; tests caught Casablanca's negative-DST distinction. No view queries. |
| `db/models/user_status_settings.rs`, `presenters/view_context.rs` | Preload actual DND, quiet-hour, meeting and OOO windows into layout preferences. Current and future windows stay in source order; opt-outs remove only their owned marker. |
| `controllers/users/push_subscriptions.rs`, `push_subscriptions/ws17_tests.rs`; `app.rs`, test support | All six named subscription HTTP scenarios, including legacy revalidation and private-IP refusal. Per-app DNS dependency permits deterministic DNS answers while testing the real endpoint guard/model/HTTP stack. Production uses system DNS. Current-user deletion scope checked additionally. Real user-agent rendering matches four Rails cases. |
| `views/templates/users/push_subscriptions/index.html`, `tests/ws17_settings.rs` | Complete subscription content byte comparison, including full row forms, fixed shared test CSRF values, actual asset URLs, escaping and whitespace. |
| `controllers/users/dnd_allowances.rs` | Repeated star remains one row; deterministic real UNIQUE-index failure at insert follows Rails' success redirect, in addition to existing concurrent HTTP requests. No mocks of the writer. |
| `db/models/activity_item/message_recorder.rs`, message callback, `tests/message_activity_test.rs`, JSON oracle | Message-only recorder: flat scoped membership/keyword queries, policy winner, active-human/self exclusion, idempotent source rows, grouped followed-thread updates, unchanged read/handled state on repeated non-grouped recording. Thirty-one actual Rails callback/candidate vectors; all eleven keyword recorder titles and thirteen message-only recorder titles. Real SQLite trace checks stay flat at five versus thirty members. |
| `campfire/controllers/messages/ws17_activity_tests.rs`, DB Cargo dev dependency | Full HTTP message callback records during DND; real rendered mentions win over keywords. Rejecting the activity INSERT rolls back message, FTS index and durable jobs. SQLite tracing is a test-only rusqlite feature. |
| `db/models/calendar_dispatch.rs`, meeting cache claim methods, status broadcast batching; `campfire/jobs/periodic.rs` | Register meeting/OOO sweeps every minute. Match active scopes (including bots), inclusive 15-minute stale threshold, missing-cache handling, steady-state no-UPDATE/no-writer path and OOO-only refresh gating. Per-member refresh enqueue and claim share a writer transaction; an enqueue failure rolls back only that member and the sweep continues. Preserve expired-already-false manual columns as the pin does. All badges precede all notices, with one lease query. |
| `db/tests/calendar_dispatch_test.rs`, `statuses/calendar_tests.rs`, calendar oracle | Twenty-five actual Rails two-tick vectors; all 11 meeting and 12 OOO named scenarios pass. Compare stored claims/manual columns, durable refresh rows, exact signed-stream HTML and actual SQLite UPDATE counts. Two independent database handles prove one winning boundary/one broadcast; corrupt cached timestamps isolate the bad member. Twelve of 13 cache titles pass; validated duplicate creation belongs to WS14 and remains deferred. |
| `app.rs`, seeded HTTP test support | Inject periodic host intervals explicitly. Production reads the same environment intervals; seeded HTTP tests run no background periodic host, matching Rails' reference server. Durable queue and broadcasts remain real. This removes the observed race with an unrequested first periodic tick. Registration itself is tested and the sweeps are invoked explicitly. |
| `db/models/notification_push.rs`, subscription batches; `campfire/jobs/notifications.rs` | Live event/board source readers until WS14/WS12 land. Durable event/board jobs re-read memberships and current reminder policy with no sender exception. Match event rounding/staleness, venue/direct title/tag, board status/escalation/path and membership recheck. WS13 huddle adapter keeps supplied payload bytes, SQL disconnected/visible scope, actual caller allowances, huddle inbox switch, strict older-than-ten-minutes throttle, and atomic throttle/delivery-job enqueue. Five registered handlers, including the final WS13 wire bridge. |
| `db/tests/notification_push_test.rs`, notification push oracle; `web_push/ws17_delivery_tests.rs` | Fifty actual Rails source/policy states including all 10 event, 4 board and 13 join-pusher titles. Claims run on real SQLite and across two database handles; repeat/eleven-minute replay. Registered handlers run against private seeded app DB and local TLS, comparing complete decrypted JSON with actual Rails source strings. The board tag is captured from the approved a6f10a25 pusher; the oracle passes its raw payload directly to the actual constructor, without synthetic nullable tags. Rejecting a delivery INSERT rolls back both throttle and triggering source write. |
| `db/tests/named_policy_test.rs`, `notification_policy.rs`; named policy oracle, pinned declarations and regeneration | Largest remaining file first: 52 individually named Rust tests. Run the unchanged pinned Ruby test declarations/setup/private helpers/assertions under an isolated fixture/clock host, not rewritten case tables: all 52 Ruby cases and 133 original assertions pass. Capture 82 constructor observations, then replay actual persisted recipient/cache/allowance state and decisions in Rust. A real SQLite trace asserts zero policy queries and one batched allowance query (zero without sender). Dynamic kind parsing rejects unknown strings with the pin's exact error message. |
| `db/tests/named_calendar_status_test.rs`, `user_status_settings.rs`; pinned OOO/meeting declarations and oracle | Next largest deferred files: all 22 OOO and 14 meeting-status titles, each an individual named Rust test. The unchanged Ruby bodies run 98 original assertions; replay all 199 operations in order on one real DB/clock per case, checking both loaded and persisted attributes before/after every call. Cache creation is source setup SQL at the WS14 seam. Match failed-save retention, expired settings, racing/repeated claims, presets, zones, quiet/label precedence and deactivation. Add the separate pure visibility/quiet/date readers and a loaded-settings deactivation wrapper that clears the owned OOO attributes with the real User write. |
| `views/users/statuses.rs`, uncached OOO wrapper, profile status/allowance partials, room/user presenters and controllers | Mount the complete pinned OOO wrapper for every other active human DM member, including blank/off members so future OOO can update live. Exact name ordering, viewer scoping, escaped note, calendar invisibility and signed streams. Add the live profile badge and viewer-scoped DND controls. Nine complete wrapper strings, three profile status sections and both full allowance forms are byte-identical to actual pinned Rails. All 10 named DM integration cases pass over seeded HTTP/real sockets, including an actual two-hour shared clock advance. Three injections fail assertions. |
| `reference-tools/ws17_browser.py`, `ws17_browser.mjs` | Eleven individually named Chromium scenarios: 7 status notifications, 1 configured/disconnected Calendar connect-link case, 1 OOO and 2 service worker. Real forms, actual persisted readback, live Audio replacement exactly as the original test, computed CSS under opposite OS theme, 390px viewport, two Rails-issued user cookies and actual browser CacheStorage. Isolated private seed copy and native server on owned 52471; no source/controller stubs or output masks. Wait for observable CSS completion after Turbo/media changes while keeping exact style assertions. The Calendar case reproduces the original href/absent-checkbox assertions with its original configured/disconnected helper inputs. Google fetch/browser execution stays WS14-owned. |
| `db/tests/status_settings_write_test.rs`, `keyword_alert_test.rs`, pinned reader declarations and regeneration | Close the four remaining exact StatusSettings reader sequences using real saves, loaded reloads and advancing clocks; complete the literal, blank-only and Unicode matcher cases already landed on this branch. The 4 reader and all 10 matcher original Ruby bodies run unchanged and pass 35 original assertions. Selected inventory now has StatusSettings 14/14 and KeywordMatcher 10/10. |
| `campfire/integrations/web_push/tests.rs` | Close five deferred subscription sequences against the real Guard/model composition and delivery transport: loopback, link-local, empty DNS, deferred construction lookup and fresh private DNS refusal with no dial. The sixth case sets all four proxy keys in an isolated child process and proves actual TLS delivery still dials the pinned public address and decrypts the expected JSON. No unsafe global test environment mutation. |
| `db/models/notification_push.rs`, `jobs/notifications.rs`, `web_push/ws17_delivery_tests.rs` | Final WS13 wire DTO and registered Notifications::HuddlePushJob bridge; actual owner JSON and single claim/durable delivery transaction. Standalone source vectors restore every row behind the captured unread badge. Existing event/board and direct huddle APIs remain final and unchanged. |
| `db/tests/named_push_gating_test.rs`, room pool handoff, pinned gating declarations | Eight exact message/thread/reminder push-gating sequences plus the forwarded-note case use real persisted messages, followed/unfollowed reply sequences, activity recording, reminder membership scopes and the actual forwarder. The original nine Ruby bodies pass 34 original assertions using unchanged mention/DNS helpers and nonjoinable isolated transactions so model commit callbacks execute before rollback. Room delivery now hands the distinct union to the pool once, in subscription ID order, matching the pin. Full encrypted transport regressions pass. |
| `views/templates/layouts/application.html`, `vectors/auth_full_pages.json`, six core application-layout goldens; `reference-tools/ws17_regenerate_auth_reference.py`, verifier | Add the exact approved #163 user-status:changed listener and regenerate actual source layout/assets through the unchanged WS9/WS6 exporters. Strict source checks cover 60 pinned files, the approved board blob and 10 approved status/layout blobs. All 15 complete auth pages and all 28 core view tests pass. Only foundation/layout/public/preload captures are copied; new popup page composition remains WS8b-r2-owned. |
| `assets/tests/reference/compiled_sha256.json`, `manifest.json`, `javascript_importmap_tags.html`, `stylesheet_link_tag_all.html`, `static_responses.json` | Original Propshaft/Rack exports match the approved two changed asset sources. All 8 reference tests pass, retaining complete compiled-byte, manifest, helper-tag and static-response checks. The gem vendor tree and asset implementation are unchanged. |
| `reference-tools/ws17_profile_ui.rb`, regeneration script, verifier and injection runner | Actual pinned source/output verification; no Rails changes, output masks, allowlist changes or new ignores. |

WS9 auth is now integrated: the same writer saves appearance, calls `authentication::update_profile`, records security audits/email rollback markers, revokes remembered devices when required, and assigns staged attachments. Rejected current-password checks render submitted public/appearance values without saving; a rejected security audit rolls back the earlier appearance save. Two new integration regressions cover those interactions. Live security/profile-session panels mount together with the settings forms. GitHub/inbox/voice/agent/Google sections still belong to WS15/WS12/WS13/WS11/WS14; whole-profile parity remains partial.

The pin's invalid status/notification render on a seeded confirmed-2FA user still produces HTTP 500 because Rails omits `@two_factor_devices`. Separate tests preserve that observed behavior; the named fixture replays explicitly use unconfirmed-credential fixture state and expect 422. Profile PATCH initializes those devices in Rails and its owned validation response is 422. This is baseline-pin status-controller evidence; #163 popup authorization/error rendering is explicitly pending WS8b-r2 integration and is not established by that old status golden. This is baseline-pin status-controller evidence; #163 popup authorization/error rendering is explicitly pending WS8b-r2 integration and is not established by that old status golden. This is baseline-pin status-controller evidence; #163 popup authorization/error rendering is explicitly pending WS8b-r2 integration and is not established by that old status golden. This is baseline-pin status-controller evidence; #163 popup authorization/error rendering is explicitly pending WS8b-r2 integration and is not established by that old status golden.

`Calendar::MeetingRefreshJob { user_id: i64 }` is durable on the default queue, version 1, JSON `{ "user_id": ... }`. WS14 must register its fetch handler; until then it fails visibly as an unknown handler. Status PATCH enqueues both opt-ins separately; a due tick refreshes both-opt-in members through the meeting sweep only. An overdue cache refreshes again on a later tick until WS14 updates it, matching Rails rather than adding queue deduplication. No Google refresh success is faked. Typed status badge/OOO facts are emitted after commit and rendered in the cable sink. Rails emits events in order, but its worker pool delivers independent stream callbacks asynchronously; socket tests compare exact bytes/counts/order within each signed stream, while a separate domain test checks actual cross-stream event emission order.

## WS13 shared transport/policy seam — exact signatures

```rust
// campfire_db::models::push_subscription::PushPayload
pub fn new(title: String, body: String, path: String, tag: Option<String>) -> Self;
// campfire::integrations::web_push::Pool
pub fn queue(&self, conn: &rusqlite::Connection, payload: &PushPayload,
             subscriptions: Vec<campfire_db::PushSubscription>) -> campfire_db::Result<()>;
pub fn deliver_later(&self, notification: Notification);
// campfire_db::UserStatusSettings
pub fn for_ids(conn: &rusqlite::Connection, ids: &[i64])
    -> campfire_db::Result<std::collections::HashMap<i64, Self>>;
// campfire_db::models::notification_policy
pub fn dnd_exceptions_for(conn: &rusqlite::Connection, ids: &[i64], sender: Option<i64>)
    -> campfire_db::Result<std::collections::HashSet<i64>>;
// campfire_db::NotificationPolicy
pub fn push(&self) -> bool;
```

WS13 builds invitation/join `PushPayload` and candidate room-membership facts. WS17's shared transport is available now. Before `Pool::queue`, preload settings once and evaluate `NotificationPolicy` with current time and the actual sender's DND allowances. Invitation uses `NotificationKind::Huddle`; join uses **`NotificationKind::HuddleJoin` with the recipient's actual `room_involvement`** (missing membership is `None`, a present SQL-null involvement is `Some(None)`). Other unused policy inputs are false/None. Join with missing, invisible, nothing or muted membership is suppressed. `Pool::queue` itself applies no policy or recipient-scope filtering.

Keep Rails' pusher scopes before delivery: visible/disconnected memberships, invitations exclude `nothing`, joins exclude `nothing` and `muted`; SQL-null exclusions follow Rails SQL rather than adding eligibility. Join also checks the huddle inbox preference and claims its ten-minute throttle only after policy and subscriptions permit an actual push. The dedicated durable huddle gate/delivery adapter is now available below; the shared transport signatures remain unchanged. WS13 still connects its source jobs/lifecycle and payload construction to this adapter. WS13 owns payload/source construction, WS17 the gate/transport. `PushPayload::new` preserves supplied strings/tag without automatic truncation. Pool reads fresh badges and delivers through current VAPID and stored pinned endpoint IP. Preserve transactional claim/enqueue when connecting the source.

## WS12/WS14 source jobs and WS13 durable adapter

```rust
// campfire_db::models::notification_push; emit on the triggering source Tx
EventReminderJob { event_id: i64 } // CLASS Event::ReminderPushJob; default queue/version 1
BoardNudgeJob { nudge_id: i64 }    // CLASS BoardAutomations::NudgePushJob; default/version 1
pub fn event_reminder_push(conn: &Connection, id: i64, now: Timestamp) -> Result<Option<PushDelivery>>;
pub fn board_nudge_push(conn: &Connection, id: i64, now: Timestamp) -> Result<Option<PushDelivery>>;
pub fn enqueue_huddle_invitation(tx: &mut Tx, room_id: i64, recipient_id: i64,
                                  sender_id: i64, payload: PushPayload) -> Result<bool>;
pub fn enqueue_huddle_join(tx: &mut Tx, room_id: i64, recipient_id: i64,
                            sender_id: i64, payload: PushPayload) -> Result<bool>;
```

WS12/14 own reminder/nudge claims and source writes; emit the ID job with `tx.emit_after_commit(Event::job(&args))` before committing that write. Minimal live readers touch existing schema only. Missing source records discard through the existing queue error mapper. Event push intentionally ignores inbox switch and membership notification involvement, and permits the first five minutes after start; board push rechecks active-human membership even if hidden/connected. Reminder DND has no sender allowance.

WS13 validates its activity item/grant, invitation/join lifecycle eligibility and sender/recipient IDs and builds `PushPayload` at the point Rails invokes its pusher, then calls this adapter on the source writer transaction. Payload bytes are captured there, preserving source ownership. `Huddle::InvitationDeliveryJob` and `Huddle::JoinDeliveryJob` are Rust durable adapters, default queue/version 1, JSON `{payload,subscription_ids}`. Join stamps the membership only after policy and actual subscriptions allow delivery; exactly ten minutes remains throttled. On enqueue failure, the entire source write and throttle roll back. Invitation retains the pin's empty scoped queue handoff. The handlers preserve that already-made decision/payload, re-read surviving subscription IDs, and use existing `Pool::queue` for current unread badge, VAPID/IP guard/encryption. Deleted subscriptions are skipped. Source callbacks and `Huddle::PushInvitationJob`/`JoinNoticeJob` integration remain WS13 seams; no grant or notifier lifecycle is faked.

## Final WS13 wire handoff

WS13's `huddle_notices::PushRequest` emits **`Notifications::HuddlePushJob`**, default queue/version 1. WS17 now registers that exact class. Its final wire shape is:

```rust
// campfire_db::models::notification_push (WS17 mirror of WS13 serialized shape)
pub enum HuddlePushKind { Huddle, HuddleJoin } // serde: "huddle", "huddle_join"
pub struct HuddlePushRequest {
    pub kind: HuddlePushKind,
    pub recipient_id: i64, pub sender_id: i64, pub room_id: i64,
    pub room_membership_id: Option<i64>,
    pub payload: PushPayload, // title/body/path; source tag string deserializes as Some(tag)
}
pub fn enqueue_huddle_request(tx: &mut Tx<'_>, request: HuddlePushRequest) -> Result<bool>;
```

WS13 continues to emit its own `PushRequest` with `enqueue_huddle_push(tx, &request)`; serialization is compatible without importing unmerged owner models. The registered WS17 handler evaluates current policy and calls the existing invitation/join adapter in one writer transaction, then the existing delivery handler uses the shared pool. That writer owns the one join throttle claim and delivery INSERT. **Do not run WS13 `prepare_push` before enqueueing a normal intent or again in the WS17 handler**: it would double-claim. Join requests require their source membership ID to still match the room/recipient row; deleted or replaced memberships skip. WS13 retains grant/activity item/lifecycle/payload ownership. Its normal source paths on the inspected branch emit intents without making a throttle claim; its isolated helper tests may still test `prepare_push` separately. Actual wire JSON, registered worker execution, complete decrypted Rails JSON and a repeated join prove the handoff. The new standalone replay exposed that the previous oracle omitted the board membership behind an unread badge; captured setup now includes those original board rows in every case, so each vector runs independently. No expected payload/badge masks were added.

## WS14 cache and dispatch seams

```rust
// campfire_db::models::calendar_dispatch
pub async fn dispatch_meetings(db: &Database, now: Timestamp) -> Result<DispatchStats>;
pub async fn dispatch_ooo(db: &Database, now: Timestamp) -> Result<DispatchStats>;
// campfire_db::MeetingCache; claims deliberately do not reload the supplied cache
pub fn claim_broadcast(&self, tx: &mut Tx, active: bool) -> Result<bool>;
pub fn claim_refresh_followup(&self, tx: &mut Tx, now: Timestamp,
                              window: jiff::SignedDuration) -> Result<bool>;
// campfire_db::UserStatusSettings; pure typed facts, emitted after commit
pub fn announce_badges_for(tx: &mut Tx, users: &[Self]) -> Result<()>;
pub fn announce_ooo_notice(&self, tx: &mut Tx);
```

WS14 supplies/validates cache creation and fetch/update execution. A follow-up claim winner must enqueue on the same supplied writer transaction. Completed fetches clear `refresh_pending_at`. Due sweeps commit each member's claim/job first and then broadcast the collected winning snapshots, matching Rails' claims-before-broadcast batch order. No Google call or successful refresh handler is stubbed.

## Current verification

Fresh clone: `.scratch/fresh-20260930-ws17`, cloned from GitHub, with its own `rust/target`. Its default and first-run seeds were independently built from `triage-reference-d7c7de92:latest`; no database or target directory was copied from the working checkout. The full workspace suite (1,751 passed, 0 failed, 11 existing ignores), strict clippy and Chromium runs below execute in that clone. Reference regenerations and restored-source defect injections execute in the assigned worktree, on its own target. Cargo uses Rust 1.98.1 and four jobs.

No new ignore, output mask, or allowlist entry was added. DB retains main's three external differential/export ignores and WS9's Rails rollback-readback ignore. The app retains main's cable recording/latency ignores and WS11's `manages_bots` ignore. Other pre-existing workspace ignores are visible in the full log. All seeded app tests ran, with zero failures. The workspace command follows rust/AGENTS.md in excluding the unchanged vendored html5ever test package; clippy checks all workspace targets.

Both the merge and final fresh clone ran `mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null` from `rust/`: exit 0, no output. Strict workspace TOML parsing was rerun with this exact command; duplicate keys fail before the summary can print:

```bash
python3 - <<'PY'
import tomllib
from pathlib import Path
v = tomllib.loads(Path('rust/Cargo.toml').read_text())
print(f"workspace dependency keys: {len(v['workspace']['dependencies'])} unique; 0 duplicates (strict TOML parse)")
PY
```

```text
workspace dependency keys: 75 unique; 0 duplicates (strict TOML parse)
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_profile_ui.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails profile UI: 6 complete appearance forms; 6 raw metadata snapshots; 1 complete subscription content; 135 zone choices
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_message_activity.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails message activity: 31 complete callback/candidate cases
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_calendar_dispatch.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails calendar dispatch: 25 complete two-tick cases
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_notification_push.py`

```text
Rails source verified: 52 files match d7c7de92; 1 board pusher matches a6f10a25
Rails notification push: 50 complete source/policy payload cases; board tag reference a6f10a25
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_named_policy.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned policy test declarations verified: byte-identical to d7c7de92
Rails named policy: 52 passed cases; 133 original Rails assertions; 82 constructor observations
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_named_calendar_status.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned calendar status declarations verified: 2 files byte-identical to d7c7de92
Rails named out_of_office: 22 passed cases; 71 original Rails assertions; 129 operations
Rails named meeting_status: 14 passed cases; 27 original Rails assertions; 70 operations
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_dm_profile.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails DM/profile: 9 complete DM wrappers; 3 profile badges; 2 allowance controls
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_named_readers.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned reader/matcher declarations verified: 2 files byte-identical to d7c7de92
Rails named readers: test/models/notifications/keyword_matcher_test.rb: 10 passed cases; 18 original Rails assertions
Rails named readers: test/models/user/status_settings_test.rb: 4 passed cases; 17 original Rails assertions
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_named_gating.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned gating declarations verified: 4 files byte-identical to d7c7de92
Rails named gating: test/models/notifications/push_gating_test.rb: 8 passed cases; 28 original Rails assertions
Rails named gating: test/models/room/push_test.rb: 1 passed cases; 6 original Rails assertions
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py profile`

```text
profile-auth-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 1.47s
profile-layout-snapshot: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 2.36s
profile-timezone-auth: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 4.75s
profile-save-theme: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 1.02s
profile-private-endpoint: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.68s
profile-unique-loser: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.62s
profile-sound-metadata: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 1.15s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py keyword`

```text
keyword-priority: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 2.02s
keyword-read-state: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 1.07s
keyword-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 2.17s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py calendar`

```text
calendar-steady-write: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.37s
calendar-racing-claim: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.29s
calendar-duplicate-refresh: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 0.67s
calendar-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 0.93s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py push-`

```text
push-board-tag: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 3.18s
push-reminder-dnd: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 2.21s
push-event-stale: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 2.49s
push-huddle-boundary: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 3.80s
push-huddle-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.12s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py named-policy`

```text
named-policy-quiet: detected
test result: FAILED. 35 passed; 17 failed; 0 ignored; 0 measured; 556 filtered out; finished in 6.83s
named-policy-bot-inbox: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.24s
named-policy-query-ceiling: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.31s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py named-status`

```text
named-status-deactivate: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.32s
named-status-expired-note: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 0.27s
named-status-meeting-dnd: detected
test result: FAILED. 33 passed; 3 failed; 0 ignored; 0 measured; 572 filtered out; finished in 3.99s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py dm-`

```text
dm-viewer-scope: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 0.48s
dm-streams-for-future-ooo: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 0.27s
dm-profile-live-presence: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 523 filtered out; finished in 1.24s
```

fresh clone: `CI=1 CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52450-52499 MAIL_TEST_PORT_RANGE=52450-52499 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4`

```text
test result: ok. 523 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 85.63s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.37s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 604 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 56.17s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.88s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.87s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.14s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.31s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.80s
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

fresh clone: `mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.80s
```

fresh clone: `python3 rust/reference-tools/ws17_browser.py`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.29s
test/system/status_notifications_test.rb: 7 passed; 0 failed
test/system/meeting_status_test.rb: 1 passed; 0 failed
test/system/out_of_office_test.rb: 1 passed; 0 failed
test/system/service_worker_test.rb: 2 passed; 0 failed
WS17 Chromium: 11 passed; 0 failed
```

assigned worktree: `CI=1 CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire profiles::ws17_tests -- --test-threads=4`

```text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 515 filtered out; finished in 3.45s
```

assigned worktree before implementing the approved tag: `mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_db ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases -- --test-threads=4`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 607 filtered out; finished in 1.37s
```

assigned worktree after implementing the approved tag: `mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_db ws17_event_board_and_huddle_pushers_match_50_actual_rails_source_cases -- --test-threads=4`

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 607 filtered out; finished in 2.20s
```

assigned worktree: `CI=1 CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire ws17_delivery:: -- --test-threads=4`

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 523 filtered out; finished in 0.23s
```

assigned worktree: `python3 rust/reference-tools/ws17_regenerate_auth_reference.py`

```text
Rails source verified: 60 files match d7c7de92; 1 board pusher matches a6f10a25; 10 status/layout files match 2e20b24c
WS9 full-page goldens: 15 complete Rails auth pages rendered from the parity seed
Rails message icon registry: 1948 names
Rails message states: 17 complete message trees
Rails WS6 review: 13 notification states, 41 helper groups, 2 frame layouts, 2 token-free message viewers
Rails core goldens: 12 layouts, 8 partials, 5 pages
Exported 20 vendored files and 463 compiled assets to /work/assets
Approved layout goldens: 15 complete foundation cases; reference 2e20b24c
```

assigned worktree before the approved layout listener: `CI=1 CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks -- --test-threads=4`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.30s
```

assigned worktree after the approved layout listener: `CI=1 CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks -- --test-threads=4`

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.43s
```

assigned worktree: `mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_views --test core -- --test-threads=4`

```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
```

assigned worktree: `python3 rust/reference-tools/ws17_injections.py auth-approved-layout`

```text
auth-approved-layout: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 525 filtered out; finished in 0.24s
```

assigned worktree: `mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_assets --test reference -- --test-threads=4`

```text
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
```

The notification-source fixture also ran twice with identical captured JSON after explicitly setting the board-membership fixture timestamps to the frozen clock. Database bulk creation otherwise stamps those setup rows with the actual database clock. Only fixture timestamps were fixed; complete captured payloads, subscription scopes and unread badge bytes remain unmodified.

## Failing assertions and restored-source checks

Every cited injection command was rerun in this continuation, including seven profile and five push injections on the final owned fixes; the raw failing summary lines above are actual assertion failures, not compiler errors. The runner restores the original source in `finally` before advancing. Seven profile injections now include WS9 integration and unsaved layout state: commit appearance before its security audit, discard the request layout snapshot, mark a blank zone explicit before the password guard, erase the saved theme, accept private DNS, remove concurrent unique-index rescue, or erase sound metadata. Three DM injections leak the viewer, omit future-OOO stream mounts, or force live profile presence offline. Keyword injections remove mention priority, erase read/handled state, or commit before recording. Calendar injections restore needless steady-state writes, remove the concurrent claim guard, duplicate both-opt-in refreshes, or commit before claim/refresh enqueue. Push injections erase the approved board tag, bypass senderless reminder DND, ignore stale events, admit exactly-ten-minute joins, or commit before delivery enqueue. Named-policy/status injections cover quietness, bot/inactive inbox eligibility, query ceilings, loaded deactivation, expired OOO notes and meeting/DND precedence. All fail real assertions. Final clippy and complete fresh-clone tests use restored source.

Original declarations remain byte-verified against `d7c7de92`. Unchanged policy bodies pass 52 cases/133 assertions; unchanged OOO/meeting bodies pass 22+14 cases/98 assertions; selected reader/matcher bodies pass 4+10 cases/35 assertions; unchanged push-gating/forwarding bodies pass 8+1 cases/34 assertions. Capture hosts supply fixtures, clock, and original helpers; assertion bodies are not rewritten into invented expected tables.

The initial final workspace run exposed a worker-fixture race: quote refresh had finished, but an automatically started `Retention::PruneJob` was still running when the test asserted the entire queue was empty. Its actual raw failure was:

```text
test result: FAILED. 520 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 248.94s
```

`jobs/tests.rs::app_in` now boots the real worker registry with automatic periodic loops disabled. Dedicated periodic-host tests explicitly construct/start their own loops and still run. No handler, assertion, or production scheduler behavior was removed. The worker-focused suite then passed, and the full fresh-clone suite above was rerun on the pushed fix.

The next main-reference integration exposed stale complete-auth assets/layout bytes in the fresh clone:

```text
test result: FAILED. 522 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 91.11s
```

The existing exporters were rerun against the approved post-change layout/assets. With those actual goldens, the focused test still failed on the missing 57-byte event listener before the Rust template changed. The listener port makes all 15 full pages match; removing it again fails the full-page assertion. The final fresh run above uses restored source and post-change captures. No comparison or expected string was masked.

After those page fixes, the fresh clone passed all 523 seeded app tests but then caught the asset crate's stale CSS/JavaScript precompile exports:

```text
test result: FAILED. 3 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s
```

The existing Propshaft/ActionDispatch exporter now runs from the same strictly verified approved image. Only its actual manifest, compiled hashes, importmap/CSS tags and static response fixtures are copied; the pinned gem vendor tree and all asset test comparisons stay unchanged. The focused 8 tests pass and the complete fresh suite is rerun afterward.

Strict clippy initially rejected a new test's boolean equality assertion under `clippy::bool_assert_comparison`. The test now uses the equivalent boolean assertion without an exemption. The full fresh-clone workspace/clippy/browser run above was rerun on that pushed follow-up.

The first fresh Chromium attempt after the additional case failed the immediate icon check with actual `invert(1)` versus expected `none`. The harness now uses its existing bounded CSS-state observation before retaining both exact filter assertions. Fresh Chromium passes all 11 afterward; source assets and expected values were unchanged.

## Precisely remaining

1. **15 exact cases:** WS12 owns 7 generic/work/caller-authorized recorder cases; WS13/WS13b owns 6 invitation-source/missed-call cases; WS14g owns 2 validated-cache/Google-fetch browser cases. These source owners have not landed on this branch or the merged main. Every exact title and seam is below. All 43 formerly deferred cases whose owned prerequisites were present are ported.
2. **Whole-profile/sidebar/room composition:** owned settings, security integration, profile badge/allowance controls and uncached DM OOO wrapper are delivered. WS8b-r/WS8b-r2 provide complete sidebar presence/DM polling composition; WS11 supplies agent profile settings, WS12 inbox, WS13 voice/huddles, WS14 Google/Event, WS15 GitHub. Eleven owned Chromium scenarios pass; full-page/pixel comparison across all source sections remains pending. The adopted #163 status-popup controllers, edit/_fields views, remaining profile/card/sidebar popup composition and new scenario file are WS8b-r2-owned and remain unported here; the common layout listener/assets integration is delivered. Existing status/presence writers stay available through UserStatusSettings::save_status and the documented broadcast methods.
3. **WS12 board and WS14e event source callbacks:** final `BoardNudgeJob { nudge_id }` and `EventReminderJob { event_id }` APIs, registry handlers, recipient policy and transport adapters are implemented. The approved board tag, real constructor payload and registered encrypted delivery are verified. WS12 still owns the source-claim callback. Source owners enqueue them on the same writer transaction that claims the reminder/nudge. `ActivityItem::record_message(&mut Tx, &Message) -> Result<Vec<ActivityItem>>` is final; WS11 must call it on streaming finalization, and WS16 must gate it during imports. Generic source authorization/grouping remains WS12.
4. **WS13 invitation/join lifecycle:** final `Notifications::HuddlePushJob` JSON and registered `enqueue_huddle_request(&mut Tx, HuddlePushRequest) -> Result<bool>` are delivered. Owner JSON was checked against `origin/rust/ws13-huddles` at `6f498b9f`; its source emitter remains `enqueue_huddle_push(&mut Tx, &PushRequest)`. WS13 integrates actual grant/activity item/source jobs and payload construction. The WS17 writer owns the single throttle claim; also calling `prepare_push` would double-claim. Group fan-out batching/performance needs the real owner notifier.
5. **WS14g refresh/cache:** final `Calendar::MeetingRefreshJob { user_id }` is default queue/version 1. WS14 registers the actual fetch/update handler and clears `refresh_pending_at` on successful fetch. The final dispatch/claim/broadcast signatures above require no transport changes. No successful Google refresh is faked.
6. **Rails rollback/readback and cutover rehearsal:** not performed for this workstream; WS9's pre-existing rollback-readback test remains explicitly ignored in the normal suite. This report claims fresh-clone/domain/HTML/transport/browser verification, not a cutover rehearsal.

## Approved Rails drift and owner integration

The shared brief explicitly adopts #162 (`a6f10a25`) for board tags. The notification capture builds `ws17-reference-board-a6f10a25:latest` from the immutable d7 image with a single COPY of the exact Git blob for `app/models/board_automations/nudge_pusher.rb`. Strict SHA256 verification checks that pusher against a6 and the other 52 owned files against d7. The raw Rails payload now carries `board-nudge-#{thread.id}`; no nullable-tag augmentation remains. The 50-state replay failed first with actual null versus captured board-nudge-9, then passed after the Rust change. Registered jobs decrypt to the complete source JSON. The final source-claim ID-job API is unchanged.

The shared brief also adopts #163 (`2e20b24c`), which is now merged as reference source. The common application-layout listener and source asset/auth/foundation golden integration are now delivered. WS8b-r2 still owns popup authorization, GET /users/:user_id/status/edit, PATCH /users/:user_id/status, the edit/_fields views, profile/card/sidebar popup mounts and the remaining client/page integration. The corresponding new Rails controller/system titles are outside the original 347-title selection and must be replayed by that owner against the post-change reference. This worker retains the current validated status writer/broadcast seam and explicitly leaves that page composition partial; no old golden is claimed as evidence for the new popup.

New #163 controller titles deferred to WS8b-r2: "the status popup renders your current status inside the card frame"; "saving from the popup returns the card and broadcasts the new badge"; "clearing from the popup leaves meeting and out-of-office settings alone"; "an invalid save from the popup re-renders the popup with its error"; "saving an unchanged status broadcasts nothing". New system titles in `test/system/status_popup_test.rb` deferred to the same owner: "setting a status from your own profile card"; "your profile in the sidebar opens your card"; "clearing a status from the popup"; "cancel goes back to the card without saving"; "the popup fits a phone screen". Each needs the exact post-change own-card frame/route/form flow described above; these ten new titles are additional to the 15 original selected deferrals.

## Named scenario coverage by file

347 selected exact Rails titles: **332 passed equivalent; 15 deferred**. `rust/plans/ws17-rails-test-inventory.json` records exact title, owner, status, Rust evidence and every deferred seam. Additional HTTP/auth integration tests are outside this inventory.

| Rails file | Passed equivalent | Deferred |
| --- | ---: | ---: |
| `test/models/notifications/policy_test.rb` | 52 | 0 |
| `test/controllers/users/statuses_controller_test.rb` | 22 | 0 |
| `test/models/user/out_of_office_test.rb` | 22 | 0 |
| `test/services/activity_items/recorder_test.rb` | 13 | 7 |
| `test/models/push/subscription_test.rb` | 18 | 0 |
| `test/models/user/meeting_status_test.rb` | 14 | 0 |
| `test/models/user/status_settings_test.rb` | 14 | 0 |
| `test/models/workspace_presence_lease_test.rb` | 14 | 0 |
| `test/models/calendar/meeting_cache_test.rb` | 12 | 1 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 0 |
| `test/models/calendar/ooo_dispatcher_test.rb` | 12 | 0 |
| `test/models/calendar/meeting_dispatcher_test.rb` | 11 | 0 |
| `test/services/activity_items/recorder_keyword_test.rb` | 11 | 0 |
| `test/controllers/users/notification_settings_controller_test.rb` | 10 | 0 |
| `test/integration/ooo_dm_notice_test.rb` | 10 | 0 |
| `test/models/event/reminder_pusher_test.rb` | 10 | 0 |
| `test/models/notifications/keyword_matcher_test.rb` | 10 | 0 |
| `test/models/notifications/push_gating_test.rb` | 8 | 2 |
| `test/models/room/push_test.rb` | 8 | 0 |
| `test/channels/workspace_presence_channel_test.rb` | 7 | 0 |
| `test/controllers/users/presences_controller_test.rb` | 7 | 0 |
| `test/system/status_notifications_test.rb` | 7 | 0 |
| `test/controllers/users/push_subscriptions_controller_test.rb` | 6 | 0 |
| `test/controllers/users/dnd_allowances_controller_test.rb` | 5 | 0 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 0 | 4 |
| `test/models/board_automations/nudge_pusher_test.rb` | 4 | 0 |
| `test/models/saved_item/reminder_pusher_test.rb` | 4 | 0 |
| `test/lib/web_push/persistent_request_test.rb` | 2 | 0 |
| `test/models/dnd_allowed_user_test.rb` | 2 | 0 |
| `test/system/meeting_status_test.rb` | 1 | 1 |
| `test/system/service_worker_test.rb` | 2 | 0 |
| `test/system/out_of_office_test.rb` | 1 | 0 |

## Every deferred exact title, owner and seam

| Rails file | Exact title | Owner | Exact missing seam |
| --- | --- | --- | --- |
| `test/services/activity_items/recorder_test.rb` | a caller-authorized record skips the source check but keeps idempotency | WS12 | ActivityItems::Recorder.record!(recipient:, source:, event_type:, skip_source_check: true): caller-authorized source scope plus grouping and recipient/source unique-index rescue; generic Rust recorder/source authorization is not landed. |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no work items but a mentions member does | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/services/activity_items/recorder_test.rb` | a status update after a work assignment keeps the assignment item and repoints the update item | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/services/activity_items/recorder_test.rb` | work assigned by a bot without an agent ignores the agent_work switch | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/services/activity_items/recorder_test.rb` | work assigned by an agent honors the recipient's agent_work switch | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/services/activity_items/recorder_test.rb` | work events notify followed thread members | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/services/activity_items/recorder_test.rb` | work updates for one thread collapse into a single item | WS12 | WorkThreadEvent.activity_recipient_ids / allows_activity_recipient? and ActivityItems::Recorder.record!(recipient:, source:, event_type:): work_assignment idempotency, grouped work_update repointing, followed membership and agent_work preference. Generic work-source recorder is not landed; message-only ActivityItem::record_message(tx, &Message) is final. |
| `test/jobs/huddle/push_invitation_job_test.rb` | a connected recipient gets no push | WS13/WS13b | Huddle::PushInvitationJob validates live invitation/grant, recipient and connection/opt-out eligibility and constructs the invitation payload; emit huddle_notices::PushRequest with enqueue_huddle_push(tx, &request), class Notifications::HuddlePushJob. WS17 HuddlePushRequest wire bridge and enqueue_huddle_request(&mut Tx, request) are final; no source invitation job is landed on this branch. |
| `test/jobs/huddle/push_invitation_job_test.rb` | an opted-out recipient gets no push subscriptions | WS13/WS13b | Huddle::PushInvitationJob validates live invitation/grant, recipient and connection/opt-out eligibility and constructs the invitation payload; emit huddle_notices::PushRequest with enqueue_huddle_push(tx, &request), class Notifications::HuddlePushJob. WS17 HuddlePushRequest wire bridge and enqueue_huddle_request(&mut Tx, request) are final; no source invitation job is landed on this branch. |
| `test/jobs/huddle/push_invitation_job_test.rb` | missing invitations are ignored | WS13/WS13b | Huddle::PushInvitationJob validates live invitation/grant, recipient and connection/opt-out eligibility and constructs the invitation payload; emit huddle_notices::PushRequest with enqueue_huddle_push(tx, &request), class Notifications::HuddlePushJob. WS17 HuddlePushRequest wire bridge and enqueue_huddle_request(&mut Tx, request) are final; no source invitation job is landed on this branch. |
| `test/jobs/huddle/push_invitation_job_test.rb` | pushes the invitation to the recipient only | WS13/WS13b | Huddle::PushInvitationJob validates live invitation/grant, recipient and connection/opt-out eligibility and constructs the invitation payload; emit huddle_notices::PushRequest with enqueue_huddle_push(tx, &request), class Notifications::HuddlePushJob. WS17 HuddlePushRequest wire bridge and enqueue_huddle_request(&mut Tx, request) are final; no source invitation job is landed on this branch. |
| `test/models/notifications/push_gating_test.rb` | group huddle push skips DND and quiet-hours recipients but their missed calls are still recorded | WS13/WS13b | Huddle::Grant.issue / InvitationResolver source lifecycle and missed-call ActivityItem after the 46-second group-ring expiry. Invitation/job must emit Notifications::HuddlePushJob PushRequest; WS17 applies DND/caller allowance and atomically claims/enqueues delivery with enqueue_huddle_request. Actual missed-call/grant source path is unmerged. |
| `test/models/notifications/push_gating_test.rb` | huddle push honors DND with a starred-caller exception | WS13/WS13b | Huddle::Grant.issue / InvitationResolver source lifecycle and missed-call ActivityItem after the 46-second group-ring expiry. Invitation/job must emit Notifications::HuddlePushJob PushRequest; WS17 applies DND/caller allowance and atomically claims/enqueues delivery with enqueue_huddle_request. Actual missed-call/grant source path is unmerged. |
| `test/models/calendar/meeting_cache_test.rb` | one cache per user | WS14g | Calendar::MeetingCache.create!(user:) validated creation must reject a duplicate user_id with RecordInvalid before INSERT; WS17 only exposes existing-row find/read and claim_broadcast(&self, &mut Tx, active)/claim_refresh_followup(&self, &mut Tx, now, window). Raw setup SQL or a unique-index error does not replay the model validation. |
| `test/system/meeting_status_test.rb` | opting in shows In a meeting for a stubbed busy interval, then clears after it ends | WS14g | Calendar::MeetingRefreshJob { user_id: i64 }, class Calendar::MeetingRefreshJob, default queue/version 1: WS14 Google events fetch/interval filtering and cache completion must run before dispatch_meetings(&Database, Timestamp) and the browser clock advance. Handler is deliberately absent, not a fabricated refresh success. |
