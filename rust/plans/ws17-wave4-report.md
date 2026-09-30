# WS17 wave 4 report — PARTIAL

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws17`.
Branch: `rust/ws17-push-presence`; starting base `bb6c5d78`; Rails pin `d7c7de92`.
Implementation commits: `7b9267a3c948cca3fdf9a8c6946a2e643fb10d50` (notification policy/status readers/leases), `32b49aa4118c7ecb57f18fa14d2ccc0204acefe7` (transport, durable thread/saved handlers, presence HTTP, PWA verification). Both pushed. A documentation-only commit follows this implementation report.

**This is a coherent partial slice, not WS17 acceptance or cutover approval.** Domain and queue checks, the owned seeded app tests, the unchanged Rails service-worker harness, and full workspace/all-target clippy pass. The full seeded app suite retains the known WS11 bot-key failure. No allowlists/masks were widened; no new ignored tests or silent seed fallbacks were added.

## Implemented behavior and files

All paths below are relative to this worktree. Rails files were read only; schema, migration history, frontend assets, and Cargo dependency/lock files were not changed.

| Files | Change and verification boundary |
| --- | --- |
| `rust/crates/db/src/models/notification_policy.rs`, `models.rs` | Public typed five-kind policy; separate push, sound, and inbox decisions; room/thread involvement, mention/reply/keyword priority; batched DND sender allowances. Missing membership and a present membership with SQL-null involvement remain distinct. 2,240 actual Rails decisions tested. Keyword winner is a policy primitive; keyword activity recording is still deferred. |
| `rust/crates/db/src/models/user_status_settings.rs` | Batched read DTO, separate from base User and rendering. Loads users and meeting caches in two queries. Manual DND/timer, minute-based quiet hours with Rails/IANA zones, expired custom-status reads, meeting/OOO precedence and effective presence. 161 actual Rails vectors including spring/autumn DST. No settings writer, presets, dispatcher claims, or full cache model API delivered. |
| `rust/crates/db/src/models/workspace_presence_lease.rs` | Read-only batched online/idle lookup; 90-second TTL, 10-minute idle threshold, legacy null activity online, active connection dominance, active identity/session joins, bounded pruning of expired/mismatched leases. Existing establish/heartbeat/delete lifecycle preserved and tested. |
| `rust/crates/db/src/models/push_subscription.rs` | Payload `tag` and exact Rails room body/title/path, without inherited automatic truncation; merged/deduped room candidate scopes; disconnected/visible membership filtering; reply authors included only when opted in; batched real policy applied. Legacy explicit `fitted` helper retained for callers, unused by production room/thread/saved paths. |
| `rust/crates/db/src/models/channel_thread.rs` | Exact room-tagged thread payload and policy entry point `push_recipients_with_policy`; batched settings/cache and sender allowances; existing recipient callback helper retained. |
| `rust/crates/db/src/models/saved_item.rs` | Exact saved tag and reminder path/body (only Rails' 140-character reminder-body truncation); `reminder_push_with_policy` has no sender bypass. Existing transactional reminder claim/enqueue preserved. |
| `rust/crates/campfire/src/config.rs` | Production VAPID subject fixed to `mailto:support@smartdata.net`, matching our initializer. Inherited upstream VAPID_SUBJECT/TLS-domain defaults removed. Explicit low-level VapidConfig test API remains. |
| `rust/crates/campfire/src/integrations/web_push.rs` | JSON includes notification tag; Smartfire test notification title/tag; invalidation follows our Rails Pool rescue: expired 410 and OpenSSL/TLS/key failures invalidate; 404 is logged and retained. Existing encryption, VAPID, bounded pool and guarded pinned-address/direct network delivery retained. |
| `rust/crates/campfire/src/integrations/web_push/tests.rs`, `integrations/testdata/oracle/web_push.rb`, `integrations/testdata/web_push_expected.json` | Golden encrypted payload regenerated from the actual pin, with Smartfire tag/subject. TLS receiver tests decrypt durable thread + saved reminder deliveries; DND consumes both durable jobs without sending; clearing DND permits both with exact paths/tags. Existing room push, encryption, headers, invalidation and pool tests rerun. |
| `rust/crates/campfire/src/jobs/notifications.rs`, `jobs.rs` | Registers `ChannelThread::PushMessageJob` and `SavedItem::ReminderPushJob` in the durable runner; reads fresh policy before queuing into Web Push Pool. Missing record/unconfigured pool no-op matches reference paths. The originating enqueue remains the existing WS8 transactional write; no new separate transaction introduced. |
| `rust/crates/campfire/src/jobs/periodic.rs`, `jobs/tests.rs` | Presence prune task every 60 seconds, 100-row batches, ordered after existing tasks as Rails. Existing strict periodic-task vector extended with the actual Rails presence-task vector. |
| `rust/crates/campfire/src/controllers/users/presences.rs`, `controllers/users.rs`, `controllers.rs` | Authenticated `GET /users/presence`; batch active-human status + lease reads; Ruby Integer coercion, valid-input cap before SQL dedup/range coercion, primary-key result order. Complete response body/content type/retained lease count compared against 12 actual middleware responses. Rejecting DELETE trigger proves no read pruning. |
| `rust/crates/campfire/src/controllers/pwa.rs` | Seed-required test compares anonymous Rust HTTP service-worker and offline-shell response bytes to pinned Rails originals; saves the actual worker response for the unchanged Node harness. Existing served bytes already matched, so no template/asset edits were needed. |
| `rust/crates/db/src/tests.rs`, `tests/notification_policy_test.rs`, `tests/workspace_presence_lease_test.rs`, `tests/push_test.rs`, `tests/ws17_vectors.json` | Policy/status truth tables, allowed-sender-vs-star regression, 14 Rails lease scenarios in 10 Rust tests, real room DND/quiet/OOO regressions, opt-in reply/dedup/muted-mention regression, exact untruncated payload behavior. |
| `rust/vectors/ws17_presence.json` | Twelve complete presence HTTP goldens from the pinned Rails controller through its real middleware/session. |
| `rust/reference-tools/ws17_vectors.rb`, `ws17_presence_vectors.rb`, `ws17_verify_reference.py`, `ws17_regenerate_vectors.py` | Reproducible generation from our pinned Rails image; source SHA-256 verification against Git pin for 22 owned reference files; 2,240 policy, 161 status, 35 Integer, 12 HTTP, one encrypted payload vector. No upstream/docs/manual-golden substitutions. |
| `rust/reference-tools/ws17_injections.py`, `ws17_service_worker_harness.sh` | Nine intentional source defects proven to fail real tests, with restoration in finally; unchanged Rails Node harness run against served bytes, with authenticated-cache poisoning detected on scratch copy and restored. |
| `rust/plans/ws17-wave4-report.md`, `ws17-rails-test-inventory.json` | Tracked report mirror plus exact named Rails test ownership/coverage inventory. Authoritative requested report also written to the parent delegation directory. |

## Design and parity notes

Domain policy/read DTOs have no HTML dependencies. Controllers authenticate, load domain state, and serialize; jobs reuse the same domain policy and pool. Meeting-cache reads do not fetch Google or mutate rows. Public primitives available to WS12/WS13/WS14 are `NotificationPolicy`, `UserStatusSettings::for_ids`, `PushPayload::new`, and `Pool::queue`; the dedicated huddle/event/nudge payload/job seams have not been finished.

Rails' historical test titles say “starred sender,” but the current policy actually consults `dnd_allowed_users`. A user star alone does not bypass DND. The test inserts an actual allowance and verifies the exception. Push and inbox recipient validity are intentionally distinct because the pinned policy gates inbox on active-human status. A Rust enum prevents constructing an unknown notification kind; the named Rails unknown-kind exception replay remains deferred.

Presence reads keep expired rows until the minute pruner; expiration is inclusive at the exact TTL instant, as Rails SQL is. Cached status expiration is read-only. Ruby accepts base-prefixed/octal/underscore Integer values; huge valid integers consume the 100-slot cap before disappearing from SQLite's key range. No flattening of nested arrays/hashes is invented. The actual presence controller's JSON is unescaped for `<&>`; the full Rails HTTP golden determined that behavior rather than applying the generic Rails JSON helper.

Web Push invalidation deliberately follows OUR Pool rescue rather than the inherited upstream intent: 404 does not destroy a row, and TLS/OpenSSL errors do. Oversized message payloads reach the transport's record-size error rather than silently changing the body. Test notifications match Rails title/tag but still execute inline: durable test-notification execution is an outstanding brief requirement.

## Final validation (commands rerun; raw summaries)

Run commands below from the worktree root. Scratch/target dirs are inside this worktree; toolchain is 1.98.1, locked Cargo, four build/test threads. App tests bind only 52400–52449; comparison build uses 52450–52499. Golden regeneration precedes final tests. Mutation checks restore source before final validation.

### Actual pinned reference and both parity seeds

```bash
python3 rust/reference-tools/ws17_regenerate_vectors.py > .scratch/regenerate-final.log 2>&1
```

```text
pinned Rails source verified: 22 files match d7c7de92
Rails WS17 vectors: 2240 policy; 161 status; 35 Integer; 12 presence HTTP; 1 encrypted payload
```
```bash
PARITY_NAMESPACE=ws17 PARITY_OWNER=ws17 PARITY_IMAGE=triage-reference-d7c7de92:latest rust/parity/bin/seed build default first_run > .scratch/seed-final.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

