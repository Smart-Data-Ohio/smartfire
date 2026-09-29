# WS7: cable, channels and broadcasts

Status: **partial for full brief acceptance**. Channel/authentication/transport slices are implemented and verified. Durable-job, periodic/reconciler integration and complete domain/view broadcast parity remain for integration with their owners. Branch: `rust/ws7-cable`. Reference pin: `79b45383`. No PR opened; the user explicitly authorized pushing this branch.

## Handoff and completed slices

The inherited WS7 commits were `5997d5d0`, `5b2f77a0`, `1312b563`, and `7faedd03`. The only unfinished useful artifact was the draft broadcast contract; checkpointed first in `34edd642`. The uncommitted `false && requires_two_factor(...)` was a mutation experiment, reproduced as a failing authorization check and restored before continuing. No stash, rebase, history rewrite, shared target directory, or changes to the Rails source.

The old close deadline checked only between calls to a ping-skipping reader, which could wait forever inside that reader. `e89ece47` and `b9a7e787` bound both ordinary frame reads and close waits across every ping, and add a real-socket test of the deadline. `b9a7e787` also regenerates both goldens from our Rails, extends the channel golden to 70 steps covering every registered channel and connection, checks thread start/stop, fixes strict clippy findings, and pins model removal-before-disconnect order. `f20e41d7` checks the broadcast contract against source.

Implemented connection behavior: signed Rails session cookie; current user plus session id; expired administrator-session destruction; the active-human two-factor gate; remote disconnect on membership destruction, sign-out, ban and deactivation. All five added channels are registered. Presence is nil-safe; thread typing rechecks room/thread membership for both start and stop; RoomMessages accepts Room/ChannelThread GIDs and guards both messages and threads suffixes. Room/user status, OOO and sidebar Turbo streams remain signature-only.

Workspace presence establishes a server-owned UUID lease, refreshes expiry (activity only for boolean true), handles pruned leases, expires idle administrator sessions on heartbeat, and deletes its own lease on unsubscribe. A rejection inside an action stops subsequent actions without inventing a rejection frame.

## Changes by file

Paths below are relative to `rust/`; this lists the full WS7 slice, including inherited commits.

- `crates/cable/src/turbo.rs`: refuse canonical CSRF fields/meta and nonempty CSP nonces in broadcast markup.
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
- `reference-tools/cable/check_mutations.py`: fourteen deliberate regressions; require failing tests and restore source in finally; terminate the process group on timeout.
- `reference-tools/cable/broadcast_contract.py`: enumerate source calls, check the contract and prove omissions/new calls/duplicates/unknown owners fail.
- `crates/cable/WS7_REPORT.md`: tracked copy of this handoff report.

## Design and integration notes

One process and one in-process hub, per decision 3. A model emits a typed BroadcastRequest only after its transaction commits. The current sink handles Membership room removal synchronously, before the queued remote-disconnect frame; controller/job callers use Broadcasts directly. New domain model broadcasts still need handlers registered in `channels/sink.rs`.

Rendering goes through `campfire_views` via request-independent presenters, or HTML supplied by the domain owner. No broadcast may contain session-bound markup. Transport tests pin action, target, stream and JSON escaping. Inherited or missing partials are **partial for HTML/pixel parity**; the contract's 18 ported entries describe implemented envelopes/call paths, not completed UI parity. There are 15 API-ready, 80 waiting-on-domain, and 2 dead source calls.

WS9 must mark `sessions.two_factor_verified_at` only on successful enrollment/challenge completion. The cable gate reads that column, and applies to every active human, including unenrolled humans. Until WS9's flows integrate, the existing Rust sign-in creates unverified human sessions that cable correctly refuses. Bot sessions do not require a second factor.

The seeded hub tests exercise the inherited in-memory runner and actual app router, not the future WS3 durable runner. No instruction declaring WS3 final or requesting its merge was received; it was not merged. The sink additions in jobs.rs must survive that integration. No parity allowlists/masks were added or widened; the existing per-socket frame sorting is unchanged.

## Rails validations and callbacks

