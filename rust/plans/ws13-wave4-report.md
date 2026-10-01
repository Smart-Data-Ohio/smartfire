# WS13 Wave 4 report — partial, lifecycle and render continuation

Branch `rust/ws13-huddles`, owned worktree `rust-ws13`. Frozen Rails pin `d7c7de9264c63015be398001d7a1094e7695a6db`. Main remains `21a7332f2d3c324f0862cdf448baf17a84395aa0`, included through the existing real merge commit `a8b6538c`. Four coherent source slices were pushed in this continuation; the report/tooling commit follows. Verified source HEAD: `e610ef11eee22803f52215dd9fb4ebafbe2740bf`.

```text
39a6c0b0b1b5003c3c35f82a52ebbc2d61257a8f Make the Rust gateway acceptance test self-contained
ef5441f9f4e1b6ea5e100c41e5881983e89f6d5c Port stream lifecycle, Stage callbacks and call moderation
596927fd32d3d2942d7d6d983edca7d94bd06692 Port Stage roles and hands with personal render effects
e610ef11eee22803f52215dd9fb4ebafbe2740bf Deliver quiet Stage succession notes through Cable
```

## Done, by file

| Files under rust/ | Final behavior and verification |
| --- | --- |
| `db/src/models/stream.rs`, `huddle_stream_liveness.rs`, `huddle_grant.rs` | Persist validated stream creation, default/explicit start time, the live uniqueness constraint, idempotent end, and common membership/user/room end paths. Emit committed changed frames and an explicit stopped event only for a different actor. Stale reconciliation now invokes the same end callbacks. Rails does not require belongs_to existence on stream updates; imported orphan coordinates can end. 20 real Rails lifecycle scenarios plus the existing 16 stale-state cases. |
| `db/src/models/stage.rs`, `membership.rs`, `user.rs` | Last-host departure runs synchronously inside the immediate writer transaction: end streams, revoke room grants, create a quiet plain Action Text timeline note, then prefer an active administrator successor or the earliest joined remaining member. No-op with another host or no remaining members. Membership removal and deactivation are wired. Quiet note causes no unread/push work. Existing WS8a room-deletion and last-active Stage-grant seams remain wired. |
| `db/src/models/call_moderation.rs`, `stage_streams.rs`, `stage_participation.rs` | Shared domain policies, transactional stream and grant effects, last-host validation, administrator rank and self-target rules. A listener-boundary role change revokes grants; host↔speaker retains them and the stream. Repeated mute preserves a fresh grant, while still ending an imported live stream. Disconnect keeps membership. Hands preserve the first timestamp and an empty lower's updated_at. |
| `campfire/src/controllers/rooms/{call_moderation,stage_streams,stage_participation}.rs`, `controllers.rs`, `rooms.rs` | Real public stream, role, hand and moderation routes: ordered authorization, exact errors/status/content types, HTML redirects, Turbo responses, CSRF on request forms, X-Stream-Id, and exact string comparison against stale stop IDs. Hand raising uses a shared per-membership/minute counter; ten successes then 429, separate membership buckets, and next-minute reset. Rails' null test-store default is unthrottled. 29 real moderation HTTP requests, 34 real role/hand requests, a Rails throttle probe, and stream security/lifecycle HTTP tests. |
| `db/src/models/huddle_effects.rs`, `campfire/src/channels/{huddle_effects,sink}.rs` | Typed render descriptions delivered after commit through WS7's guarded publisher: badge, sidebar/event dots, per-viewer panels/rosters, affected member panel, one persistent rejoin on publish-boundary crossing, stream-stopped event, and quiet Stage timeline note. A Stage-owned note descriptor uses the existing message presenter; generic WS8b messaging descriptors remain WS8b's seam. Real sockets verify delivery, no extra host↔speaker rejoin, idempotence and silent rollback. |
| `views/src/huddle_stage.rs`, `views/templates/rooms/stage/*`, `huddle_stage_vectors.json` | Seven owned Stage partials and the additional event venue live dot match 400 complete Rails renders (50 inputs × 8 fragments). Cover live/quiet streams, raised-hand queue order including fractional timestamps, administrator rank, muted roles, multiple/no hosts, and exact forms/hidden inputs/newlines. Shared frames contain no session secrets. Personal request forms receive CSRF tokens. |
| `campfire/src/channels/huddle_stage_note_vectors.json`, `huddle_effects_tests.rs` | A fixed-ID real Rails system note render matches Rust bytes, and a real last-host removal delivers the note to the room's socket. |
| `campfire/src/controllers/internal_huddle_tests.rs`, `reference-tools/huddle_gateway_node.mjs` | Node acceptance installs the committed gateway lockfile in a test-created tempfile and uses its pinned ws wrapper. No pre-existing `.scratch/node-deps` dependency. Await all Rust proxy requests before asserting final persisted liveness, eliminating the left-request race. Original gateway implementation and suite are unchanged; all 16 pass from the fresh clone. |
| `reference-tools/huddle_{stream_lifecycle,moderation,stage_views,participation,stage_note}.rb`, committed JSON vectors | Actual production Rails models/controllers/partials are the oracle. Verified sessions, explicit fixture identities and fixed time; only external Cable/job delivery is captured. Integration oracle disables forgery protection like Rails integration tests; Rust requests use real CSRF. Clear Rails execution context between scenarios, as the parity seed does. No masks or output normalization to hide HTML differences. |
| `reference-tools/huddle_discrimination.py`, `ws13_verify_{corpora,reference,declarations}.py`, `plans/ws13-deferred-tests.md` | 67 compiled regressions caught; all source files restored. All 16 full golden files regenerate byte-identically. Reference classes/initializer match the frozen pin by SHA256. All 548 original titles retained and checked, with per-file counts sorted largest first. |

