# WS17 Wave 4 — partial, continuing in pushed slices

Reference Rails `d7c7de92`; merged main `21a7332f` with merge commit `56aa9f62`. Branch `rust/ws17-push-presence`. This report accompanies the current slice; the delivery reply gives the pushed SHA. The tracked report is the exact mirror of the requested external report.

## Delivered

Earlier pushed slices (`7b9267a3`, `32b49aa4`, `76c34826`, `0adcae95`, `12c448d7`) provide typed notification policy, tagged Web Push with Smartfire subject and IP pinning, durable room/thread/saved/test push transport, presence leases/HTTP/pruner, validated dirty-tracked settings writes, status and notification PATCH, DND allowances, keyword-list replacement, cache reconciliation, manual OOO claims, and complete badge/OOO broadcast HTML. PWA worker/offline bytes are pinned. Existing Rails oracle vectors remain exercised by the full suites below.

Profile/subscription/allowance slice pushed at `ba916992`; keyword recording pushed at `fd411985`; calendar dispatch pushed at `9dff8776`; notification push adapters pushed at `7eaf269f`; named policy replay pushed at `e90bb4c5`; named OOO/meeting replay pushed at `1021be6a`; current DM/profile integration slice:

| Files | Change and verification |
| --- | --- |
| `campfire/controllers/users/profiles.rs`, `profiles/ws17_tests.rs` | Persist theme, text size and time zone through the existing validated settings writer, mark an explicitly submitted zone (including blank) as chosen, atomically roll back other submitted fields on invalid appearance. Preserve unsaved form values. Profile errors populate the profile path and do not inherit the settings-controller missing-device failure. Seven seeded HTTP tests cover writes, invalid rollback, IANA/legacy selection and actual layout sound markers. |
| `views/users/settings.rs`, `profile_zones.json`, `templates/users/profiles/_appearance.html`, profile show; `presenters/status_settings.rs` | Mechanical appearance form port, error wrappers and escaping. Six complete form strings match actual Rails. All 135 friendly zone choices retain TZInfo identifiers and current base-offset labels using pinned TZInfo transitions; tests caught Casablanca's negative-DST distinction. No view queries. |
| `db/models/user_status_settings.rs`, `presenters/view_context.rs` | Preload actual DND, quiet-hour, meeting and OOO windows into layout preferences. Current and future windows stay in source order; opt-outs remove only their owned marker. |
| `controllers/users/push_subscriptions.rs`, `push_subscriptions/ws17_tests.rs`; `app.rs`, test support | All six named subscription HTTP scenarios, including legacy revalidation and private-IP refusal. Per-app DNS dependency permits deterministic DNS answers while testing the real endpoint guard/model/HTTP stack. Production uses system DNS. Current-user deletion scope checked additionally. Real user-agent rendering matches four Rails cases. |
| `views/templates/users/push_subscriptions/index.html`, `tests/ws17_settings.rs` | Complete subscription content byte comparison, including full row forms, fixed shared test CSRF values, actual asset URLs, escaping and whitespace. |
| `controllers/users/dnd_allowances.rs` | Repeated star remains one row; deterministic real UNIQUE-index failure at insert follows Rails' success redirect, in addition to existing concurrent HTTP requests. No mocks of the writer. |
| `db/models/activity_item/message_recorder.rs`, message callback, `tests/message_activity_test.rs`, JSON oracle | Message-only recorder: flat scoped membership/keyword queries, policy winner, active-human/self exclusion, idempotent source rows, grouped followed-thread updates, unchanged read/handled state on repeated non-grouped recording. Thirty-one actual Rails callback/candidate vectors; all eleven keyword recorder titles and thirteen message-only recorder titles. Real SQLite trace checks stay flat at five versus thirty members. |
| `campfire/controllers/messages/ws17_activity_tests.rs`, DB Cargo dev dependency | Full HTTP message callback records during DND; real rendered mentions win over keywords. Rejecting the activity INSERT rolls back message, FTS index and durable jobs. SQLite tracing is a test-only rusqlite feature. |
| `db/models/calendar_dispatch.rs`, meeting cache claim methods, status broadcast batching; `campfire/jobs/periodic.rs` | Register meeting/OOO sweeps every minute. Match active scopes (including bots), inclusive 15-minute stale threshold, missing-cache handling, steady-state no-UPDATE/no-writer path and OOO-only refresh gating. Per-member refresh enqueue and claim share a writer transaction; an enqueue failure rolls back only that member and the sweep continues. Preserve expired-already-false manual columns as the pin does. All badges precede all notices, with one lease query. |
| `db/tests/calendar_dispatch_test.rs`, `statuses/calendar_tests.rs`, calendar oracle | Twenty-five actual Rails two-tick vectors; all 11 meeting and 12 OOO named scenarios pass. Compare stored claims/manual columns, durable refresh rows, exact signed-stream HTML and actual SQLite UPDATE counts. Two independent database handles prove one winning boundary/one broadcast; corrupt cached timestamps isolate the bad member. Twelve of 13 cache titles pass; validated duplicate creation belongs to WS14 and remains deferred. |
| `app.rs`, seeded HTTP test support | Inject periodic host intervals explicitly. Production reads the same environment intervals; seeded HTTP tests run no background periodic host, matching Rails' reference server. Durable queue and broadcasts remain real. This removes the observed race with an unrequested first periodic tick. Registration itself is tested and the sweeps are invoked explicitly. |
| `db/models/notification_push.rs`, subscription batches; `campfire/jobs/notifications.rs` | Live event/board source readers until WS14/WS12 land. Durable event/board jobs re-read memberships and current reminder policy with no sender exception. Match event rounding/staleness, venue/direct title/tag, board status/escalation/path and membership recheck. WS13 huddle adapter keeps supplied payload bytes, SQL disconnected/visible scope, actual caller allowances, huddle inbox switch, strict older-than-ten-minutes throttle, and atomic throttle/delivery-job enqueue. Four new registered handlers. |
| `db/tests/notification_push_test.rs`, notification push oracle; `web_push/ws17_delivery_tests.rs` | Fifty actual Rails source/policy states including all 10 event, 4 board and 13 join-pusher titles. Claims run on real SQLite and across two database handles; repeat/eleven-minute replay. Registered handlers run against private seeded app DB and local TLS, comparing complete decrypted JSON with actual Rails strings (no output masks). Rejecting a delivery INSERT rolls back both throttle and triggering source write. |
| `db/tests/named_policy_test.rs`, `notification_policy.rs`; named policy oracle, pinned declarations and regeneration | Largest remaining file first: 52 individually named Rust tests. Run the unchanged pinned Ruby test declarations/setup/private helpers/assertions under an isolated fixture/clock host, not rewritten case tables: all 52 Ruby cases and 133 original assertions pass. Capture 82 constructor observations, then replay actual persisted recipient/cache/allowance state and decisions in Rust. A real SQLite trace asserts zero policy queries and one batched allowance query (zero without sender). Dynamic kind parsing rejects unknown strings with the pin's exact error message. |
| `db/tests/named_calendar_status_test.rs`, `user_status_settings.rs`; pinned OOO/meeting declarations and oracle | Next largest deferred files: all 22 OOO and 14 meeting-status titles, each an individual named Rust test. The unchanged Ruby bodies run 98 original assertions; replay all 199 operations in order on one real DB/clock per case, checking both loaded and persisted attributes before/after every call. Cache creation is source setup SQL at the WS14 seam. Match failed-save retention, expired settings, racing/repeated claims, presets, zones, quiet/label precedence and deactivation. Add the separate pure visibility/quiet/date readers and a loaded-settings deactivation wrapper that clears the owned OOO attributes with the real User write. |
| `views/users/statuses.rs`, uncached OOO wrapper, profile status/allowance partials, room/user presenters and controllers | Mount the complete pinned OOO wrapper for every other active human DM member, including blank/off members so future OOO can update live. Exact name ordering, viewer scoping, escaped note, calendar invisibility and signed streams. Add the live profile badge and viewer-scoped DND controls. Nine complete wrapper strings, three profile status sections and both full allowance forms are byte-identical to actual pinned Rails. All 10 named DM integration cases pass over seeded HTTP/real sockets, including an actual two-hour shared clock advance. Three injections fail assertions. |
| `reference-tools/ws17_browser.py`, `ws17_browser.mjs` | Ten individually named Chromium scenarios: 7 status notifications, 1 OOO and 2 service worker. Real forms, actual persisted readback, live Audio replacement exactly as the original test, computed CSS under opposite OS theme, 390px viewport, two Rails-issued user cookies and actual browser CacheStorage. Isolated private seed copy and native server on owned 52471; no source/controller stubs or output masks. Wait for observable CSS completion after the Turbo render. Google fetch/browser execution stays WS14-owned. |
| `db/tests/status_settings_write_test.rs`, `keyword_alert_test.rs`, pinned reader declarations and regeneration | Close the four remaining exact StatusSettings reader sequences using real saves, loaded reloads and advancing clocks; complete the literal, blank-only and Unicode matcher cases already landed on this branch. The 4 reader and all 10 matcher original Ruby bodies run unchanged and pass 35 original assertions. Selected inventory now has StatusSettings 14/14 and KeywordMatcher 10/10. |
| `campfire/integrations/web_push/tests.rs` | Close five deferred subscription sequences against the real Guard/model composition and delivery transport: loopback, link-local, empty DNS, deferred construction lookup and fresh private DNS refusal with no dial. The sixth case sets all four proxy keys in an isolated child process and proves actual TLS delivery still dials the pinned public address and decrypts the expected JSON. No unsafe global test environment mutation. |
| `db/models/notification_push.rs`, `jobs/notifications.rs`, `web_push/ws17_delivery_tests.rs` | Final WS13 wire DTO and registered Notifications::HuddlePushJob bridge; actual owner JSON and single claim/durable delivery transaction. Standalone source vectors restore every row behind the captured unread badge. Existing event/board and direct huddle APIs remain final and unchanged. |
| `db/tests/named_push_gating_test.rs`, room pool handoff, pinned gating declarations | Eight exact message/thread/reminder push-gating sequences plus the forwarded-note case use real persisted messages, followed/unfollowed reply sequences, activity recording, reminder membership scopes and the actual forwarder. The original nine Ruby bodies pass 34 original assertions using unchanged mention/DNS helpers and nonjoinable isolated transactions so model commit callbacks execute before rollback. Room delivery now hands the distinct union to the pool once, in subscription ID order, matching the pin. Full encrypted transport regressions pass. |
| `reference-tools/ws17_profile_ui.rb`, regeneration script, verifier and injection runner | Actual pinned source/output verification; no Rails changes, output masks, allowlist changes or new ignores. |