### Domain and durable queue

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -p campfire_jobs -- --test-threads=4 > .scratch/domain-final.log 2>&1
```

```text
test result: ok. 416 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 40.13s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The 52 queue unit tests plus two crash/restart integration tests ran. DB's three existing explicitly ignored reference/export tests did not run: `scenario_matches_ruby`, `fixtures_match_ruby_row_for_row`, and `export_database_for_rails` require their special oracle/export environment. No Rails rollback/readback claim is made for this slice.

### Full seeded app suite (known failure retained)

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 WS17_SERVICE_WORKER_OUTPUT="$PWD/.scratch/service-worker-served.js" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```

```text
test result: FAILED. 308 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 26.10s
```

Exit 101 is solely `controllers::presenters::accounts::tests::manages_bots`: resetting Bender's bot key leaves the original seeded key displayed. This is WS11's known failure; it is not suppressed on this branch. Both required seeds were built; no tests silently skipped for missing seed. Two pre-existing explicit ignores remain: cable `golden::record_reference` (reference recorder), and `jobs::tests::push_latency` (measurement).

### Owned seeded HTTP/durable-delivery checks

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52400-52449 MAIL_TEST_PORT_RANGE=52400-52449 WS17_SERVICE_WORKER_OUTPUT="$PWD/.scratch/service-worker-served.js" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire ws17_ -- --test-threads=4 --nocapture > .scratch/app-ws17-final.log 2>&1
```

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 305 filtered out; finished in 0.48s
```

The filtered-out count is intentional for this focused run; the unfiltered seeded suite above ran separately. All new seed-dependent tests use `expect`, so absent seeds fail rather than return early.

### Service worker: unchanged Rails harness against Rust HTTP bytes

```bash
bash rust/reference-tools/ws17_service_worker_harness.sh > .scratch/service-worker-harness.log 2>&1
```

```text
service worker harness: all checks passed
5 service worker harness check(s) failed
service worker harness: all checks passed
```

Nineteen unchanged harness checks pass before and after the scratch-only mutation. The five failures are the expected detection of authenticated/API caching. HTTP bytes and offline shell are also tested; real browser retry/status/meeting/OOO interaction tests are not claimed.

### Full workspace/all-target clippy

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.69s
```

Exit 0; no crate exclusions and no warning suppressions added. Broader workspace tests, storage media-byte tests, browser screenshot differentials, and full DB differential/rollback are not run by this report.

### Fresh-main comparison

Comparison source is an immutable archive of the freshly fetched `origin/main` SHA `21a7332f2d3c324f0862cdf448baf17a84395aa0`, inside `.scratch/rust`. Both pinned seeds were copied into that independent source tree, with `.scratch/rust/target` created for its hardcoded test scratch path. This branch has not merged main without a lead instruction.

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/.scratch/target-baseline" CAMPFIRE_REFERENCE="$PWD" CABLE_TEST_PORT_RANGE=52450-52499 MAIL_TEST_PORT_RANGE=52450-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/baseline-app-final.log 2>&1
```

```text
test result: ok. 306 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 24.60s
```

That main snapshot explicitly ignores the bot-key failure (WS19b), in addition to recorder/measurement. Its one `skipping locally` line comes from the intentional missing-seed unit test; integration tests had both seeds and ran. The known bot test was then forced to execute on the same fresh source:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/.scratch/target-baseline" CAMPFIRE_REFERENCE="$PWD" CABLE_TEST_PORT_RANGE=52450-52499 MAIL_TEST_PORT_RANGE=52450-52499 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/rust/Cargo.toml -p campfire --bin campfire manages_bots -- --ignored --test-threads=4 --nocapture > .scratch/baseline-bot-final.log 2>&1
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 308 filtered out; finished in 0.81s
```

