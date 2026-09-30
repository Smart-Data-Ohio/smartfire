# WS13 continuation report — partial

Verified source HEAD: `ad4de9dead5158a6a904785a99c044c9e1ba4c47` on `rust/ws13-huddles`. The resumed work produced two pushed source slices:

- `c6f342e4`: public huddle access, credentials and aggregate presence.
- `ad4de9de`: voice/Stage room controllers, owned render components, ordered Cable effects and call-channel deletion.

Required main `21a7332f` remains included through the previous merge commit `a8b6538c`; no rebase. A read-back during this continuation found `origin/main` at `2e20b24c3f2be9db8a646a1352c159b4afacad0e` (auth merge #161 and Rails changes #162/#163). Those later changes are not included in this branch. The lead's latest supplied base was `21a7332f`, and the shared rule asks workers to merge when the lead announces main's move. The Rails oracle remains the required frozen `d7c7de9264c63015be398001d7a1094e7695a6db` image, independently checked below.

## Delivered

Public `/rooms/:room_id/huddle` show/create, participants/leave and `/users/huddle_presence` are wired. Authentication failures return exact JSON before CSRF and configuration checks, with no-store even on errors. Bot-key, bot-session and inactive-user ordering matches Rails. Membership and soft-deletion scope are enforced; direct/group-DM display names use WS8a's existing domain method. Join credentials use existing strict token/grant APIs, including Stage roles and server mute. Leave affects only this session and room, preserves revocation state and other devices, and is idempotent. Aggregate presence deduplicates users, retains identities and queries grants once with a users preload. The query-count Rails declaration remains open because SQL tracing is not yet ported.

Voice/Stage show/new/create/edit/update and inherited index/destroy routes are mapped. Namespaced show redirects to generic room show. Creation restrictions, creator/administrator authorization, type isolation, icon normalization/validation, membership audit changes and redirects match 47 production Rails requests. The Stage sole-host check and revision share the existing immediate writer transaction and re-read hosts inside it. A present blank ID and an omitted list retain Rails' distinct behavior. Ordinary generic `/rooms/:id` deletion for voice/Stage delegates to WS8a's `room_delete::begin_destroy`: memberships, grants and live streams end synchronously; the durable destroy claim and audit persist; JSON has no redirect. Two additional real Rails deletion vectors cover that contract. Inherited namespaced destroy retains the Rails nil-room error behavior.

Real sockets verify create's per-member section prepend, removed members' header-stack removal followed by row removal, and remaining members' row replacement followed by header replacement. Shared frames carry no CSRF token or CSP nonce. A discovered callback configuration gap was fixed: membership removal uses the booted app's captured huddle configuration, with environment fallback only when no app is installed.

Byte comparisons over the default parity seed cover 16 complete render cases plus three huddle layout panels: new/edit voice/Stage forms, quiet/live shared/member sidebar rows, voice/Stage header identities and both new-page content blocks. There is no HTML normalization or output mask. CSRF placeholders are explicit Rails renderer inputs, following the views foundation's existing deterministic token protocol. The form user rows include current Rails profile-card attributes. Real requests now enable the three existing application-layout huddle panels when configured.

Both edit templates exist, but their full page composition is not accepted yet. The current shared edit layout lacks GitHub and inbound-email sections; room-show and sidebar composition still need the owned call/stage components inserted. Full request-wide page bytes remain open. The existing merged CSP additions are unchanged: wasm-unsafe-eval and paired LiveKit WebSocket/HTTP connect origins. Their Rails vector tests passed in the fresh suite; the initializer hash matches the frozen pin.

No model, job or service declaration was newly ported after the WS13b split. No domain signatures, Rails sources, schema/migrations, asset overrides, sidecars, masks/allowlists or dependencies/lockfile were changed. The source changes are under `rust/`.

## Stable seams and integration notes

WS13b owns every model/job/service file in the catalogue. Its 179 remaining declarations and the previously covered 37 domain declarations are preserved unchanged. The lead merges the branches; the new controllers use existing room, membership, grant, stream and deletion APIs.

WS17 policy and transport remain at the previously delivered seams; these signatures are unchanged:

```rust
// db::models::huddle_notices
pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest);
pub fn prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool)
    -> Result<Option<PushDelivery>>;
// PushRequest: kind Huddle/HuddleJoin, recipient_id, sender_id, room_id,
// room_membership_id: Option<i64>, payload { title, body, path, tag }.

// db::models::huddle_invitations
pub fn publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool)
    -> Result<()>;
// RingRequest: recipient_id, sender_id, invitation: serde_json::Value.

pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>;
```

WS17 evaluates current policy and enqueues transport in the transaction that prepares delivery and claims the join throttle. Policy denial/no subscriptions do not burn the claim; enqueue failure rolls it back. `Notifications::HuddlePushJob` and `Notifications::HuddleRingJob` remain durable unknown-handler failures, rather than stub successes, until WS17 registers them. Existing presence, cleanup, join-notice and push-invitation handlers remain registered.

Shared files for the lead/WS8b-r to reconcile: additive concerns authentication overrides from `c6f342e4`; `controllers.rs` route registrations; `rooms.rs` voice/Stage scopes, generic call deletion delegation and open/closed scope isolation; layout configuration; and the membership-removal sink's captured configuration. New view models are in `campfire_views::rooms::calls`; `call_channels::row(app, conn, room) -> campfire_db::Result<CallRow>` supplies a neutral broadcast row. Room/sidebar request composition is still outstanding; do not treat the new row templates as a completed sidebar integration.

## Fresh-clone verification

A new self-contained clone was made only after both source commits were pushed. It inherited no worktree `.scratch` fixtures, Node dependencies, seeds or build target. Its seeds and target were built locally. Source checkout:

```sh
git clone --single-branch --branch rust/ws13-huddles --no-local . .scratch/fresh-public-check
```

All commands in this subsection ran from `.scratch/fresh-public-check`. Both seed builds exited zero:

```sh
mkdir -p .scratch
PARITY_NAMESPACE=ws13 PARITY_OWNER=ws13 PARITY_CPUS=2 PARITY_IMAGE=ws13-reference:d7c7de92 rust/parity/bin/seed build default first_run > .scratch/fresh-seed.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The whole workspace, including html5ever and doctests, exited zero:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > .scratch/fresh-workspace-test.log 2>&1
```

Raw summary lines in executable order (first is the app; 428-pass line is the database):

```text
test result: ok. 358 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 40.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.64s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 428 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 59.87s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.93s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.24s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.58s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.44s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.00s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.95s
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

Aggregate: 1,412 passed, zero failed, 11 ignored. Explicit Node acceptance below executes one ignored app test. Other ignores remain WS11 manages_bots, reference-frame recording, push latency measurement, three external Rails DB/export tests, exact media bytes, mail export and two kit documentation examples. Exact-media container acceptance and PEBBLE ACME were not run. No seeded app failure was skipped.

The gateway's unchanged own suite ran through the Rust internal endpoints, installing its committed Node lockfile in a test-created tempfile, and exited zero:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture > .scratch/fresh-gateway-node.log 2>&1
```

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 2949.938581
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 361 filtered out; finished in 3.33s
```

Workspace all-targets clippy exited zero:

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 01s
```

Fresh seed validation by real Rails, both exit zero:

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/fresh-seed-default-verify.log 2>&1
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/fresh-seed-first-run-verify.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

Locked metadata, duplicate workspace-key check and required-base ancestry ran in the clone, exit zero:

```sh
python3 - <<'PYCODE' > .scratch/fresh-metadata.log
import pathlib,subprocess,tomllib
root=pathlib.Path.cwd()
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','metadata','--locked','--format-version','1'],cwd=root/'rust',stdout=subprocess.DEVNULL,check=True)
print('cargo metadata --locked --format-version 1: exit 0')
d=tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(d)} unique; no duplicates')
subprocess.run(['git','merge-base','--is-ancestor','21a7332f','HEAD'],check=True)
print('Required main 21a7332f is an ancestor of HEAD: exit 0')
print('Fresh clone source HEAD: '+subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip())
PYCODE
```

```text
cargo metadata --locked --format-version 1: exit 0
Workspace dependency keys: 75 unique; no duplicates
Required main 21a7332f is an ancestor of HEAD: exit 0
Fresh clone source HEAD: ad4de9dead5158a6a904785a99c044c9e1ba4c47
```

## Differential and failing-first evidence

Before wiring the voice/Stage controllers, both access tests failed on 501-versus-403/422 assertions, not compilation failures. They subsequently passed with the implementation. This continuation also reran the following deliberate compiled regressions; each failed an actual assertion and the mutation tool restored the original source in `finally`. No other Cargo process touched this worktree during mutation. From the owning worktree, exit zero:

```sh
python3 rust/reference-tools/huddle_discrimination.py --only '^call-channel-' > .scratch/call-discrimination.log 2>&1
```

```text
call-channel-update-policy-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 361 filtered out; finished in 0.75s
call-channel-sole-host-check-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 361 filtered out; finished in 0.75s
call-channel-broadcast-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 361 filtered out; finished in 4.24s
call-channel-removal-header-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 361 filtered out; finished in 0.91s
call-channel-deletion-seam-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 361 filtered out; finished in 0.89s
WS13 discrimination: 5 compiled regressions detected; sources restored
```

All 19 complete golden files were regenerated by the actual pinned production Rails models/controllers/partials and compared as complete files, exit zero:

```sh
python3 rust/reference-tools/ws13_verify_corpora.py > .scratch/current-corpora.log 2>&1
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
call_channels: reference rerun byte-identical; 47 cases
call_views: reference rerun byte-identical; 16 cases
public: reference rerun byte-identical; 65 cases
WS13 corpora: 19 complete files regenerated byte-identically
```

Reference image identity (including public controllers, owned partials and CSP) was rechecked against git's frozen pin, exit zero:

```sh
python3 rust/reference-tools/ws13_verify_reference.py > .scratch/current-reference.log
```

```text
Reference identity: 32 files match d7c7de92
```

Every retained original Rails title and its count was checked, exit zero:

```sh
python3 rust/reference-tools/ws13_verify_declarations.py > .scratch/current-declarations.log
```

```text
Rails declaration catalogue: 548 titles retained; 147 passed; 401 partial/deferred; 33 files; source titles match
```

## Remaining, exactly

WS13 is partial. No browser system declaration is newly claimed passed. `rust/plans/ws13-deferred-tests.md` retains every original title, its passed marker and per-file counts, with files ordered by original size. It now has 147 covered declarations and 401 open declarations: 222 belong to WS13 and 179 belong to WS13b. Counts are original declarations, not Rust test or vector counts.

1. Full public HTML composition: insert the owned huddle launcher/participant stack and Stage panel/roster into generic room-show navigation; retain voice/Stage STI DOM IDs, correct settings routes and room labels; render typed voice/Stage rows into actual sidebar voice/stage sections, including membership flags; finish both edit-page compositions with shared GitHub/inbound-email sections. Compare full new/edit/room/sidebar requests byte-for-byte over the parity seed. The current generic presenter maps these room types to Closed, and the sidebar still uses the generic shared row: those are known remaining integration gaps, not accepted parity.
2. The remaining 116 controller/integration declarations, including internal-token shape expansion and read/write query assertions, public huddle enqueue/revocation/sign-out/concurrency/fan-out assertions, creation no-difference checks, Stage zero-host/empty/concurrent edit cases, icon fan-out, scope combinations, and the five header-presence integration cases. Keep the catalogue's exact remaining titles as the work list, largest files first. The query-count presence declaration remains open even though the implementation batches the query.
3. The 71 ordinary stubbed browser declarations across huddle invitations/join notices/audio/roster/presence, voice channels and Stage. They need the public composition and browser harness; they do not qualify for a real-LiveKit defer.
4. Thirty-five explicitly gated real-LiveKit declarations may stay deferred: all 31 in `test/system/huddles_test.rb` and four in `test/system/stage_test.rb`, because they require `LIVEKIT_SYSTEM_TESTS=1` and a configured real LiveKit server.
5. WS13b's remaining 179 model/job/service declarations are that worker's responsibility. WS17 policy/transport integration stays at the stable seams above for the lead to reconcile.

Current public-file declaration counts:

| Rails file | Passed | Open |
| --- | ---: | ---: |
| `test/controllers/rooms/huddles_controller_test.rb` | 23/36 | 13 |
| `test/controllers/rooms/stages_controller_test.rb` | 15/24 | 9 |
| `test/controllers/rooms/voices_controller_test.rb` | 13/18 | 5 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 5/6 | 1 |

The final report/catalogue commit contains no Rust implementation changes beyond the verified source HEAD above. No PR was opened. Stop point: two coherent pushed source slices, independently built seeds, zero-failure fresh workspace, gateway acceptance and clean workspace clippy.
