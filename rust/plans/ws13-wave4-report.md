# WS13 Wave 4 report — partial, four continued behavior slices

Branch `rust/ws13-huddles`, worktree `rust-ws13`, reference our Rails `d7c7de92`. Main `21a7332f` remains merged by real merge commit `a8b6538c`; no rebase, PR, or deployment. The following four coherent source slices were committed and pushed during this continuation (newest first). The verification/report commit follows them; the final pushed HEAD is supplied in the handoff reply.

```text
70032480d98d2eaf68e2d33f9bc3d8c767337abd Match Stage hand mutation and validation semantics
b66909372f8b695d7908301f1412d5eac7f74d89 Reconcile stale presenter state before due huddle cleanups
442bdc8ec68f4eaa4df77806bf5e0dbe59173f8f Resolve overdue huddle invitations in the process task
6e303fe52fffdd65654b11a4e7e17706786fb2b4 Match huddle issuance invitations, retries and suppression
```

## Done and precise limits

1. **Issuance:** `HuddleGrant::issue` clears the caller's open started/missed invitations, then fans out to the other eligible humans in a direct room. It preserves existing read timestamps while handling late joins; excludes bots, inactive users, and off/hidden members; respects current in-call sightings; applies inclusive two-minute invitation/grant dedupe; prioritizes the current grant's owned row; repoints an unhandled same-attempt row at up to ten minutes; and creates banner-only intents when huddle inbox items are switched off. Muted members still ring. Rails' in-call recipient and recent sibling checks intentionally do not apply the active/revoked scope. Exact item IDs, row fields/timestamps, ordered ring payloads, and invitation job IDs match **49 Rails cases**. Sound policy and transport delivery remain at WS17's seam.
2. **Overdue invitations:** strict older-than-45-seconds resolution, per-user filtering, unchanged handled/recent rows, missing/other sources, missed versus handled outcomes, preserved read timestamps, revoked grants as join evidence, group recipients and repeat-run idempotence match **29 Rails cases**. WS12 can call the lazy resolver seam.
3. **Process reconciler:** all three state passes now run in Rails order: overdue invitations, stale presenter streams, then due LiveKit cleanup. The first two run with admin configuration absent. Each item/stream commits separately; a later failure preserves earlier progress, is logged, and does not stop the subsequent phase. Actual scheduler execution, SQL-enforced phase order, retained earlier rows, and real cleanup HTTP requests are tested. Stale persisted state matches **16 Rails cases**, with strict 30-second sightings, room/membership scope, revoked grants, other devices, ended history, deleted rooms and imported orphan coordinates. This is **state parity only**: the common Stream ended render callbacks are still missing. The reconciler is wired, but full reconciler observable/render acceptance remains partial until those callbacks land.
4. **Hands:** membership raise/lower mutations, first timestamp preservation, unchanged updated_at on empty lowers, speaker/host rejection, non-stage validations, imported inconsistent hands and role changes clearing hands match **17 Rails cases**. Public authorization, minute-bucket throttling and roster/controls rendering remain open.

Earlier pushed slices remain present and were reverified: tokens and strict shapes; Twirp and cleanup jobs/backoff; all six revocation seams; actual gateway endpoints; committed participant stacks (one Askama partial, **50 byte-identical renders**); join/leave/call-ended payloads (**67 cases**); invitation/join push payload and subscription/throttle preparation (**36 cases**). The existing gateway Node implementation and test suite were unchanged. No Rails, schema/migration, dependency/lockfile, asset override, sidecar, parity mask or allowlist changes.

These new 111 differential cases are four Rust test declarations. Six additional real app tests cover queue effects, rollback, the process loop and phase failures. Vector counts are not Rust test counts or completed original Rails declarations.

## Files changed in this continuation

