# WS13 Wave 4 report — partial, grants and gateway slice

Branch: `rust/ws13-huddles`. Reference: our Rails at `d7c7de92`. Merged `origin/main` at `21a7332f` with merge commit `a8b6538c` (no rebase). Pushed source slices: `d90687c6` and `c7ea133dbd2b8e0c06f94021059d65c5a3b3fb10`. The final handoff commit also adds the exact 20-second Rails sighting vector, fixes that test's assumption about the latest stored sighting, and updates these reports. No PR or deployment.

## Status and restart point

This completes a coherent **grant authorization/revocation and internal gateway protocol slice**, not all WS13 acceptance or all effects of step 1. Six revocation triggers write the revoked state and cleanup snapshot in the triggering transaction. Internal authorize/show/left are now real Rust endpoints and the gateway's own 16 Node tests pass against them.

First-sighting `Huddle::BroadcastPresenceJob` and `Huddle::JoinNoticeJob` are persisted atomically. Their handlers are not registered yet: the existing queue uses the default queue for unknown classes, and the runner marks them failed with their rows retained. Handler registration must also retry those failed rows. The gateway commit message used the inaccurate shorthand "unregistered queue"; this is the actual behavior. Presence/leave/call-ended/stream rendering and invitation fan-out remain unfinished. No dummy successful handler was added. Public huddle issuance controllers remain unported, so the authorization-only `HuddleGrant::issue` method is not yet exposed as a complete join flow.

Restart with step 2: add the Stream model and stale-stream pass, the full reconciler ordering and overdue invitation resolver, last-host departure/successor handling, and hand/moderation domain policies. Then complete notice/push handlers and post-issuance invitations before wiring the public controllers and 19 views.

## Changes by file

| Files under rust/ | Change |
| --- | --- |
| `crates/db/src/models/huddle_grant.rs`, `models.rs` | Typed grant reads, exact payload, current relationship authorization, scoped transactional issuance, 256-bit random participant identity, active reuse, requested-room eligibility, at most three unique-conflict attempts with savepoints, 20-second call window, 10-second persisted sightings, first-sighting jobs, disconnect floor, participants/identity readers, scoped revocations and cleanup snapshots. Stage's last-active-grant stream state changes synchronously in that transaction. |
| `crates/db/src/models/membership.rs` | Before-destroy grant revocation and stream-state end; validated stage-role and server-mute changes. Listener boundary crossings revoke; host-speaker transitions retain identity and bulk-update role without updated_at. Last-host demotion guard runs in the immediate transaction. |
| `crates/db/src/models/session.rs`, `user.rs` | Sign-out revocation before deleting sessions; the Rails non-active status-update callback revokes user grants, covering ban and deactivation. |
| `crates/db/src/models/room_delete.rs` | Replace WS8a's duplicate grant revocation routine with the shared model. Preserve begin_destroy/DestroyJob sequencing, one idempotent room cleanup and no per-participant cleanups for room revocation. Existing cleanup unlinking is retained. |
| `crates/db/src/tests/huddle_grant_test.rs`, `tests.rs`, `models/huddle_grant_vectors.json` | 13 real SQLite tests, two writer handles/threads for issuance, real trigger-generated unique conflicts, six revocation paths, 16 Rails authorization states and 8 sighting snapshots, including the exact 20-second boundary. |
| `crates/campfire/src/controllers/internal_huddle.rs`, `controllers.rs` | Route authorize/show/left; shared gateway secret check uses existing subtle-backed secure_compare, configuration ordering, verified token coordinates, 401/403/404/503 and left 422, exact JSON/no-store, read-only steady state, locked invalid/due-sighting writes, record_seen=0, numeric-prefix IDs and timestamp floors. API actions omit the session/CSRF chain like Rails. |
| `crates/campfire/src/config.rs`, `huddle.rs` | Boot/call configuration boundary and redacted Debug for huddle config; no new dependency. |
| `crates/campfire/src/controllers/internal_huddle_tests.rs`, `presenters/test_support.rs`, `huddle/gateway_vectors.json` | 8 regular new app tests (including the small integer-cast test in the controller module), one explicitly invoked Node launcher, private seeded apps and clocks, 39 real Rails HTTP cases, invalid-grant enforcement and real HTTP queue rejection rollback. Test-only Rust controls/servers never mount in production. |
| `reference-tools/huddle_grants.rb`, `huddle_gateway.rb` | Call our real pinned models/controllers. Only external enqueue effects are captured. Gateway cases commit normally so cleanup after-commit effects are measured; an earlier rollback-based oracle was corrected before accepting vectors. |
| `reference-tools/huddle_gateway_node.mjs` | Load-time test harness adapter for the unchanged gateway source and unchanged test assertions. Fixture tokens/coordinates come from Rails. Ordinary authorize/check/left requests forward to real Rust over TCP. Saved-token/revocation injections revoke real SQLite grants; outage, stall and malformed body injections stay at the proxy. Rust and Node fixture sockets use only 52300-52399. |
| `reference-tools/huddle_discrimination.py` | Extend the existing 10 compiled probes with secret bypass, revoked-grant acceptance, removed-member acceptance, revocation bypass and disconnect-floor bypass. All 15 produce assertion failures and restore their sources. |
| `plans/ws13-deferred-tests.md`, this report | Per-declaration deferred/partial traceability and the current handoff. |

