# WS13 Wave 4 report — partial

Branch: `rust/ws13-huddles`. Reference: our Rails app at `d7c7de92`; worktree started from `bb6c5d78`. Code head: `e90378c7a8607794a56d476c158259bee6cab98f`. The report commit adds only this report's Rust mirror and the deferred-test inventory. Source slices were committed and pushed as `9d747f4d`, `fd3e74f9`, and `e90378c7`. No PR or deployment.

## Delivered slice

Validated and reused WS1's LiveKit token minting and strict verifier. Added the LiveKit configuration/RoomService protocol and durable cleanup consumer, with the cleanup step running inside the existing single-process huddle scheduler. This does **not** implement the huddle gateway endpoints or grant issuance/lifecycle. WS13 as a whole is partial.

| Files under `rust/` | Change |
| --- | --- |
| `crates/campfire/src/main.rs`, `huddle.rs` | Domain service module; Rails presence/configuration checks; distinct host/port comparison; ws/wss → http/https Twirp URL construction; scoped RemoveParticipant and room-create DeleteRoom admin tokens; 3-second open and 5-second read timeouts; 2xx/404 success; no redirects; safe errors discard upstream bodies. |
| `crates/campfire/src/huddle/tests.rs`, `huddle/protocol_vectors.json` | 12 new protocol/consumer tests. 7 fixed-time Rails join tokens, 98 verifier shapes, 15 endpoint cases, and both admin tokens. Real TCP servers use only ports 52300–52399. No persisted auth headers. |
| `crates/db/src/models.rs`, `models/huddle_cleanup.rs`, `models/huddle_cleanup_vectors.json` | Cleanup create/find/dedup, enqueue lease, due selection, transactional claims, capped retry backoff and completion; 5 new DB tests, including competing database handles and 14 Rails state snapshots. |
| `crates/campfire/src/jobs/huddle.rs`, `jobs.rs` | Register Huddle::CleanupJob with one queue attempt, consume its enqueue lease once, perform network calls after the claim commits, persist success or retain database retry state. |
| `crates/campfire/src/jobs/periodic.rs` | Register the cleanup step on HUDDLE_RECONCILE_INTERVAL (default 5 seconds), selecting at most 100 due rows in ID order and isolating individual failures. Invitation and stale-stream steps remain unregistered. |
| `crates/campfire/src/channels/sink.rs` | Small WS7 seam: use the shared Rails configuration check for membership-removal broadcasts. One new vector test caught the old u16 narrowing of a port accepted by Rails' configuration comparison. No broadcast guard changes. |
| `reference-tools/huddle_protocol.rb`, `huddle_cleanup.rb` | Generate vectors by calling our pinned Ruby services/model. The cleanup oracle uses actual TCP/RoomService calls and captures only the enqueue seam; it does not stub cleanup or its network implementation. |
| `reference-tools/huddle_discrimination.py` | Compile 10 deliberate regressions, require assertion failures rather than compiler failures, restore sources in finally, and retain logs under the worktree's scratch directory. |
| `plans/ws13-deferred-tests.md`, `plans/ws13-wave4-report.md` | Every remaining dedicated Rails test declaration with its owner; committed mirror of this report. |

The new code has 18 tests: 13 in the binary (12 huddle-module tests plus the broadcast configuration test), 5 in the DB crate. The focused huddle filter also runs 4 inherited WS7 tests, hence 17 + 5 in the raw focused summaries. Vector cases and cleanup snapshots are assertions inside these tests, not additional test counts.

## Design and row audit

No schema, migration, dependency, lockfile, asset, template, parity-mask or allowlist change. Rails/sidecar sources were not edited. The Huddle/TokenVerifier/RoomService/InternalController/HuddleGrant files and HuddleCleanup file in the reference image were compared byte-for-byte with git archives of the pin before trusting the vectors. The existing reference image was tagged into the owning `ws13-reference:d7c7de92` namespace; all containers created by this work used ws13 names.

The token work calls `rails_compat::jwt::livekit`: exact HS256 claim order/values, nbf minus 5 seconds, 120-second join expiry, 60-second admin expiry, publish sources, listener/server-muted restrictions, refreshed-token omissions, forbidden claims, and opaque room HMAC names are checked against Rails. Creating random participant identities and issuing grants remain unported; the vectors supply a deterministic identity.

Only `huddle_cleanups` is written by the new model. Rails validates room_name presence, identity presence for remove_participant, and the operation enum; huddle_grant is optional. Rust enforces these on create and completion, uses the existing two partial unique indexes, and keeps snapshots usable after the grant link is removed. Creation/completion touch updated_at; enqueue/claim use Rails' update_columns semantics and leave updated_at unchanged. Claim last_attempted_at and next_attempt_at share one captured timestamp. The backoff is 15, 30, 60, 120, 240, 480, 900 seconds and then stays at 900.