User/profile security, GitHub/inbox/voice settings and connected-service UI belong to WS9/WS11/WS12/WS13/WS14/WS15. The existing basic profile update path still needs those owners' callbacks. This slice adds only the owned appearance attributes, without claiming whole-profile parity.

The pin's invalid status/notification render on a seeded confirmed-2FA user still produces HTTP 500 because Rails omits `@two_factor_devices`. Separate tests preserve that observed behavior; the named fixture replays explicitly use unconfirmed-credential fixture state and expect 422. Profile PATCH initializes those devices in Rails and its owned validation response is 422.

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

The current slice has focused verification below; the six previous slices retain their previous final evidence until the end-of-session rerun. Fresh-clone verification is required before the final handoff and remains pending while this worker continues.

`mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null` was rerun in `rust/` and exited 0 with no output. Strict TOML parsing rejected duplicate keys and reported:

```text
workspace dependency keys: 75 unique; 0 duplicates (strict TOML parse)
```

All commands run in this worktree, own `rust/target`, pinned toolchain and `-j 4`. Seeded app tests require the built default/first-run seeds, not a silent local skip. Existing app ignores are main's cable recording/latency tests and WS11's `manages_bots`; no new ignore was added. DB's three existing oracle/export ignores require their dedicated external environment.

`python3 rust/reference-tools/ws17_regenerate_named_gating.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned gating declarations verified: 4 files byte-identical to d7c7de92
Rails named gating: test/models/notifications/push_gating_test.rb: 8 passed cases; 28 original Rails assertions
Rails named gating: test/models/room/push_test.rb: 1 passed cases; 6 original Rails assertions
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db named_push_gating_test`

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 552 filtered out; finished in 0.20s
```

`CI=1 CABLE_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire integrations::web_push::tests:: -- --test-threads=4`

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 405 filtered out; finished in 0.72s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.77s
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db notification_push_test`

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 550 filtered out; finished in 3.29s
```

`CI=1 CABLE_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire ws17_delivery`

```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.27s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.10s
```

`CI=1 CABLE_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire ws17_ -- --test-threads=4`

```text
test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 309 filtered out; finished in 10.69s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.00s
```

`python3 rust/reference-tools/ws17_regenerate_named_readers.py`

```text
pinned Rails source verified: 53 files match d7c7de92
pinned reader/matcher declarations verified: 2 files byte-identical to d7c7de92
Rails named readers: test/models/notifications/keyword_matcher_test.rb: 10 passed cases; 18 original Rails assertions
Rails named readers: test/models/user/status_settings_test.rb: 4 passed cases; 17 original Rails assertions
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db status_settings_write_test`

```text
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 528 filtered out; finished in 0.61s
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db keyword_alert_test`

```text
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 538 filtered out; finished in 0.08s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.24s
```

`python3 rust/reference-tools/ws17_browser.py`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.94s
test/system/status_notifications_test.rb: 7 passed; 0 failed
test/system/out_of_office_test.rb: 1 passed; 0 failed
test/system/service_worker_test.rb: 2 passed; 0 failed
WS17 Chromium: 10 passed; 0 failed
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.50s
```