| Files under rust/ | Change |
| --- | --- |
| `crates/db/src/models/huddle_grant.rs` | Carry the reused grant's previous issuance timestamp into after-issued processing; invitation writes and durable intents commit with issuance. Existing eligibility, reuse, unique retry and revocation rules retained. |
| `crates/db/src/models/huddle_invitations.rs`, `models.rs` | Issuance clearing, recipient fan-out, owned/same-attempt refresh, dedupe/suppression; typed ring intent and policy-supplied publisher; lazy/per-item overdue resolver. |
| `crates/db/src/models/activity_item.rs`, `huddle_notices.rs` | Huddle-aware activity payload hook and mark_handled read-stamp preservation; share the existing inbox preference caster. Other source/event types keep the ordinary activity frame. |
| `crates/db/src/models/huddle_stream_liveness.rs` | Re-read the current live row and current presenter liveness under the writer transaction; persist stale ends. Rails' declared quality validation runs; this app does not require belongs_to association validation on stream updates. Full Stream callbacks remain open. |
| `crates/db/src/models/membership.rs` | Hand raise/lower domain methods using the existing call-attribute validations; exact no-op and timestamp behavior. |
| `crates/campfire/src/jobs/huddle.rs`, `jobs/periodic.rs` | Register all three reconciler state phases in the existing in-process task; isolate resolver/stale failures; keep admin-gated cleanup and its 100-row cap/backoff. |
| `crates/campfire/src/jobs/tests.rs`, `huddle/tests.rs` | Real issuance ring/push intents, recent-ring join suppression, failed enqueue rollback, actual process scheduling, phase order/failure isolation and earlier-row persistence. Existing join worker test now sets up a quiet grant directly; issuance would correctly suppress the join notice it tests. |
| `crates/db/src/tests/huddle_invitations_test.rs`, `tests/huddle_notices_test.rs`, `tests.rs` | Four complete differential replay tests, shared fixture insertion, exact timestamps/rows/payloads/errors, group/filter and repeated-resolution assertions. |
| `crates/db/src/models/huddle_{issuance,resolver,stale_stream,hand}_vectors.json` | Golden output from the pinned real Rails implementation. The stale-stream corpus explicitly certifies persisted state, not HTML. |
| `reference-tools/huddle_{issuance,resolver,stale_streams,hands}.rb` | Real model methods/callbacks run; only external Cable/job delivery and the Rails ring-policy quiet_check test seam are captured. Fixture session tokens are explicit inputs, so regeneration is reproducible. No output masks. Issuance snapshots explicitly read main.sqlite_sequence because fixture loading creates a shadowing temporary table, and reload memberships before snapshotting updated/removal state. Random generated grant identities are verified by the existing grant tests, not compared as deterministic issuance output. |
| `reference-tools/huddle_discrimination.py` | Expand the compiled assertion-failure discrimination corpus from 33 to 49 deliberate regressions; restore all sources in finally. |
| `reference-tools/ws13_verify_declarations.py`, `plans/ws13-deferred-tests.md`, this report | Retain every original title, verify it against Rails, and report complete versus partial declaration counts by file. |

## Cross-workstream seams

**WS17 retains all policy and transport integration.** Nothing here assumes that a request authorizes delivery; neither WS17 class has a stub success handler.

Existing push seam, in `db::models::huddle_notices`:

```rust
pub struct PushRequest {
    pub kind: PushKind, // huddle or huddle_join
    pub recipient_id: i64,
    pub sender_id: i64,
    pub room_id: i64,
    pub room_membership_id: Option<i64>,
    pub payload: PushPayload, // title, body, path, tag
}
// Job class: Notifications::HuddlePushJob
pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest);
pub fn prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool)
    -> Result<Option<PushDelivery>>;
```

WS17 evaluates Notifications::Policy from current records and enqueues its actual delivery in the **same transaction** that prepares subscriptions and claims the join throttle. WS13 owns subscription selection and conditional throttle: disconnected is connected_at NULL or strictly older than 60 seconds, independent of connections; off/hidden excluded for both kinds, muted additionally excluded for joins; join needs subscriptions before a strict older-than-600-seconds claim, does not touch updated_at, and never burns the claim on denial/no subscriptions. Invitations preserve Rails' empty-subscription queue-call behavior. The existing trigger test proves enqueue failure rolls back the claim and triggering inbox write.

New audible-ring seam, in `db::models::huddle_invitations`:

```rust
pub struct RingRequest {
    pub recipient_id: i64,
    pub sender_id: i64,
    pub invitation: serde_json::Value,
}
// Job class: Notifications::HuddleRingJob
pub fn publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool)
    -> Result<()>;
```

