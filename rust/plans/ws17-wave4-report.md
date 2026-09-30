# WS17 wave 4 report — PARTIAL

Worktree `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws17`, branch `rust/ws17-push-presence`, Rails pin `d7c7de92`. Starting base `bb6c5d78`.

Merged `origin/main` `21a7332f2d3c324f0862cdf448baf17a84395aa0` with merge commit `56aa9f62ffbf3641b3ddd944d3da95a30407c654` and pushed it. Locked metadata succeeds; 75 workspace dependency keys are unique. The full seeded app suite now has **zero failures**: 334 passed, 3 explicitly ignored. Main's existing ignores are the cable reference recorder, latency measurement and `manages_bots` pending WS11. No new ignored tests, silent seed fallbacks, masks or allowlist changes.

This is a coherent partial delivery, not WS17 acceptance or cutover approval. Prior pushed implementation commits: `7b9267a3c948cca3fdf9a8c6946a2e643fb10d50`, `32b49aa4118c7ecb57f18fa14d2ccc0204acefe7`; prior report `aab111036aec6dc51a96d98d25126b3636032ad5`. The first continuation slice, settings writers and allowances, is pushed at `76c34826643f3850dcd3ba11b3a69aa9ad8f2819`. This report is committed with the next form/notification-controller/durable-test-push slice; its pushed SHA is in the delivery reply.

## What changed, by file and observable boundary

All paths are relative to this worktree. Rails source, schema, migrations, Cargo manifests and lockfile were not edited.