The prior pushed token/Twirp/cleanup slice remains intact: 7 fixed-time join tokens, 98 verification shapes, 15 URL cases, both admin tokens, and 14 cleanup states. Its existing cleanup worker and cleanup-only in-process scheduler remain registered. No new schema, migrations, dependency keys, lockfile edits, assets, templates, Rails changes, sidecar changes, parity masks or allowlists.

## Data and callback audit

HuddleGrant validates identity/room_name presence and identity uniqueness. Its optional associations are checked by issuance/authorization; schema-required IDs remain required. Authorization verifies active human user, session ownership, matching membership user/room, an alive room, matching stage role on stage rooms and matching server mute. Revoked grants never authorize or get reused. Every invalid grant is re-read under the immediate write transaction before revocation; the ordinary authorized check stays read-only unless a sighting is due.

Issuance re-reads the requested session/membership/room, revokes a mismatched membership's old grant, and revokes only other-room grants of that session still within the in-call window. Partial unique indexes plus BEGIN IMMEDIATE serialize competing issuances. A trigger creating a real identity conflict proves three bounded attempts and rollback without leaked grant rows.

Issuance/revocation stamp updated_at. Host-speaker role updates, seen and left use Rails bulk/update_columns semantics and preserve updated_at. Seen enqueues presence and, unless another active grant of the same user is in call, a join notice in that same transaction. Rejecting the second enqueue through the actual HTTP path returns 500 and rolls back both last_seen_at and the first job row. A disconnect cannot erase a newer sighting, and leaving keeps authorization active.

Membership call updates validate stage-only role/hand columns, listener-only raised hands, mute only on stage/voice rooms, category ownership, current room/user and the last-host guard before revoking anything. Stream ends here preserve Rails' quality validation and synchronous last-grant state; full Stream creation, other lifecycle callbacks, ended broadcasts and succession remain step 2/4. The non-active User update callback also protects deactivation, but its broader hostless-stage succession remains unfinished.

The new first-sighting jobs remain durably visible, but unknown-handler execution fails on the default queue. Their real handlers and replay of failed rows remain required. Issuance invitation handling and revocation/leave/call-ended/voice/stream delivery are not claimed. The module comments and deferred catalogue identify these boundaries.

## Verification