This reproduces the same original-key-visible assertion; exit 101 is expected evidence, not a passing test.

## Failing first and mutation evidence

Before the policy changes, the three real room-push regressions for DND/non-allowed sender, quiet hours, and OOO failed:

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.21s
```

The historical pre-fix output is retained in scratch; it is not represented as a command that should still fail after the fix. Reproducible current source mutations were rerun by the committed driver. It accepts only a compiled test failure, not a build error, and restores each source in `finally`.

```bash
python3 rust/reference-tools/ws17_injections.py > .scratch/injections-final.log 2>&1
```

```text
policy-quiet-gate: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.20s
reply-recipient: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.20s
expired-status: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.17s
prune-on-read: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.12s
valid-presence: detected
test result: FAILED. 1 passed; 9 failed; 0 ignored; 0 measured; 409 filtered out; finished in 0.47s
ghost-presence: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.19s
presence-http-body: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.37s
web-push-tag: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.00s
service-worker-bytes: detected
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 310 filtered out; finished in 0.33s
```

These cover ignored quiet/DND gates, opt-in reply omission, expired status still visible, pruning during reads, broken valid leases, ghost presence, wrong complete HTTP body, missing encoded tag, and changed worker bytes. The independent Node mutation covers authenticated response caching. Restored source is used for the final successful owned checks above.

## Cross-workstream touches and exact remaining work

1. **WS17 continuation — pushers/jobs:** finish event reminder and board nudge payload adapters plus durable handlers (source lifecycle WS14 / WS12). Add dedicated huddle invitation/join transport adapters and handlers, fresh-policy recipient scopes, caller allowance handling, disconnected/inbox preference checks, and conditional 10-minute join throttle that is not burned on suppressed/no-subscription sends. WS13 owns payload building; only shared public policy/payload/pool primitives are provided here. Put the test-notification enqueue in its triggering transaction and execute it durably. Regenerate per-pusher payload tables for all seven; current generated encrypted transport vector plus room/thread/saved assertions do not constitute the complete seven-pusher Rails table.
2. **WS17 + WS12 — keyword/activity recorder:** wire the existing main keyword matcher and policy inbox winner into actual message/activity recording, including author exclusion, source authorization, visibility/mute semantics, winner collapse/idempotency and bounded query count. No minimal keyword ActivityItem writer was added in this slice. WS12 owns the full recorder and activity lifecycle; WS17 owns the keyword wiring. Existing matcher tests are rerun, not evidence of recorder integration.
3. **WS17 continuation — settings writes/controllers/views:** complete presence/custom-status/DND/quiet-hours/OOO writer APIs and Rails validations, presets, rollback-safe keyword replacement, allowance model/writers/JSON/HTML, notification preferences, and statuses/DND allowances/notification-settings controllers. Port byte-identical parity-seed HTML and layout notification markers/sound gates. Read DTOs do not replace these routes. Push-subscription registration/deletion and dev test-notification controller named scenarios need full replay, especially existing invalid row re-registration; existing code is retained, not newly certified here.
4. **WS17 + WS14 — meeting/OOO:** finish conditional claim state, racing-update semantics, minute dispatch tasks, badge/presence broadcasts and group/direct-room status/OOO notices with renderer registration. WS14 supplies cache refresh/Google/event data; WS17 owns dispatcher claims/broadcasts. Cache DTO readers are not the cache write/validation/epoch-window API. Malformed timestamp shapes and invalid persisted zones beyond the supplied vectors remain to be exhaustively matched.
5. **WS17 + WS7/WS8b — broadcasts and atomic HTTP validation:** heartbeat channel is inherited and all seven native cable scenarios rerun. The new endpoint and minute pruner are done. Settings-triggered sidebar/status/DM presence broadcasts and meeting/OOO broadcasts remain. Durable thread/saved execution is verified with real DB commits, queue workers and decrypted TLS deliveries; full controller enqueue-failure rollback scenarios must be replayed through the WS8b routes. Existing WS8 transaction hooks were not changed by these handlers.
6. **WS17 continuation — remaining test acceptance:** exact deferred titles and owners below, including browser status/service-worker retry/meeting/OOO tests. Pure policy/status vectors cover many deferred scenarios' predicates, but do not substitute for each Rails persisted/controller/browser setup. Keep the known WS11 bot failure visible until its owner fixes it. Broader workspace suite, full Rails DB rollback/differential and pixels remain unverified in this slice.

No Rails behavior decision was intentionally changed. Open integration choice for the lead: land this partial branch to unblock WS12/WS13/WS14 policy/transport consumers, then assign the continuation above; this report does not ask for cutover or claim full WS17 completion.

## Rails named-test inventory

The inventory is extracted from the Git pin, not from remembered estimates. The tracked JSON gives an exact name, owner, status and evidence for every selected Rails test. “Equivalent rerun” includes inherited WS7/WS8 tests that ran in the full suite. “Deferred exact scenario” is deliberately conservative: shared truth tables may cover the leaf rule, but the entire named Rails setup was not newly certified. All test families named in the brief are included, plus keyword matcher/recorder integration. Unrelated agent/huddle-source/UI families remain with their workstreams.

Inventory: **347 named tests, 60 equivalent scenarios rerun, 287 exact scenarios deferred**.

### `test/models/notifications/policy_test.rb`

- Deferred: a room mention records and pushes for a mentions member — owner: WS17 continuation.
- Deferred: a room mention still records with notifications off but sends no push — owner: WS17 continuation.
- Deferred: an invisible room membership gets nothing at all — owner: WS17 continuation.
- Deferred: a room reply records and pushes for mentions and everything members — owner: WS17 continuation.
- Deferred: a room reply stays silent with notifications off — owner: WS17 continuation.
- Deferred: a plain room message pushes everything followers without an inbox item — owner: WS17 continuation.
- Deferred: a plain room message does nothing for mentions members — owner: WS17 continuation.
- Deferred: a room keyword match records without pushing for mentions and notifications-off members — owner: WS17 continuation.
- Deferred: an everything member's keyword match still pushes as a broadcast — owner: WS17 continuation.
- Deferred: a room message without a membership records nothing, not even mentions or keywords — owner: WS17 continuation.
- Deferred: mention beats reply beats keyword for one room message — owner: WS17 continuation.
- Deferred: a followed thread records activity and pushes — owner: WS17 continuation.
- Deferred: an unfollowed thread stays silent for plain messages — owner: WS17 continuation.
- Deferred: a thread mention records and pushes for mentions and everything members — owner: WS17 continuation.
- Deferred: a muted thread gets nothing, not even mentions or keywords — owner: WS17 continuation.
- Deferred: a thread reply records and pushes for followers only — owner: WS17 continuation.
- Deferred: a thread keyword match records for unfollowed members without pushing — owner: WS17 continuation.
- Deferred: room notifications off suppresses thread activity but not keywords — owner: WS17 continuation.
- Deferred: a non-member of the thread gets nothing — owner: WS17 continuation.
- Deferred: bots and deactivated recipients record nothing — owner: WS17 continuation.
- Deferred: a muted room mention records and pushes — owner: WS17 continuation.
- Deferred: a muted room keyword match records without pushing — owner: WS17 continuation.
- Deferred: a muted room reply stays silent — owner: WS17 continuation.
- Deferred: muted room thread activity stays silent for followers — owner: WS17 continuation.
- Deferred: a muted board post stays silent for members outside the thread — owner: WS17 continuation.
- Deferred: DND silences a muted room mention push but keeps the inbox item — owner: WS17 continuation.
- Deferred: manual DND suppresses push and sound but still records the inbox item — owner: WS17 continuation.
- Deferred: a starred sender still pushes through DND — owner: WS17 continuation.
- Deferred: a preloaded DND exception decides without another lookup — owner: WS17 continuation.
- Deferred: quiet hours suppress push inside the window only — owner: WS17 continuation.
- Deferred: quiet hours follow the recipient's time zone — owner: WS17 continuation.
- Deferred: a starred sender still pushes through quiet hours — owner: WS17 continuation.
- Deferred: reminders push unless DND is on, and carry no sender exception — owner: WS17 continuation.
- Deferred: huddle invitations push unless DND is on without a starred caller — owner: WS17 continuation.
- Deferred: huddle join notices push for live memberships and record no inbox item — owner: WS17 continuation.
- Deferred: huddle join notices stay silent when muted, off, hidden, or no membership — owner: WS17 continuation.
- Deferred: huddle join notices honor DND with a starred-caller exception — owner: WS17 continuation.
- Deferred: huddle join notices stay silent during meetings and out of office — owner: WS17 continuation.
- Deferred: quiet-during-meetings suppresses push and sound but still records the inbox item — owner: WS17 continuation.
- Deferred: quiet-during-meetings pushes outside busy intervals — owner: WS17 continuation.
- Deferred: quiet-during-meetings needs meeting status on — owner: WS17 continuation.
- Deferred: a starred sender still pushes through quiet-during-meetings — owner: WS17 continuation.
- Deferred: reminders and huddles stay silent during meetings with no sender exception — owner: WS17 continuation.
- Deferred: out of office suppresses push and sound but still records the inbox item — owner: WS17 continuation.
- Deferred: out of office pushes when the member keeps notifications on — owner: WS17 continuation.
- Deferred: a starred sender still pushes through out of office — owner: WS17 continuation.
- Deferred: reminders and huddles stay silent during out of office — owner: WS17 continuation.
- Deferred: calendar out of office quiets like a manual one — owner: WS17 continuation.
- Deferred: an expired out of office pushes again — owner: WS17 continuation.
- Deferred: a missing recipient pushes nothing — owner: WS17 continuation.
- Deferred: an unknown kind raises — owner: WS17 continuation.
- Deferred: dnd exceptions load for a batch in one query — owner: WS17 continuation.

### `test/models/notifications/push_gating_test.rb`

- Deferred: room push skips a DND recipient but the inbox item is still recorded — owner: WS17 continuation.
- Deferred: room push skips a DND-presence recipient but the inbox item is still recorded — owner: WS17 continuation.
- Deferred: room push still reaches a starred sender's recipient during DND — owner: WS17 continuation.
- Deferred: room push skips a recipient inside quiet hours — owner: WS17 continuation.
- Deferred: thread push notifies followers with the thread payload — owner: WS17 continuation.
- Deferred: thread push skips a DND follower — owner: WS17 continuation.
- Deferred: a thread reply pushes its follower author but not an unfollowed one — owner: WS17 continuation.
- Deferred: reminder push skips a DND attendee — owner: WS17 continuation.
- Deferred: huddle push honors DND with a starred-caller exception — owner: WS17 continuation.
- Deferred: group huddle push skips DND and quiet-hours recipients but their missed calls are still recorded — owner: WS17 continuation.

### `test/models/room/push_test.rb`

- Equivalent rerun: deliver new message to other room users with push subscriptions — `rust/crates/db/src/tests/push_test.rs::deliver_new_message_to_other_room_users_with_push_subscriptions`.
- Equivalent rerun: notifies subscribed users — `rust/crates/db/src/tests/push_test.rs::notifies_subscribed_users`.
- Equivalent rerun: replies notify their author only when opted in — `rust/crates/db/src/tests/push_test.rs::ws17_replies_notify_only_opted_in_authors_and_merge_duplicate_scopes`.
- Equivalent rerun: message pushes carry the room tag so notifications group per room — `rust/crates/db/src/tests/push_test.rs::payloads`.
- Deferred: a forwarded note follows the mention push path while its snapshot does not — owner: WS17 continuation.
- Equivalent rerun: does not notify for connected rooms — `rust/crates/db/src/tests/push_test.rs::does_not_notify_for_connected_rooms`.
- Equivalent rerun: does not notify for invisible rooms — `rust/crates/db/src/tests/push_test.rs::does_not_notify_for_invisible_rooms`.
- Equivalent rerun: destroys invalid subscriptions — `rust/crates/campfire/src/integrations/web_push/tests.rs::pushes_messages_and_destroys_expired_subscriptions`.

### `test/models/push/subscription_test.rb`

- Equivalent rerun: valid subscription with permitted endpoint — `rust/crates/db/src/tests/push_test.rs::valid_subscription_with_permitted_endpoint`.
- Equivalent rerun: rejects endpoint with non-https scheme — `rust/crates/db/src/tests/push_test.rs::rejects_endpoint_with_non_https_scheme`.
- Equivalent rerun: rejects endpoint with non-permitted host — `rust/crates/db/src/tests/push_test.rs::rejects_endpoint_with_non_permitted_host`.
- Equivalent rerun: rejects endpoint whose host only suffix-matches a permitted host — `rust/crates/db/src/tests/push_test.rs::rejects_endpoint_whose_host_only_suffix_matches_a_permitted_host`.
- Equivalent rerun: rejects blank endpoint — `rust/crates/db/src/tests/push_test.rs::rejects_blank_endpoint`.
- Equivalent rerun: rejects endpoint on a non-default port — `rust/crates/db/src/tests/push_test.rs::rejects_endpoint_on_a_non_default_port`.
- Equivalent rerun: rejects endpoint that resolves to private IP — `rust/crates/db/src/tests/push_test.rs::rejects_endpoint_that_resolves_to_private_ip`.
- Deferred: rejects endpoint that resolves to loopback IP — owner: WS17 continuation.
- Deferred: rejects endpoint that resolves to link-local IP (AWS IMDS) — owner: WS17 continuation.
- Deferred: rejects endpoint whose host resolves to nothing without raising — owner: WS17 continuation.
- Equivalent rerun: resolved_endpoint_ip returns the pinned public IP — `rust/crates/db/src/tests/push_test.rs::resolved_endpoint_ip_returns_the_pinned_public_ip`.
- Deferred: endpoint resolution is deferred from the enqueue path to the delivery worker — owner: WS17 continuation.
- Deferred: delivery is skipped when the endpoint no longer resolves to a public IP — owner: WS17 continuation.
- Equivalent rerun: delivery is skipped for a non-permitted host even when it resolves publicly — `rust/crates/db/src/tests/push_test.rs::delivery_is_skipped_for_a_non_permitted_host_or_port`.
- Equivalent rerun: delivery is skipped for a permitted host on a non-default port — `rust/crates/db/src/tests/push_test.rs::delivery_is_skipped_for_a_non_permitted_host_or_port`.
- Equivalent rerun: delivery sends with the pinned endpoint_ip — `rust/crates/campfire/src/integrations/web_push/tests.rs::delivers_to_the_pinned_address_with_the_gems_headers`.
- Equivalent rerun: the encoded message carries the notification tag for room grouping — `rust/crates/campfire/src/integrations/web_push/tests.rs::encodes_the_message_like_json_generate`.
- Equivalent rerun: accepts all permitted push service domains — `rust/crates/db/src/tests/push_test.rs::accepts_all_permitted_push_service_domains`.

### `test/lib/web_push/persistent_request_test.rb`

- Equivalent rerun: pins delivery to endpoint_ip instead of re-resolving the host — `rust/crates/campfire/src/integrations/web_push/tests.rs::delivers_to_the_pinned_address_with_the_gems_headers`.
- Deferred: ignores proxy env so the pin can't be routed through a re-resolving proxy — owner: WS17 continuation.

### `test/models/workspace_presence_lease_test.rb`

- Equivalent rerun: an unexpired lease backed by an existing session makes its user online — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_fresh_lease_is_online_and_has_a_uuid_and_90_second_ttl`.
- Equivalent rerun: a stale lease does not make its user online — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_expiry_is_inclusive_and_reads_never_prune`.
- Equivalent rerun: deleting one lease leaves a user online while another connection is live — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_deleting_one_connection_keeps_the_other_live`.
- Equivalent rerun: a lease without its session does not make its user online — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_revoked_session_refresh_deletes_lease`.
- Equivalent rerun: refresh deletes the lease when its session has been revoked — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_revoked_session_refresh_deletes_lease`.
- Equivalent rerun: cannot establish or refresh presence for an inactive user — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_inactive_user_cannot_establish_or_refresh`.
- Equivalent rerun: cannot establish a lease for another user's session — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_session_mismatch_is_absent_and_pruned`.
- Equivalent rerun: online lookup rejects a lease whose session belongs to another user — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_session_mismatch_is_absent_and_pruned`.
- Equivalent rerun: prune removes leases whose session belongs to another user — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_session_mismatch_is_absent_and_pruned`.
- Equivalent rerun: prune removes expired leases in bounded batches — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_prune_has_bounded_batches`.
- Equivalent rerun: a fresh lease reads online, then idle after ten quiet minutes — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_quiet_heartbeat_extends_only_expiry_and_active_heartbeat_restores_online`.
- Equivalent rerun: an active heartbeat extends activity while a quiet one only extends the lease — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_quiet_heartbeat_extends_only_expiry_and_active_heartbeat_restores_online`.
- Equivalent rerun: any active connection keeps the user online — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_any_recent_or_legacy_null_activity_keeps_user_online`.
- Equivalent rerun: users without a lease are absent from the presence map — `rust/crates/db/src/tests/workspace_presence_lease_test.rs::ws17_absent_users_and_expired_leases_are_not_returned`.