The request carries Rails' complete invitation payload at mutation time, excluding silent until WS17 supplies the decision. It includes the zero-ID/empty-path suppressed variant. WS17's decision corresponds to `Notifications::Policy.new(recipient:, sender:, kind: :huddle).sound?`; the helper writes silent and publishes through WS7 after commit, to active humans only. Register/reconcile this proposed class with WS17 at merge time. Both WS17 classes currently remain durable unknown-handler failures, preserving intents for the lead to recover after registering the real handlers; replay only the exact unknown-class error. Presence/JoinNotice/PushInvitation handlers remain real, registered, and recover only their own exact historical unknown errors.

**WS12:** `resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>` is the lazy inbox seam. The process loop uses `overdue_ids(conn, now, user_id) -> Result<Vec<i64>>` then `resolve_item(tx, id) -> Result<()>` per writer transaction. Route huddle state changes through ActivityItem::broadcast_change/mark_handled so their invitation intent is retained.

**WS8a/WS6/WS7:** synchronous last-active Stage grant and room-deletion stream state remain wired. The remaining Stream/Stage render callbacks must use the existing broadcast guard and per-viewer rendering; no complete render parity claim is made for the newly ended streams.

## Original Rails declaration counts

All 548 original titles remain listed in `plans/ws13-deferred-tests.md`. Nine resolver declarations and three Stage hand declarations have complete original assertion coverage, with their exact titles marked Passed and mapped to the executable corpora. Other files conservatively retain all original declarations even where lower-level coverage exists. LiveKit system tests remain deferred for LIVEKIT_SYSTEM_TESTS=1 with a real LiveKit server. WS17 owns policy/transport; other retained work is WS13 unless its individual entry names a collaborator.

| Rails file | Original | Assertions covered (passed) | Partial/deferred |
| --- | ---: | ---: | ---: |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 0 | 29 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 0 | 19 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 0 | 36 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 0 | 15 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 0 | 20 |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 0 | 38 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 0 | 16 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 0 | 24 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 0 | 18 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 0 | 6 |
| `test/integration/huddle_presence_test.rb` | 5 | 0 | 5 |
| `test/jobs/huddle/broadcast_presence_job_test.rb` | 2 | 0 | 2 |
| `test/jobs/huddle/join_notice_job_test.rb` | 4 | 0 | 4 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 4 | 0 | 4 |
| `test/models/huddle/invitation_resolver_test.rb` | 9 | 9 | 0 |
| `test/models/huddle/join_notifier_test.rb` | 33 | 0 | 33 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 0 | 13 |
| `test/models/huddle/ring_policy_test.rb` | 8 | 0 | 8 |
| `test/models/huddle_grant_test.rb` | 33 | 0 | 33 |
| `test/models/huddle_invitation_test.rb` | 38 | 0 | 38 |
| `test/models/huddle_revocation_test.rb` | 9 | 0 | 9 |
| `test/models/rooms/stage_test.rb` | 27 | 3 | 24 |
| `test/models/rooms/voice_test.rb` | 5 | 0 | 5 |
| `test/models/stream_test.rb` | 27 | 0 | 27 |
| `test/services/huddle/reconciler_test.rb` | 4 | 0 | 4 |
| `test/system/huddle_audio_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_invitations_test.rb` | 10 | 0 | 10 |
| `test/system/huddle_join_notices_test.rb` | 16 | 0 | 16 |
| `test/system/huddle_presence_test.rb` | 6 | 0 | 6 |
| `test/system/huddle_roster_test.rb` | 8 | 0 | 8 |
| `test/system/huddles_test.rb` | 31 | 0 | 31 |
| `test/system/stage_test.rb` | 15 | 0 | 15 |
| `test/system/voice_channels_test.rb` | 12 | 0 | 12 |
| **Total** | **548** | **12** | **536** |


## Fail-first and discrimination evidence