CSP additions already exist in merged `campfire/src/security.rs`: wasm-unsafe-eval and LiveKit's paired WebSocket/HTTP origins. The existing whole-header Rails vector test passed in the fresh seeded suite; the actual initializer hash also matches the pin. This continuation changes no CSP policy.

Earlier pushed work was reverified: strict token shapes, Twirp and durable cleanup/backoff, six revocation triggers, gateway contract, 50 participant-stack renders, 67 notice cases, 36 push-preparation cases, 49 issuance cases, 29 resolver cases and 17 hand-domain cases. The in-process reconciler remains invitations → stale streams → due cleanups, with per-row commits, error isolation and real registered job handlers. No Rails/schema/migration/asset override/sidecar/parity mask/allowlist or dependency-lock changes.

## Cross-workstream seams

WS17 retains policy and transport integration; the lead reconciles these unchanged seams. Neither class has a stub success handler:

```rust
// db::models::huddle_notices
// class Notifications::HuddlePushJob
pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest);
pub fn prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool)
    -> Result<Option<PushDelivery>>;
// PushRequest { kind: Huddle/HuddleJoin, recipient_id, sender_id, room_id,
//   room_membership_id: Option<i64>, payload: { title, body, path, tag } }

// db::models::huddle_invitations
// class Notifications::HuddleRingJob
pub fn publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool)
    -> Result<()>;
// RingRequest { recipient_id, sender_id, invitation: serde_json::Value }

// WS12 lazy inbox resolver
pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>;
```

WS17 evaluates current Notifications::Policy and enqueues transport delivery in the same transaction as prepare_push's subscription selection and strict join-throttle claim. Policy denial/no subscriptions never burns the claim; enqueue failure rolls it back. Invitation empty-subscription behavior is preserved. Ring intents capture the complete invitation without silent until the supplied sound decision; publish_ring emits through WS7 after commit, active humans only. Both WS17 classes remain durable unknown-handler failures for exact-class recovery once real handlers are registered. Presence, cleanup, join-notice and push-invitation handlers are real and registered. WS8b remains responsible for its generic message/other partial descriptors; Stage's quiet note now has its own registered handler.

## Fresh-clone verification

All commands below were run again during this continuation. Rust 1.98.1, locked Cargo, -j4, private target, assigned ports. The fresh clone contains tracked branch files only; no target, seed or scratch content was copied from the worktree. Its own committed seed builder created both databases from the pinned Rails image. The clone's source was fast-forwarded to the final source SHA before testing. The report/tooling commit changes no Rust runtime source.

From the owned worktree:

```sh
git clone --single-branch --branch rust/ws13-huddles --no-local . .scratch/fresh-check
```

In `.scratch/fresh-check`:

```sh
mkdir -p .scratch
PARITY_OWNER=ws13 PARITY_CPUS=2 PARITY_IMAGE=ws13-reference:d7c7de92 rust/parity/bin/seed build default first_run > .scratch/fresh-seed.log 2>&1
git fetch origin
git merge --ff-only origin/rust/ws13-huddles
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > .scratch/fresh-workspace-test.log 2>&1
```