### `test/channels/workspace_presence_channel_test.rb`

- Equivalent rerun: subscribing establishes a server-owned presence lease — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_subscribing_establishes_a_server_owned_presence_lease`.
- Equivalent rerun: unsubscribing deletes only this connection lease — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_unsubscribing_deletes_only_this_connections_lease`.
- Equivalent rerun: heartbeat extends the lease — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_heartbeat_extends_the_lease`.
- Equivalent rerun: heartbeat rejects and deletes presence after session revocation — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_heartbeat_rejects_and_deletes_presence_after_session_revocation`.
- Equivalent rerun: heartbeat destroys an idle-timed-out administrator session and rejects — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_heartbeat_destroys_an_idle_timed_out_administrator_session_and_rejects`.
- Equivalent rerun: heartbeat replaces a valid lease that was pruned — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_heartbeat_replaces_a_valid_lease_that_was_pruned`.
- Equivalent rerun: rejects a subscription without a verified session — `rust/crates/campfire/src/channels/tests/reference_test.rs::workspace_presence_rejects_a_subscription_without_a_session`.

### `test/controllers/users/presences_controller_test.rb`

- Equivalent rerun: returns presence and custom status for workspace users — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: an expired custom status reads as blank — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: a presence lookup never prunes expired leases — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: invisible members read offline — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: returns the meeting label while in a meeting — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: a custom status wins over the meeting label in the lookup — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.
- Equivalent rerun: returns the OOO label while out of office — `rust/crates/campfire/src/controllers/users/presences.rs::ws17_presence_http_bodies_match_rails_vectors`.

