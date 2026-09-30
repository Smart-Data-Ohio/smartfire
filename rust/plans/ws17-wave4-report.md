# WS17 wave 4 report — PARTIAL

Worktree `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws17`, branch `rust/ws17-push-presence`, Rails pin `d7c7de92`.

Merged `origin/main` `21a7332f2d3c324f0862cdf448baf17a84395aa0` with merge commit `56aa9f62ffbf3641b3ddd944d3da95a30407c654` and pushed it. Locked metadata succeeds, workspace dependencies have no duplicate keys. The full seeded app suite now has **zero failures**. Main's three explicit ignores remain: the cable reference recorder, latency measurement, and `manages_bots` pending WS11. No new ignores, seed fallbacks, masks, or allowlist changes were introduced.

The next coherent slice adds settings writers and DND allowances. This report is committed with that slice; its pushed SHA is in the delivery reply. Prior pushed implementation commits are `7b9267a3c948cca3fdf9a8c6946a2e643fb10d50`, `32b49aa4118c7ecb57f18fa14d2ccc0204acefe7`; prior report commit `aab111036aec6dc51a96d98d25126b3636032ad5`.

## Done and boundaries

- Notification policy: separate push/sound/inbox decisions for five kinds, 2,240 actual Rails vectors; batched DND sender allowances distinct from user stars. Room recipient merge, visibility, reply opt-in, and deduplication; exact untruncated room/thread payloads, tagged saved reminders. Thread and saved reminder durable handlers deliver through the existing bounded encrypted pool.
- Web Push: Smartfire tag and VAPID subject, encrypted reference golden, IP-pinned guarded transport, exact Rails invalidation behavior (410 and TLS/key failures invalidate; 404 is retained). Test-notification title/tag corrected; execution remains inline pending the next slice.
- Status/presence: batched user/cache readers with 161 actual Rails vectors; read-only expiration, DST/quiet windows, meeting/OOO precedence; 90-second leases, 10-minute idle, bounded minute pruning; authenticated presence endpoint with 12 full Rails response goldens and 35 Ruby Integer vectors. PWA and offline shell bytes compare to Rails and the unchanged harness previously passed.
- New settings writers: changed-column persistence and timestamps, strict presence/theme/text-size/time-zone validation, custom-status lengths/expiry presets, OOO note/end validation and presets, quiet-hour virtual setters and validation, DND timer reconciliation. Actual Rails generation covers 200 preset cases, 14 clock setters, 15 error vectors and 636 legal zone names. These are domain writers; status/notification controllers and forms are still outstanding.
- Keyword **settings** replacement strips, dedupes, caps, preserves Rails validation messages, and rolls back both list and settings even when the caller rescues. This does not yet implement keyword activity recording.
- DND allowance model and authenticated POST/DELETE controllers: current-user scope, active human lookup, self/bot/missing rejection, idempotent lookup/create and scoped removal, concurrent HTTP requests result in one row. User stars do not provide DND exceptions. User deactivation clears manual OOO columns in the existing transaction; other WS11 deactivation callbacks are outside this slice.

New/changed writer files: `rust/crates/db/src/models/user_status_settings{.rs,/writes.rs}`, `dnd_allowed_user.rs`, `user.rs`, `models.rs`, `slash_commands/time_parser.rs`; `tests/status_settings_write_test.rs` and two settings JSON fixtures; `campfire/src/controllers/users/dnd_allowances.rs` and controller registration; `reference-tools/ws17_{settings_vectors.rb,regenerate_settings.py,verify_reference.py,workspace_dependencies.py,injections.py}`. The source verifier checks 29 owned Rails files against the pin. The custom OOO parser reuses the existing Rails TimeZone#parse equivalent, rather than slash-command relative-time parsing. No schema, migration, Rails, dependency or lockfile changes.

## WS13 transport seam

Available now, with these exact public signatures:

```rust
// campfire_db::models::push_subscription::PushPayload
pub fn new(title: String, body: String, path: String, tag: Option<String>) -> Self;
// campfire::integrations::web_push::Pool
pub fn queue(&self, conn: &rusqlite::Connection, payload: &PushPayload,
             subscriptions: Vec<PushSubscription>) -> campfire_db::Result<()>;
pub fn deliver_later(&self, notification: Notification);
// UserStatusSettings
pub fn for_ids(conn: &rusqlite::Connection, ids: &[i64])
    -> campfire_db::Result<std::collections::HashMap<i64, Self>>;
```