| Files | Delivered behavior and verification |
| --- | --- |
| `rust/crates/db/src/models/notification_policy.rs`, `models.rs` | Typed five-kind policy with separate push/sound/inbox decisions; real DND sender allowances distinct from user stars. 2,240 actual Rails decisions. Missing membership and present SQL-null involvement remain distinct. Keyword winner priority is a policy primitive; activity recording remains deferred. |
| `models/user_status_settings.rs` | Batched user/cache readers, read-only expiration, DND/quiet/meeting/OOO precedence, Rails/IANA zones and DST; 161 actual Rails decisions. Added style/time-zone fields for writes. No Google fetch or HTML dependencies. |
| `models/user_status_settings/writes.rs`, `slash_commands/time_parser.rs` | Loaded-instance dirty tracking, changed-column writes and timestamps, presence/theme/text-size/zone and length validations, custom-status expiry and OOO presets, quiet-hour parsing/validation, DND timer reconciliation. Strict 636-name Rails zone table. Unchanged expired OOO end can save, changed past end fails. OOO custom parse reuses the existing TimeZone#parse equivalent. 200 actual Rails preset cases, 14 clock setters, 15 validation vectors and a two-loaded-instance dirty-write oracle. |
| `models/dnd_allowed_user.rs`, `models/user.rs` | Allowance ownership, active-human/self/duplicate validation, idempotent create, scoped remove; deactivation clears manual OOO columns in the existing transaction. Other WS11 deactivation callbacks are outside this slice. |
| `models/workspace_presence_lease.rs` | Read-only online/idle batches; inclusive 90-second expiration, 10-minute idle, legacy null activity online, active session/identity joins, bounded pruning. Establish/heartbeat/delete lifecycle retained. |
| `models/push_subscription.rs`, `channel_thread.rs`, `saved_item.rs` | Exact tagged untruncated room/thread payloads and Rails' saved-reminder body truncation; merged/deduped eligible room scopes and opted-in reply authors; batch policy. Added serializable durable test-notification DTO. Keyword settings replacement is atomic; keyword activity recording is still missing. |
| `rust/crates/db/src/tests/status_settings_write_test.rs`, `tests.rs`, settings JSON fixtures | 17 new writer tests, presets and validations, independent allowances, timer and nested-savepoint rollback scenarios. Earlier policy/status/presence/push regressions rerun in the full DB suite. |
| `rust/crates/campfire/src/controllers/users/dnd_allowances.rs`, `controllers/users.rs`, `controllers.rs` | Authenticated current-user-scoped POST/DELETE; self 422, bot/inactive/missing 404; concurrent real HTTP requests leave one allowance. The deterministic simulated unique-index-loser Rails scenario remains deferred. |
| `controllers/users/notification_settings.rs` | Authenticated PATCH; permitted scalar assignments, Rails boolean/clock casts, timer rules, atomic list+settings save, exact validation response status and unsaved form values, old keyword list on failure. All ten named Rails controller scenarios plus five extra HTTP regressions pass. Twelve actual compound-parameter/list-writer vectors cover nil, booleans, numbers, arrays, hashes, nested arrays, escaping and control characters. SQL failure and nil boolean both roll back the list. |
| `controllers/presenters/status_settings.rs`, `controllers/presenters.rs`, `controllers/users/profiles.rs` | Gather plain view facts before rendering: settings, calendar configuration/account/scope/error, allowance people ordered by case-folded name, keywords ordered by phrase, OOO date in the user's zone and validation messages. Failed notification writes render the profile with submitted settings and rolled-back keyword rows. |
| `rust/crates/views/src/users/settings.rs`, `users.rs`, `templates/users/profiles/{_status,_notifications,show}.html`, `tests/ws17_settings.rs`, golden JSON | Mechanical ports of the two owned Rails forms: whitespace, attribute order, escaping, fields, error wrappers, calendar connection states, allowance buttons and OOO display. Twenty-four complete partial strings across twelve actual Rails states compare byte for byte. Integrated both forms into the profile. The full profile page still lacks unrelated upstream-to-Smartfire sections; whole-page byte parity is not claimed. Status form rendering is delivered, but its PATCH route is still unported (501). |
| `controllers/users/push_subscriptions/test_notifications.rs`, `jobs/notifications.rs` | Current-user-scoped test push persisted as `Push::Subscription::TestNotificationJob`; UUID body and absolute request URL captured at enqueue. Execution loads the owned subscription and fresh badge, then queues exact Smartfire Test/tag payload. Explicit test sends even in DND. The inline Rails endpoint has no job class: this Rust adapter is the brief's required durable execution, not a claimed Rails class. |
| `integrations/web_push.rs`, `pool.rs`, `tests.rs`, `config.rs`, encrypted golden | Smartfire tag and `mailto:support@smartdata.net`, retained bounded encryption/VAPID/IP-pinned transport. Actual Rails invalidation: 410 and TLS/key failures invalidate; 404 retained. Inline test helper now test-only; unused VAPID getter removed. Actual HTTP enqueue-failure rollback/readback and encrypted queued test delivery pass, along with durable thread/saved delivery and policy tests. |
| `controllers/users/presences.rs`, `jobs/periodic.rs`, `controllers/pwa.rs` | Authenticated presence batch endpoint: 35 Ruby Integer vectors and twelve complete Rails responses; read never prunes, periodic minute sweep does. Service-worker/offline-shell served bytes match the pin. Unchanged Node harness rerun against freshly saved Rust HTTP worker bytes. |
| `rust/reference-tools/ws17_*`, `rust/vectors/ws17_keyword_input.json`, `rust/plans/ws17-rails-test-inventory.json`, this report | Pin verification for 29 owned Rails files, reproducible actual Rails generators, failure-injection runner and dependency-key check. Exact named scenario ownership/evidence list and external report mirror. |

## Design notes and cross-workstream touches

Domain writers and policy know no HTML. View templates take plain structs, and controllers load all query data before rendering. Settings saves compare against originally loaded values, as Rails dirty tracking does, so two instances editing independent fields preserve both edits. The new actual Rails two-instance oracle failed before this correction and passes afterward. Notification replacement uses a savepoint around both the keyword list and settings; caller rescue still cannot commit a deleted old list. Owned status/style validations are ported; unrelated User validators/callbacks remain with their respective domains. The test push's durable row is inserted by the existing app sink on the same database writer transaction. Its triggering HTTP failure leaves neither a job nor changed subscription; no detached enqueue transaction. Queue crash/restart tests ran.

Per the brief, the test endpoint now redirects once enqueue commits; delivery errors follow asynchronous pool handling and invalidation. The pin performs that test delivery inline, so synchronous delivery failure timing is an explicit architectural change required by the durable-job instruction. Missing/deleted or newly non-owned subscriptions no-op when the job executes. No DND policy is applied to an explicit test push, matching the pin. UUID and path are frozen at enqueue; badge is fresh at execution.