### `test/models/user/status_settings_test.rb`

- Deferred: defaults to automatic presence, no DND, and the system theme — owner: WS17 continuation.
- Deferred: rejects unknown presence, theme, and time zone values — owner: WS17 continuation.
- Deferred: DND is manual-only outside quiet hours — owner: WS17 continuation.
- Deferred: the DND presence silences like the DND switch — owner: WS17 continuation.
- Deferred: quiet hours cover an overnight window in the user's time zone — owner: WS17 continuation.
- Deferred: quiet hours need a start and an end while enabled — owner: WS17 continuation.
- Deferred: malformed quiet-hours input reads as blank — owner: WS17 continuation.
- Deferred: quiet-hours input tolerates seconds — owner: WS17 continuation.
- Deferred: an expired custom status reads as blank — owner: WS17 continuation.
- Deferred: custom status expiry presets resolve in the user's time zone — owner: WS17 continuation.
- Deferred: an unknown custom status expiry raises — owner: WS17 continuation.
- Deferred: replacing keywords strips, dedupes, and caps the list — owner: WS17 continuation.
- Deferred: replacing keywords past the cap fails with errors — owner: WS17 continuation.
- Deferred: effective presence folds the manual setting over the lease state — owner: WS17 continuation.

### `test/models/user/meeting_status_test.rb`

