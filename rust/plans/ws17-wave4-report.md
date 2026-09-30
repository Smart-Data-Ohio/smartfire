# WS17 Wave 4 — partial, continuing in pushed slices

Reference Rails `d7c7de92`; merged main `21a7332f` with merge commit `56aa9f62`. Branch `rust/ws17-push-presence`. This report accompanies the current slice; the delivery reply gives the pushed SHA. The tracked report is the exact mirror of the requested external report.

## Delivered

Earlier pushed slices (`7b9267a3`, `32b49aa4`, `76c34826`, `0adcae95`, `12c448d7`) provide typed notification policy, tagged Web Push with Smartfire subject and IP pinning, durable room/thread/saved/test push transport, presence leases/HTTP/pruner, validated dirty-tracked settings writes, status and notification PATCH, DND allowances, keyword-list replacement, cache reconciliation, manual OOO claims, and complete badge/OOO broadcast HTML. PWA worker/offline bytes are pinned. Existing Rails oracle vectors remain exercised by the full suites below.

Profile/subscription/allowance slice pushed at `ba916992`; keyword recording pushed at `fd411985`; calendar dispatch pushed at `9dff8776`; current notification push slice:

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

All commands run in this worktree, own `rust/target`, pinned toolchain and `-j 4`. Seeded app tests require the built default/first-run seeds, not a silent local skip. Existing app ignores are main's cable recording/latency tests and WS11's `manages_bots`; no new ignore was added. DB's three existing oracle/export ignores require their dedicated external environment.

`python3 rust/reference-tools/ws17_regenerate_profile_ui.py`

```text
pinned Rails source verified: 42 files match d7c7de92
Rails profile UI: 6 complete appearance forms; 1 complete subscription content; 135 zone choices
```

`python3 rust/reference-tools/ws17_injections.py profile`

```text
profile-save-theme: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.54s
profile-private-endpoint: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.41s
profile-unique-loser: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.47s
profile-sound-metadata: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 382 filtered out; finished in 0.49s
```

`python3 rust/reference-tools/ws17_regenerate_message_activity.py`

```text
pinned Rails source verified: 45 files match d7c7de92
Rails message activity: 31 complete callback/candidate cases
```

`python3 rust/reference-tools/ws17_injections.py keyword`

```text
keyword-priority: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.90s
keyword-read-state: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.12s
keyword-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 384 filtered out; finished in 0.42s
```

`python3 rust/reference-tools/ws17_regenerate_calendar_dispatch.py`

```text
pinned Rails source verified: 45 files match d7c7de92
Rails calendar dispatch: 25 complete two-tick cases
```

`python3 rust/reference-tools/ws17_injections.py calendar`

```text
calendar-steady-write: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 456 filtered out; finished in 0.08s
calendar-racing-claim: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 456 filtered out; finished in 0.40s
calendar-duplicate-refresh: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 409 filtered out; finished in 0.10s
calendar-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 409 filtered out; finished in 0.65s
```

`python3 rust/reference-tools/ws17_regenerate_notification_push.py`

```text
pinned Rails source verified: 50 files match d7c7de92
Rails notification push: 50 complete source/policy payload cases
```

`python3 rust/reference-tools/ws17_injections.py push-`

