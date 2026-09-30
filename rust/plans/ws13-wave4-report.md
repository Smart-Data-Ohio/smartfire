# WS13 continuation report — partial

Verified implementation HEAD: `d0145e5aa5c5eaa10af9e00eb29d235f7cbb6f6d` on `rust/ws13-huddles`. Six coherent source slices were committed and pushed in this continuation:

- `daa7c005`: composed huddle/Stage headers and complete voice/Stage new/edit pages.
- `6f498b9f`: all stream controller assertions and Stage page permissions.
- `d8a4e237`: viewer-specific voice/Stage sidebar sections and live dots.
- `8a55932c`: public huddle invitation, revocation/sign-out and leave request effects.
- `967dacfe`: internal signed-token, liveness and disconnect request effects.
- `d0145e5a`: remaining shared voice/Stage CRUD request assertions and icon Cable effects.

No new main merge or rebase was performed, as instructed. Required base `21a7332f` remains an ancestor through the existing merge. The oracle is still frozen Rails `d7c7de9264c63015be398001d7a1094e7695a6db`. All implementation changes are under `rust/`; the external report is mirrored at `rust/plans/ws13-wave4-report.md`.

## Delivered and accepted boundaries

The complete room header has 28 byte-identical production Rails region renders across Open, Closed, Direct, Voice and Stage rooms, quiet/live states, configuration states and Stage viewers. Requests now compose the room identity, pins/header overflow, involvement controls, presence stack, join CTA, Stage publish hint and stage panel. Room-kind/param-key mapping now preserves Voice and Stage STI identities. Actual Stage requests prove role forms, administrator moderation boundaries, muted administrator self-unmute, live presenter identity and stop permissions.

Complete voice/Stage new/edit page HTML and content have 14 byte-identical parity-seed renders. These include GitHub subscriptions, disabled/create/rotate inbound email settings and ordinary-member permission whitespace. Ordered brand/custom icon names now load at runtime. **The full-page goldens supply explicit plain Rails chrome inputs for VAPID, Drive, recent searches and notification policy. They prove render composition, not completion of those runtime provider adapters.** Only the established renderer CSRF/nonce inputs are deterministic (`GLOBAL`, `method:action`, `NONCE`); HTML is not normalized or masked.

The Voice and Stage sidebar section region has eight byte-identical production Rails renders. The oracle renders the real parent sidebar and selects the contiguous region by byte offsets, without serialization or rewriting. Actual sidebar requests now use call rows, current live state and per-viewer unread/mute/menu flags; `voice_rooms`/`stage_rooms` broadcast targets exist. The Stage live-dot declaration is complete. **The surrounding workspace shell, shared/direct/Board rows, favourites placement and category composition are still open.** Existing shared sidebar presenter signatures remain unchanged; these owned sections are an additive adapter.

Controller coverage now includes all 38 stream declarations, all 16 Stage view declarations, 35/36 public huddle declarations, 27/29 internal declarations, 23/24 Stage CRUD declarations and all 18 Voice CRUD declarations. The new tests capture committed invitation-job inserts without depending on worker consumption timing; validate exact invitation recipients/source grants; prove opaque grant reuse, membership removal, sign-out cleanup, public-URL alias denial, other-device preservation and exactly one room refresh through real Cable. Internal tests execute all 98 recorded signed token shapes through HTTP, both explicitly declared listener omissions, the 9/11-second liveness boundary, token-expired-independent lookup, stale-link cleanup and disconnect fan-out. CRUD tests cover silent denials, non-member message isolation, unknown-icon creation, listener defaults, empty/hostless rooms and member-only icon row/header replacements.

Existing CSP additions remain unchanged: `wasm-unsafe-eval` and paired LiveKit WebSocket/HTTP connect origins. The initializer is included in the 53-file frozen-source identity check and its existing tests pass in the fresh workspace suite.

The catalogue now retains all 548 original declarations: 237 passed and 311 partial/deferred, an increase of 90 covered original declarations in this continuation. Counts are declaration counts, not vector counts. No WS13b model/job/service declaration was newly ported or counted; its 37 previously passed and 179 remaining declarations are unchanged. No domain signatures, dependencies/lockfile, Rails sources, schema, asset overrides or parity masks were changed.

## Changed files and design notes

Paths in this table are relative to `rust/`; related templates/tests share a row.

