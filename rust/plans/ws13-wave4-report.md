# WS13 Wave 4 report — partial, committed presence and notice jobs

Branch: `rust/ws13-huddles`. Reference: our Rails at `d7c7de92`. Main `21a7332f` was merged with the real merge commit `a8b6538c`, without rebasing. The previous grants/gateway handoff was `62be89b7`. This continuation pushed presence/render slice `d9dc3024daa09cdcd24450b9d15415038a9a543c` and notice/job/push-seam slice f8ad31a2ae6e845547f3a566c6790d6e0ff7e9b7. No PR or deployment.

## Done and exact restart point

This is a coherent pushed **partial** handoff. Grant issue/revoke/leave now publish committed participant stacks; the first-sighting presence worker renders the same stacks. Revoking an in-call grant or leaving sends Rails' call-ended and leave/dismissal payloads in the correct order. Join-notice and invitation-push workers are registered with real implementations, discard missing sources successfully, and enqueue typed payload intents for WS17. The cleanup handler and cleanup-only in-process scheduler remain registered from the prior slice. The previous six revocation hooks, fixed-time tokens/Twirp client and internal gateway contract remain intact: 7 join tokens, 98 verification shapes, 15 URL cases, 14 cleanup states, 16 grant authorization states, 8 sighting snapshots and 39 HTTP request-response cases.

Finish the rest of step 1 first: `after_issued!` invitation clearing, direct-recipient ringing/suppression and attempt ownership/dedup, followed by the remaining Stage/Stream effects in step 2. Public huddle issuance is still unported; `HuddleGrant::issue` must not be presented as the complete join flow. Step 2's overdue-invitation and stale-stream passes, the full voice/stage lifecycle and the public controllers/other views are not implemented in this continuation.

## Changes by file

| Files under rust/ | Result |
| --- | --- |
| `crates/db/src/models/huddle_effects.rs`, `huddle_grant.rs`, `models.rs` | Typed post-commit presence effects on successful issuance, actual revocation and successful leave. In-call revoke callbacks reread committed rows before call-ended/leave notices; leave keeps the stale-disconnect floor and skips duplicate leave notices on already revoked grants. Existing WS8a membership/session/user/room hooks reach these callbacks. |
| `crates/campfire/src/channels/huddle_effects.rs`, `sink.rs`, `mod.rs` | Presence reader loads an alive room and current distinct participants, renders one sidebar fragment for fan-out and one header fragment, and publishes through WS7's existing guard. Rendering stays out of DB/domain modules. Missing room or incomplete huddle configuration stays silent. |
| `crates/views/src/huddle.rs`, `lib.rs`, `templates/rooms/huddles/_participants.html` | The participant ERB partial ported to Askama with exact whitespace, Rails escaping, avatar limits, room-kind labels and signed avatar paths. 50 exact reference renders: five room kinds × sidebar/header × five roster sizes. This proves that partial given the captured locals, not full public-screen parity. |
| `crates/campfire/src/channels/huddle_effects_tests.rs`, `tests/huddle_grant_test.rs` | Production boot, real durable presence worker and authenticated Action Cable socket prove both stacks arrive after the first sighting, rollback sends no frame and leaves the grant live, and committed revocation clears the roster. Domain sink remains empty inside the transaction. |
| `crates/db/src/models/huddle_notices.rs` | Join/leave/rejoin and call-ended JSON, viewer-specific direct names, active-human gates, current liveness/revocation checks, second-device leave dedup, room/member visibility, recent-ring/rejoin/call-ended windows, complete push payloads and typed enqueue seam. No HTML or transport implementation in the domain. |
| `crates/db/src/tests/huddle_notices_test.rs`, `models/huddle_notice_vectors.json`, `models/huddle_push_vectors.json`, `tests.rs` | 67 real Rails notice/callback scenarios, exact ordered stream/payload comparisons; 36 actual invitation/join push scenarios exercise policy decisions as inputs, current subscription scope and conditional throttle. The latter does not implement or certify WS17's policy itself. |
| `crates/campfire/src/jobs/huddle.rs`, `jobs/tests.rs` | Real `Huddle::BroadcastPresenceJob`, `Huddle::JoinNoticeJob`, `Huddle::PushInvitationJob` handlers alongside existing cleanup. Boot recovery retries only the exact previous unknown-handler failures for these registered classes, preserving genuine failures. Actual workers persist both push-intent kinds. A real SQLite enqueue rejection rolls back both an activity write and the join throttle claim. |
| `reference-tools/huddle_presence.rb`, `huddle_notices.rb`, `huddle_pushes.rb` | Run our pinned Rails implementation. Only external Cable/push delivery is captured. Grant callback setup commits the original grant first, then performs a separate update so Rails' actual `after_update_commit` fires; a create-and-update in one setup transaction would not test that callback. |
| `reference-tools/huddle_discrimination.py` | 33 compiled deliberate regressions fail assertions; every source is restored in `finally`. Covers security, cleanup, committed presence, notice callbacks, real workers, scope, policy gate, throttle and dropped enqueue. |
| `plans/ws13-deferred-tests.md`, `ws13-presence-slice.md`, this report | Preserve each deferred/partial declaration, grouped file counts, the first pushed slice's historical checkpoint and this current handoff. |