- WorkspacePresenceLease: connection_id presence/uniqueness (fresh UUID plus SQLite unique index), expires_at presence (TTL value), belongs_to user/session (active persisted user and matching persisted session plus FKs). Establish/refresh/delete are ported. Refresh matches update_columns: updated_at stays unchanged, expiry advances, activity advances only for boolean true. No reads/establish calls prune. Periodic prune and presence endpoint belong to WS17 and remain deferred.
- Membership: ported after_destroy_commit room-removal broadcast followed by reset_user_remote_connections; preserved existing direct-member-key refresh. Presence connection/read timestamp paths reuse inherited Connectable helpers. Stage/category validators and the update!-validation fallback in Connectable are not comprehensively ported here: WS2/WS8 own category/core validation; WS13 owns stage-only attributes, hands, mute, host-retention and grant/stage callbacks. Huddle/Agent grant revocation and last-stage-host handling defer to WS13/WS11; thread-membership removal to WS8; calendar synchronization to WS14. WS7 writes no new stage/category fields.
- Session: expired-session destruction reuses the existing destroy helper, including lease/setup-secret deletion. User existence, token generation, last_active_at initialization and verified timestamp are inherited WS4/model behavior; no new session-creation path. HuddleGrant before_destroy revocation remains WS13's responsibility. WS9 owns two-factor enrollment/challenge writes.
- Message/Room/User: no new production row-writing paths in this takeover. Existing core validations/callbacks remain WS2/WS8 responsibility; streaming/agent paths WS11. The broadcast contract explicitly identifies unported quote, reaction, thread, integration and sidebar-view behavior rather than claiming those model callbacks complete.

## Checks and raw summaries

Commands below were rerun in this takeover. Cargo commands use `rust/` as the working directory, TMPDIR at the worktree's `.scratch/tmp`, CARGO_TARGET_DIR at this worktree's `rust/target`, and CABLE_TEST_PORT_RANGE=47000-47039. The initial failing handoff matrix used its inherited OS-assigned listener before worker-range support was added; subsequent WebSocket checks used the assigned range. The existing default parity seed was present; --nocapture confirms zero silent seed skips. No release builds.

```sh
CABLE_TEST_PORT_RANGE=47000-47039 TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" cargo test -j 4 -p campfire channels:: -- --nocapture
```

```text
test result: ok. 90 passed; 0 failed; 1 ignored; 0 measured; 161 filtered out; finished in 21.56s
```

90 ran; 1 recorder ignored in the normal suite, explicitly run below; 161 other application tests filtered out. All four seeded hub tests ran.

```sh
CABLE_TEST_PORT_RANGE=47000-47039 TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" cargo test -j 4 -p campfire_cable -p campfire_db
```

```text
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 169 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Cable: 18 unit, 1 replay and 20 protocol tests ran; its recorder was separately executed. Database: 169 ran; 3 reference differential/fixture/export tests ignored and remain unproven in this session. Both doctest suites contain zero tests.

```sh
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" cargo clippy -j 4 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.36s
```

Exit 0, no warnings. Four narrow dead-code allowances document APIs awaiting WS8/WS14 callers; no broad clippy suppression.

```sh
bash rust/reference-tools/cable/record.sh  # worktree root
```

```text
reference source hashes: 17 matched; 0 mismatched
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out; finished in 131.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 9.97s
reference recording: 70 channel steps and the cable protocol golden regenerated
```

Both normally ignored recorders ran. The pinned image is tagged `ws7-reference:79b45383`; the script verified all 15 channel Ruby files plus the lease model and Turbo initializer against this checkout, and removed its own container afterwards. Signed values came from our Rails rather than being edited by hand.

The Rails black-box source suite also ran in the pinned image, with test sources mounted read-only:

```sh
docker run --rm --name ws7-rails-channel-tests --cpus 2 --user "$(id -u):$(id -g)" --env-file rust/parity/.env.reference -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -e RAILS_LOG_LEVEL=warn -e PARITY_REDIS=1 -v "$PWD/test:/rails/test:ro" -v "$PWD/.scratch/rails-tests/db:/rails/storage/db" -v "$PWD/.scratch/rails-tests/files:/rails/storage/files" -v "$PWD/.scratch/rails-tests/tmp:/rails/tmp" -v "$PWD/.scratch/rails-tests/log:/rails/log" ws7-reference:79b45383 bin/rails test test/channels
```

```text
50 runs, 103 assertions, 0 failures, 0 errors, 0 skips
```

The root channel tests account for the brief's 41; including the nested connection tests gives 50. The Rust reference specs cover these assertions through real sockets, with SQL where Rails bypasses callbacks; a nil stub session is represented by a session deleted after opening the real socket.

```sh
python3 reference-tools/cable/broadcast_contract.py --check --self-test
```

```text
broadcast contract: 115 source calls, 115 indexed, 0 missing, 0 extra
broadcast contract injection tests: 4 passed; 0 failed
```

## Tests shown failing first / against broken implementations

The pinging-socket regression failed against the inherited deadline before the fix:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 11.11s
```