| Files | Change |
| --- | --- |
| `crates/campfire/src/controllers/rooms.rs`, `rooms/call_navigation.rs`, `rooms/call_channels.rs` | Compose plain navigation/settings data and call form sections through existing read and domain APIs. |
| `crates/campfire/src/controllers/presenters.rs`, `presenters/view_context.rs` | Preserve Voice/Stage STI kinds, load ordered icon names and expose plain preference inputs to render tests. |
| `crates/campfire/src/controllers/users/sidebars.rs` | Read current call rows and attach viewer membership/menu flags. |
| `crates/campfire/src/controllers.rs`, `controllers/internal_huddle_tests.rs`, `rooms/call_channel_broadcast_tests.rs` | Register/reuse request tests and real-socket test helpers; no production domain signature change. |
| `controllers/rooms/call_page_tests.rs`, `stage_page_tests.rs`, `stream_controller_tests.rs`, `huddle_declaration_tests.rs`, `call_channel_declaration_tests.rs`, `controllers/internal_huddle_declaration_tests.rs` under `crates/campfire/src/` | New full-page/region and original declaration assertions. |
| `crates/campfire/src/controllers/rooms/call_channel_tests.rs` | Supply the additive plain settings field to existing form tests. |
| `crates/campfire/src/controllers/rooms/{form_page,page_view,sidebar_view}_vectors.json` | Commit actual frozen Rails outputs and plain render inputs. |
| `crates/views/src/messages.rs`, `rooms.rs` | Add Voice/Stage kinds, additive navigation data and room page/sidebar composition hooks. |
| `crates/views/src/rooms/{navigation,edit_sections,calls}.rs`, `crates/views/src/users.rs` | Typed, database-free render models and call sidebar sections. |
| `crates/views/src/{huddle,huddle_stage}.rs` | Add PartialEq derives to retain the surrounding ShowView trait contract; no field or domain behavior change. |
| `crates/views/templates/rooms/show.html`, `rooms/show/{_workspace_nav,_identity,_pins_panel,_header_overflow}.html` | Header composition and empty join-banner target. |
| `crates/views/templates/rooms/calls/{_new,_edit,_github_section,_inbound_section}.html`, `rooms/{voices,stages}/{new,edit,_form}.html` | Complete owned CRUD layouts/settings sections and exact form whitespace. |
| `crates/views/templates/users/sidebars/{show,_call_sections}.html` | Insert the owned call sections and real broadcast targets. |
| `reference-tools/huddle_{form_pages,page_views,sidebar_views}.rb` | Production Rails oracles using explicit renderer inputs. |
| `reference-tools/ws13_verify_{reference,declarations,corpora}.py`, `huddle_discrimination.py` | Frozen source identities, retained title/count checks, whole-file regeneration and compiled regression probes. |
| `plans/ws13-deferred-tests.md`, `plans/ws13-wave4-report.md` | Exact per-file ownership/pass counts and current acceptance evidence. |

Render models do not query or mutate the database; request adapters read plain inputs, and existing domain methods perform every lifecycle mutation. No additional user input is needed. Lead reconciliation of the shared parent chrome/sidebar adapters and WS17 registration remains an integration boundary.

## Stable WS13b, WS17 and view seams

WS13b retains ownership of every model/job/service file. All request tests call its existing room, membership, grant, stream and room-deletion interfaces. The lead merges both branches.

WS17 policy and transport integration remains at these unchanged seams:

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
pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>;
// RingRequest: recipient_id, sender_id, invitation: serde_json::Value.
```

WS17 evaluates current policy and enqueues transport in the transaction that prepares delivery/claims the join throttle. Policy denial and empty subscriptions do not burn the claim; enqueue failure rolls it back. `Notifications::HuddlePushJob` and `Notifications::HuddleRingJob` stay durable unknown-handler failures until WS17 registers them. Presence, cleanup, join-notice and push-invitation handlers remain registered.

Shared view integration changes for the lead/WS8b-r to reconcile: additive `ShowView.navigation`, Voice/Stage `RoomKind` variants and param keys, ordered icon names in Layout, and `SidebarShow.call_memberships`. The new plain models are `rooms::navigation::Navigation`, `rooms::edit_sections::EditSections`, and `users::SidebarCalls { ctx, rows: &[CallRow], can_create }`. `call_channels::row(app, conn, room) -> campfire_db::Result<CallRow>` and the domain signatures are unchanged. `call_navigation::model` and `edit_sections` are read-only request adapters. Runtime recent-search, Drive and notification-policy chrome providers remain with their owners.

## Fresh-clone acceptance

The clone was created from the committed branch after all six source pushes. It inherited no seed directories, Node modules, scratch fixtures or build target from the worktree. Its own seeds, target and gateway dependencies were built independently. Source checkout command, run from the owning worktree:

```sh
git clone --single-branch --branch rust/ws13-huddles --no-local . .scratch/fresh-composition-check
```

All commands below ran from `.scratch/fresh-composition-check` (with the metadata command running Cargo in its `rust/` directory). Tests retain `-j4` and `--test-threads=4`; no deadlines were widened.

```sh
mkdir -p .scratch
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/seed build default first_run > .scratch/fresh-seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > .scratch/fresh-workspace-test.log 2>&1
```

Exit zero. Full raw Cargo summary lines, including integration tests and doc tests:

```text
test result: ok. 388 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 74.51s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 49.23s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.84s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.24s
test result: ok. 428 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 129.44s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.82s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.46s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 29.93s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.91s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.03s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.99s
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