Decision 2 governs enqueue failures: job persistence and the triggering write share a SQLite transaction. A rejected enqueue rolls the transaction back. This intentionally differs from Rails' Redis after-commit failure rescue, but preserves immediately-due work when an already-existing cleanup's enqueue transaction fails. The actual HTTP DELETE room path was tested with a background_jobs rejection trigger; room, memberships, messages and cleanups were unchanged after the 500 response. No isolated fake queue is used in that test.

The real registered worker is also tested in a child test process with fixture LiveKit environment variables, so parallel tests' environment is not changed. The child boots the production registry/runner over its own seed copy; the parent serves actual TCP. A compiled bypass of the handler makes this test fail on its 10-second deadline. Network failures consume the durable delivery without bypassing the cleanup row's own retry schedule. Claims/backoff are persisted, and direct reconciliation handles due work without queue delivery. A process-kill/restart rehearsal is not claimed.

## Cross-workstream boundaries

- WS1: reuse only; its source is restored after mutation probes, with no committed change.
- WS3: reuse durable JobQueue/EventSink and Periodic host. The cleanup JobKind uses attempts=1 to match Rails and avoid queue retries bypassing next_attempt_at.
- WS7: shared configuration checker only. The conservative session-bound broadcast guard and publishers are unchanged. The existing three HuddleNoticeChannel authorization/stream tests ran successfully.
- WS8a: existing room_delete::CleanupJob shape/class remains intact; the new consumer is registered for the cleanup rows/jobs that room deletion already produces. The database model reexports that existing job type. No existing room-deletion, membership, session or ban callback chain is claimed complete.
- WS8b: HTTP DELETE rollback is verified against its currently wired synchronous destroy path. Switching the room controller to begin_destroy remains WS8b work; no controller route was changed here.
- WS17: no push transport/policy implementation, invocation seam or payload enqueue was fabricated. WS13 still owns invitation/join payloads and enqueue integration; WS17 owns transport and Notifications::Policy.
- WS11: the only full app-suite failure is the common brief's known manages_bots failure. It is not fixed or claimed green here.

## Failure-first proof

The initial placeholder RoomService produced 5 assertion failures, and the initial no-claim cleanup produced 3. The old broadcast config checker failed the new pinned Rails high-port case before consolidation. The committed discrimination script re-proves these and the token security properties. Final rerun:

```sh
python3 rust/reference-tools/huddle_discrimination.py > .scratch/huddle-discrimination.log 2>&1
```

```text
broadcast-config-port-truncation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
twirp-placeholder: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 313 filtered out; finished in 0.03s
endpoint-separation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
listener-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
server-muted-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
forbidden-token-permissions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
expired-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 0.00s
cleanup-placeholder: test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.15s
cleanup-backoff: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 406 filtered out; finished in 0.07s
cleanup-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 317 filtered out; finished in 10.10s
WS13 discrimination: 10 compiled regressions detected; sources restored
```

Missing/wrong gateway-secret, revoked/removed-member grant, and endpoint-level listener/server-mute tests have **not** been shown failing first: those endpoints and the grant model are explicitly deferred. Token-level forbidden publishing, server mute, expiry and malformed shapes are proven by the listed compiled mutations/vectors.

## Commands and raw verification

Run from `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13`. Own target and scratch, Cargo four jobs, no release profile. Commands below were run in this worktree; all final test/lint summaries are pasted directly from logs.

The reference seed builds:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/seed build default > .scratch/seed-build.log 2>&1
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/seed build first_run > .scratch/seed-first-run.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The first workspace attempt revealed first_run was absent; it was built and the final full workspace command below reran afterward. There are zero missing-seed or missing-reference skips in that final run. The default seed's huddle_cleanups table was confirmed empty before runtime tests.

Fresh Rails regeneration, both cmp commands exit 0 without output:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_protocol.rb > .scratch/huddle-protocol-final.json 2> .scratch/huddle-protocol-final.log
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_cleanup.rb > .scratch/huddle-cleanup-final.json 2> .scratch/huddle-cleanup-final.log
cmp .scratch/huddle-protocol-final.json rust/crates/campfire/src/huddle/protocol_vectors.json
cmp .scratch/huddle-cleanup-final.json rust/crates/db/src/models/huddle_cleanup_vectors.json
```

The comparison/count probe printed:

```text
Rails huddle vectors: 7 join tokens; 98 verification shapes; 15 endpoint cases; 14 cleanup states; byte-identical regeneration
```

Focused (zero ignored/skipped):

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db huddle -- --nocapture > .scratch/huddle-final.log 2>&1
```

```text
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 301 filtered out; finished in 0.67s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.24s
```

