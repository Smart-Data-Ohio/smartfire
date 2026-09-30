# WS7: cable, channels and broadcasts

Status: **review fixes complete; full brief acceptance remains partial for domain and HTML/pixel parity**. Branch `rust/ws7-cable`. Review implementation commits `5f56e8aa` and `e1d041e6`. Merged `origin/main` (`d2b21210`, including WS3, final WS4 #150, WS5 and Rails #148) in `a8ec54aef7dea0596b524df2fb6272c31a474bc9`. Reference pin `fec615be`; the private image is `ws7-reference:fec615be`. Push authorized by the user; no PR.

## Review corrections

1. The only merge conflict was `crates/campfire/src/jobs.rs`. Kept WS3's durable job queue, atomic persistence, registry, workers, periodic loops and shutdown, plus WS7's weak app reference and synchronous ordered Broadcast/DisconnectUser sink. The request-warmed-message regression GETs an existing message, renders it detached through the same fragment cache, verifies no CSRF field/token slot/nonce, and requires real socket delivery. WS4's merged tokenless cached forms and token-slot mechanism fix the contamination.
2. `crates/cable/src/turbo.rs` now parses HTML with html5ever and examines real start-tag attributes, including single/unquoted and entity-encoded attributes. Text, comments and raw-text elements do not trigger the guard. Real authenticity fields, CSRF meta tags, CSP nonce attributes/meta still fail closed. `channels/broadcasts.rs` additionally rejects actual unresolved WS4 token slots using the renderer's process-specific marker. The reviewer's exact HTTP body `<div>nonce="example"</div>` now reaches a real authorized socket.
3. `controllers/rooms/involvements.rs` supplies the real membership-specific direct-room partial on direct mute transitions. HTTP/socket tests require the actual row id and non-empty template. The broader HTTP test exercises real message, presentation, boost, shared-room and direct-room partial wiring. FakePartials remains a transport test double, not proof that controllers supply their partials. The audit found inherited shared membership/unread-local gaps, missing stage/voice/board rows, and inherited direct-row markup without the fork's muted class/profile controls. Contract entries 61–63 are corrected to API ready with explicit HTML limits; direct mute no longer deletes the row. Full fork sidebar markup remains WS6/WS8/WS12/WS13 work.
4. Repeated public `subscribed` actions are bounded to two receivers per subscription/stream. The Rails golden explicitly records a second delivery after one repeated action, so idempotence would break that evidence. A real-socket test sends 32 repeated actions, consumes exactly two frames and requires silence afterwards. The first two receivers preserve recorded Rails behavior; further duplicates are ignored.

The contract contains all **115** source primitives. The lead and independent reviewer confirmed the brief's 140 figure was incorrect. Current states: 15 ported envelopes/call paths, 18 API ready, 80 waiting on their domain, 2 dead. No parity masks or allowlists changed.

## Handoff and completed slices

The inherited WS7 commits were `5997d5d0`, `5b2f77a0`, `1312b563`, and `7faedd03`. The only unfinished useful artifact was the draft broadcast contract; checkpointed first in `34edd642`. The uncommitted `false && requires_two_factor(...)` was a mutation experiment, reproduced as a failing authorization check and restored before continuing. No stash, rebase, history rewrite, shared target directory, or changes to the Rails source.

The old close deadline checked only between calls to a ping-skipping reader, which could wait forever inside that reader. `e89ece47` and `b9a7e787` bound both ordinary frame reads and close waits across every ping, and add a real-socket test of the deadline. `b9a7e787` also regenerates both goldens from our Rails, extends the channel golden to 70 steps covering every registered channel and connection, checks thread start/stop, fixes strict clippy findings, and pins model removal-before-disconnect order. `f20e41d7` checks the broadcast contract against source.

Implemented connection behavior: signed Rails session cookie; current user plus session id; expired administrator-session destruction; the active-human two-factor gate; remote disconnect on membership destruction, sign-out, ban and deactivation. All five added channels are registered. Presence is nil-safe; thread typing rechecks room/thread membership for both start and stop; RoomMessages accepts Room/ChannelThread GIDs and guards both messages and threads suffixes. Room/user status, OOO and sidebar Turbo streams remain signature-only.

Workspace presence establishes a server-owned UUID lease, refreshes expiry (activity only for boolean true), handles pruned leases, expires idle administrator sessions on heartbeat, and deletes its own lease on unsubscribe. A rejection inside an action stops subsequent actions without inventing a rejection frame.

## Changes by file

Paths below are relative to `rust/`; this lists the full WS7 slice, including inherited commits.

- `crates/cable/src/turbo.rs`, `crates/cable/Cargo.toml`, `Cargo.lock`: structurally reject session-bearing attributes; add html5ever and lexical regression cases.
- `crates/cable/src/channel.rs`: cap duplicate stream receivers at two.
- `crates/cable/tests/protocol.rs`: real-socket coverage of that refusal.
- `crates/cable/tests/support/mod.rs`: optional worker-specific listening-port range.
- `crates/cable/tests/golden.rs`: replay/record harness, worker port support and recording instructions.
- `crates/cable/tests/golden/fixtures.rb`: mark the recording session verified before connecting.
- `crates/cable/tests/golden/reference.json`: regenerate protocol frames and the two signed stream names using our Rails keys.
- `crates/cable/BROADCASTS.md`: streams, targets, partials, owners, states, and complete source index.
- `crates/campfire/src/channels.rs`: register all channels and carry role/status/session identity.
- `crates/campfire/src/channels/activity.rs`: active-human activity stream.
- `crates/campfire/src/channels/agents.rs`: non-bot agents stream.
- `crates/campfire/src/channels/huddle_notice.rs`: active-human huddle notices.
- `crates/campfire/src/channels/unread_threads.rs`: per-user unread-thread stream.
- `crates/campfire/src/channels/workspace_presence.rs`: lease lifecycle and heartbeat expiry.
- `crates/campfire/src/channels/connection.rs`: session identification, expiry and two-factor gate.
- `crates/campfire/src/channels/presence.rs`: nil-safe membership callbacks.
- `crates/campfire/src/channels/room.rs`: shared room lookup/id casting.
- `crates/campfire/src/channels/room_messages.rs`: guarded thread/room streams; compact target enum.
- `crates/campfire/src/channels/threads.rs`: thread parent-room lookup and GlobalID helper.
- `crates/campfire/src/channels/typing_notifications.rs`: conversation-specific typing and action-time authorization.
- `crates/campfire/src/channels/broadcasts.rs`: typed streams/targets, pre-rendered HTML primitives, existing message/room fanout and reaction API.
- `crates/campfire/src/channels/sink.rs`: ordered model-event broadcasts, room removal and huddle-config predicate.
- `crates/campfire/src/channels/revocation.rs`: move model event delivery into the common cable sink.
- `crates/campfire/src/channels/tests.rs`: register authorization, reference and hub specs.
- `crates/campfire/src/app/tests.rs`: pending/verified Rails-cookie cable handshake over the assigned listener range; test-only access to the channel helper in `channels.rs`.
- `crates/campfire/src/channels/tests/support.rs`: fixture-backed real sockets, lease helpers and bounded frame/close reads.
- `crates/campfire/src/channels/tests/authorization_test.rs`: seven-principal authorization matrix and expired-row removal.
- `crates/campfire/src/channels/tests/reference_test.rs`: black-box equivalents of Rails channel assertions; both typing actions after revocation.
- `crates/campfire/src/channels/tests/hub_test.rs`: booted seeded app, model sink, job broadcasts, sign-out and deactivation.
- `crates/campfire/src/channels/tests/broadcasts_test.rs`: conversation targeting, unread filtering, sidebar fanout and JSON envelopes.
- `crates/campfire/src/channels/tests/channels_test.rs`: verified test sessions and updated identity assertions.
- `crates/campfire/src/channels/tests/golden.rs`: 70-step script, bounded collection, prefixed reference-container support and new broadcast triggers.
- `crates/campfire/src/channels/tests/golden/fixtures.rb`: Rails-issued cookies, pending/bot sessions, thread rows and signed names.
- `crates/campfire/src/channels/tests/golden/trigger.rb`: Rails-side mutations and broadcast markers.
- `crates/campfire/src/channels/tests/golden/reference.json`: fresh Rails rows and recorded frames for all channels.
- `crates/db/src/events.rs`: typed `Broadcast`/`BroadcastRequest` and additive `Event::Broadcast`.
- `crates/db/src/lib.rs`, `crates/db/src/models.rs`: exports for broadcasts and presence leases.
- `crates/db/src/models/membership.rs`: removal broadcast before connection reset, after commit.
- `crates/db/src/models/workspace_presence_lease.rs`: establish/refresh/delete and identity validity.
- `crates/db/src/tests/membership_test.rs`, `crates/db/src/tests/room_test.rs`: exact broadcast-before-disconnect assertions.
- `crates/campfire/src/jobs.rs`: synchronously deliver Broadcast/DisconnectUser through the hub sink in event order.
- `crates/campfire/src/controllers/messages.rs`, `controllers/messages/boosts.rs`, `controllers/rooms/opens.rs`, `controllers/rooms/closeds.rs`, `controllers/rooms/involvements.rs`: adapt existing callers to the broadcast API.
- `crates/campfire/src/controllers/presenters/page.rs`: broadcast sidebar partial hook, with current limitations documented.
- `crates/campfire/src/integrations/jobs.rs`: adapt message-removal broadcast calls.
- `reference-tools/cable/record.sh`: fresh private Rails database, ws7-prefixed containers, port 47040, source-hash verification and both recorders.
- `reference-tools/cable/check_mutations.py`: nineteen deliberate regressions; require failing tests and restore source in finally; terminate the process group on timeout.
- `reference-tools/cable/broadcast_contract.py`: enumerate source calls, check the contract and prove omissions/new calls/duplicates/unknown owners fail.
- `crates/cable/WS7_REPORT.md`: tracked copy of this handoff report.

## Design and integration notes

One process and one in-process hub, per decision 3. A model emits a typed BroadcastRequest only after its transaction commits. The current sink handles Membership room removal synchronously, before the queued remote-disconnect frame; controller/job callers use Broadcasts directly. New domain model broadcasts still need handlers registered in `channels/sink.rs`.

Rendering goes through `campfire_views` via request-independent presenters, or HTML supplied by the domain owner. No broadcast may contain session-bound markup. Transport tests pin action, target, stream and JSON escaping. Inherited or missing partials are **partial for HTML/pixel parity**; the contract's 15 ported entries describe implemented envelopes/call paths, not completed UI parity. There are 18 API-ready, 80 waiting-on-domain, and 2 dead source calls.

WS9 must mark `sessions.two_factor_verified_at` only on successful enrollment/challenge completion. The cable gate reads that column, and applies to every active human, including unenrolled humans. Until WS9's flows integrate, the existing Rust sign-in creates unverified human sessions that cable correctly refuses. Bot sessions do not require a second factor.

The seeded hub tests now boot the merged WS3 app and exercise its actual router, ordered event sink and ad hoc workers. Durable queue behavior is covered by the merged application/job suites. A durable domain job, periodic task or reconciler broadcasting to a socket is not separately proven here; that acceptance remains partial until its owner supplies such a test. No parity allowlists/masks were added or widened; the existing per-socket frame sorting is unchanged.

## Rails validations and callbacks

- WorkspacePresenceLease: connection_id presence/uniqueness (fresh UUID plus SQLite unique index), expires_at presence (TTL value), belongs_to user/session (active persisted user and matching persisted session plus FKs). Establish/refresh/delete are ported. Refresh matches update_columns: updated_at stays unchanged, expiry advances, activity advances only for boolean true. No reads/establish calls prune. Periodic prune and presence endpoint belong to WS17 and remain deferred.
- Membership: ported after_destroy_commit room-removal broadcast followed by reset_user_remote_connections; preserved existing direct-member-key refresh. Presence connection/read timestamp paths reuse inherited Connectable helpers. Stage/category validators and the update!-validation fallback in Connectable are not comprehensively ported here: WS2/WS8 own category/core validation; WS13 owns stage-only attributes, hands, mute, host-retention and grant/stage callbacks. Huddle/Agent grant revocation and last-stage-host handling defer to WS13/WS11; thread-membership removal to WS8; calendar synchronization to WS14. WS7 writes no new stage/category fields.
- Session: expired-session destruction reuses the existing destroy helper, including lease/setup-secret deletion. User existence, token generation, last_active_at initialization and verified timestamp are inherited WS4/model behavior; no new session-creation path. HuddleGrant before_destroy revocation remains WS13's responsibility. WS9 owns two-factor enrollment/challenge writes.
- Message/Room/User: no new production row-writing paths in this takeover. Existing core validations/callbacks remain WS2/WS8 responsibility; streaming/agent paths WS11. The broadcast contract explicitly identifies unported quote, reaction, thread, integration and sidebar-view behavior rather than claiming those model callbacks complete.

## Cross-workstream touches and remaining scope

Small existing controller/presenter hooks remain in WS8/WS6 paths; the direct involvement renderer is the only new production domain change in this follow-up. `jobs.rs` is a conflict resolution preserving WS3's queue and WS7's event sink, not a queue redesign. Broadcast events are still after commit; the durable job persistence path remains transactional. No new validators or production row-write paths in these review fixes.

Partial: full fork message/reaction/sidebar HTML (WS6/WS8); shared membership/unread locals and direct muted/profile/huddle markup (WS6/WS8); stage/voice rows (WS13), board rows (WS12); 80 remaining domain primitives and 18 API-ready entries with owners in BROADCASTS.md; periodic/reconciler socket delivery (WS3/WS13), WorkspacePresence prune/endpoint (WS17); WS9's verified-session flows; prior stage/grant/calendar model validations/callbacks listed above. The inherited `manages_bots` controller assertion is outside WS7. The involvement controller also still lacks Rails' mute-time persisted-unread clearing and JSON response behavior (WS8); no claim of full controller parity. No open question about the primitive count remains.

## Checks and raw summaries

All checks below were rerun in this review follow-up. Cargo commands ran in this worktree with `-j 4`, its own `rust/target`, `TMPDIR=/home/riels/.cache/rust-port/ws7/tmp`, and `CABLE_TEST_PORT_RANGE=47000-47039`. The reference port was 47040. Default and first_run seeds were present and migrated by the pinned Rails image; no silent seed skips. No release build. Complete raw logs live under `/home/riels/.cache/rust-port/ws7/evidence/`.

The initial full application run exposed a stale cable test expecting access from a pending session, and a stale first_run seed lacking WS3's migration. Updating the test to assert pending rejection followed by verified access, and migrating first_run, left exactly the known `manages_bots` failure. The production gate was not relaxed.

### WS7 channels

```sh
cargo test -j 4 -p campfire --bin campfire channels:: -- --nocapture
```

```text
test result: ok. 96 passed; 0 failed; 1 ignored; 0 measured; 178 filtered out; finished in 21.51s
```

96 ran; one Rails recorder ignored in the normal suite and explicitly executed below. All ten seeded hub tests ran. Both the original receiver deadline test and the new bounded-subscription test passed.

### Full application binary

```sh
cargo test -j 4 -p campfire --bin campfire -- --nocapture
```

```text
test result: FAILED. 272 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 22.12s
```

Exit 101: only `controllers::presenters::accounts::tests::manages_bots` failed, on the inherited plaintext bot-key assertion. 272 passed; the two ignored cases are the Rails recorder (run below) and `jobs::tests::push_latency` (an optional measurement). `channels::tests::golden::replays_reference_frames` passed, including revoke A. No filtered tests and no seed skips.

### Cable, database and durable jobs

```sh
cargo test -j 4 -p campfire_cable -p campfire_db -p campfire_jobs
```

```text
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 169 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 2.84s
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

In order: cable units 19, protocol golden replay 1 (recorder ignored here, explicitly run below), real-socket protocol 20, DB 169, durable queue 43, process-crash recovery 2, then three crates with zero doc tests. The three DB oracle/export tests are ignored and not re-executed in this follow-up; rollback/full Ruby DB differential acceptance remains partial here.

### Strict workspace clippy

```sh
cargo clippy -j 4 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.69s
```

Exit 0; no warnings. Excluded html5ever is the upstream/vendored dependency, per rust/AGENTS.md.

### Explicit application binary clippy

```sh
cargo clippy --manifest-path rust/Cargo.toml -j 4 -p campfire --bin campfire -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.69s
```

Ran from the worktree root. Exit 0; no warnings.

### Fresh Rails recording

```sh
WS7_SCRATCH=/home/riels/.cache/rust-port/ws7 bash reference-tools/cable/record.sh
```

```text
reference source hashes: 22 matched; 0 mismatched
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 274 filtered out; finished in 128.18s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 10.50s
reference recording: 70 channel steps and the cable protocol golden regenerated
```

Both ignored recorders ran. The image matches all 15 channel Ruby files, the lease model, Turbo authorization initializer, MessagesController, messages helper, cached boost/reaction forms and poll partial (22 files). Seventy channel steps plus cable frames were recorded from Rails, without hand-editing frames or signed names. The new recording changes fixture timestamps/cookies, with no normalization/allowlist changes. The script removed its ws7-reference-47040 container.

### Rails channel assertions

```sh
docker run --rm --name ws7-rails-channel-tests --cpus 2 --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/test:/rails/test:ro" -v /home/riels/.cache/rust-port/ws7/rails-tests/db:/rails/storage/db -v /home/riels/.cache/rust-port/ws7/rails-tests/files:/rails/storage/files -v /home/riels/.cache/rust-port/ws7/rails-tests/tmp:/rails/tmp -v /home/riels/.cache/rust-port/ws7/rails-tests/log:/rails/log ws7-reference:fec615be bin/rails test test/channels
```

```text
50 runs, 103 assertions, 0 failures, 0 errors, 0 skips
```

Ran from the worktree root. 50 tests, 103 assertions, no skipped tests; the container was removed.

### Broadcast inventory and injection checks

```sh
python3 reference-tools/cable/broadcast_contract.py --check --self-test
```

```text
broadcast contract: 115 source calls, 115 indexed, 0 missing, 0 extra
broadcast contract injection tests: 4 passed; 0 failed
```

Current merged Rails source: 115 primitives and 115 indexed, including refreshed MessagesController line numbers. Four injection checks prove omissions, duplicates, additions and unknown owners fail.

### Seed schema migration

Both commands ran from the worktree root against private copies, then copied the migrated database back to this worktree's ignored seed directory. Storage/media/labels and rows were preserved. No production database or shared seed was touched.

```sh
docker run --rm --name ws7-seed-migrate --cpus 2 --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v /home/riels/.cache/rust-port/ws7/seed-migration/db:/rails/storage/db -v /home/riels/.cache/rust-port/ws7/seed-migration/files:/rails/storage/files ws7-reference:fec615be bin/rails db:migrate
```

```text
== 20260929193000 CreateBackgroundJobs: migrated (0.0015s) ====================
```

```sh
docker run --rm --name ws7-first-run-migrate --cpus 2 --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v /home/riels/.cache/rust-port/ws7/first-run-migration/db:/rails/storage/db -v /home/riels/.cache/rust-port/ws7/first-run-migration/files:/rails/storage/files ws7-reference:fec615be bin/rails db:migrate
```

```text
== 20260929193000 CreateBackgroundJobs: migrated (0.0015s) ====================
```

## Failing-first evidence

Before production fixes, the reviewer's exact HTTP nonce input returned 200 but its real subscriber timed out, and direct mute emitted `<template></template>`. The initial repeated-action test received duplicates after its expected delivery. `review-before.log` also included an incidental warmed-message assertion comparing JSON escape spellings; that test was corrected to compare decoded HTML, and its token checks and delivery now pass. It was not a WS4 token-contamination failure after the merge.

The committed mutation harness independently restores each reviewed defect against the final socket tests: a nonce-text false positive, a missing direct-room partial, and removal of the receiver cap. All three fail for the intended runtime reason, not compilation failure. It also catches unresolved real token slots and missing real message partials, as well as the prior authorization/revocation checks. Every source mutation is restored in finally.

```sh
WS7_SCRATCH=/home/riels/.cache/rust-port/ws7 python3 reference-tools/cable/check_mutations.py
```

```text
mutation: two-factor gate
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.77s
mutation: expired session
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.71s
mutation: non-member
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 5.24s
mutation: bot
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.95s
mutation: banned
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.75s
mutation: deactivated
$ cargo test -j 4 -p campfire channels::tests::authorization_test::authorization_matrix
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.74s
mutation: inactive human
$ cargo test -j 4 -p campfire channels::tests::reference_test::activity_rejects_inactive_users
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.09s
mutation: typing thread parent
$ cargo test -j 4 -p campfire channels::tests::reference_test::typing_a_thread
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.09s
mutation: typing membership recheck
$ cargo test -j 4 -p campfire channels::tests::reference_test::typing_revoked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.16s
mutation: thread suffix guard
$ cargo test -j 4 -p campfire channels::tests::reference_test::thread_messages_the_stock
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.10s
mutation: workspace rejection
$ cargo test -j 4 -p campfire channels::tests::reference_test::workspace_presence_heartbeat_rejects
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.19s
mutation: workspace idle expiry
$ cargo test -j 4 -p campfire channels::tests::reference_test::workspace_presence_heartbeat_destroys
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.13s
mutation: sign-out disconnect
$ cargo test -j 4 -p campfire channels::tests::hub_test::signing_out
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 10.23s
mutation: golden frame difference
$ cargo test -j 4 -p campfire channels::tests::golden::replays_reference_frames
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 20.92s
mutation: nonce text false positive
$ cargo test -j 4 -p campfire channels::tests::hub_test::quote_text_post_delivers_to_socket
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 1.25s
mutation: direct mute missing partial
$ cargo test -j 4 -p campfire channels::tests::hub_test::direct_mute_keeps_the_rendered_sidebar_row
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.23s
mutation: repeated stream receivers
$ cargo test -j 4 -p campfire channels::tests::channels_test::performing_subscribed_bounds_stream_receivers
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.37s
mutation: unresolved token slot
$ cargo test -j 4 -p campfire channels::tests::hub_test::unresolved_token_slots_never_reach_a_socket
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.13s
mutation: real message partial
$ cargo test -j 4 -p campfire channels::tests::hub_test::http_broadcasts_supply_real_nonempty_partials
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 274 filtered out; finished in 0.30s
mutation checks: 19 caught; 0 survived
```

The initial close-deadline, auth matrix and callback failures from the earlier takeover remain historical handoff evidence; the current full suite and mutation harness above recheck their final behavior. No blanket full-suite success claim: the bot test and the explicit ignored/partial integrations remain as stated.