The inherited two-factor bypass failed the matrix (`TwoFactorPending` connected). The broader database suite also exposed stale event assertions before they were corrected:

```text
test result: FAILED. 167 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 2.98s
```

The committed mutation checker then demonstrated all security/frame gates failing against deliberately broken implementations:

```sh
TMPDIR="$PWD/../.scratch/tmp" CARGO_TARGET_DIR="$PWD/target" CABLE_TEST_PORT_RANGE=47000-47039 python3 reference-tools/cable/check_mutations.py
```

```text
mutation: two-factor gate
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.71s
mutation: expired session
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.88s
mutation: non-member
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 5.27s
mutation: bot
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.81s
mutation: banned
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.85s
mutation: deactivated
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.84s
mutation: inactive human
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.11s
mutation: typing thread parent
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.11s
mutation: typing membership recheck
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.13s
mutation: thread suffix guard
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.11s
mutation: workspace rejection
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.17s
mutation: workspace idle expiry
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 0.14s
mutation: sign-out disconnect
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 10.26s
mutation: golden frame difference
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 251 filtered out; finished in 20.91s
mutation checks: 14 caught; 0 survived
```

Each mutation restored its source before proceeding. The final positive tests and clippy ran after the last restoration. An earlier non-member mutation exposed the same ping-related hang in ordinary frame reads; those reads now have an overall deadline too. The mutation subprocess timeout was fixed to terminate its process group, and the earlier leftover test process was terminated by exact PID.

## Cross-workstream touches

Inherited WS7 work touched DB events/models/exports, jobs.rs, controller broadcast call sites, page partial adapters and integrations/jobs.rs. This takeover additionally adjusted only two DB event assertions, added reference-tools/cable verification utilities, and wrote the explicitly requested shared report. No Rails source, migrations, schema, dependency versions, parity masks or other worktrees were changed. The tracked report mirrors the shared handoff path.

## Open questions and precise remaining work

1. **Upstream WS4 and WS3/WS13 integration — partial:** the local WS4 remote-tracking ref has four additional commits not merged here: `09fe9c09` (cached message tokens), `09ca2872` (bounded/rate-limited CSP bodies), `061b7d44` (streamed body limit), and `1e59cc8d` (whole-body validation). Integrate them under the lead's merge order, especially the cached-token fix before claiming rendered-broadcast parity. For WS3, when the lead declares WS3 final, merge its branch with a merge commit, retain synchronous Broadcast/DisconnectUser delivery, and prove broadcasts through the real durable runner, a registered periodic task and the actual huddle reconciler reach sockets. Current tests prove the inherited runner and shared hub, not those future hosts.
2. **WS6/WS8/WS11–15 domain/view broadcasts — partial:** connect the 15 API-ready and 80 waiting entries in BROADCASTS.md and port their exact partials/HTML. In particular WS8 must replace the inherited boost append/remove controller behavior with the reference reaction replacement, implement thread/quote/pin/poll callbacks, and integrate missing room header/sidebar variants. Stage/voice/board sidebar HTML still waits on WS6/WS12/WS13. Domain model BroadcastRequests need explicit sink handlers.
3. **WS9 — integration pending:** complete enrollment/challenge flows and only stamp the session after verification; retain the implemented cable gate.
4. **WS17 — domain pending:** wire prune and the presence endpoint to the lease contract; channel-side lease handling is implemented.
5. **WS2/WS8/WS11/WS13/WS14 — model callbacks/validation partial:** complete the named Membership/Session/dependent-domain obligations above. This slice does not claim complete Rails validation/callback parity for stage/category/general fallback membership saves.
6. **Inventory count:** direct source enumeration produces 115 primitive sites, versus the brief's 140. The lead should reconcile the inventory definition/reference snapshot. All 115 actual sites are indexed with zero omissions; no fictional sites were added.
7. **Unproven checks:** the three ignored DB reference differential/export checks, full application/workspace tests, two-user rendered-partial parity, all UI/pixel parity, and runtime paths listed above. The preserved shared-main `manages_bots` issue remains WS11's responsibility; it was not exercised by this focused run.

Local evidence is under this worktree's `.scratch/evidence/`. The implementation is ready to integrate as coherent tested slices; full WS7 acceptance remains partial for the explicit dependencies above.