Before the issuance callback, the replay failed on fresh_direct's missing invitation rows; before the stale pass, the real process test failed because a quiet presenter remained live. These are historical red runs in authorized scratch, not current failures:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 423 filtered out; finished in 0.15s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 341 filtered out; finished in 0.12s
```

The following command was rerun, exit 0. It compiles each regression and requires a panic/test assertion failure, rejecting compile failures, then restores its source. Wrong/missing gateway authentication, revoked/removed grants, listener/server-mute publishing and invalid token acceptance are covered alongside the new banned/bot issuance gates, ring suppression, dedupe, resolver and hand permissions:

```sh
python3 rust/reference-tools/huddle_discrimination.py > .scratch/huddle-discrimination-final.log 2>&1
```

```text
issuance-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.08s
issuance-banned-caller: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 4.00s
issuance-bot-caller: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 4.20s
issuance-off-hidden-recipient: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.94s
issuance-inbox-suppression-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 1.63s
issuance-reused-grant-dedupe-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 4.15s
issuance-owned-priority-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 2.15s
issuance-sound-policy-ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 2.20s
resolver-wait-shortened: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.18s
resolver-revoked-join-evidence-ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 1.60s
stale-exact-thirty-kept: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.46s
stale-other-presenter-kept: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.93s
reconciler-admin-gate-skips-invitations: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 5.10s
reconciler-stale-pass-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.10s
hand-speaker-permission-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.58s
hand-repeat-idempotence-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.26s
push-connection-scope-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 1.19s
push-throttle-shortened: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 3.88s
push-policy-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.97s
push-denied-burns-throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 1.08s
notice-revoked-and-stale-joiner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 1.49s
notice-inactive-humans: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.11s
notice-second-device-leave: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 4.77s
notice-rejoin-window: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 2.11s
notice-call-ended-while-others-remain: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 5.75s
notice-join-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 10.11s
notice-invitation-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 10.13s
notice-push-request-dropped: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.08s
notice-revocation-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 4.97s
notice-leave-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 5.16s
presence-before-commit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.15s
presence-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 3.11s
presence-sink-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 3.10s
presence-recovery-disabled: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.18s
gateway-secret-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.40s
revoked-grant-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.45s
removed-member-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.47s
grant-revocation-bypassed: test result: FAILED. 5 passed; 9 failed; 0 ignored; 0 measured; 413 filtered out; finished in 0.66s
disconnect-floor-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.11s
broadcast-config-port-truncation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
twirp-placeholder: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.03s
endpoint-separation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
listener-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
server-muted-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
forbidden-token-permissions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
expired-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.00s
cleanup-placeholder: test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.24s
cleanup-backoff: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.09s
cleanup-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 10.10s
WS13 discrimination: 49 compiled regressions detected; sources restored

```

## Fresh final verification

All commands cited below were rerun in this worktree. Rust 1.98.1, locked Cargo, -j 4, own rust/target and authorized scratch. The existing default and first_run seeds were built from the pin in the preceding slice and both were validated again by real Rails. CI=1 makes missing seeds fail. The printed missing-seed helper message deliberately tests that helper; the actual seeded suite ran.

Seed validation, both exit 0:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/seed-default-final.log 2>&1
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/seed-first_run-final.log 2>&1
```

Raw summary fields from the two verifier outputs, default then first_run:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

Workspace, exit 0. App is the first line; DB is the 424-pass line:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > .scratch/workspace-final.log 2>&1
```

```text
test result: ok. 339 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 27.59s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.50s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 424 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 47.52s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.14s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.48s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.66s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.54s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.39s
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

Totals from the raw summaries: **1,379 passed, zero failures, 11 explicit ignores**. App: **339 passed, zero failures**. Its four default ignores are reference recording, measurement, WS11 manages_bots, and the external Node launcher. Node was explicitly run below, leaving ten ignored declarations unexecuted overall. Existing cable/DB recorders/exporters and two kit doc examples remain ignored. ACME returned with PEBBLE_MINICA unset; exact media bytes printed the existing version gate (vectors libvips 8.16.1/ffmpeg 7.1.5; host 8.18.6/n9.0.2). No ACME or exact media-byte parity claim.

Gateway's unchanged own node --test script/livekit-gateway/ suite against real Rust HTTP (the existing adapter only preserves original explicit fault injections); exit 0, after source restoration. Node and pinned ws@8.21.3 are installed in authorized scratch.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node -- --ignored --nocapture > .scratch/gateway-final.log 2>&1
```

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 2954.739912
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 342 filtered out; finished in 3.00s
```