Parsed totals from those lines: **1442 passed, zero failed, 11 ignored**. The four app ignores include the explicitly run Node launcher and existing owner deferrals; `manages_bots` remains ignored pending WS11. The Node launcher was run explicitly below.

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399 NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture > .scratch/fresh-gateway-node.log 2>&1
```

The launcher installs the committed gateway lockfile in its own temporary directory and invokes the gateway's own `node --test` suite against real Rust endpoints. Exit zero:

```text
ℹ tests 16
ℹ pass 16
ℹ fail 0
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 391 filtered out; finished in 5.82s
```

```sh
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/fresh-clippy.log 2>&1
```

Exit zero:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 25s
```

```sh
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/fresh-seed-default-verify.log 2>&1
PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/fresh-seed-first-run-verify.log 2>&1
```

Both exit zero, default then first_run raw summary lines:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
```

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
Fresh clone source HEAD: d0145e5aa5c5eaa10af9e00eb29d235f7cbb6f6d
```

## Frozen reference and failing-first evidence

These three commands also ran in the fresh clone and exited zero:

```sh
python3 rust/reference-tools/ws13_verify_reference.py > .scratch/fresh-reference.log 2>&1
python3 rust/reference-tools/ws13_verify_declarations.py > .scratch/fresh-declarations.log 2>&1
python3 rust/reference-tools/ws13_verify_corpora.py > .scratch/fresh-corpora.log 2>&1
```

```text
Reference identity: 53 files match d7c7de92
Rails declaration catalogue: 548 titles retained; 237 passed; 311 partial/deferred; 33 files; source titles match
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
form_pages: reference rerun byte-identical; 14 cases
sidebar_views: reference rerun byte-identical; 8 cases
page_views: reference rerun byte-identical; 28 cases
public: reference rerun byte-identical; 65 cases
WS13 corpora: 22 complete files regenerated byte-identically
```

The source check includes the four primary controller/view test files, both CRUD test files, the CSP initializer, owned controllers/models/helpers/views and icon catalogue. Corpora are regenerated by real production Rails; comparing whole files does not normalize generated HTML. Existing domain corpora were rerun unchanged, not newly ported.

The following deliberate compiled regressions were run in the owning worktree, sequentially with sources restored afterward. Each required an assertion failure; compilation errors do not count. The first selector was run before the later sidebar mutation was added, and matched three cases at that slice. All nine were detected:

```sh
python3 rust/reference-tools/huddle_discrimination.py --only '^(composed-|stream-controller-id)' > .scratch/composition-discrimination.log 2>&1
python3 rust/reference-tools/huddle_discrimination.py composed-sidebar-call-sections-bypassed > .scratch/sidebar-discrimination.log 2>&1
python3 rust/reference-tools/huddle_discrimination.py --only '^public-huddle-(returned|leave)' > .scratch/huddle-declarations-discrimination.log 2>&1
python3 rust/reference-tools/huddle_discrimination.py --only '^internal-(first|authorization)' > .scratch/internal-declarations-discrimination.log 2>&1
python3 rust/reference-tools/huddle_discrimination.py call-channel-creator-update-guard-bypassed > .scratch/call-channel-declarations-discrimination.log 2>&1
```