Seed build raw lines (exit 0):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Workspace exit 0, including html5ever. Raw summary lines in executable order; app is the first line and database is the 428-pass line:

```text
test result: ok. 350 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 32.31s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.71s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 428 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 60.53s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.65s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.56s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.54s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Aggregate of those lines: 1,404 passed, zero failed, 11 ignored. Explicit Node execution below runs one of the ignored tests. The remaining ten are: WS11 manages_bots, reference-frame recording, push latency measurement, three external Rails database/export tests, storage's pinned-media bytes test, mail export, and two kit documentation examples. Pinned exact-media acceptance and PEBBLE ACME are unexecuted. These are recorded limitations, separate from the zero-failure seeded app suite.

Fresh clone, explicit gateway acceptance, exit 0. It installs locked Node dependencies into its own test-created tempfile:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture > .scratch/fresh-gateway-node.log 2>&1
```

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 3063.914616
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 353 filtered out; finished in 3.32s
```

Fresh clone workspace clippy, exit 0:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.19s
```

Both freshly built seeds validated again by real Rails, exit 0:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/fresh-seed-default-verify.log 2>&1
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/fresh-seed-first-run-verify.log 2>&1
```

Raw fields, default then first_run:

```text
  "passed": 29,
  "failed": 0
```

```text
  "passed": 4,
  "failed": 0