No Rails, schema/migration, dependency/lockfile, sidecar, asset override, parity mask or allowlist changes. The gateway implementation and its tests remain unchanged.

## WS17 transport seam and ownership

Defined in `campfire_db::models::huddle_notices`:

```rust
pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest) -> ();
pub fn prepare_push(
    tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool,
) -> campfire_db::Result<Option<PushDelivery>>;
```

`PushRequest` implements `campfire_db::Job` with class **`Notifications::HuddlePushJob`**. Its serialized fields are `kind` (`huddle` or `huddle_join`), `recipient_id`, `sender_id`, `room_id`, `room_membership_id: Option<i64>` and `payload: PushPayload { title, body, path, tag }`. Titles are `NAME started a huddle` / `NAME joined your huddle`, body `Join from the conversation`, path `/rooms/ID`, tag `huddle-ID`.

WS17 must register that handler, read the current recipient/sender/room and membership, evaluate its own `Notifications::Policy` (invitation kind uses no room-membership argument; join kind uses the membership), then call `prepare_push` inside the same `db.write` transaction as its durable **transport enqueue**. `PushDelivery` carries the exact `payload` and `Vec<PushSubscription>`. Do not recursively enqueue the same `PushRequest`. A rejected transport enqueue must fail that transaction so the conditional join throttle rolls back.

WS13 owns the huddle subscription selection and ten-minute join throttle here. Disconnected means `connected_at IS NULL OR connected_at < now - 60s`, independent of `connections`; exact 60s is connected. Hidden/off memberships are excluded for both kinds; muted memberships are additionally excluded for joins. A join requires subscriptions before the strict `< now - 600s` conditional claim, does not stamp `updated_at`, and never burns the window when denied or subscription-less. Invitations retain Rails' empty-subscription queue-call behavior.

There is deliberately no dummy transport handler. Today the unknown WS17 class runs on `default`, becomes **failed with its row retained**, and authorizes no delivery. WS17 must retry those exact unknown-handler rows when registering its handler (same narrow recovery rule as the three newly registered huddle classes). This slice proves durable intents and helper behavior, not browser push delivery or full policy parity. WS13 still owns suppressed-ring `silent` integration and complete invitation lifecycle; WS17 owns shared policy and transport.

## New reference-case pass counts

These are exact differential **cases**, not 153 additional Rust test declarations or completed original Rails tests.

| Reference behavior/file | Cases passed |
| --- | ---: |
| `rooms/huddles/_participants.html.erb` through `HuddleGrant#broadcast_voice_presence` | 50 |
| `huddle/join_notifier.rb`: join / leave | 37 / 8 |
| `huddle_grant.rb`: call-ended / real revoke / disconnect callbacks | 16 / 3 / 3 |
| `huddle/invitation_pusher.rb`: scope/payload/queue decision | 18 |
| `huddle/join_pusher.rb`: scope/payload/conditional throttle | 18 |
| **Total newly captured cases** | **153** |

The notice corpus is one Rust test declaration and the push-scope corpus is another. Six additional seeded-app tests, one DB presence test and one view test were added over the previous gateway handoff; workspace pass counts below count declarations once. WS7's three existing HuddleNoticeChannel tests also execute in the seeded app suite.

## Failure-first evidence

Before committed presence effects, the DB callback test failed with `leave did not emit committed presence`; before its worker was registered, the queue test retained an unknown-handler failure. Before join/invitation worker registration the missing-source worker test failed instead of consuming its two rows: `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 334 filtered out; finished in 10.13s`. These historical logs are in scratch and are not described as current passing tests.

