# WS17 Wave 4 — partial, continuing in pushed slices

Reference Rails `d7c7de92`; merged main `21a7332f` with merge commit `56aa9f62`. Branch `rust/ws17-push-presence`. This report accompanies the current slice; the delivery reply gives the pushed SHA. The tracked report is the exact mirror of the requested external report.

## Delivered

Earlier pushed slices (`7b9267a3`, `32b49aa4`, `76c34826`, `0adcae95`, `12c448d7`) provide typed notification policy, tagged Web Push with Smartfire subject and IP pinning, durable room/thread/saved/test push transport, presence leases/HTTP/pruner, validated dirty-tracked settings writes, status and notification PATCH, DND allowances, keyword-list replacement, cache reconciliation, manual OOO claims, and complete badge/OOO broadcast HTML. PWA worker/offline bytes are pinned. Existing Rails oracle vectors remain exercised by the full suites below.

Current profile/subscription/allowance slice:

| Files | Change and verification |
| --- | --- |
| `campfire/controllers/users/profiles.rs`, `profiles/ws17_tests.rs` | Persist theme, text size and time zone through the existing validated settings writer, mark an explicitly submitted zone (including blank) as chosen, atomically roll back other submitted fields on invalid appearance. Preserve unsaved form values. Profile errors populate the profile path and do not inherit the settings-controller missing-device failure. Seven seeded HTTP tests cover writes, invalid rollback, IANA/legacy selection and actual layout sound markers. |
| `views/users/settings.rs`, `profile_zones.json`, `templates/users/profiles/_appearance.html`, profile show; `presenters/status_settings.rs` | Mechanical appearance form port, error wrappers and escaping. Six complete form strings match actual Rails. All 135 friendly zone choices retain TZInfo identifiers and current base-offset labels using pinned TZInfo transitions; tests caught Casablanca's negative-DST distinction. No view queries. |
| `db/models/user_status_settings.rs`, `presenters/view_context.rs` | Preload actual DND, quiet-hour, meeting and OOO windows into layout preferences. Current and future windows stay in source order; opt-outs remove only their owned marker. |
| `controllers/users/push_subscriptions.rs`, `push_subscriptions/ws17_tests.rs`; `app.rs`, test support | All six named subscription HTTP scenarios, including legacy revalidation and private-IP refusal. Per-app DNS dependency permits deterministic DNS answers while testing the real endpoint guard/model/HTTP stack. Production uses system DNS. Current-user deletion scope checked additionally. Real user-agent rendering matches four Rails cases. |
| `views/templates/users/push_subscriptions/index.html`, `tests/ws17_settings.rs` | Complete subscription content byte comparison, including full row forms, fixed shared test CSRF values, actual asset URLs, escaping and whitespace. |
| `controllers/users/dnd_allowances.rs` | Repeated star remains one row; deterministic real UNIQUE-index failure at insert follows Rails' success redirect, in addition to existing concurrent HTTP requests. No mocks of the writer. |
| `reference-tools/ws17_profile_ui.rb`, regeneration script, verifier and injection runner | Actual pinned source/output verification; no Rails changes, output masks, allowlist changes or new ignores. |

User/profile security, GitHub/inbox/voice settings and connected-service UI belong to WS9/WS11/WS12/WS13/WS14/WS15. The existing basic profile update path still needs those owners' callbacks. This slice adds only the owned appearance attributes, without claiming whole-profile parity.

The pin's invalid status/notification render on a seeded confirmed-2FA user still produces HTTP 500 because Rails omits `@two_factor_devices`. Separate tests preserve that observed behavior; the named fixture replays explicitly use unconfirmed-credential fixture state and expect 422. Profile PATCH initializes those devices in Rails and its owned validation response is 422.

`Calendar::MeetingRefreshJob { user_id: i64 }` is durable on the default queue, version 1, JSON `{ "user_id": ... }`. WS14 must register its fetch handler; until then it fails visibly as an unknown handler. Both opt-ins enqueue separately. No Google refresh success is faked. Typed status badge/OOO facts are emitted after commit and rendered in the cable sink. Rails emits events in order, but its worker pool delivers independent stream callbacks asynchronously; socket tests compare exact bytes/counts/order within each signed stream, while a separate domain test checks actual cross-stream event emission order.

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