```

Locked metadata, duplicate workspace-key detection and main ancestry, run in the fresh clone, exit 0:

```sh
python3 - <<'PYCODE' > .scratch/fresh-metadata.log
import pathlib,subprocess,tomllib
root=pathlib.Path.cwd()
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','metadata','--locked','--format-version','1'],cwd=root/'rust',stdout=subprocess.DEVNULL,check=True)
print('cargo metadata --locked --format-version 1: exit 0')
d=tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(d)} unique; no duplicates')
subprocess.run(['git','merge-base','--is-ancestor','21a7332f','HEAD'],check=True)
print('Main 21a7332f is an ancestor of HEAD: exit 0')
print('Fresh clone source HEAD: '+subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip())
PYCODE
```

```text
cargo metadata --locked --format-version 1: exit 0
Workspace dependency keys: 75 unique; no duplicates
Main 21a7332f is an ancestor of HEAD: exit 0
Fresh clone source HEAD: e610ef11eee22803f52215dd9fb4ebafbe2740bf
```

## Fail-first and differential evidence

Before implementation, the stream and role/hand security tests failed on 501 versus expected 403. Before the Stage note handler, the real socket test failed because no note arrived. These were assertion failures, not compile failures. Each subsequently passed in the fresh clone. Two early fixture mistakes (missing seeded membership and fixture administrator rank) were corrected and are not counted as fail-first evidence.

All 67 deliberate regressions were compiled and rejected by actual panics/assertion failures; the tool rejects compilation failures and restores each source in finally. This covers gateway secret, revoked/removed grants, expired/invalid/listener/server-muted tokens, rank and rate limits, mutation rollback, callback/panel/rejoin delivery, stale stops, issuance/resolver suppression and the process reconciler. The owning worktree was isolated from other source builds during mutation; the fresh clone verification used its own immutable checkout.

From the owned worktree, rerun exit 0:

```sh
python3 rust/reference-tools/huddle_discrimination.py > .scratch/final-discrimination.log 2>&1
```

```text
stage-note-delivery-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 3.12s
role-rank-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.43s
role-demotion-without-grant-left-live: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.25s
role-personal-panel-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.19s
role-unnecessary-rejoin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.36s
hand-repeat-roster-noisy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 4.46s
hand-rate-limit-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.43s
moderation-enqueue-failure-swallowed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.07s
stream-start-seen-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.43s
stream-stale-stop-protection-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.51s
stream-quality-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.31s
stream-committed-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 3.12s
stream-explicit-stopped-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.91s
stage-admin-successor-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.29s
stage-succession-note-noisy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.08s
stream-user-without-grant-left-live: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.20s
stage-forms-visible-to-listeners: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.01s
moderation-administrator-rank-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.43s
issuance-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.19s
issuance-banned-caller: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 7.56s
issuance-bot-caller: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 7.22s
issuance-off-hidden-recipient: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.93s
issuance-inbox-suppression-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.92s
issuance-reused-grant-dedupe-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 4.29s
issuance-owned-priority-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 2.67s
issuance-sound-policy-ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 2.52s
resolver-wait-shortened: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.26s
resolver-revoked-join-evidence-ignored: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.75s
stale-exact-thirty-kept: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.91s
stale-other-presenter-kept: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.93s
reconciler-admin-gate-skips-invitations: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 5.10s
reconciler-stale-pass-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.11s
hand-speaker-permission-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.58s
hand-repeat-idempotence-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.27s
push-connection-scope-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.30s
push-throttle-shortened: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 4.26s
push-policy-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.89s
push-denied-burns-throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.86s
notice-revoked-and-stale-joiner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.18s
notice-inactive-humans: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.09s
notice-second-device-leave: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 4.06s
notice-rejoin-window: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.47s
notice-call-ended-while-others-remain: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 4.54s
notice-join-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 10.10s
notice-invitation-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 10.10s
notice-push-request-dropped: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.07s
notice-revocation-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 5.25s
notice-leave-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 6.63s
presence-before-commit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.43s
presence-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 3.10s
presence-sink-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 3.13s
presence-recovery-disabled: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.11s
gateway-secret-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.53s
revoked-grant-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.41s
removed-member-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.45s
grant-revocation-bypassed: test result: FAILED. 5 passed; 9 failed; 0 ignored; 0 measured; 417 filtered out; finished in 0.68s
disconnect-floor-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.24s
broadcast-config-port-truncation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
twirp-placeholder: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 349 filtered out; finished in 0.03s
endpoint-separation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
listener-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
server-muted-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
forbidden-token-permissions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
expired-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 0.00s
cleanup-placeholder: test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 426 filtered out; finished in 0.44s
cleanup-backoff: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 430 filtered out; finished in 0.07s
cleanup-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 353 filtered out; finished in 10.21s
WS13 discrimination: 67 compiled regressions detected; sources restored
```

Whole golden files regenerated from real pinned Rails, compared without masks, exit 0:

```sh
python3 rust/reference-tools/ws13_verify_corpora.py > .scratch/final-corpora.log 2>&1
```

```text
protocol: reference rerun byte-identical
cleanup: reference rerun byte-identical
grants: reference rerun byte-identical
gateway: reference rerun byte-identical; 39 cases
presence: reference rerun byte-identical; 50 cases
notices: reference rerun byte-identical; 67 cases
pushes: reference rerun byte-identical; 36 cases
issuance: reference rerun byte-identical; 49 cases
resolver: reference rerun byte-identical; 29 cases
stale_streams: reference rerun byte-identical; 16 cases
hands: reference rerun byte-identical; 17 cases
stream_lifecycle: reference rerun byte-identical; 20 cases
moderation: reference rerun byte-identical; 29 cases
stage_views: reference rerun byte-identical; 50 cases
participation: reference rerun byte-identical; 34 cases
stage_note: reference rerun byte-identical
WS13 corpora: 16 complete files regenerated byte-identically
```

Actual reference classes/initializer checked against the pin, exit 0:

```sh
python3 rust/reference-tools/ws13_verify_reference.py > .scratch/final-source-identity.log
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
app/models/rooms/stage.rb: pin SHA256 matches reference image
app/models/rooms/voice.rb: pin SHA256 matches reference image
app/controllers/rooms/call_moderation_controller.rb: pin SHA256 matches reference image
app/controllers/rooms/stage/streams_controller.rb: pin SHA256 matches reference image
app/controllers/rooms/stage/roles_controller.rb: pin SHA256 matches reference image
app/controllers/rooms/stage/hands_controller.rb: pin SHA256 matches reference image
config/initializers/content_security_policy.rb: pin SHA256 matches reference image
Reference identity: 18 files match d7c7de92
```

## Original Rails declaration accounting

Every original title is retained in `plans/ws13-deferred-tests.md`. Passed titles combine the real request/state corpora, complete fragment goldens, real Cable delivery tests and existing WS8a model tests; vector counts are not declaration counts. Whole fan-out declarations, controller combinations not exercised, original lock/preload/query assertions and public page flows stay open. Largest files appear first.

```sh
python3 rust/reference-tools/ws13_verify_declarations.py > .scratch/final-declarations.log
```

```text
Rails declaration catalogue: 548 titles retained; 91 passed; 457 partial/deferred; 33 files; source titles match
```

| Rails file | Original | Assertions covered (passed) | Partial/deferred |
| --- | ---: | ---: | ---: |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 16 | 22 |
| `test/models/huddle_invitation_test.rb` | 38 | 0 | 38 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 0 | 36 |
| `test/models/huddle/join_notifier_test.rb` | 33 | 0 | 33 |
| `test/models/huddle_grant_test.rb` | 33 | 0 | 33 |
| `test/system/huddles_test.rb` | 31 | 0 | 31 |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 0 | 29 |
| `test/models/rooms/stage_test.rb` | 27 | 12 | 15 |
| `test/models/stream_test.rb` | 27 | 14 | 13 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 0 | 24 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 16 | 4 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 13 | 6 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 0 | 18 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 0 | 16 |
| `test/system/huddle_join_notices_test.rb` | 16 | 0 | 16 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 9 | 6 |
| `test/system/stage_test.rb` | 15 | 0 | 15 |
| `test/models/huddle/join_pusher_test.rb` | 13 | 0 | 13 |
| `test/system/voice_channels_test.rb` | 12 | 0 | 12 |
| `test/system/huddle_invitations_test.rb` | 10 | 0 | 10 |
| `test/models/huddle/invitation_resolver_test.rb` | 9 | 9 | 0 |
| `test/models/huddle_revocation_test.rb` | 9 | 0 | 9 |
| `test/models/huddle/ring_policy_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_audio_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_roster_test.rb` | 8 | 0 | 8 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 0 | 6 |
| `test/system/huddle_presence_test.rb` | 6 | 0 | 6 |
| `test/integration/huddle_presence_test.rb` | 5 | 0 | 5 |
| `test/models/rooms/voice_test.rb` | 5 | 2 | 3 |
| `test/jobs/huddle/join_notice_job_test.rb` | 4 | 0 | 4 |
| `test/jobs/huddle/push_invitation_job_test.rb` | 4 | 0 | 4 |
| `test/services/huddle/reconciler_test.rb` | 4 | 0 | 4 |
| `test/jobs/huddle/broadcast_presence_job_test.rb` | 2 | 0 | 2 |
| **Total** | **548** | **91** | **457** |

## Exactly what remains

1. WS13 public controllers: `Rooms::HuddlesController` show/create/participants/leave; `Users::HuddlePresenceController#show`; voice and stage creation/edit/update/member-management/deletion routes. The stream/roles/hands/moderation routes are implemented, but full room pages, navigation/sidebar and layout integration remain incomplete.
2. Eleven of the original nineteen owned view integrations/golden checks (some WS6 template scaffolds already exist): `rooms/voices/{new,edit,_form}.html.erb`, `rooms/stages/{new,edit,_form}.html.erb`, `layouts/{_huddle,_huddle_invitation,_huddle_join_notice}.html.erb`, and `users/sidebars/rooms/{_voice,_stage}.html.erb`. Seven owned Stage fragments and the participant partial are ported; the additional WS14 event venue dot is also byte-identical. Full parity-seed pages and these eleven views must still be compared byte-for-byte. Existing CSP additions are verified above.
3. The 457 original partial/deferred declarations listed individually in the catalogue. Of these, 35 real-LiveKit declarations may remain deferred for LIVEKIT_SYSTEM_TESTS=1 with a configured server (31 HuddlesTest plus four explicitly gated StageTest cases). The 71 other browser system declarations use stubs and remain ordinary WS13 work after public HTML/controller integration. They are not covered by the real-LiveKit exception. The remaining model/controller/job/service/integration assertions remain WS13 work, with WS17 policy/transport integration retained at the named seams.
4. Lead/WS17 integration: register/reconcile HuddlePushJob and HuddleRingJob with current Notifications::Policy and real transport, preserving atomic claim/enqueue and recovering only each exact unknown-handler error. No policy or transport delivery is claimed here.
5. Retained external acceptance gaps: actual LiveKit/browser runs, successful TLS/open-timeout RoomService probes and broader network taxonomy, process-kill/restart rehearsal, production acceptance, and pinned media/PEBBLE tests. No PR or deployment was performed.

Stop point: the pushed lifecycle/render slices and fresh-clone checks are coherent. WS13 is partial until the public controllers/views and remaining declarations above are completed.