A fresh Rails vector also showed a real naming defect before the fix: Ruby preserves a non-breaking space inside a first name, whereas Rust's `split_whitespace` truncated it. The implementation now uses Ruby's ASCII separators, matching WS8a's existing direct-room naming rule. Raw red summary (`.scratch/notices-unicode-fail-first.log`):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 1.91s
```

Fresh rerun of the compiled security/behavior discrimination command, exit 0; all 33 mutations produced compiled assertion failures, then restoration:

```sh
python3 rust/reference-tools/huddle_discrimination.py > .scratch/huddle-callback-discrimination-final.log 2>&1
```


```text
push-connection-scope-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.79s
push-throttle-shortened: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 2.70s
push-policy-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.74s
push-denied-burns-throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.71s
notice-revoked-and-stale-joiner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.98s
notice-inactive-humans: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.08s
notice-second-device-leave: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 3.48s
notice-rejoin-window: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 1.39s
notice-call-ended-while-others-remain: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 4.48s
notice-join-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 10.08s
notice-invitation-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 10.10s
notice-push-request-dropped: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.08s
notice-revocation-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 6.42s
notice-leave-callback-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 7.14s
presence-before-commit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.15s
presence-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 3.11s
presence-sink-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 3.10s
presence-recovery-disabled: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.11s
gateway-secret-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.36s
revoked-grant-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.42s
removed-member-authorized: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.35s
grant-revocation-bypassed: test result: FAILED. 5 passed; 9 failed; 0 ignored; 0 measured; 409 filtered out; finished in 0.62s
disconnect-floor-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.09s
broadcast-config-port-truncation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
twirp-placeholder: test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 332 filtered out; finished in 0.03s
endpoint-separation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
listener-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
server-muted-publishing: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
forbidden-token-permissions: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
expired-token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 0.00s
cleanup-placeholder: test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 418 filtered out; finished in 0.41s
cleanup-backoff: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 422 filtered out; finished in 0.12s
cleanup-worker-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 336 filtered out; finished in 10.09s
WS13 discrimination: 33 compiled regressions detected; sources restored
```


## Current verification

All commands below ran again in this worktree. Cargo uses mise Rust 1.98.1, `--locked -j 4`, this worktree's `rust/target` and authorized scratch. Default and first_run seeds were built from the pin in the preceding slice and are present; seeded test execution sets CI=1, so missing real seeds fail. The printed missing-seed helper message tests that helper intentionally, not a silently skipped suite.

Full workspace, exit 0; raw summaries below. App is the first line, DB the eighth, view unit/integration are the 37/28 lines.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > .scratch/workspace-callback-final.log 2>&1
```

```text
test result: ok. 333 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 27.69s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.75s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 420 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 51.35s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.50s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.58s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.19s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.21s
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
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
```

Totals calculated from those raw lines: **1,369 passed, 0 failed, 11 explicit ignores**. App has **333 passed, zero failures**. The default app ignores are the reference recorder, measurement, WS11 `manages_bots`, and this external Node launcher. Node is explicitly executed below, leaving ten ignored declarations unexecuted overall. Existing DB recorders/exporters and two kit doc examples remain ignored. A front-server test returned because PEBBLE_MINICA was unset, so ACME integration remains unverified. Storage media-byte checks also printed the existing version gate: vector libvips 8.16.1 / ffmpeg 7.1.5 versus host 8.18.6 / n9.0.2. No exact media-byte or ACME parity is claimed.


The gateway's own unchanged `node --test script/livekit-gateway/` suite, invoked by the Rust launcher with the existing test adapter, forwards real authorize/check/left requests to real Rust HTTP endpoints. Revocation injections mutate actual grants; original outage/stall/malformed-response injections stay at the proxy. Requires Node and the pinned ws@8.21.3 installed in authorized scratch. Rust/Node sockets use 52300–52399. This final run was sequential after all mutation sources were restored; exit 0.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node -- --ignored --nocapture > .scratch/gateway-callback-final.log 2>&1
```

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 3673.766895
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 336 filtered out; finished in 3.75s
```


Clippy includes the entire workspace, vendored html5ever and all targets, with warnings denied; exit 0.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-callback-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.94s
```


Locked metadata was rerun from `rust/` after the merge and again after the current checks; exit 0, stdout redirected as requested, no lockfile change.

```sh
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```
Workspace keys checked again from the worktree root; Python tomllib rejects duplicate TOML keys:

```sh
python3 - <<'PY'
import pathlib,tomllib
p=pathlib.Path('rust/Cargo.toml')
d=tomllib.loads(p.read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(d)} unique; no duplicates')
PY
```

```text
Workspace dependency keys: 75 unique; no duplicates
```


All seven oracle commands and byte comparisons were rerun against our pinned Rails image. This exact Python orchestration writes only authorized scratch and invokes the reference tools sequentially:

```sh
python3 - <<'PY' > .scratch/callback-reference-final.log
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
}
for name,path in outputs.items():
 output_path=root/'.scratch'/f'huddle-{name}-final.json'
 with output_path.open('wb') as output, (root/'.scratch'/f'huddle-{name}-final.log').open('wb') as error:
  subprocess.run(['rust/parity/bin/reference','runner','-e','RAILS_LOG_LEVEL=fatal',f'rust/reference-tools/huddle_{name}.rb'],env=env,stdout=output,stderr=error,check=True)
 subprocess.run(['cmp',str(output_path),str(root/path)],check=True)
 data=json.loads(output_path.read_bytes())
 count=len(data['cases']) if 'cases' in data else None
 print(f'{name}: reference rerun byte-identical; cmp exit 0'+(f'; {count} cases' if count is not None else ''),flush=True)