Keep Rails' pusher scopes before delivery: visible/disconnected memberships, invitations exclude `nothing`, joins exclude `nothing` and `muted`; SQL-null exclusions follow Rails SQL rather than adding eligibility. Join also checks the huddle inbox preference and claims its ten-minute throttle only after policy and subscriptions permit an actual push. The dedicated durable huddle adapter/DTO and throttle-claim seam are **still outstanding**; the signatures above are the shared transport, not a claim that huddle push jobs are finished. WS13 owns payload/source construction, WS17 the gate/transport. `PushPayload::new` preserves supplied strings/tag without automatic truncation. Pool reads fresh badges and delivers through current VAPID and stored pinned endpoint IP. Preserve transactional claim/enqueue when connecting the source.


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

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -p campfire_views -- --test-threads=4`

```text
test result: ok. 436 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 37.73s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.92s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -- --test-threads=4`

```text
test result: ok. 380 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 29.94s
```

`mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.69s
```

The pre-change appearance HTTP test failed on persisted `system` versus submitted `dark`: `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 370 filtered out; finished in 0.48s`. Complete form comparison also failed before correcting dynamic zone labels; subscription content failed before correcting its exact leading/collection whitespace. The four committed injections remove the theme save, accept private DNS, remove unique-index rescue, and erase sound metadata; each must produce an actual failed test, not a compiler error. Sources are restored by the runner before final suites.

## Precisely remaining

1. Profile/UI integration stays partial: Smartfire's complete profile contains other owners' sections; sidebar/DM presence composition must connect to WS8b/WS13's full room templates and request-free broadcasts. Badge and OOO notice-line bytes and transport are already ported. Full page/browser/pixel parity and Rails rollback/readback rehearsal have not run. Appearance, subscription and allowance gaps above are closed.
2. Keyword-alert activity recording (priority, active-human/membership/thread gating, grouping/idempotency, callbacks) is next; main has only the matcher and reminder inbox writer. WS12 owns full source/render/access rules.
3. Meeting/OOO due sweeps, meeting conditional claims, stale refresh deduplication, periodic registration, expiration cleanup and concurrent dispatcher tests remain. WS14 owns Google execution at the documented job seam.
4. Event, board and huddle pushers/jobs, durable huddle DTO/throttle claim, recipient/source payload vectors and atomic claim/enqueue tests remain. WS12/14/13 supply sources/payloads; WS17 supplies policy and transport. Room handler audit remains.
5. Remaining exact named scenarios below, largest files first; pure vector coverage is not claimed as a replay of every named sequence.

## Named scenario coverage by file

347 selected exact Rails titles: **115 passed equivalent; 232 deferred**. `rust/plans/ws17-rails-test-inventory.json` records exact title, owner, status and Rust evidence. Additional profile/UI HTTP cases are outside this pre-existing selected inventory.

| Rails file | Passed equivalent | Deferred |
| --- | ---: | ---: |
| `test/models/notifications/policy_test.rb` | 0 | 52 |
| `test/controllers/users/statuses_controller_test.rb` | 22 | 0 |
| `test/models/user/out_of_office_test.rb` | 0 | 22 |
| `test/services/activity_items/recorder_test.rb` | 0 | 20 |
| `test/models/push/subscription_test.rb` | 13 | 5 |
| `test/models/user/meeting_status_test.rb` | 0 | 14 |
| `test/models/user/status_settings_test.rb` | 10 | 4 |
| `test/models/workspace_presence_lease_test.rb` | 14 | 0 |
| `test/models/calendar/meeting_cache_test.rb` | 0 | 13 |
| `test/models/huddle/join_pusher_test.rb` | 0 | 13 |
| `test/models/calendar/ooo_dispatcher_test.rb` | 0 | 12 |
| `test/models/calendar/meeting_dispatcher_test.rb` | 0 | 11 |
| `test/services/activity_items/recorder_keyword_test.rb` | 0 | 11 |
| `test/controllers/users/notification_settings_controller_test.rb` | 10 | 0 |
| `test/integration/ooo_dm_notice_test.rb` | 0 | 10 |
| `test/models/event/reminder_pusher_test.rb` | 0 | 10 |
| `test/models/notifications/keyword_matcher_test.rb` | 7 | 3 |
| `test/models/notifications/push_gating_test.rb` | 0 | 10 |
| `test/models/room/push_test.rb` | 7 | 1 |
| `test/channels/workspace_presence_channel_test.rb` | 7 | 0 |
| `test/controllers/users/presences_controller_test.rb` | 7 | 0 |
| `test/system/status_notifications_test.rb` | 0 | 7 |
| `test/controllers/users/push_subscriptions_controller_test.rb` | 6 | 0 |
| `test/controllers/users/dnd_allowances_controller_test.rb` | 5 | 0 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 0 | 4 |
| `test/models/board_automations/nudge_pusher_test.rb` | 0 | 4 |
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
| `test/models/calendar/meeting_cache_test.rb` | in_meeting? is true inside an interval, with an inclusive start and exclusive end | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | in_meeting? is false without intervals | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | in_meeting? ignores malformed pairs instead of raising | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | quiet_window_epochs returns epoch windows and skips malformed pairs | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | in_ooo? is true inside an OOO interval, with an inclusive start and exclusive end | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | in_ooo? reads only the OOO intervals, not the busy ones | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | ooo_end_covering returns the latest covering end | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | ooo_end_covering is nil while uncovered | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | ooo_window_epochs returns epoch windows and skips malformed pairs | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | claim_broadcast! wins the first claim and each flip, and loses re-runs | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | claim_broadcast! lets only one concurrent claimant win | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | one cache per user | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_cache_test.rb` | the cache row references its member with a cascading foreign key | WS17 continuation (claims/readers); WS14 (cache persistence/feed) |
| `test/models/calendar/meeting_dispatcher_test.rb` | a meeting start broadcasts the badge | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a re-run with no flip broadcasts nothing | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a steady-state tick issues no claim write | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a meeting end broadcasts the badge | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a broadcast carries the meeting label | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a stale cache enqueues a refresh | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a missing cache enqueues a refresh without broadcasting | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | a fresh cache enqueues nothing | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | members who never opted in are ignored | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | deactivated members are ignored | WS17 continuation |
| `test/models/calendar/meeting_dispatcher_test.rb` | one failing member does not stop the sweep | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a manual OOO start broadcasts the badge and the DM notice | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a re-run with no flip broadcasts nothing | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a steady-state tick issues no claim write | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | an OOO end broadcasts and clears the manual columns | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | the broadcasts carry the OOO label, the note, and the return date | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a calendar OOO start broadcasts the badge and the notice | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a stale OOO-only cache enqueues a refresh | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a member with both opt-ins refreshes through the meeting dispatcher only | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | a missing cache enqueues a refresh without broadcasting | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | members with neither a manual OOO nor the calendar opt-in are ignored | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | deactivated members are ignored | WS17 continuation |
| `test/models/calendar/ooo_dispatcher_test.rb` | one failing member does not stop the sweep | WS17 continuation |
| `test/models/event/reminder_pusher_test.rb` | pushes the reminder to going and maybe attendees who are still members | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | push reminders ignore the event_reminders inbox switch | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | the push body names the venue | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | a direct room reminder is titled by the organizer | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | the push body counts down the actual minutes | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | the push body uses the singular minute | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | an event starting now says so | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | a recently started event still says starting now | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | an event that already ended is skipped | WS17 continuation (push); WS14 (event source) |
| `test/models/event/reminder_pusher_test.rb` | an event that started long ago is skipped | WS17 continuation (push); WS14 (event source) |
| `test/models/board_automations/nudge_pusher_test.rb` | pushes the nudge payload to the recipient subscriptions | WS17 continuation (push); WS12 (board source) |
| `test/models/board_automations/nudge_pusher_test.rb` | escalations push with the escalated prefix | WS17 continuation (push); WS12 (board source) |
| `test/models/board_automations/nudge_pusher_test.rb` | skips a recipient who left the board | WS17 continuation (push); WS12 (board source) |
| `test/models/board_automations/nudge_pusher_test.rb` | dnd silences the push like other reminders | WS17 continuation (push); WS12 (board source) |
| `test/models/huddle/join_pusher_test.rb` | pushes the join to the recipient's subscriptions and stamps the throttle | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a second push inside ten minutes is throttled | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a push ten minutes later goes out again | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a DND recipient gets no push and burns no throttle window | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a starred joiner still pushes through DND | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a recipient in quiet hours gets no push | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a recipient quiet in a meeting gets no push | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | an out-of-office recipient gets no push unless they keep notifications on | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a connected recipient gets no push and burns no throttle window | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a switched-off or hidden room gets no push | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a muted room gets no push and burns no throttle window | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a recipient with huddle invitations switched off gets no push and burns no throttle window | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
| `test/models/huddle/join_pusher_test.rb` | a recipient with no subscriptions burns no throttle window | WS17 continuation (transport/policy/claims); WS13 (payload/source) |
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
| `test/services/activity_items/recorder_keyword_test.rb` | a keyword match on a room message records a keyword alert | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a keyword match needs a word boundary | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | the author never matches their own keywords | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | an invisible member matches nothing but a notifications-off member matches | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a mention wins over a keyword match for the same message | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a thread keyword match reaches thread members only | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a muted thread member matches no keywords | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | thread activity wins over a keyword match for a follower | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a room keyword match loads only the matching members' memberships | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | a room message keeps candidate queries flat as the roster grows | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_keyword_test.rb` | matching queries the keyword table a constant number of times as followers grow | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | records a mention for an opted-in active human and excludes the author | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a reply follows the reply author's current preference | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | mention takes precedence when one message matches multiple activity reasons | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | followed thread activity uses thread preferences | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work events notify followed thread members | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a direct mention reaches a member with notifications off but not an invisible one | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no reply but a mentions member does | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no thread activity but a mentions member does | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a member with notifications off gets no work items but a mentions member does | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | changing involvement leaves existing items untouched | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | ten thread replies collapse into one thread activity item that reads unread again | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a status update after a work assignment keeps the assignment item and repoints the update item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work updates for one thread collapse into a single item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a thread message that mentions and replies to a follower yields one mention item | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by an agent honors the recipient's agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | work assigned by a bot without an agent ignores the agent_work switch | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a caller-authorized record skips the source check but keeps idempotency | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | recording the same source twice is idempotent | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | recording a thread message queries memberships a constant number of times as followers grow | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
| `test/services/activity_items/recorder_test.rb` | a root message loads only the mentionee and reply-author memberships | WS17 continuation (keyword integration); WS12 (full recorder/lifecycle) |