The form adapter reads Google account connection/scope facts without reading tokens, mutating caches or fetching Google. WS14 supplies the real cache/feed and refresh handler; status controller opt-in/opt-out reconciliation is not yet delivered. The profile's appearance/preferences, auth/security and integration sections still require their owning workstreams; this is not full-page acceptance. WS12 owns user stars and the full activity recorder; DND exceptions remain separate. No implementation swarm, PR or production operation was performed.

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

## Validation — rerun commands and raw summaries

Run from the worktree root. Toolchain 1.98.1, locked Cargo and at most four build/test threads. Scratch/target are in this worktree; ports only 52400–52499. Both pinned parity seeds are present. Every new seeded test uses `expect`; app suite is run with `CAMPFIRE_TEST_REQUIRE_SEED=1`. The intentional missing-seed unit test may log a local skip, but no integration test silently skips. Source restored after mutation checks before final runs.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml > /dev/null
python3 rust/reference-tools/ws17_workspace_dependencies.py
```

Metadata exit 0, no stdout. Raw: `workspace dependency keys: 75 unique; 0 duplicates`.

```bash
python3 rust/reference-tools/ws17_regenerate_settings.py > .scratch/settings-regenerate-final.log 2>&1
```

```text
pinned Rails source verified: 29 files match d7c7de92
Rails settings vectors: 200 presets; 14 clock setters; 15 validations; 636 legal zone names; 1 dirty-write scenario
```

```bash
python3 rust/reference-tools/ws17_regenerate_settings_views.py > .scratch/settings-views-regenerate.log 2>&1
```

```text
pinned Rails source verified: 29 files match d7c7de92
Rails settings HTML: 12 states; 24 complete partials
```

```bash
python3 rust/reference-tools/ws17_regenerate_keyword_inputs.py > .scratch/keyword-input-regenerate.log 2>&1
```

```text
pinned Rails source verified: 29 files match d7c7de92
Rails keyword input: 12 parameter and writer scenarios
```

```bash
python3 rust/reference-tools/ws17_injections.py settings- > .scratch/settings-injections-final.log 2>&1
```

```text
settings-validation: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 435 filtered out; finished in 0.09s
settings-keyword-rollback: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 435 filtered out; finished in 0.09s
settings-active-allowance: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 435 filtered out; finished in 0.09s
settings-dnd-timer: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 435 filtered out; finished in 0.08s
settings-stale-write: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 435 filtered out; finished in 0.14s
```

```bash
python3 rust/reference-tools/ws17_injections.py next- > .scratch/settings-next-injections.log 2>&1
```

```text
next-form-bytes: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
next-controller-errors: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.39s
next-test-push-tag: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.47s
```

Eight intentional defects detected: validation removal, broken savepoint rollback, invalid allowance target, clearing a live timer, overwriting concurrent settings changes, wrong form bytes, wrong error response status, wrong queued test tag. Mutation runners exit 0, expected failing test summaries above, restored source. Before controller registration, failed-save HTTP test returned 501 instead of 422 and failed (one test); the initial view comparison failed on actual byte order/newline differences before fixing them. No compile failures are counted as discriminating tests.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -p campfire_views -- --test-threads=4 > .scratch/settings-next-domain-views-final.log 2>&1
```

```text
test result: ok. 433 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 43.80s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Summaries in order: DB, jobs unit, two crash/restart integrations, views unit, views core, owned settings HTML, then three doc-test groups. DB has three pre-existing special oracle/export ignores (`scenario_matches_ruby`, `fixtures_match_ruby_row_for_row`, `export_database_for_rails`). They need their dedicated environment and were not run. Twenty-four complete settings strings pass in the one owned-HTML harness test.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 > .scratch/settings-next-app-final.log 2>&1
```

```text
test result: ok. 334 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 29.94s
```