```text
composed-stage-publish-hint-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 372 filtered out; finished in 0.58s
composed-edit-github-section-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 372 filtered out; finished in 0.21s
stream-controller-id-header-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 372 filtered out; finished in 0.70s
WS13 discrimination: 3 compiled regressions detected; sources restored
composed-sidebar-call-sections-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 374 filtered out; finished in 0.64s
WS13 discrimination: 1 compiled regressions detected; sources restored
public-huddle-returned-grant-id-corrupted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.55s
public-huddle-leave-effects-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.81s
WS13 discrimination: 2 compiled regressions detected; sources restored
internal-first-sighting-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 385 filtered out; finished in 2.11s
internal-authorization-payload-corrupted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 385 filtered out; finished in 0.44s
WS13 discrimination: 2 compiled regressions detected; sources restored
call-channel-creator-update-guard-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 391 filtered out; finished in 0.61s
WS13 discrimination: 1 compiled regressions detected; sources restored
```

The first composed Stage request failed before implementation on the missing listener publishing hint; the sidebar request failed on absent call-section targets before its adapter was written. The restored final room-controller filter passed before the last source push:

```text
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 336 filtered out; finished in 17.86s
```

## Timing flake retained without threshold changes

An earlier local app/views run during the first view slice failed `jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner` at `jobs/tests.rs:937`. Its empty-queue assertion saw an unrelated running `Retention::PruneJob`. Raw summary from `.scratch/composition-app-views.log`:

```text
test result: FAILED. 360 passed; 1 failed; 4 ignored; 0 measured; 0 filtered out; finished in 62.14s
```

This was reported and left unchanged. No concurrency reduction, deadline widening, ignore or targeted flake rerun was introduced. The required fresh-clone workspace run subsequently completed at normal concurrency with the 388/0/4 app result above. A trial controller-throttle mutation also showed why liveness values alone do not prove absence of transactions: the domain repeats its throttle, so both SQL-count declarations remain explicitly open.

## Per-file WS13 declaration counts and exact remainder

All original titles, including the 35 LiveKit inventory entries, remain in `rust/plans/ws13-deferred-tests.md`. Passed means every original assertion is covered, not simply that its endpoint exists.

| Rails file | Original | Passed | Partial/deferred |
| --- | ---: | ---: | ---: |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 38 | 0 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 35 | 1 |
| `test/system/huddles_test.rb` | 31 | 0 | 31 |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 27 | 2 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 23 | 1 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 16 | 4 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 13 | 6 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 18 | 0 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 16 | 0 |
| `test/system/huddle_join_notices_test.rb` | 16 | 0 | 16 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 9 | 6 |
| `test/system/stage_test.rb` | 15 | 0 | 15 |
| `test/system/voice_channels_test.rb` | 12 | 0 | 12 |
| `test/system/huddle_invitations_test.rb` | 10 | 0 | 10 |
| `test/system/huddle_audio_test.rb` | 8 | 0 | 8 |
| `test/system/huddle_roster_test.rb` | 8 | 0 | 8 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 5 | 1 |
| `test/system/huddle_presence_test.rb` | 6 | 0 | 6 |
| `test/integration/huddle_presence_test.rb` | 5 | 0 | 5 |

There are **26 remaining controller/integration declarations**, precisely: one public-huddle concurrent-issuance exception case; two internal SQL transaction/write/count cases; one Stage sole-host lock/revision transaction case; four roles fan-out/declaration combinations; six call-moderation declarations; six hands declarations; one aggregate-presence grants-query-count case; and all five huddle-presence integration declarations. The next largest controller files after Stage are roles and call moderation; Voice is complete.

Full generic room-show content (messages/composer, unread/OOO/poll integrations, member/thread panels and surrounding page chrome) and the complete workspace sidebar shell/favourites/categories are still open. The 14 complete CRUD page goldens do not close runtime recent-search/Drive/notification-policy provider integration. Shared owners and the lead reconcile those adapters. This report does not claim all 11 view integrations or full request-wide room-page byte identity.

All **106 browser system declarations** remain open. The **71 ordinary stubbed cases** need the browser harness and remaining page composition; they do not qualify for a real-LiveKit defer. Expressible LiveKit server operations retain the existing injectable RoomService/network fixture seam; no new test contacts a real LiveKit server or fabricates transport success. The **35 cases not expressible through that administrative-client seam** stay individually inventoried: all 31 in `test/system/huddles_test.rb` and four explicitly gated cases in `test/system/stage_test.rb`. They require real LiveKit/WebRTC media, reconnect/participant behavior and `LIVEKIT_SYSTEM_TESTS=1`; recorded administrative responses cannot supply those browser/media assertions.

WS13b's **179 remaining model/job/service declarations** and WS17's policy/transport registration stay with those owners. The lead merges their branches. Source work stops at six coherent pushed slices; the final report/catalogue update adds no implementation changes beyond the fresh-verified HEAD. No PR was opened.