WS13 builds invitation/join `PushPayload` and determines candidate recipient IDs. WS17 owns the policy gate and pool delivery. Before `Pool::queue`, preload `UserStatusSettings::for_ids` once for candidate IDs; evaluate `NotificationPolicy` with `NotificationKind::Huddle`, current time and `dnd_exceptions_for(conn, &ids, Some(sender_id))`. Pool itself does not apply huddle policy. Queue receives already permitted subscriptions, reads fresh unread badges, and schedules delivery in its bounded pool; delivery uses current VAPID config and pinned endpoint IP. `PushPayload::new` preserves strings verbatim, including the explicit tag; no automatic truncation. Dedicated durable huddle job adapters and their DTO seam are **still outstanding**, so this is the shared transport seam only. Room/thread/saved handlers demonstrate use; claims/enqueue must remain in the triggering transaction.

## Validation rerun for this slice

All commands below were rerun from the worktree root. Rust toolchain 1.98.1, locked Cargo, at most four build/test threads. Seeded app tests use both existing pinned seeds with `CAMPFIRE_TEST_REQUIRE_SEED=1`; none silently skip. Only ports 52400–52499 are used. Mutation runner restores files in `finally` before final tests.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml > /dev/null
python3 rust/reference-tools/ws17_workspace_dependencies.py
```

Raw: metadata exit 0, no stdout; `workspace dependency keys: 75 unique; 0 duplicates`.

```bash
python3 rust/reference-tools/ws17_regenerate_settings.py > .scratch/settings-regenerate-final.log 2>&1
```

```text
pinned Rails source verified: 29 files match d7c7de92
Rails settings vectors: 200 presets; 14 clock setters; 15 validations; 636 legal zone names
```

```bash
python3 rust/reference-tools/ws17_injections.py settings- > .scratch/settings-injections-final.log 2>&1
```

```text
settings-validation: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 434 filtered out; finished in 0.07s
settings-keyword-rollback: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 434 filtered out; finished in 0.07s
settings-active-allowance: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 434 filtered out; finished in 0.08s
settings-dnd-timer: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 434 filtered out; finished in 0.07s
```

The four intentional failures discriminate missing validation, non-atomic keyword replacement, invalid allowance targets and clearing a live DND timer. Their failures are expected; runner exit 0, source restored. The allowance HTTP test also failed before route registration (one failure) and passed after implementation.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -- --test-threads=4 > .scratch/settings-db-jobs-final.log 2>&1
```

```text
test result: ok. 432 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 45.09s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

DB's three existing reference/export tests remain explicitly ignored because they need their special oracle/export environment. Both queue crash/restart integration tests ran. Settings-writer module has 16 passing tests in this full run.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 > .scratch/settings-app-final.log 2>&1
```

```text
test result: ok. 315 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 27.98s
```