All commands below ran in this worktree with mise Rust 1.98.1, -j 4 and rust/target. Seeded app runs set CI=1 so a missing actual seed fails. Both default and first_run seeds were present at the reference pin. The one printed missing-seed message is the intentional missing_seed_may_skip_locally helper test, not an actual controller/channel suite skip.

Post-merge locked metadata ran immediately and was repeated from rust/:

```sh
cd rust
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Exit 0, no lockfile change. Parsing rust/Cargo.toml with Python tomllib also succeeds (duplicate keys would fail):

```text
Workspace dependency keys: 75 unique; no duplicates
```

Immediately after the merge, seeded app summary was `test result: ok. 319 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 24.41s`. WS19b's manages_bots ignore is retained until WS11.

Latest app rerun after the all-endpoint secret assertions:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -- --test-threads=4 --nocapture > .scratch/app-continued-final.log 2>&1
```

```text
test result: ok. 327 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 29.38s
```

Full workspace (exit 0), followed by the app rerun above and the DB rerun below after strengthening the exact 20-second vector:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > .scratch/workspace-continued-final.log 2>&1
```

```text
test result: ok. 327 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 26.72s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.58s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 417 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 44.77s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.59s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.44s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.30s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.62s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
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

Reported workspace totals: 1,359 passed, 0 failed, 11 explicit ignores. One passing front-server test explicitly returned because PEBBLE_MINICA was unset; ACME integration is unverified. Storage prints the pre-existing version-dependent media-byte comparison skip (vector libvips 8.16.1 / ffmpeg 7.1.5 versus host 8.18.6 / n9.0.2). Those external boundaries are not counted as verified parity. The seeded app has zero failures and no missing-data skip.

The 11 default ignores are the existing reference recorders/exporters, performance measurement and two kit doc examples, plus manages_bots and the external Node launcher. The Node launcher was explicitly run separately, so ten ignored declarations remain unexecuted in these commands.

Latest complete DB rerun (strict boundary vector included):

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_db -- --test-threads=4 --nocapture > .scratch/db-continued-final.log 2>&1
```

```text
test result: ok. 417 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 39.31s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Gateway dependency matches the unchanged package.json pin; installed only in authorized scratch:

```sh
mkdir -p .scratch/node-deps
NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache" npm install --prefix .scratch/node-deps --no-save --package-lock=false --ignore-scripts ws@8.21.3 > .scratch/node-install.log 2>&1
```

Install raw summary: `up to date in 593ms`.

Own gateway suite, explicitly invoked with real Rust endpoints (exit 0). The Rust launcher runs `node --import ./rust/reference-tools/huddle_gateway_node.mjs --test script/livekit-gateway/` and controls only private test apps. The final run was sequential after mutation sources were restored; an earlier overlapping Node run was discarded.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node -- --ignored --nocapture > .scratch/gateway-node-final.log 2>&1
```

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 3195.620551
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 330 filtered out; finished in 3.26s
```