PY
```

```text
protocol: reference rerun byte-identical; cmp exit 0
cleanup: reference rerun byte-identical; cmp exit 0
grants: reference rerun byte-identical; cmp exit 0
gateway: reference rerun byte-identical; cmp exit 0; 39 cases
presence: reference rerun byte-identical; cmp exit 0; 50 cases
notices: reference rerun byte-identical; cmp exit 0; 67 cases
pushes: reference rerun byte-identical; cmp exit 0; 36 cases
```
The actual reference classes (grant, notifier, both pushers, preferences, membership, stream, internal controller) were also SHA256-compared with git show at the pin; no source substitution:

```sh
python3 - <<'PY' > .scratch/callback-source-identity.log
import hashlib,subprocess
paths=['app/models/huddle_grant.rb','app/models/huddle/join_notifier.rb','app/models/huddle/join_pusher.rb','app/models/huddle/invitation_pusher.rb','app/models/user/inbox_preferences.rb','app/models/membership.rb','app/models/stream.rb','app/controllers/internal/huddle_controller.rb']
raw=subprocess.check_output(['docker','run','--rm','--name','ws13-callback-source-check','--network','none','--entrypoint','sha256sum','ws13-reference:d7c7de92',*[f'/rails/{path}' for path in paths]],text=True)
for line in raw.splitlines():
 digest,path=line.split()
 local=path.removeprefix('/rails/')
 expected=hashlib.sha256(subprocess.check_output(['git','show',f'd7c7de92:{local}'])).hexdigest()
 assert digest==expected, local
print(f'Reference source identity: {len(paths)} files byte-identical to d7c7de92')
PY
```

```text
Reference source identity: 8 files byte-identical to d7c7de92
```


## Exactly what remains, in requested order

1. **Step 1, WS13:** complete post-issuance `clear_open_invitations!`, direct-recipient fan-out, recent-issuance and two-minute item dedup, owned-row/same-attempt refresh, suppressed-ring payload and policy-driven `silent`. The four huddle job handlers now exist; Presence/JoinNotice/PushInvitation exact unknown failures are recovered. Full Stream/Stage render callbacks remain step 2. Broader token-shape HTTP coverage beyond the existing 39 requests/98 lower-level signed shapes and permissive Rails timestamp/header cases also remains.
2. **Step 2, WS13:** full Stream model/create/end/quality/access callbacks and four broadcast updates; stale-after-30s sweep; full in-process reconciler in Rails order (overdue invitations >45s, stale streams >30s, existing due cleanup pass); voice/stage start/stop/current-call policies; last-host departure quiet timeline note and successor promotion, including deactivation; hand raise/lower/throttle, rank/self-moderation, server-mute/disconnect and role/rejoin events. WS8a's synchronous room-deletion stream ending and last-active Stage grant state are already preserved; a complete lifecycle/render parity claim is still pending.
3. **Step 3, WS13 + WS17:** complete invitation resolver and missed/dismissed/handled transition effects, full attempt ownership/dedup and suppression integration, normal ActivityItem invitation payload/render integration, complete notice delivery through public flows, and exact shared-policy/transport integration. Join/leave/rejoin/call-ended JSON and push payloads/helper scopes are done for the corpus above. WS17 registers/replays its transport seam; no actual delivery is claimed here.
4. **Step 4, WS13 using WS7/WS6:** all public huddle show/create/participants/leave, user-presence, voice/stage, role/hand/stream and moderation controllers; the other 18 huddle views and sidebar/header/group-DM locals integrated into the actual pages. The one participant partial is proven byte-identical for 50 renders. Existing WS4 CSP wasm/LiveKit-origin additions are present and their tests pass, but the complete public-screen/browser parity and any further CSP integration are pending.
5. **Step 5, WS13:** `plans/ws13-deferred-tests.md` retains all 548 original declarations across 33 files by title and owner, now with a grouped count table. They include partially covered declarations and are not 548 failing Rust tests. No entire original declaration was newly certified merely because a lower-level vector passed. Actual new case and Rust pass counts are above. Real LiveKit system tests remain deferred for **LIVEKIT_SYSTEM_TESTS=1 with a real LiveKit server**; the gateway's unchanged Node suite is proven against actual Rust HTTP.

Unproven external boundaries retained from the earlier handoff: successful TLS/open-timeout RoomService probes, broader network error taxonomy, process-kill/restart rehearsal, real LiveKit/production acceptance. No source failure was reassigned as inherited. There is no open design decision; completion is partial, and WS17's concrete seam is ready for lead reconciliation.