- Deferred: meeting status and quiet-during-meetings default off — owner: WS17 continuation.
- Deferred: in_meeting? needs the opt-in and a covering interval — owner: WS17 continuation.
- Deferred: in_meeting? is false without a cache row — owner: WS17 continuation.
- Deferred: the meeting label shows while in a meeting — owner: WS17 continuation.
- Deferred: a custom status wins over the meeting label — owner: WS17 continuation.
- Deferred: an expired custom status yields to the meeting label — owner: WS17 continuation.
- Deferred: manual DND wins over the meeting label — owner: WS17 continuation.
- Deferred: the DND presence wins over the meeting label — owner: WS17 continuation.
- Deferred: quiet hours win over the meeting label — owner: WS17 continuation.
- Deferred: invisible hides the meeting label — owner: WS17 continuation.
- Deferred: quiet-during-meetings never suppresses the meeting label — owner: WS17 continuation.
- Deferred: quiet-during-meetings only works while meeting status is on — owner: WS17 continuation.
- Deferred: quiet-during-meetings applies through a custom status — owner: WS17 continuation.
- Deferred: quiet-during-meetings is off outside busy intervals — owner: WS17 continuation.

### `test/models/user/out_of_office_test.rb`

- Deferred: out of office defaults off — owner: WS17 continuation.
- Deferred: a manual OOO is active until its end, then reads as off — owner: WS17 continuation.
- Deferred: setting an OOO end in the past is invalid, but an expired end left behind still saves — owner: WS17 continuation.
- Deferred: a note longer than 140 characters is invalid — owner: WS17 continuation.
- Deferred: the status line names the return date and the note — owner: WS17 continuation.
- Deferred: the return date renders in the OOO member's own zone — owner: WS17 continuation.
- Deferred: OOO wins over a custom status, DND, and the meeting label — owner: WS17 continuation.
- Deferred: invisible hides the OOO label but OOO still reads as active — owner: WS17 continuation.
- Deferred: OOO quiet never suppresses the OOO label — owner: WS17 continuation.
- Deferred: calendar OOO needs the opt-in and a covering interval — owner: WS17 continuation.
- Deferred: a calendar OOO outside its intervals reads as off — owner: WS17 continuation.
- Deferred: overlapping manual and calendar OOO show the later end — owner: WS17 continuation.
- Deferred: the note shows only while the manual OOO is active — owner: WS17 continuation.
- Deferred: OOO presets run to the end of the day in the member's zone — owner: WS17 continuation.
- Deferred: the Monday preset is a week out on Mondays — owner: WS17 continuation.
- Deferred: the custom preset parses a datetime-local value in the member's zone — owner: WS17 continuation.
- Deferred: an unknown preset raises — owner: WS17 continuation.
- Deferred: claim_ooo_broadcast! wins the first claim and each flip, and loses re-runs — owner: WS17 continuation.
- Deferred: claiming an end clears the expired manual columns — owner: WS17 continuation.
- Deferred: claiming an end keeps a manual OOO set racing the sweep — owner: WS17 continuation.
- Deferred: OOO quiets notifications unless the member keeps them on — owner: WS17 continuation.
- Deferred: deactivating clears the manual OOO columns — owner: WS17 continuation.

### `test/models/calendar/meeting_cache_test.rb`

- Deferred: in_meeting? is true inside an interval, with an inclusive start and exclusive end — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: in_meeting? is false without intervals — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: in_meeting? ignores malformed pairs instead of raising — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: quiet_window_epochs returns epoch windows and skips malformed pairs — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: in_ooo? is true inside an OOO interval, with an inclusive start and exclusive end — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: in_ooo? reads only the OOO intervals, not the busy ones — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: ooo_end_covering returns the latest covering end — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: ooo_end_covering is nil while uncovered — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: ooo_window_epochs returns epoch windows and skips malformed pairs — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: claim_broadcast! wins the first claim and each flip, and loses re-runs — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: claim_broadcast! lets only one concurrent claimant win — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: one cache per user — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).
- Deferred: the cache row references its member with a cascading foreign key — owner: WS17 continuation (claims/readers); WS14 (cache persistence/feed).

### `test/models/calendar/meeting_dispatcher_test.rb`

- Deferred: a meeting start broadcasts the badge — owner: WS17 continuation.
- Deferred: a re-run with no flip broadcasts nothing — owner: WS17 continuation.
- Deferred: a steady-state tick issues no claim write — owner: WS17 continuation.
- Deferred: a meeting end broadcasts the badge — owner: WS17 continuation.
- Deferred: a broadcast carries the meeting label — owner: WS17 continuation.
- Deferred: a stale cache enqueues a refresh — owner: WS17 continuation.
- Deferred: a missing cache enqueues a refresh without broadcasting — owner: WS17 continuation.
- Deferred: a fresh cache enqueues nothing — owner: WS17 continuation.
- Deferred: members who never opted in are ignored — owner: WS17 continuation.
- Deferred: deactivated members are ignored — owner: WS17 continuation.
- Deferred: one failing member does not stop the sweep — owner: WS17 continuation.

### `test/models/calendar/ooo_dispatcher_test.rb`

- Deferred: a manual OOO start broadcasts the badge and the DM notice — owner: WS17 continuation.
- Deferred: a re-run with no flip broadcasts nothing — owner: WS17 continuation.
- Deferred: a steady-state tick issues no claim write — owner: WS17 continuation.
- Deferred: an OOO end broadcasts and clears the manual columns — owner: WS17 continuation.
- Deferred: the broadcasts carry the OOO label, the note, and the return date — owner: WS17 continuation.
- Deferred: a calendar OOO start broadcasts the badge and the notice — owner: WS17 continuation.
- Deferred: a stale OOO-only cache enqueues a refresh — owner: WS17 continuation.
- Deferred: a member with both opt-ins refreshes through the meeting dispatcher only — owner: WS17 continuation.
- Deferred: a missing cache enqueues a refresh without broadcasting — owner: WS17 continuation.
- Deferred: members with neither a manual OOO nor the calendar opt-in are ignored — owner: WS17 continuation.
- Deferred: deactivated members are ignored — owner: WS17 continuation.
- Deferred: one failing member does not stop the sweep — owner: WS17 continuation.