Clippy includes the whole workspace, vendored html5ever and all targets (exit 0):

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-continued-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.69s
```

## Failure-first evidence

Before implementation, three real SQLite hook tests failed because membership removal, sign-out and ban retained active grants: `test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.13s`. Before adding routes, the four initial gateway tests failed on the unported response's missing no-store contract. The stronger security proof below uses valid tokens while bypassing secret/relationship checks, so malformed-token rejection cannot mask a missing gateway-secret check.

The additional exact 20-second Rails vector initially exposed a test assumption that last_seen_at always equals the current clock. Rails correctly skipped the one-second refresh; the stale-floor assertion now reads the persisted sighting. Production liveness code did not change for this correction.

Reran the committed discrimination command (exit 0):

```sh
python3 rust/reference-tools/huddle_discrimination.py > .scratch/huddle-discrimination-continued-final.log 2>&1
```

```text
gateway-secret-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.39s
revoked-grant-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.45s
removed-member-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.37s
grant-revocation-bypassed: test result: FAILED. 5 passed; 8 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.58s
disconnect-floor-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 419 filtered out; finished in 0.22s
broadcast-config-port-truncation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
twirp-placeholder: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 326 filtered out; finished in 0.03s
endpoint-separation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
listener-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
server-muted-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
forbidden-token-permissions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
expired-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 0.00s
cleanup-placeholder: test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 415 filtered out; finished in 0.34s
cleanup-backoff: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 419 filtered out; finished in 0.08s
cleanup-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out; finished in 10.08s
WS13 discrimination: 15 compiled regressions detected; sources restored
```

## Oracle regeneration and source identity

The new reference scripts were rerun and byte-compared to the committed JSON. The older token/Twirp and cleanup scripts were rerun too:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_protocol.rb > .scratch/huddle-protocol-continued.json 2> .scratch/huddle-protocol-continued.log
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_cleanup.rb > .scratch/huddle-cleanup-continued.json 2> .scratch/huddle-cleanup-continued.log
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_grants.rb > .scratch/huddle-grants-regenerated.json 2> .scratch/huddle-grants-regenerated.log
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_gateway.rb > .scratch/huddle-gateway-regenerated.json 2> .scratch/huddle-gateway-regenerated.log
```

cmp results: exit 0 for all four. Counts: 7 join tokens, 98 token shapes, 15 endpoint cases, 14 cleanup states, 16 grant authorization states, 8 sighting snapshots and 39 gateway request-response cases. New source verification compared SHA256 of HuddleGrant, Membership, User, Stream and Internal::HuddleController from the ws13 reference image with git show d7c7de92; all five are byte-identical. All created containers used ws13 names. No golden was generated from docs or upstream's app.

## Remaining work and owners

1. **Step 1 effects, WS13:** finish after_issued invitation clearing/ringing; revocation/leave call-ended, leave/join and voice-presence effects; participant reader ordering/dedup assertions; HTTP coverage of every existing token-shape vector and more permissive Ruby timestamp/header forms beyond the proven corpus. Route-controller/render integration remains step 4. First-sighting jobs are durable, but their handlers and replay of any failed rows remain required.
2. **Step 2, WS13:** full Stream model and callbacks; live start/stop/quality/current-call policies; stale-after-30s sweep; last-host departure session end, quiet timeline note and successor promotion (including deactivation); voice/stage lifecycle; hands/throttle; full rank/self-moderation and stage-role policies. Complete the in-process reconciler in Rails order: invitations >45s, stale streams >30s, then existing due cleanups. Only the cleanup step is registered today.
3. **Step 3, WS13 with WS17 seam:** invitation dedup/refresh/owned-attempt rows and suppression, invitation resolver, join/leave/rejoin notices, call-ended payloads, ring policy and invitation/join push payload construction and atomic enqueue. WS17 owns transport and Notifications::Policy, not these payloads.
4. **Step 4, WS13 with existing WS7/WS6 APIs:** public huddle show/create/participants/leave and user presence, voice/stage, role/hand/stream/moderation controllers, all 19 HTML views, sidebar/header/group-DM participant locals and broadcasts through WS7's guard. Existing WS4 CSP additions are present and their tests pass; browser/HTML parity still needs verification. No pixel/byte parity is claimed for new huddle screens.
5. **Step 5, WS13:** every remaining/partially covered Rails declaration is retained by name in `plans/ws13-deferred-tests.md` (548 original declarations across 33 files), with owner and LiveKit system reason. Do not treat lower-level property/vector coverage as completing a declaration's remaining UI or delivery assertions. Real LiveKit tests remain deferred with LIVEKIT_SYSTEM_TESTS=1; the gateway Node suite is now proven against Rust.

Also still unproven from the prior slice: successful TLS/open-timeout RoomService probes, broad network error taxonomy, process-kill/restart rehearsal, real LiveKit/production acceptance. No new inherited failure was assigned away; after merging main every executed seeded app test passes. No open design conflict is pending.