Zero seeded app failures, three main ignores as listed above. Full suite includes allowance, notification settings, actual enqueue-trigger rollback/readback and decrypted durable delivery. No full DB rollback/readback through Rails or cutover rehearsal is claimed.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::users::notification_settings -- --test-threads=4 > .scratch/notification-named-final.log 2>&1
```

```text
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 322 filtered out; finished in 0.89s
```

Named notification-controller file: all ten selected Rails scenarios pass, plus five extra HTTP regressions (SQL rollback, twelve parameter coercion cases, sorted/invalid keywords, nil boolean, required hash shape). Filtered-out count is intentional; full app run above ran separately.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_TEST_REQUIRE_SEED=1 CABLE_TEST_PORT_RANGE=52400-52499 MAIL_TEST_PORT_RANGE=52400-52499 WS17_SERVICE_WORKER_OUTPUT="$PWD/.scratch/service-worker-served.js" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire ws17_service_worker_is_served_byte_identical_to_rails -- --test-threads=4 > .scratch/service-worker-http-current.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.33s
```

```bash
bash rust/reference-tools/ws17_service_worker_harness.sh > .scratch/service-worker-harness-current.log 2>&1
```

```text
service worker harness: all checks passed
5 service worker harness check(s) failed
service worker harness: all checks passed
```

Nineteen unchanged Node checks pass before and after an intentional scratch-only cache-poisoning mutation; its five failures are expected. Fresh worker response bytes were saved by the preceding HTTP test. Browser status/meeting/OOO interaction and screenshot scenarios remain deferred.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/settings-next-clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.63s
```

Exit 0, no crate exclusions or warning suppressions. Broader workspace tests, storage media-byte differential, full DB differential/rollback and browser screenshots were not run in this slice.

## Precisely remaining, in the requested order

1. **Step 1 remains partial.** Implement `Users::StatusesController#update` (currently 501), status/presence/custom expiry clear/error/unsaved-value requests, manual OOO set/clear/note and calendar opt-in/out reconciliation with transactional refresh job seam and immediate badges/notices. Finish time-zone/theme/text-size profile writes and whole owned profile HTML boundaries, status badges and presence sidebar/DM presentation. Push-subscription controller named scenarios and index HTML byte checks remain; durable explicit test notification is now delivered. Allowance controller still needs the exact repeated-star and deterministic unique-index-loser scenarios. Domain writers, both rendered forms, notification PATCH and allowance routes are done.
2. **Keyword-alert recording not started.** Wire matcher priority into transactional activity recording, with minimal writer if WS12 has not landed; active-human eligibility, room/thread membership/muting, mention/reply/keyword winner, inbox remains recorded through DND. This slice changes keyword settings, not the activity recorder.
3. **Meeting/OOO dispatcher claims and broadcasts not started.** Atomic conditional claims, due sweeps, stale refresh deduplication, periodic tasks, manual expiry cleanup, badge/sidebar/DM/OOO notice rendering/broadcasts and opt-out/inactive/malformed-cache/concurrency tests. WS14 supplies Google/cache feed.
4. **Event, board and huddle pushers/jobs not started.** Policy/recipient adapters, exact payload vectors, source seams (WS14 events, WS12 board, WS13 huddle payloads), durable execution, transactional claim/enqueue, huddle join throttle, rollback/restart tests. Audit room durable handler parity; thread/saved/test durable handlers delivered. Shared huddle transport signatures are above; dedicated job DTO signature remains to be agreed/delivered with WS13.
5. **Remaining named scenario replays** below, including system/browser cases. Each exact deferred title has an owner and current state in the inventory. No WS17 completion claim.

## Named Rails replay counts, grouped by file

347 selected exact Rails titles: **85 equivalent scenarios rerun; 262 deferred**. Counts are Rails titles, not Rust harness test counts. `rust/plans/ws17-rails-test-inventory.json` records every selected title, owner, status and passing Rust evidence. Pure vector coverage does not automatically mark all exact scenario sequences ported.

| Rails file | Passed equivalent | Deferred |
| --- | ---: | ---: |
| `test/channels/workspace_presence_channel_test.rb` | 7 | 0 |
| `test/controllers/users/dnd_allowances_controller_test.rb` | 3 | 2 |
| `test/controllers/users/notification_settings_controller_test.rb` | 10 | 0 |
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
