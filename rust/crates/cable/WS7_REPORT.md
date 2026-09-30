# WS7: cable, channels and broadcasts

Status: **review fixes complete; full brief acceptance remains partial for domain and HTML/pixel parity**. Branch `rust/ws7-cable`. Earlier review implementation commits `5f56e8aa` and `e1d041e6`; the foreign-content fix was `da75e109`. This follow-up implements the lead's conservative-backstop decision after Astra's round-four probes against that commit. Merged `origin/main` (`d2b21210`, including WS3, final WS4 #150, WS5 and Rails #148) in `a8ec54aef7dea0596b524df2fb6272c31a474bc9`. Reference pin `fec615be`; the private image is `ws7-reference:fec615be`. Push authorized by the user; no PR.

## Review corrections

1. The only merge conflict was `crates/campfire/src/jobs.rs`. Kept WS3's durable job queue, atomic persistence, registry, workers, periodic loops and shutdown, plus WS7's weak app reference and synchronous ordered Broadcast/DisconnectUser sink. The request-warmed-message regression GETs an existing message, renders it detached through the same fragment cache, verifies no CSRF field/token slot/nonce, and requires real socket delivery. WS4's merged tokenless cached forms and token-slot mechanism fix the contamination.
2. `crates/cable/src/turbo.rs` uses html5ever's fragment tree builder in an HTML `<body>` context and examines DOM element attributes in both scripting modes. It traverses SVG, MathML and template contents, then applies a conservative raw-source attribute backstop if neither tree flags a value. HTML textarea/title/script text, comments and escaped message text do not trigger the guard. The existing field/meta/nonce checks are retained; the raw backstop also refuses suspicious name/nonce attributes that the tree builder discards. `channels/broadcasts.rs` additionally rejects actual unresolved WS4 token slots using the renderer's process-specific marker. The reviewer's exact HTTP body `<div>nonce="example"</div>` now reaches a real authorized socket.
3. `controllers/rooms/involvements.rs` supplies the real membership-specific direct-room partial on direct mute transitions. HTTP/socket tests require the actual row id and non-empty template. The broader HTTP test exercises real message, presentation, boost, shared-room and direct-room partial wiring. FakePartials remains a transport test double, not proof that controllers supply their partials. The audit found inherited shared membership/unread-local gaps, missing stage/voice/board rows, and inherited direct-row markup without the fork's muted class/profile controls. Contract entries 61–63 are corrected to API ready with explicit HTML limits; direct mute no longer deletes the row. Full fork sidebar markup remains WS6/WS8/WS12/WS13 work.
4. Repeated public `subscribed` actions are bounded to two receivers per subscription/stream. The Rails golden explicitly records a second delivery after one repeated action, so idempotence would break that evidence. A real-socket test sends 32 repeated actions, consumes exactly two frames and requires silence afterwards. The first two receivers preserve recorded Rails behavior; further duplicates are ignored.

5. Per the lead decision, this guard is our own conservative addition, with no Rails equivalent. Both scripting modes and a raw-source backstop now reject the `noscript`, select/style nonce and both select/CSRF-meta probes. The select/title input is deliberately refused, pinned by `session_bound_select_title_is_an_intentional_conservative_refusal`. No claim of exact browser-parser conformance.

The contract contains all **115** source primitives. The lead and independent reviewer confirmed the brief's 140 figure was incorrect. Current states: 15 ported envelopes/call paths, 18 API ready, 80 waiting on their domain, 2 dead. No parity masks or allowlists changed.

## Handoff and completed slices

The inherited WS7 commits were `5997d5d0`, `5b2f77a0`, `1312b563`, and `7faedd03`. The only unfinished useful artifact was the draft broadcast contract; checkpointed first in `34edd642`. The uncommitted `false && requires_two_factor(...)` was a mutation experiment, reproduced as a failing authorization check and restored before continuing. No stash, rebase, history rewrite, shared target directory, or changes to the Rails source.

The old close deadline checked only between calls to a ping-skipping reader, which could wait forever inside that reader. `e89ece47` and `b9a7e787` bound both ordinary frame reads and close waits across every ping, and add a real-socket test of the deadline. `b9a7e787` also regenerates both goldens from our Rails, extends the channel golden to 70 steps covering every registered channel and connection, checks thread start/stop, fixes strict clippy findings, and pins model removal-before-disconnect order. `f20e41d7` checks the broadcast contract against source.

Implemented connection behavior: signed Rails session cookie; current user plus session id; expired administrator-session destruction; the active-human two-factor gate; remote disconnect on membership destruction, sign-out, ban and deactivation. All five added channels are registered. Presence is nil-safe; thread typing rechecks room/thread membership for both start and stop; RoomMessages accepts Room/ChannelThread GIDs and guards both messages and threads suffixes. Room/user status, OOO and sidebar Turbo streams remain signature-only.

Workspace presence establishes a server-owned UUID lease, refreshes expiry (activity only for boolean true), handles pruned leases, expires idle administrator sessions on heartbeat, and deletes its own lease on unsubscribe. A rejection inside an action stops subsequent actions without inventing a rejection frame.

## Changes by file

Paths below are relative to `rust/`; this lists the full WS7 slice, including inherited commits.

- `crates/cable/src/turbo.rs`: preserve the iterative DOM walk in two scripting modes and add an independent source-tag/attribute scan. Four prior SVG/MathML regressions remain; six new tests cover noscript, both CSRF meta names within select, select/style nonce, an intentional select/title refusal, and raw attributes on arbitrary tags (including duplicates, case, quotes, slash separators and character references). Regressions check bare fragments and Turbo templates. Existing text controls remain green.
- `Cargo.toml`, `crates/cable/Cargo.toml`, `Cargo.lock`: add `markup5ever_rcdom` 0.35 (resolved to `0.35.0+unofficial`), matching the existing html5ever 0.35. The DOM exists only during the guard check; broadcasts still send the original bytes.
- `crates/cable/src/channel.rs`: cap duplicate stream receivers at two.
- `crates/cable/tests/protocol.rs`: real-socket refusal of SVG/MathML, noscript and select token/nonce payloads, including the intentional select/title refusal. Existing exact-delivery assertions for ordinary text and textarea/title controls remain.
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

Small existing controller/presenter hooks remain in WS8/WS6 paths; the direct involvement renderer was the only production domain change in the earlier review follow-up. The previous parser follow-up added shared dependency declarations/lockfile. This round changes only cable and the report; no new dependency or cross-workstream production change. `jobs.rs` is a conflict resolution preserving WS3's queue and WS7's event sink, not a queue redesign. Broadcast events are still after commit; the durable job persistence path remains transactional. No new validators or production row-write paths in these review fixes.

Partial: full fork message/reaction/sidebar HTML (WS6/WS8); shared membership/unread locals and direct muted/profile/huddle markup (WS6/WS8); stage/voice rows (WS13), board rows (WS12); 80 remaining domain primitives and 18 API-ready entries with owners in BROADCASTS.md; periodic/reconciler socket delivery (WS3/WS13), WorkspacePresence prune/endpoint (WS17); WS9's verified-session flows; prior stage/grant/calendar model validations/callbacks listed above. The inherited `manages_bots` controller assertion is outside WS7. The involvement controller also still lacks Rails' mute-time persisted-unread clearing and JSON response behavior (WS8); no claim of full controller parity. No open question about the primitive count remains.

## Conservative guard design notes

This guard is port-owned; Rails has no equivalent. It is a conservative backstop rather than a promise of exact browser parsing. `session_bound` returns the first reason found by the existing tree walk with scripting enabled, the same walk with scripting disabled, or the source scan. Disabling scripting exposes noscript children. Both walks explicitly visit template contents and foreign elements.

The independent source scan inspects literal `<` followed by an ASCII letter up to the next `>`. It refuses any tag's `name` equal to `authenticity_token`, `csrf-token` or `csrf-param`, or any non-empty `nonce`. Attribute names are case-insensitive; quoted/unquoted values and duplicate attributes are inspected directly. Only character-reference decoding uses html5ever's attribute lexer, with one synthetic value: element insertion rules, duplicate filtering and EOF recovery cannot discard source attributes first.

To satisfy the lead's requirement to keep the existing text controls green, clearly terminated comments and ordinary HTML textarea/title/script text are exempted from the raw backstop. Text exemptions do not apply within select or foreign content, or for self-closing text tags; ambiguous markup stays eligible for refusal. This is a deliberate conservative boundary, not another browser-conformance implementation. Escaped `&lt;input...` and text `name=&quot;authenticity_token&quot;` remain safe, as do words inside attribute values and empty nonce attributes.

`<select><title><input name=authenticity_token></title></select>` is an intentional over-refusal: current browsers treat it as title text, but our templates never emit it. The named test checks both the public guard and source backstop so this refusal does not depend on html5ever's obsolete select rules. The old guard already refused this probe; it was not a missing detection and is not claimed as failing-first evidence.

Broadcasts still send the original HTML bytes. Both Rails golden JSON files are byte-for-byte unchanged from `da75e109` and both replays pass. No new row writes, Rails validations/callbacks, schema migration, parity masks or allowlists. The existing domain deferrals above remain partial.

## Checks and raw summaries

All commands below ran in this worktree's `rust/` directory with its own target/cache and the assigned socket ports. Seeds were present, all ten seeded hub tests ran and no seed-dependent tests silently skipped. The previous broader takeover/merge verification is preserved in report history at `ef1f21d7`; the prior foreign-content verification is preserved at `da75e109`. Rails recording/assertions, DB/jobs, contract injection and nineteen mutations were not rerun for this guard-only follow-up; the committed Rails goldens were replayed without regenerating them. Raw logs are in `/home/riels/.cache/rust-port/ws7/evidence/`.

### Failing-first evidence

Before production changes, the noscript input, select/style nonce and both select/CSRF-meta probes returned `None`. The arbitrary-tag attribute backstop test also returned `None`: five failing detection tests. The deliberate select/title refusal and all existing controls passed. The real socket test accepted the noscript token fragment with one recipient instead of refusing it. All reviewer probes are covered by the committed tests; the known intentional over-refusal is explicitly documented rather than represented as a new failure.

### Before the fix: unit regressions

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target CABLE_TEST_PORT_RANGE=47000-47039 cargo test -j 4 -p campfire_cable --lib session_bound_ -- --nocapture
```

```text
test result: FAILED. 8 passed; 5 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.00s
```

Exit 101: five failed, eight passed, seventeen unrelated units filtered. The intentional select/title refusal already passed; both prior text-control tests passed. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-failing-first-unit.log`.

### Before the fix: real socket

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target CABLE_TEST_PORT_RANGE=47000-47039 cargo test -j 4 -p campfire_cable --test protocol broadcasts_carrying_session_bound_markup_are_refused -- --nocapture
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s
```

Exit 101. A real subscriber was registered; the noscript fragment returned one recipient, expected zero. Nineteen unrelated protocol tests filtered. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-failing-first-socket.log`.

### After the fix: complete cable suite

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target CABLE_TEST_PORT_RANGE=47000-47039 cargo test -j 4 -p campfire_cable
```

```text
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Exit 0, no warnings. In order: 30 units, one protocol golden replay (recorder ignored), 20 real-socket protocol tests, zero doc tests. All six new units and existing text controls passed. The socket test proves refusal of every reviewer payload, with the previous exact delivery assertions still passing. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-cable.log`.

### After the fix: full application binary

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target CABLE_TEST_PORT_RANGE=47000-47039 cargo test -j 4 -p campfire --bin campfire -- --nocapture
```

```text
test result: FAILED. 272 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 22.18s
```

Exit 101, no warnings. Only `controllers::presenters::accounts::tests::manages_bots` fails on the inherited bot-key assertion (WS11). `channels::tests::golden::replays_reference_frames` passes, including revoke A; all 96 channel tests pass. Two ignored tests are the Rails recorder and optional job latency measurement. No filtered tests or seed skips. Full application acceptance remains partial because of the known bot test. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-campfire-bin.log`.

### Strict workspace clippy

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target cargo clippy -j 4 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.45s
```

Exit 0, no warnings. Excluded workspace html5ever is the vendored dependency, per rust/AGENTS.md. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-clippy.log`.

### Explicit application binary clippy

```sh
TMPDIR=/home/riels/.cache/rust-port/ws7/tmp CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws7/rust/target cargo clippy -j 4 -p campfire --bin campfire -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.20s
```

Exit 0, no warnings. Raw log: `/home/riels/.cache/rust-port/ws7/evidence/conservative-guard-clippy-bin.log`.