Full seeded workspace including doctests (`--no-fail-fast` keeps other targets running after the known binary failure; vendored html5ever excluded from tests as AGENTS.md prescribes):

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > .scratch/workspace-final.log 2>&1
```

```text
test result: FAILED. 315 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 27.77s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.05s
test result: ok. 404 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 44.00s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.37s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.62s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.71s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
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
error: 1 target failed:
    `-p campfire --bin campfire`
```

Summary arithmetic: 1331 passed, 1 failed, 9 explicitly ignored across Cargo summaries; 1332 ran. All seeded binary tests ran: 315 passed, 1 failed, 2 explicitly ignored. The sole failure is controllers::presenters::accounts::tests::manages_bots, named inherited by the common brief. No unexpected failure was attributed to main without comparison.

The explicit ignored tests are the two binary recorder/measurement tests; the separate cable recorder; three DB oracle/export probes; one mail export probe; and two kit doc examples. No new test is ignored. Native storage printed its existing version-dependent byte-comparison skip, because host libvips/ffmpeg differ from the vectors. This is an unverified media-byte boundary, not an omitted seed test:

```text
skipping byte comparisons that depend on versions: vectors have libvips "8.16.1" / "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", local libvips 8.18.6 / ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
```

Clippy includes the entire workspace and all targets (including vendored html5ever), with warnings denied:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.64s
```

Locked metadata exits 0; stdout is JSON saved on disk, no lockfile modification:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > .scratch/cargo-metadata.json
```

## Rails test mapping and what remains

The four declarations in test/services/huddle/room_service_test.rb are ported to the scoped-admin TCP test, the 404 test and the no-redirect/safe-error test. The five in test/models/huddle_cleanup_test.rb are ported across the DB tests and real consumer/worker/HTTP-rollback tests: enqueue failure, server backoff, direct reconciliation, single-use queued lease, and disabled admin configuration. Decision 2's atomic rollback replaces the Rails enqueue-failure expectation as explained above. The three HuddleNoticeChannel declarations are inherited from WS7 and verified by the focused/full runs, not newly implemented here.

The remaining **548 dedicated Rails test declarations in 33 files** are listed individually, with owner, in `rust/plans/ws13-deferred-tests.md`. They are not counted as ported merely because a verifier-level assertion has a vector. This covers the approximately 450 non-system cases plus system coverage in the brief's inventory without pretending unit case counts are tests. System tests need LIVEKIT_SYSTEM_TESTS=1 and real LiveKit and are deferred for that reason.

Precise remaining WS13 work:

1. HuddleGrant issuance/eligibility/reuse/retry, random participant identity, authorized?/authorize_or_revoke, 20-second in-call and 10-second seen-touch windows, participants/identities, record_seen/left and atomic presence/join-notice jobs. The existing data model does not yet expose HuddleGrant.
2. Internal authorize/grant/left routes: constant-time secret comparison, bearer coercions, claim → current grant lookup, exact payload/status/no-store semantics and Rails timestamp coercions. Generate request/response vectors for every outcome. Prove wrong/missing secret, revoked grant and removed member failure-first.
3. Grant revocation/callbacks on membership, session, user ban, room deletion, stage publish-boundary change and server mute; never resurrect revoked grants; partial active-grant index. Wire WS8a's in-call ended/leave/voice effects and synchronous last-Stage-grant stream behavior. Existing cleanup snapshot APIs/consumer are ready for those callers.
4. Stream and Stage/Voice model lifecycles, validation and callbacks; host/speaker/listener roles, hands, moderation and stream stale/end behavior. Add stale-stream and 45-second invitation resolution to the reconciler, before its cleanup step as Rails orders them.
5. Invitations, dedup/attempt retry, call-ended banners, join/leave notices, RingPolicy, payload construction and atomic push enqueue. WS17 retains push transport/Notifications::Policy; add/invoke its seam when integrating these.
6. Thin controllers/routes and all 19 HTML views, group-DM participant locals, sidebar huddle presence, HuddleNoticeChannel/WS7 publication and byte/pixel parity. CSP additions already exist in security.rs and pass its current Rails vectors; no new HTML/browser proof is claimed here.
7. Run node --test script/livekit-gateway/ with its actual authorization backend switched to the Rust internal endpoints. This command was not run as acceptance against fixtures; the required Rust endpoints do not exist yet. Real LiveKit system tests and cloud/network boundary checks remain deferred.
8. Broader RoomService transport error-class parity (including malformed HTTP/missing ENV/out-of-range admin-call ports), successful TLS/open-timeout-specific probes, process-kill/restart rehearsal, exhaustive Rails model/controller/job/integration tests, and the native media-byte boundary remain unproven. No production deployment or complete huddle parity acceptance.

Restart at item 1, then item 2: the token protocol, RoomService and cleanup consumer are available and verified, but no user-facing huddle endpoint is connected yet.