Entire workspace clippy, including html5ever, all targets and denied warnings; exit 0:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.20s
```


## Oracle regeneration and source identity

Every one of the eleven corpora was regenerated by real Rails and byte-compared again, exit 0. This exact orchestration invokes the reference tools sequentially and writes only authorized scratch:

```sh
python3 - <<'PYCODE' > .scratch/huddle-reference-final.log
import json,os,pathlib,subprocess
root=pathlib.Path.cwd()
env=dict(os.environ,PARITY_NAMESPACE='ws13',PARITY_IMAGE='ws13-reference:d7c7de92',PARITY_OWNER='ws13')
outputs={
 'protocol':'rust/crates/campfire/src/huddle/protocol_vectors.json',
 'cleanup':'rust/crates/db/src/models/huddle_cleanup_vectors.json',
 'grants':'rust/crates/db/src/models/huddle_grant_vectors.json',
 'gateway':'rust/crates/campfire/src/huddle/gateway_vectors.json',
 'presence':'rust/crates/views/src/huddle_presence_vectors.json',
 'notices':'rust/crates/db/src/models/huddle_notice_vectors.json',
 'pushes':'rust/crates/db/src/models/huddle_push_vectors.json',
 'issuance':'rust/crates/db/src/models/huddle_issuance_vectors.json',
 'resolver':'rust/crates/db/src/models/huddle_resolver_vectors.json',
 'stale_streams':'rust/crates/db/src/models/huddle_stale_stream_vectors.json',
 'hands':'rust/crates/db/src/models/huddle_hand_vectors.json',
}
for name,path in outputs.items():
 output_path=root/'.scratch'/f'huddle-{name}-final.json'
 with output_path.open('wb') as output, (root/'.scratch'/f'huddle-{name}-final.log').open('wb') as error:
  subprocess.run(['rust/parity/bin/reference','runner','-e','RAILS_LOG_LEVEL=fatal',f'rust/reference-tools/huddle_{name}.rb'],env=env,stdout=output,stderr=error,check=True)
 subprocess.run(['cmp',str(output_path),str(root/path)],check=True)
 data=json.loads(output_path.read_bytes())
 count=len(data['cases']) if 'cases' in data else None
 print(f'{name}: reference rerun byte-identical; cmp exit 0'+(f'; {count} cases' if count is not None else ''),flush=True)
PYCODE
```

```text
protocol: reference rerun byte-identical; cmp exit 0
cleanup: reference rerun byte-identical; cmp exit 0
grants: reference rerun byte-identical; cmp exit 0
gateway: reference rerun byte-identical; cmp exit 0; 39 cases
presence: reference rerun byte-identical; cmp exit 0; 50 cases
notices: reference rerun byte-identical; cmp exit 0; 67 cases
pushes: reference rerun byte-identical; cmp exit 0; 36 cases
issuance: reference rerun byte-identical; cmp exit 0; 49 cases
resolver: reference rerun byte-identical; cmp exit 0; 29 cases
stale_streams: reference rerun byte-identical; cmp exit 0; 16 cases
hands: reference rerun byte-identical; cmp exit 0; 17 cases
```

Actual reference classes were SHA256-compared with git show at the pin, exit 0:

```sh
python3 - <<'PYCODE' > .scratch/source-identity-final.log
import hashlib,subprocess
paths=['app/models/huddle_grant.rb','app/models/huddle/join_notifier.rb','app/models/huddle/join_pusher.rb','app/models/huddle/invitation_pusher.rb','app/models/user/inbox_preferences.rb','app/models/membership.rb','app/models/stream.rb','app/controllers/internal/huddle_controller.rb','app/models/activity_item.rb','app/models/huddle/invitation_resolver.rb','app/models/huddle/ring_policy.rb']
raw=subprocess.check_output(['docker','run','--rm','--name','ws13-source-check-final','--network','none','--entrypoint','sha256sum','ws13-reference:d7c7de92',*[f'/rails/{path}' for path in paths]],text=True)
for line in raw.splitlines():
 digest,path=line.split(); local=path.removeprefix('/rails/')
 expected=hashlib.sha256(subprocess.check_output(['git','show',f'd7c7de92:{local}'])).hexdigest()
 assert digest==expected,local
 print(f'{local}: pin SHA256 matches reference image')