### `test/models/saved_item/reminder_pusher_test.rb`

- Equivalent rerun: pushes the reminder to the saver with a message link — `rust/crates/db/src/tests/saved_item_test.rs::pushes_the_reminder_to_the_saver_with_a_message_link`.
- Equivalent rerun: no push goes out after the saver loses room access — `rust/crates/db/src/tests/saved_item_test.rs::no_push_goes_out_after_the_saver_loses_room_access`.
- Equivalent rerun: no push goes out while the saver is in Do Not Disturb — `rust/crates/campfire/src/integrations/web_push/tests.rs::ws17_durable_thread_and_saved_reminder_jobs_apply_policy_and_deliver`.
- Equivalent rerun: a thread message reminder links into its thread — `rust/crates/db/src/tests/saved_item_test.rs::a_thread_message_reminder_links_into_its_thread`.

### `test/models/event/reminder_pusher_test.rb`

- Deferred: pushes the reminder to going and maybe attendees who are still members — owner: WS17 continuation (push); WS14 (event source).
- Deferred: push reminders ignore the event_reminders inbox switch — owner: WS17 continuation (push); WS14 (event source).
- Deferred: the push body names the venue — owner: WS17 continuation (push); WS14 (event source).
- Deferred: a direct room reminder is titled by the organizer — owner: WS17 continuation (push); WS14 (event source).
- Deferred: the push body counts down the actual minutes — owner: WS17 continuation (push); WS14 (event source).
- Deferred: the push body uses the singular minute — owner: WS17 continuation (push); WS14 (event source).
- Deferred: an event starting now says so — owner: WS17 continuation (push); WS14 (event source).
- Deferred: a recently started event still says starting now — owner: WS17 continuation (push); WS14 (event source).
- Deferred: an event that already ended is skipped — owner: WS17 continuation (push); WS14 (event source).
- Deferred: an event that started long ago is skipped — owner: WS17 continuation (push); WS14 (event source).

### `test/models/board_automations/nudge_pusher_test.rb`

- Deferred: pushes the nudge payload to the recipient subscriptions — owner: WS17 continuation (push); WS12 (board source).
- Deferred: escalations push with the escalated prefix — owner: WS17 continuation (push); WS12 (board source).
- Deferred: skips a recipient who left the board — owner: WS17 continuation (push); WS12 (board source).
- Deferred: dnd silences the push like other reminders — owner: WS17 continuation (push); WS12 (board source).

### `test/models/huddle/join_pusher_test.rb`

- Deferred: pushes the join to the recipient's subscriptions and stamps the throttle — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a second push inside ten minutes is throttled — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a push ten minutes later goes out again — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a DND recipient gets no push and burns no throttle window — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a starred joiner still pushes through DND — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a recipient in quiet hours gets no push — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a recipient quiet in a meeting gets no push — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: an out-of-office recipient gets no push unless they keep notifications on — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a connected recipient gets no push and burns no throttle window — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a switched-off or hidden room gets no push — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a muted room gets no push and burns no throttle window — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a recipient with huddle invitations switched off gets no push and burns no throttle window — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a recipient with no subscriptions burns no throttle window — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).

### `test/jobs/huddle/push_invitation_job_test.rb`

- Deferred: pushes the invitation to the recipient only — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: an opted-out recipient gets no push subscriptions — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: a connected recipient gets no push — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).
- Deferred: missing invitations are ignored — owner: WS17 continuation (transport/policy/claims); WS13 (payload/source).

### `test/controllers/users/statuses_controller_test.rb`

- Deferred: updates presence and the custom status with an expiry — owner: WS17 continuation.
- Deferred: clears the custom status — owner: WS17 continuation.
- Deferred: rejects an unknown presence with errors — owner: WS17 continuation.
- Deferred: requires sign-in — owner: WS17 continuation.
- Deferred: opting into meeting status enqueues a first refresh — owner: WS17 continuation.
- Deferred: opting out of meeting status drops the cached intervals — owner: WS17 continuation.
- Deferred: opting out while in a meeting broadcasts the cleared badge — owner: WS17 continuation.
- Deferred: opting out without cached intervals broadcasts nothing — owner: WS17 continuation.
- Deferred: saving other status settings leaves meeting refreshes alone — owner: WS17 continuation.
- Deferred: sets out of office with a preset and a note, and broadcasts it — owner: WS17 continuation.
- Deferred: sets out of office with a custom date and time in the member's zone — owner: WS17 continuation.
- Deferred: rejects an unknown OOO preset without saving anything — owner: WS17 continuation.
- Deferred: rejects a blank or past custom OOO end without saving anything — owner: WS17 continuation.
- Deferred: rejects an OOO note over 140 characters — owner: WS17 continuation.
- Deferred: clears out of office early and broadcasts the cleared state — owner: WS17 continuation.
- Deferred: clearing early ends only the manual OOO while calendar OOO covers — owner: WS17 continuation.
- Deferred: edits the OOO note alone — owner: WS17 continuation.
- Deferred: opting into calendar OOO enqueues a first refresh — owner: WS17 continuation.
- Deferred: opting out of calendar OOO clears its intervals and broadcasts — owner: WS17 continuation.
- Deferred: opting out of calendar OOO keeps the row while meeting status is on — owner: WS17 continuation.
- Deferred: opting out of meeting status keeps the row while calendar OOO is on — owner: WS17 continuation.
- Deferred: saving other status settings leaves calendar OOO refreshes alone — owner: WS17 continuation.

### `test/controllers/users/dnd_allowances_controller_test.rb`

- Deferred: starring and unstarring someone for DND — owner: WS17 continuation.
- Deferred: starring twice stays a single exception — owner: WS17 continuation.
- Deferred: a concurrent star reports success instead of an error — owner: WS17 continuation.
- Deferred: cannot star yourself — owner: WS17 continuation.
- Deferred: cannot star a bot — owner: WS17 continuation.

### `test/controllers/users/notification_settings_controller_test.rb`

- Deferred: enables DND with quiet hours and keywords — owner: WS17 continuation.
- Deferred: disables DND and clears keywords — owner: WS17 continuation.
- Deferred: enables quiet-during-meetings — owner: WS17 continuation.
- Deferred: toggles keep-notifying while out of office, defaulting to off — owner: WS17 continuation.
- Deferred: quiet hours without a window render errors — owner: WS17 continuation.
- Deferred: a failed save keeps the previous keywords — owner: WS17 continuation.
- Deferred: requires sign-in — owner: WS17 continuation.
- Deferred: enabling DND after a timed expiry starts it indefinitely — owner: WS17 continuation.
- Deferred: disabling DND clears a running timer — owner: WS17 continuation.
- Deferred: saving settings preserves a running DND timer — owner: WS17 continuation.