`python3 rust/reference-tools/ws17_regenerate_dm_profile.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails DM/profile: 9 complete DM wrappers; 3 profile badges; 2 allowance controls
```

`python3 rust/reference-tools/ws17_injections.py dm-`

```text
dm-viewer-scope: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.12s
dm-streams-for-future-ooo: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.45s
dm-profile-live-presence: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.43s
```

`CI=1 CABLE_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire ws17_ -- --test-threads=4`

```text
test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 309 filtered out; finished in 11.17s
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views --test ws17_dm_profile`

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.37s
```

`python3 rust/reference-tools/ws17_regenerate_profile_ui.py`

```text
pinned Rails source verified: 50 files match d7c7de92
Rails profile UI: 6 complete appearance forms; 1 complete subscription content; 135 zone choices
```

`python3 rust/reference-tools/ws17_injections.py profile`

```text
profile-save-theme: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.43s
profile-private-endpoint: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.43s
profile-unique-loser: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.46s
profile-sound-metadata: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.43s
```

`python3 rust/reference-tools/ws17_regenerate_message_activity.py`

```text
pinned Rails source verified: 50 files match d7c7de92
Rails message activity: 31 complete callback/candidate cases
```

`python3 rust/reference-tools/ws17_injections.py keyword`

```text
keyword-priority: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 1.38s
keyword-read-state: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.11s
keyword-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.47s
```

`python3 rust/reference-tools/ws17_regenerate_calendar_dispatch.py`

```text
pinned Rails source verified: 50 files match d7c7de92
Rails calendar dispatch: 25 complete two-tick cases
```

`python3 rust/reference-tools/ws17_injections.py calendar`

```text
calendar-steady-write: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.09s
calendar-racing-claim: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.10s
calendar-duplicate-refresh: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.10s
calendar-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.41s
```

`python3 rust/reference-tools/ws17_regenerate_notification_push.py`

```text
pinned Rails source verified: 53 files match d7c7de92
Rails notification push: 50 complete source/policy payload cases
```

`python3 rust/reference-tools/ws17_injections.py push-`

```text
push-reminder-dnd: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 1.14s
push-event-stale: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 1.38s
push-huddle-boundary: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 2.24s
push-huddle-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.11s
```

`python3 rust/reference-tools/ws17_regenerate_named_policy.py`

```text
pinned Rails source verified: 50 files match d7c7de92
pinned policy test declarations verified: byte-identical to d7c7de92
Rails named policy: 52 passed cases; 133 original Rails assertions; 82 constructor observations
```

`python3 rust/reference-tools/ws17_injections.py named-policy`

```text
named-policy-quiet: detected
test result: FAILED. 35 passed; 17 failed; 0 ignored; 0 measured; 496 filtered out; finished in 4.16s
named-policy-bot-inbox: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.18s
named-policy-query-ceiling: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.23s
```

`python3 rust/reference-tools/ws17_regenerate_named_calendar_status.py`

```text
pinned Rails source verified: 50 files match d7c7de92
pinned calendar status declarations verified: 2 files byte-identical to d7c7de92
Rails named out_of_office: 22 passed cases; 71 original Rails assertions; 129 operations
Rails named meeting_status: 14 passed cases; 27 original Rails assertions; 70 operations
```

`python3 rust/reference-tools/ws17_injections.py named-status`

```text
named-status-deactivate: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.32s
named-status-expired-note: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 547 filtered out; finished in 0.12s
named-status-meeting-dnd: detected
test result: FAILED. 33 passed; 3 failed; 0 ignored; 0 measured; 512 filtered out; finished in 1.36s
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db named_calendar_status_test`

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 512 filtered out; finished in 0.83s
```

`mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db named_policy_test`

```text
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 496 filtered out; finished in 1.81s
```

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -p campfire_views -- --test-threads=4`

```text
test result: ok. 545 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 47.03s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.93s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -- --test-threads=4`

```text
test result: ok. 409 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 37.86s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.99s
```

The three new named status injections retain stale loaded OOO on deactivation, leak an expired manual note into calendar OOO, and ignore manual/scheduled DND for the meeting label; all fail real assertions. Before the loaded-settings wrapper, the named replay failed with `test result: FAILED. 35 passed; 1 failed; 0 ignored; 0 measured; 512 filtered out; finished in 0.79s`. The three named policy injections bypass quietness, allow bot/inactive inbox items, and issue a second allowance query; all fail actual assertions. Every cited regeneration/injection command is rerun for this final slice before the full suites. Four new push injections bypass senderless reminder DND, skip event staleness, make the huddle throttle boundary inclusive, and commit before enqueue. Every injection fails a real test and sources are restored. The pre-change periodic registration test failed with `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 385 filtered out; finished in 0.00s`. Four calendar injections restore steady-state claim writes, remove the concurrent claim guard, duplicate both-opt-in refreshes, and commit before the refresh/claim; all produce actual failed tests. The pre-change keyword callback failed with `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 439 filtered out; finished in 0.04s`. Three keyword injections remove mention priority, reset read/handled state on conflict, and commit before the callback write; all produce real failed tests. The pre-change appearance HTTP test failed on persisted `system` versus submitted `dark`: `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.48s`. Complete form comparison also failed before correcting dynamic zone labels; subscription content failed before correcting its exact leading/collection whitespace. The four committed injections remove the theme save, accept private DNS, remove unique-index rescue, and erase sound metadata; each must produce an actual failed test, not a compiler error. Sources are restored by the runner before final suites.

## Precisely remaining

1. Profile/UI integration stays partial: Smartfire's complete profile contains other owners' sections; sidebar presence composition must connect to WS8b/WS13's full room templates and request-free broadcasts. Profile badge and uncached DM OOO composition, line bytes and transport are now ported. The 10 owned browser scenarios now pass; full-page/pixel parity and Rails rollback/readback rehearsal remain pending. Appearance, subscription and allowance gaps above are closed.
2. Message keyword recording is delivered. WS12 still owns generic/caller-authorized recording, work events, inbox queries/controllers/source rendering and access rules. `ActivityItem::record_message(tx: &mut Tx, message: &Message) -> Result<Vec<ActivityItem>>` is the minimal shared seam. WS11 must call it on a live stream finalize; WS16 must gate it for importing together with the existing message callback chain. This slice gates normal creation on non-streaming/non-system-note state; edits do not re-record.
3. Meeting/OOO due sweeps and conditional claims/broadcasts are delivered. WS14 owns validated cache creation (the one remaining cache title), Google fetch execution and refresh completion at the documented job seam.
4. Event/board pushers, registered durable jobs and the huddle policy/throttle/durable-delivery adapter are delivered. WS12/14 must connect their source claims/callbacks to the documented ID jobs; WS13 must connect invitation/join source jobs and payloads to the adapter. Four named invitation-source job titles remain deferred to WS13. Room handler union/forwarded-note audit is complete. Huddle fan-out batching against the eventual WS13 notifier still needs owner integration/performance verification.
5. Remaining exact named scenarios below, largest files first. User::OutOfOfficeTest (22/22) and User::MeetingStatusTest (14/14) are now complete. OOO DM integration is now complete (10/10). The next file is push gating (10), then generic/work/caller-authorized recorder cases and status-notification browser cases (7 each). These exact sequences remain deferred; no broader replay coverage is claimed.

## Named scenario coverage by file

347 selected exact Rails titles: **331 passed equivalent; 16 deferred**. `rust/plans/ws17-rails-test-inventory.json` records exact title, owner, status and Rust evidence. Additional profile/UI HTTP cases are outside this pre-existing selected inventory.

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
| `test/system/meeting_status_test.rb` | 0 | 2 |
| `test/system/service_worker_test.rb` | 2 | 0 |
| `test/system/out_of_office_test.rb` | 1 | 0 |

## Every deferred exact title and owner

| Rails file | Exact title | Owner |
| --- | --- | --- |
| `test/services/activity_items/recorder_test.rb` | a caller-authorized record skips the source check but keeps idempotency | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no work items but a mentions member does | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a status update after a work assignment keeps the assignment item and repoints the update item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by a bot without an agent ignores the agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by an agent honors the recipient's agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work events notify followed thread members | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work updates for one thread collapse into a single item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/jobs/huddle/push_invitation_job_test.rb` | a connected recipient gets no push | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | an opted-out recipient gets no push subscriptions | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | missing invitations are ignored | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | pushes the invitation to the recipient only | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/notifications/push_gating_test.rb` | group huddle push skips DND and quiet-hours recipients but their missed calls are still recorded | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | huddle push honors DND with a starred-caller exception | WS17 continuation |
| `test/system/meeting_status_test.rb` | opting in shows In a meeting for a stubbed busy interval, then clears after it ends | WS17 continuation |
| `test/system/meeting_status_test.rb` | the profile links to connect without a Google account | WS17 continuation |
| `test/models/calendar/meeting_cache_test.rb` | one cache per user | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