```text
push-reminder-dnd: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 459 filtered out; finished in 1.81s
push-event-stale: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 459 filtered out; finished in 1.48s
push-huddle-boundary: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 459 filtered out; finished in 3.22s
push-huddle-atomic: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.11s
```

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -p campfire_views -- --test-threads=4`

```text
test result: ok. 457 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 39.96s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.82s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -- --test-threads=4`

```text
test result: ok. 409 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 34.58s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.79s
```

Four new push injections bypass senderless reminder DND, skip event staleness, make the huddle throttle boundary inclusive, and commit before enqueue. Every injection fails a real test and sources are restored. The pre-change periodic registration test failed with `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 385 filtered out; finished in 0.00s`. Four calendar injections restore steady-state claim writes, remove the concurrent claim guard, duplicate both-opt-in refreshes, and commit before the refresh/claim; all produce actual failed tests. The pre-change keyword callback failed with `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 439 filtered out; finished in 0.04s`. Three keyword injections remove mention priority, reset read/handled state on conflict, and commit before the callback write; all produce real failed tests. The pre-change appearance HTTP test failed on persisted `system` versus submitted `dark`: `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.48s`. Complete form comparison also failed before correcting dynamic zone labels; subscription content failed before correcting its exact leading/collection whitespace. The four committed injections remove the theme save, accept private DNS, remove unique-index rescue, and erase sound metadata; each must produce an actual failed test, not a compiler error. Sources are restored by the runner before final suites.

## Precisely remaining

1. Profile/UI integration stays partial: Smartfire's complete profile contains other owners' sections; sidebar/DM presence composition must connect to WS8b/WS13's full room templates and request-free broadcasts. Badge and OOO notice-line bytes and transport are already ported. Full page/browser/pixel parity and Rails rollback/readback rehearsal have not run. Appearance, subscription and allowance gaps above are closed.
2. Message keyword recording is delivered. WS12 still owns generic/caller-authorized recording, work events, inbox queries/controllers/source rendering and access rules. `ActivityItem::record_message(tx: &mut Tx, message: &Message) -> Result<Vec<ActivityItem>>` is the minimal shared seam. WS11 must call it on a live stream finalize; WS16 must gate it for importing together with the existing message callback chain. This slice gates normal creation on non-streaming/non-system-note state; edits do not re-record.
3. Meeting/OOO due sweeps and conditional claims/broadcasts are delivered. WS14 owns validated cache creation (the one remaining cache title), Google fetch execution and refresh completion at the documented job seam.
4. Event/board pushers, registered durable jobs and the huddle policy/throttle/durable-delivery adapter are delivered. WS12/14 must connect their source claims/callbacks to the documented ID jobs; WS13 must connect invitation/join source jobs and payloads to the adapter. Four named invitation-source job titles remain deferred to WS13. Room handler audit remains. Huddle fan-out batching against the eventual WS13 notifier still needs owner integration/performance verification.
5. Remaining exact named scenarios below, largest files first; pure vector coverage is not claimed as a replay of every named sequence.

## Named scenario coverage by file

347 selected exact Rails titles: **201 passed equivalent; 146 deferred**. `rust/plans/ws17-rails-test-inventory.json` records exact title, owner, status and Rust evidence. Additional profile/UI HTTP cases are outside this pre-existing selected inventory.

| Rails file | Passed equivalent | Deferred |
| --- | ---: | ---: |
| `test/models/notifications/policy_test.rb` | 0 | 52 |
| `test/controllers/users/statuses_controller_test.rb` | 22 | 0 |
| `test/models/user/out_of_office_test.rb` | 0 | 22 |
| `test/services/activity_items/recorder_test.rb` | 13 | 7 |
| `test/models/push/subscription_test.rb` | 13 | 5 |
| `test/models/user/meeting_status_test.rb` | 0 | 14 |
| `test/models/user/status_settings_test.rb` | 10 | 4 |
| `test/models/workspace_presence_lease_test.rb` | 14 | 0 |
| `test/models/calendar/meeting_cache_test.rb` | 12 | 1 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 0 |
| `test/models/calendar/ooo_dispatcher_test.rb` | 12 | 0 |
| `test/models/calendar/meeting_dispatcher_test.rb` | 11 | 0 |
| `test/services/activity_items/recorder_keyword_test.rb` | 11 | 0 |
| `test/controllers/users/notification_settings_controller_test.rb` | 10 | 0 |
| `test/integration/ooo_dm_notice_test.rb` | 0 | 10 |
| `test/models/event/reminder_pusher_test.rb` | 10 | 0 |
| `test/models/notifications/keyword_matcher_test.rb` | 7 | 3 |
| `test/models/notifications/push_gating_test.rb` | 0 | 10 |
| `test/models/room/push_test.rb` | 7 | 1 |
| `test/channels/workspace_presence_channel_test.rb` | 7 | 0 |
| `test/controllers/users/presences_controller_test.rb` | 7 | 0 |
| `test/system/status_notifications_test.rb` | 0 | 7 |
| `test/controllers/users/push_subscriptions_controller_test.rb` | 6 | 0 |
| `test/controllers/users/dnd_allowances_controller_test.rb` | 5 | 0 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 0 | 4 |
| `test/models/board_automations/nudge_pusher_test.rb` | 4 | 0 |
| `test/models/saved_item/reminder_pusher_test.rb` | 4 | 0 |
| `test/lib/web_push/persistent_request_test.rb` | 1 | 1 |
| `test/models/dnd_allowed_user_test.rb` | 2 | 0 |
| `test/system/meeting_status_test.rb` | 0 | 2 |
| `test/system/service_worker_test.rb` | 0 | 2 |
| `test/system/out_of_office_test.rb` | 0 | 1 |

## Every deferred exact title and owner

| Rails file | Exact title | Owner |
| --- | --- | --- |
| `test/models/notifications/policy_test.rb` | a room mention records and pushes for a mentions member | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a room mention still records with notifications off but sends no push | WS17 continuation |
| `test/models/notifications/policy_test.rb` | an invisible room membership gets nothing at all | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a room reply records and pushes for mentions and everything members | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a room reply stays silent with notifications off | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a plain room message pushes everything followers without an inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a plain room message does nothing for mentions members | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a room keyword match records without pushing for mentions and notifications-off members | WS17 continuation |
| `test/models/notifications/policy_test.rb` | an everything member's keyword match still pushes as a broadcast | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a room message without a membership records nothing, not even mentions or keywords | WS17 continuation |
| `test/models/notifications/policy_test.rb` | mention beats reply beats keyword for one room message | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a followed thread records activity and pushes | WS17 continuation |
| `test/models/notifications/policy_test.rb` | an unfollowed thread stays silent for plain messages | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a thread mention records and pushes for mentions and everything members | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a muted thread gets nothing, not even mentions or keywords | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a thread reply records and pushes for followers only | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a thread keyword match records for unfollowed members without pushing | WS17 continuation |
| `test/models/notifications/policy_test.rb` | room notifications off suppresses thread activity but not keywords | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a non-member of the thread gets nothing | WS17 continuation |
| `test/models/notifications/policy_test.rb` | bots and deactivated recipients record nothing | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a muted room mention records and pushes | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a muted room keyword match records without pushing | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a muted room reply stays silent | WS17 continuation |
| `test/models/notifications/policy_test.rb` | muted room thread activity stays silent for followers | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a muted board post stays silent for members outside the thread | WS17 continuation |
| `test/models/notifications/policy_test.rb` | DND silences a muted room mention push but keeps the inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | manual DND suppresses push and sound but still records the inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a starred sender still pushes through DND | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a preloaded DND exception decides without another lookup | WS17 continuation |
| `test/models/notifications/policy_test.rb` | quiet hours suppress push inside the window only | WS17 continuation |
| `test/models/notifications/policy_test.rb` | quiet hours follow the recipient's time zone | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a starred sender still pushes through quiet hours | WS17 continuation |
| `test/models/notifications/policy_test.rb` | reminders push unless DND is on, and carry no sender exception | WS17 continuation |
| `test/models/notifications/policy_test.rb` | huddle invitations push unless DND is on without a starred caller | WS17 continuation |
| `test/models/notifications/policy_test.rb` | huddle join notices push for live memberships and record no inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | huddle join notices stay silent when muted, off, hidden, or no membership | WS17 continuation |
| `test/models/notifications/policy_test.rb` | huddle join notices honor DND with a starred-caller exception | WS17 continuation |
| `test/models/notifications/policy_test.rb` | huddle join notices stay silent during meetings and out of office | WS17 continuation |
| `test/models/notifications/policy_test.rb` | quiet-during-meetings suppresses push and sound but still records the inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | quiet-during-meetings pushes outside busy intervals | WS17 continuation |
| `test/models/notifications/policy_test.rb` | quiet-during-meetings needs meeting status on | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a starred sender still pushes through quiet-during-meetings | WS17 continuation |
| `test/models/notifications/policy_test.rb` | reminders and huddles stay silent during meetings with no sender exception | WS17 continuation |
| `test/models/notifications/policy_test.rb` | out of office suppresses push and sound but still records the inbox item | WS17 continuation |
| `test/models/notifications/policy_test.rb` | out of office pushes when the member keeps notifications on | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a starred sender still pushes through out of office | WS17 continuation |
| `test/models/notifications/policy_test.rb` | reminders and huddles stay silent during out of office | WS17 continuation |
| `test/models/notifications/policy_test.rb` | calendar out of office quiets like a manual one | WS17 continuation |
| `test/models/notifications/policy_test.rb` | an expired out of office pushes again | WS17 continuation |
| `test/models/notifications/policy_test.rb` | a missing recipient pushes nothing | WS17 continuation |
| `test/models/notifications/policy_test.rb` | an unknown kind raises | WS17 continuation |
| `test/models/notifications/policy_test.rb` | dnd exceptions load for a batch in one query | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | room push skips a DND recipient but the inbox item is still recorded | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | room push skips a DND-presence recipient but the inbox item is still recorded | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | room push still reaches a starred sender's recipient during DND | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | room push skips a recipient inside quiet hours | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | thread push notifies followers with the thread payload | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | thread push skips a DND follower | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | a thread reply pushes its follower author but not an unfollowed one | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | reminder push skips a DND attendee | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | huddle push honors DND with a starred-caller exception | WS17 continuation |
| `test/models/notifications/push_gating_test.rb` | group huddle push skips DND and quiet-hours recipients but their missed calls are still recorded | WS17 continuation |
| `test/models/room/push_test.rb` | a forwarded note follows the mention push path while its snapshot does not | WS17 continuation |
| `test/models/push/subscription_test.rb` | rejects endpoint that resolves to loopback IP | WS17 continuation |
| `test/models/push/subscription_test.rb` | rejects endpoint that resolves to link-local IP (AWS IMDS) | WS17 continuation |
| `test/models/push/subscription_test.rb` | rejects endpoint whose host resolves to nothing without raising | WS17 continuation |
| `test/models/push/subscription_test.rb` | endpoint resolution is deferred from the enqueue path to the delivery worker | WS17 continuation |
| `test/models/push/subscription_test.rb` | delivery is skipped when the endpoint no longer resolves to a public IP | WS17 continuation |
| `test/lib/web_push/persistent_request_test.rb` | ignores proxy env so the pin can't be routed through a re-resolving proxy | WS17 continuation |
| `test/models/user/status_settings_test.rb` | DND is manual-only outside quiet hours | WS17 continuation |
| `test/models/user/status_settings_test.rb` | quiet hours cover an overnight window in the user's time zone | WS17 continuation |
| `test/models/user/status_settings_test.rb` | an expired custom status reads as blank | WS17 continuation |
| `test/models/user/status_settings_test.rb` | effective presence folds the manual setting over the lease state | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | meeting status and quiet-during-meetings default off | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | in_meeting? needs the opt-in and a covering interval | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | in_meeting? is false without a cache row | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | the meeting label shows while in a meeting | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | a custom status wins over the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | an expired custom status yields to the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | manual DND wins over the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | the DND presence wins over the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | quiet hours win over the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | invisible hides the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | quiet-during-meetings never suppresses the meeting label | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | quiet-during-meetings only works while meeting status is on | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | quiet-during-meetings applies through a custom status | WS17 continuation |
| `test/models/user/meeting_status_test.rb` | quiet-during-meetings is off outside busy intervals | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | out of office defaults off | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | a manual OOO is active until its end, then reads as off | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | setting an OOO end in the past is invalid, but an expired end left behind still saves | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | a note longer than 140 characters is invalid | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | the status line names the return date and the note | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | the return date renders in the OOO member's own zone | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | OOO wins over a custom status, DND, and the meeting label | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | invisible hides the OOO label but OOO still reads as active | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | OOO quiet never suppresses the OOO label | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | calendar OOO needs the opt-in and a covering interval | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | a calendar OOO outside its intervals reads as off | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | overlapping manual and calendar OOO show the later end | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | the note shows only while the manual OOO is active | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | OOO presets run to the end of the day in the member's zone | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | the Monday preset is a week out on Mondays | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | the custom preset parses a datetime-local value in the member's zone | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | an unknown preset raises | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | claim_ooo_broadcast! wins the first claim and each flip, and loses re-runs | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | claiming an end clears the expired manual columns | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | claiming an end keeps a manual OOO set racing the sweep | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | OOO quiets notifications unless the member keeps them on | WS17 continuation |
| `test/models/user/out_of_office_test.rb` | deactivating clears the manual OOO columns | WS17 continuation |
| `test/models/calendar/meeting_cache_test.rb` | one cache per user | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/jobs/huddle/push_invitation_job_test.rb` | pushes the invitation to the recipient only | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | an opted-out recipient gets no push subscriptions | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | a connected recipient gets no push | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/jobs/huddle/push_invitation_job_test.rb` | missing invitations are ignored | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/system/service_worker_test.rb` | the worker caches static assets and never authenticated responses | WS17 continuation |
| `test/system/service_worker_test.rb` | the offline shell renders with working retry behavior | WS17 continuation |
| `test/system/status_notifications_test.rb` | setting presence and a custom status | WS17 continuation |
| `test/system/status_notifications_test.rb` | enabling DND mutes sounds and persists quiet hours | WS17 continuation |
| `test/system/status_notifications_test.rb` | chat sounds follow the live quiet-hours window without a reload | WS17 continuation |
| `test/system/status_notifications_test.rb` | switching the theme applies without a reload flash | WS17 continuation |
| `test/system/status_notifications_test.rb` | switching the text size rescales the page | WS17 continuation |
| `test/system/status_notifications_test.rb` | button icons follow the manual theme, not the OS | WS17 continuation |
| `test/system/status_notifications_test.rb` | the status form works at phone width | WS17 continuation |
| `test/system/meeting_status_test.rb` | opting in shows In a meeting for a stubbed busy interval, then clears after it ends | WS17 continuation |
| `test/system/meeting_status_test.rb` | the profile links to connect without a Google account | WS17 continuation |
| `test/system/out_of_office_test.rb` | set OOO until tomorrow, badge and DM notice show for another user, then clear it | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | a DM with an OOO member shows the notice above the composer | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | the notice escapes the member's note | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | the notice renders per viewer, never from a shared fragment | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | a group DM shows one line per OOO recipient | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | a channel shows no notice even while a member is out | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | a DM with nobody out shows no notice | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | an invisible member's manual OOO shows no notice | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | an invisible member's calendar OOO shows no notice | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | an invisible member's OOO flip broadcasts an emptied notice line | WS17 continuation |
| `test/integration/ooo_dm_notice_test.rb` | an OOO end broadcasts an emptied notice line | WS17 continuation |
| `test/models/notifications/keyword_matcher_test.rb` | treats phrases literally, not as patterns | WS17 continuation |
| `test/models/notifications/keyword_matcher_test.rb` | ignores blank phrases and blank text | WS17 continuation |
| `test/models/notifications/keyword_matcher_test.rb` | a phrase matches across a line break and not inside a longer Unicode word | WS17 continuation |
| `test/services/activity_items/recorder_test.rb` | work events notify followed thread members | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no work items but a mentions member does | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a status update after a work assignment keeps the assignment item and repoints the update item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work updates for one thread collapse into a single item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by an agent honors the recipient's agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by a bot without an agent ignores the agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a caller-authorized record skips the source check but keeps idempotency | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