### `test/models/dnd_allowed_user_test.rb`

- Deferred: allows starring another active person — owner: WS17 continuation.
- Deferred: rejects duplicates, self-stars, and bots — owner: WS17 continuation.

### `test/controllers/users/push_subscriptions_controller_test.rb`

- Deferred: create new push subscription — owner: WS17 continuation.
- Deferred: touch existing subscription — owner: WS17 continuation.
- Deferred: rejects subscription with non-permitted endpoint — owner: WS17 continuation.
- Deferred: rejects subscription with endpoint resolving to a private IP — owner: WS17 continuation.
- Deferred: re-registering a legacy invalid subscription is rejected with 422 — owner: WS17 continuation.
- Deferred: destroy a push subscription via dev mode — owner: WS17 continuation.

### `test/system/service_worker_test.rb`

- Deferred: the worker caches static assets and never authenticated responses — owner: WS17 continuation.
- Deferred: the offline shell renders with working retry behavior — owner: WS17 continuation.

### `test/system/status_notifications_test.rb`

- Deferred: setting presence and a custom status — owner: WS17 continuation.
- Deferred: enabling DND mutes sounds and persists quiet hours — owner: WS17 continuation.
- Deferred: chat sounds follow the live quiet-hours window without a reload — owner: WS17 continuation.
- Deferred: switching the theme applies without a reload flash — owner: WS17 continuation.
- Deferred: switching the text size rescales the page — owner: WS17 continuation.
- Deferred: button icons follow the manual theme, not the OS — owner: WS17 continuation.
- Deferred: the status form works at phone width — owner: WS17 continuation.

### `test/system/meeting_status_test.rb`

- Deferred: opting in shows In a meeting for a stubbed busy interval, then clears after it ends — owner: WS17 continuation.
- Deferred: the profile links to connect without a Google account — owner: WS17 continuation.

### `test/system/out_of_office_test.rb`

- Deferred: set OOO until tomorrow, badge and DM notice show for another user, then clear it — owner: WS17 continuation.

### `test/integration/ooo_dm_notice_test.rb`

- Deferred: a DM with an OOO member shows the notice above the composer — owner: WS17 continuation.
- Deferred: the notice escapes the member's note — owner: WS17 continuation.
- Deferred: the notice renders per viewer, never from a shared fragment — owner: WS17 continuation.
- Deferred: a group DM shows one line per OOO recipient — owner: WS17 continuation.
- Deferred: a channel shows no notice even while a member is out — owner: WS17 continuation.
- Deferred: a DM with nobody out shows no notice — owner: WS17 continuation.
- Deferred: an invisible member's manual OOO shows no notice — owner: WS17 continuation.
- Deferred: an invisible member's calendar OOO shows no notice — owner: WS17 continuation.
- Deferred: an invisible member's OOO flip broadcasts an emptied notice line — owner: WS17 continuation.
- Deferred: an OOO end broadcasts an emptied notice line — owner: WS17 continuation.

### `test/models/notifications/keyword_matcher_test.rb`

- Equivalent rerun: matches case-insensitively — `rust/crates/db/src/tests/keyword_alert_test.rs::matches_case_insensitively`.
- Equivalent rerun: matches on word boundaries only — `rust/crates/db/src/tests/keyword_alert_test.rs::matches_on_word_boundaries_only`.
- Equivalent rerun: matches multi-word phrases — `rust/crates/db/src/tests/keyword_alert_test.rs::matches_multi_word_phrases`.
- Deferred: treats phrases literally, not as patterns — owner: WS17 continuation.
- Equivalent rerun: returns every user with a match — `rust/crates/db/src/tests/keyword_alert_test.rs::returns_every_user_with_a_match`.
- Equivalent rerun: overlapping phrases across users all match — `rust/crates/db/src/tests/keyword_alert_test.rs::overlapping_phrases_across_users_all_match`.
- Equivalent rerun: nested phrases match the same user once — `rust/crates/db/src/tests/keyword_alert_test.rs::nested_phrases_match_the_same_user_once`.
- Equivalent rerun: repeated phrases match every holder once — `rust/crates/db/src/tests/keyword_alert_test.rs::repeated_phrases_match_every_holder_once`.
- Deferred: ignores blank phrases and blank text — owner: WS17 continuation.
- Deferred: a phrase matches across a line break and not inside a longer Unicode word — owner: WS17 continuation.

### `test/services/activity_items/recorder_keyword_test.rb`

- Deferred: a keyword match on a room message records a keyword alert — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a keyword match needs a word boundary — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: the author never matches their own keywords — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: an invisible member matches nothing but a notifications-off member matches — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a mention wins over a keyword match for the same message — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a thread keyword match reaches thread members only — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a muted thread member matches no keywords — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: thread activity wins over a keyword match for a follower — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a room keyword match loads only the matching members' memberships — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a room message keeps candidate queries flat as the roster grows — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: matching queries the keyword table a constant number of times as followers grow — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).

### `test/services/activity_items/recorder_test.rb`

- Deferred: records a mention for an opted-in active human and excludes the author — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a reply follows the reply author's current preference — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: mention takes precedence when one message matches multiple activity reasons — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: followed thread activity uses thread preferences — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: work events notify followed thread members — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a direct mention reaches a member with notifications off but not an invisible one — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a member with notifications off gets no reply but a mentions member does — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a member with notifications off gets no thread activity but a mentions member does — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a member with notifications off gets no work items but a mentions member does — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: changing involvement leaves existing items untouched — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: ten thread replies collapse into one thread activity item that reads unread again — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a status update after a work assignment keeps the assignment item and repoints the update item — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: work updates for one thread collapse into a single item — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a thread message that mentions and replies to a follower yields one mention item — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: work assigned by an agent honors the recipient's agent_work switch — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: work assigned by a bot without an agent ignores the agent_work switch — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a caller-authorized record skips the source check but keeps idempotency — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: recording the same source twice is idempotent — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: recording a thread message queries memberships a constant number of times as followers grow — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).
- Deferred: a root message loads only the mentionee and reply-author memberships — owner: WS17 continuation (keyword integration); WS12 (full recorder/lifecycle).

## Handoff state

Implementation and report are on `rust/ws17-push-presence`; no PR opened. No source outside the authorized worktree was edited. The only authorized external write is the requested report. Scratch logs, seeds, independent comparison checkout and build output remain under this worktree. See the final handoff reply for the documentation commit's pushed SHA; the implementation SHA above is the validated source boundary.