Three new allowance HTTP tests passed in this full run. Main merge alone had 312 passes and zero failures; the current run includes the three added tests.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/settings-clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.44s
```

No warning suppression or crate exclusions. Broader workspace tests, full DB differential/rollback, browser screenshots and browser interaction scenarios were not run in this slice.

## Precisely remaining, in the requested order

1. Finish status/notification controllers and byte-identical owned HTML, status clear/errors and unsaved values, cache opt-in/opt-out reconciliation and refresh job seam, time-zone/theme/text-size profile writes, push-subscription named scenarios and byte checks, durable test-notification job plus HTTP rollback/readback test. Writers and allowance controllers above are delivered, but step 1 is **partial**.
2. Keyword-alert activity recording: wire matcher winner priorities and minimal transactional activity writer if WS12's recorder has not landed; record active humans only, no muted-room leaks or DND suppression of inbox.
3. Meeting/OOO atomic claims and dispatcher sweeps/broadcasts; status badges, sidebar/DM updates and OOO DM notice HTML; periodic tasks/opt-out/inactive/malformed-cache/concurrency scenarios. WS14 owns Google/cache feeds; this slice adds no Google fetch.
4. Event, board nudge and huddle policy pushers and durable handlers, transactional claim/enqueue, exact payload vectors/seams and rollback/restart checks; room job audit. WS14 owns event source, WS12 board source, WS13 huddle payloads/source. Dedicated huddle DTO signature must be documented when delivered.
5. Remaining exact named scenario replays below, including browser system cases and dispatcher/controller jobs; group by source file with pass counts. Pure-vector coverage does not automatically count every exact scenario as ported.

## Named Rails scenario inventory, grouped by file

Machine-readable exact titles, status, owner and Rust evidence: `rust/plans/ws17-rails-test-inventory.json`. 347 selected Rails titles: 75 equivalent scenarios rerun, 272 deferred. Counts below refer to named Rails scenarios, not Rust harness test counts. Concurrent allowance HTTP succeeds, but the deterministic simulated unique-index-loser Rails scenario remains deferred.

| Rails file | Passed equivalent | Deferred |
| --- | ---: | ---: |
| `test/channels/workspace_presence_channel_test.rb` | 7 | 0 |
| `test/controllers/users/dnd_allowances_controller_test.rb` | 3 | 2 |
| `test/controllers/users/notification_settings_controller_test.rb` | 0 | 10 |
| `test/controllers/users/presences_controller_test.rb` | 7 | 0 |
| `test/controllers/users/push_subscriptions_controller_test.rb` | 0 | 6 |
| `test/controllers/users/statuses_controller_test.rb` | 0 | 22 |
| `test/integration/ooo_dm_notice_test.rb` | 0 | 10 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 0 | 4 |
| `test/lib/web_push/persistent_request_test.rb` | 1 | 1 |
| `test/models/board_automations/nudge_pusher_test.rb` | 0 | 4 |
| `test/models/calendar/meeting_cache_test.rb` | 0 | 13 |
| `test/models/calendar/meeting_dispatcher_test.rb` | 0 | 11 |
| `test/models/calendar/ooo_dispatcher_test.rb` | 0 | 12 |
| `test/models/dnd_allowed_user_test.rb` | 2 | 0 |
| `test/models/event/reminder_pusher_test.rb` | 0 | 10 |
| `test/models/huddle/join_pusher_test.rb` | 0 | 13 |
| `test/models/notifications/keyword_matcher_test.rb` | 7 | 3 |
| `test/models/notifications/policy_test.rb` | 0 | 52 |
| `test/models/notifications/push_gating_test.rb` | 0 | 10 |
| `test/models/push/subscription_test.rb` | 13 | 5 |
| `test/models/room/push_test.rb` | 7 | 1 |
| `test/models/saved_item/reminder_pusher_test.rb` | 4 | 0 |
| `test/models/user/meeting_status_test.rb` | 0 | 14 |
| `test/models/user/out_of_office_test.rb` | 0 | 22 |
| `test/models/user/status_settings_test.rb` | 10 | 4 |
| `test/models/workspace_presence_lease_test.rb` | 14 | 0 |
| `test/services/activity_items/recorder_keyword_test.rb` | 0 | 11 |
| `test/services/activity_items/recorder_test.rb` | 0 | 20 |
| `test/system/meeting_status_test.rb` | 0 | 2 |
| `test/system/out_of_office_test.rb` | 0 | 1 |
| `test/system/service_worker_test.rb` | 0 | 2 |
| `test/system/status_notifications_test.rb` | 0 | 7 |

## Every deferred title and owner

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
| `test/controllers/users/statuses_controller_test.rb` | updates presence and the custom status with an expiry | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | clears the custom status | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | rejects an unknown presence with errors | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | requires sign-in | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting into meeting status enqueues a first refresh | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out of meeting status drops the cached intervals | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out while in a meeting broadcasts the cleared badge | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out without cached intervals broadcasts nothing | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | saving other status settings leaves meeting refreshes alone | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | sets out of office with a preset and a note, and broadcasts it | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | sets out of office with a custom date and time in the member's zone | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | rejects an unknown OOO preset without saving anything | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | rejects a blank or past custom OOO end without saving anything | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | rejects an OOO note over 140 characters | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | clears out of office early and broadcasts the cleared state | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | clearing early ends only the manual OOO while calendar OOO covers | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | edits the OOO note alone | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting into calendar OOO enqueues a first refresh | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out of calendar OOO clears its intervals and broadcasts | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out of calendar OOO keeps the row while meeting status is on | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | opting out of meeting status keeps the row while calendar OOO is on | WS17 continuation |
| `test/controllers/users/statuses_controller_test.rb` | saving other status settings leaves calendar OOO refreshes alone | WS17 continuation |
| `test/controllers/users/dnd_allowances_controller_test.rb` | starring twice stays a single exception | WS17 continuation |
| `test/controllers/users/dnd_allowances_controller_test.rb` | a concurrent star reports success instead of an error | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | enables DND with quiet hours and keywords | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | disables DND and clears keywords | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | enables quiet-during-meetings | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | toggles keep-notifying while out of office, defaulting to off | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | quiet hours without a window render errors | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | a failed save keeps the previous keywords | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | requires sign-in | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | enabling DND after a timed expiry starts it indefinitely | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | disabling DND clears a running timer | WS17 continuation |
| `test/controllers/users/notification_settings_controller_test.rb` | saving settings preserves a running DND timer | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | create new push subscription | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | touch existing subscription | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | rejects subscription with non-permitted endpoint | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | rejects subscription with endpoint resolving to a private IP | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | re-registering a legacy invalid subscription is rejected with 422 | WS17 continuation |
| `test/controllers/users/push_subscriptions_controller_test.rb` | destroy a push subscription via dev mode | WS17 continuation |
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