print(f'Reference identity: {len(paths)} files match d7c7de92')
PYCODE
```

```text
app/models/huddle_grant.rb: pin SHA256 matches reference image
app/models/huddle/join_notifier.rb: pin SHA256 matches reference image
app/models/huddle/join_pusher.rb: pin SHA256 matches reference image
app/models/huddle/invitation_pusher.rb: pin SHA256 matches reference image
app/models/user/inbox_preferences.rb: pin SHA256 matches reference image
app/models/membership.rb: pin SHA256 matches reference image
app/models/stream.rb: pin SHA256 matches reference image
app/controllers/internal/huddle_controller.rb: pin SHA256 matches reference image
app/models/activity_item.rb: pin SHA256 matches reference image
app/models/huddle/invitation_resolver.rb: pin SHA256 matches reference image
app/models/huddle/ring_policy.rb: pin SHA256 matches reference image
Reference identity: 11 files match d7c7de92
```

## Locked metadata and declaration validation

Locked metadata stdout was redirected as requested, TOML parsing checks duplicate workspace keys, and main ancestry was rechecked; exit 0:

```sh
python3 - <<'PYCODE' > .scratch/metadata-final.log
import pathlib,subprocess,tomllib
root=pathlib.Path.cwd()
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','metadata','--locked','--format-version','1'],cwd=root/'rust',stdout=subprocess.DEVNULL,check=True)
print('cargo metadata --locked --format-version 1: exit 0')
d=tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(d)} unique; no duplicates')
subprocess.run(['git','merge-base','--is-ancestor','21a7332f','HEAD'],check=True)
print('Main 21a7332f is an ancestor of HEAD: exit 0')
PYCODE
```

```text
cargo metadata --locked --format-version 1: exit 0
Workspace dependency keys: 75 unique; no duplicates
Main 21a7332f is an ancestor of HEAD: exit 0
```

All catalogue titles and counts were checked against the actual Rails files, exit 0:

```sh
python3 rust/reference-tools/ws13_verify_declarations.py > .scratch/declarations-final.log
```

```text
Rails declaration catalogue: 548 titles retained; 12 passed; 536 partial/deferred; 33 files; source titles match
```

## Exactly what remains

1. **WS13 lifecycle:** full Stream create/live/end/quality APIs and callbacks; the four badge/sidebar/event-dot/per-viewer-panel updates for every explicit/automatic end; explicit other-actor stream-stopped event; complete voice/stage lifecycle and start/stop call policies; hostless-stage transaction, quiet plain Action Text timeline note and successor promotion, including membership destruction and user deactivation; remaining role/rejoin effects; hand controller authorization/minute-bucket throttle; call moderation rank/self rules and mute/unmute/disconnect responses/effects. Persisted stale ends are implemented but their render callbacks are explicitly pending. Preserve WS8a's already-wired deleted-room/last-active Stage grant behavior.
2. **WS17 integration, by lead:** reconcile/register both named policy/transport seams with WS17's implementation, then recover their exact unknown-handler failures. No policy or transport delivery was implemented here or claimed verified.
3. **WS13 public controllers/views:** huddle show/create/participants/leave, user-presence, voice/stage, roles/hands/streams and call moderation; the remaining **18** views and actual sidebar/header/group-DM page integration, byte-identical for the parity seed. The one participant partial is already proven for 50 renders. Existing WS4 wasm-unsafe-eval/LiveKit connect-src support and tests remain present; public browser/CSP integration acceptance and any further additions are pending.
4. **WS13 tests:** the remaining **536** partial/deferred original declarations grouped above and individually retained in the catalogue. Real LiveKit system declarations stay deferred with the explicit LIVEKIT_SYSTEM_TESTS=1/real-server reason. The existing gateway Node suite has been verified against Rust.
5. Retained unproven external boundaries: successful TLS/open-timeout RoomService probes, broader network error taxonomy, process-kill/restart rehearsal, real LiveKit/production acceptance. No newly failing source test was reassigned as inherited.

No open design decision or approval request. This is a coherent pushed **partial** delivery. Stop point: issuance, resolver, reconciler state sweeps and hand mutations are verified; resume at the complete Stream/Stage lifecycle/render slice.
