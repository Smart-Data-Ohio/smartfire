# WS13 continuation report — partial

Fresh-verified implementation HEAD: `cdb9b7578047ca7f5d44aa2740e74f4e6fcac054` on `rust/ws13-huddles`. Four coherent source slices were committed and pushed in this continuation:

- `b65e72be`: complete the remaining 26 controller/integration declarations.
- `abf4ecde`: runtime recent-search, Drive/Picker and notification-sound chrome adapters.
- `66e4c66d`: Markdown composer, member/thread panels and poll-builder composition.
- `cdb9b757`: complete viewer-specific sidebar frame composition.

No main merge or rebase was performed. Base `21a7332f` remains an ancestor. WS13b/WS17 domain and delivery signatures are unchanged. All implementation changes are under `rust/`; this external report is mirrored at `rust/plans/ws13-wave4-report.md`.

## Delivered boundaries

All **226/226 original controller/integration declarations** now have complete assertions. The last 26 are backed by nine tests in `remaining_call_tests.rs`, `remaining_query_tests.rs` and `remaining_presence_tests.rs`: Stage action/type/privacy isolation, administrator host repair, other-administrator moderation, mute/unmute publishing transitions and cleanup, imported-grant drift, exact personalized hand/promotion/mute Cable frames and silent repeat raises. Executed SQLite traces cover every configured reader and the writer: at most eight steady-state statements with no transaction/write; exactly one denial grant update and cleanup insert; one grants query and users preload across three rooms; host check plus member insert inside one immediate transaction; and a barrier-controlled membership-revocation race that returns the controlled JSON denial without issuing a grant. All five presence integration declarations inspect actual room GET HTML.

Runtime chrome now loads recent searches in update order with the Rails ten-entry limit; Drive scope without token decryption; public Picker configuration; manual/presence DND; quiet hours; cached meeting intervals; and manual/calendar OOO sound intervals. **23 persisted Rails states and eight Picker configurations** are checked through both plain adapter data and actual voice-new HTTP metadata/dropdowns. This is read-only rendering data; WS17 retains policy and transport. Icon names now use the existing tracked `campfire/vendor/icons.yml`, removing the out-of-Rust include that could depend on local state.

The generic room page now uses the native Markdown composer, schedule-send dialog, member panel, thread panel, multi-select/tracked-work controls and poll builder. **30 complete Rails fragments** match byte for byte across Closed, named/unnamed Direct, Voice and Stage rooms and three Drive modes. Assets, form tokens, room ids, STI targets and viewer/neutral labels are ordinary template inputs. These fixtures are not runtime HTML. **Full surrounding room/page byte identity remains open.**

The complete sidebar turbo frame has **17 byte-identical cold Rails renders** over the parity seed. This includes workspace identity/destinations/tools, shared/Board/Voice/Stage/DM rows, placeholders, favourites, category controls, room menus and the #163 profile-card trigger. Actual request tests verify persisted placement, escaped names and administrator-only group-DM deletion flags. DM avatar order follows the preloaded membership association while the label sorts independently; collection whitespace also matches. Model-valid Voice/DM category assignments retain Rails' simultaneous home/category placement. Each oracle scenario clears its own isolated cache before changing startup configuration; an enabled process does not inherit disabled-process fragments.

The sidebar oracle is frozen `d7c7de9264c63015be398001d7a1094e7695a6db` plus only the exact sidebar template from `2e20b24c3f2be9db8a646a1352c159b4afacad0e`. The override is tracked under `reference-tools/sidebar_reference`, its SHA256 is checked, and its isolated image is rebuilt from that tracked context in the fresh clone. No root Rails sources or main branch were edited. Existing 14 full CRUD-page goldens still target the frozen shared layout with explicit chrome inputs; this report does not claim post-#163 shared application-layout parity.

CSP additions remain unchanged: `wasm-unsafe-eval` and paired LiveKit WebSocket/HTTP connect origins. The frozen initializer is included in the source identity check and its existing tests pass in the fresh workspace suite.

## Original declaration counts by file

Counts are original Rails declarations, not Rust-test/vector counts. All 548 titles remain inventoried across 33 files: **263 passed, 285 open**. WS13b's historical 37 passed / 179 open domain declarations remain untouched; the lead reconciles its separate branch. The 106 browser system declarations remain open by explicit instruction.

| Rails file | Original | Passed | Open |
| --- | ---: | ---: | ---: |
| `test/controllers/rooms/stage/streams_controller_test.rb` | 38 | 38 | 0 |
| `test/controllers/rooms/huddles_controller_test.rb` | 36 | 36 | 0 |
| `test/controllers/internal/huddle_controller_test.rb` | 29 | 29 | 0 |
| `test/controllers/rooms/stages_controller_test.rb` | 24 | 24 | 0 |
| `test/controllers/rooms/stage/roles_controller_test.rb` | 20 | 20 | 0 |
| `test/controllers/rooms/call_moderation_controller_test.rb` | 19 | 19 | 0 |
| `test/controllers/rooms/voices_controller_test.rb` | 18 | 18 | 0 |
| `test/controllers/rooms/stage_view_test.rb` | 16 | 16 | 0 |
| `test/controllers/rooms/stage/hands_controller_test.rb` | 15 | 15 | 0 |
| `test/controllers/users/huddle_presence_controller_test.rb` | 6 | 6 | 0 |
| `test/integration/huddle_presence_test.rb` | 5 | 5 | 0 |
| **Controllers/integration** | **226** | **226** | **0** |

## Changes by file and design notes

Paths below are relative to `rust/`; related partials/fixtures share a row.

| Files | Change |
| --- | --- |
| `crates/campfire/src/config.rs`, `controllers/presenters.rs`, `presenters/view_context.rs`, `presenters/runtime_chrome.rs` | Load public Picker configuration and persisted chrome inputs; use the already tracked icon catalogue. |
| `controllers/presenters/{sql_probe,test_support}.rs` under `crates/campfire/src/` | Test-only executed-SQL tracing and a shutdown seam for the measured app's own job runner. Reader pool size and suite concurrency are unchanged. |
| `controllers/rooms/{remaining_call_tests,remaining_query_tests,remaining_presence_tests}.rs` | Full assertions for the remaining 26 declarations. |
| `controllers/presenters/{chrome_tests.rs,chrome_vectors.json}` | Recorded sound/Drive states and configuration/request tests. |
| `controllers/rooms.rs`, `controllers/rooms/{room_composition_tests.rs,room_composition_vectors.json}` | Load the neutral DM thread-panel label and assert actual requests/30 whole fragments. |
| `crates/views/src/rooms.rs`, `rooms/composition.rs`, `templates/rooms/show.html`, `templates/rooms/composition/{_composer_none,_composer_legacy,_composer_picker,_member_panel,_thread_panel,_poll_builder}.html` | Integrate native room components with request-specific helpers, assets, labels and tokens. |
| `controllers/users/sidebars.rs`, `sidebars/{composition,tests}.rs`, `controllers/users/full_sidebar_vectors.json` | Load viewer-specific full sidebar data, preserve Rails placement/order/menu flags, verify requests and 17 whole frames. |
| `crates/views/src/users.rs`, `users/sidebar.rs`, `templates/users/sidebars/show.html`, `templates/users/sidebars/composition/{_shell,_shared,_direct,_placeholder,_favorites,_category}.html` | Native parent/frame/row/category composition; retain existing fallback presenter signatures. |
| `reference-tools/huddle_{chrome,room_composition,full_sidebar}.rb` | Real isolated Rails recorders with ordinary plain inputs and whole captured output. |
| `reference-tools/huddle_{room_composition_templates,sidebar_templates,sidebar_reference}.py`, `reference-tools/sidebar_reference/` | Native template transcription and a tracked, hash-checked #163 oracle overlay. |
| `reference-tools/ws13_verify_{reference,declarations,corpora}.py`, `huddle_discrimination.py` | Frozen identities, retained declaration counts, whole-file regeneration and actual compiled assertion-failure probes. |
| `plans/ws13-deferred-tests.md`, `plans/ws13-wave4-report.md` | Current per-file coverage, untouched system inventory and raw fresh-clone acceptance. |

Views consume plain data and do not query or mutate the database. Production request adapters read through the existing domain APIs. SQL instrumentation is test-only; successful teardown detaches every callback before freeing its state. No dependency/lockfile, schema, root Rails source or parity-mask change was made. There are no open questions requiring user input; the shared view and transport reconciliation boundaries below remain for the lead.

## Stable seams and shared view reconciliation

```rust
// db::models::huddle_notices
pub fn enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest);
pub fn prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool)
    -> Result<Option<PushDelivery>>;
// PushRequest: Huddle/HuddleJoin, recipient_id, sender_id, room_id,
// room_membership_id: Option<i64>, payload { title, body, path, tag }.

// db::models::huddle_invitations
pub fn publish_ring(tx: &mut Tx<'_>, request: &RingRequest, sound_allowed: bool)
    -> Result<()>;
pub fn resolve_overdue(tx: &mut Tx<'_>, user_id: Option<i64>) -> Result<()>;
```

WS17 retains Notifications::Policy and transport registration. `Notifications::HuddlePushJob` and `Notifications::HuddleRingJob` remain durable unknown-handler failures until it registers them; no successful transport delivery is fabricated. Existing WS13b room/membership/grant/stream/job/service interfaces are unchanged, and no model/job/service Rails declarations were newly ported here.

Additive view seams for the lead/WS8b-r2: `ShowView.thread_panel_name`, `SidebarShow.composition: Option<&users::sidebar::Sidebar>`, `rooms::composition::render`, and `users::sidebar::Sidebar::render`. Existing shared sidebar presenter signatures and `call_channels::row` are unchanged. The request adapter binds the full frame; shared/direct row broadcasts still use WS8a's older presenters and need reconciliation with this composition.

## Fresh-clone acceptance

The committed source branch was cloned with no seed, target, Node modules or untracked fixtures inherited from the worktree:

```sh
git clone --single-branch --branch rust/ws13-huddles --no-local . .scratch/fresh-sidebar-check
```

Commands below were rerun from that clone. The metadata command runs Cargo in its `rust/` directory. Common environment:

```sh
export TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CI=1 CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399
export PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2
```

```sh
rust/parity/bin/seed build default first_run > .scratch/fresh-seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > .scratch/fresh-workspace-test.log 2>&1
```

Exit zero: **1,458 passed, zero failed, 11 existing ignores across 48 raw summaries**. Seeded app: 404/0/4; DB: 428/0/3. No concurrency reduction, threshold widening or new ignores.

```text
test result: ok. 404 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 40.27s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.94s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 428 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 76.40s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.33s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.66s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.02s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.67s
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

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/fresh-clippy.log 2>&1
```

Exit zero, raw final line:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 28s
```

```sh
NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=4 > .scratch/fresh-node.log 2>&1
```

The pinned gateway dependencies were installed in the clone's own scratch directory. Its existing 16 Node tests execute through real Rust endpoints, with only the original explicit outage/stall/malformed injections retained. Raw summary:

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 3356.522444
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 407 filtered out; finished in 3.91s
```

```sh
python3 rust/reference-tools/ws13_verify_reference.py > .scratch/fresh-source-identities.log 2>&1
python3 rust/reference-tools/ws13_verify_declarations.py > .scratch/fresh-declarations.log 2>&1
python3 rust/reference-tools/ws13_verify_corpora.py > .scratch/fresh-corpora.log 2>&1
```

All exit zero. Raw identity/catalogue summaries and complete regeneration summaries:

```text
Reference identity: 73 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Rails declaration catalogue: 548 titles retained; 263 passed; 285 partial/deferred; 33 files; source titles match
Sidebar reference: tracked #163 source SHA256 verified; isolated image built
room_composition: reference rerun byte-identical; 30 cases
chrome: reference rerun byte-identical; 23 cases
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
full_sidebar: reference rerun byte-identical; 17 cases
WS13 corpora: 25 complete files regenerated byte-identically
```

```sh
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default > .scratch/fresh-seed-default-verify.log 2>&1
rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run > .scratch/fresh-seed-first-run-verify.log 2>&1
```

Both exit zero, raw outputs:

```text
{
  "checks": {
    "markdown": true,
    "thread_replies": true,
    "forwards": true,
    "polls_open": true,
    "polls_closed": true,
    "pins": true,
    "saved_pending": true,
    "saved_done": true,
    "scheduled_pending": true,
    "scheduled_sent": true,
    "group_dms": true,
    "voice": true,
    "stage": true,
    "boards": true,
    "all_work_states": true,
    "events": true,
    "agents": true,
    "approvals": true,
    "activity": true,
    "github_recording": true,
    "fizzy_recording": true,
    "x_recording": true,
    "linkedin_recording": true,
    "link_recording": true,
    "remembered_two_factor": true,
    "enrolled_two_factor": true,
    "setup_secret": true,
    "verified_sessions": true,
    "no_presence_leases": true
  },
  "passed": 29,
  "failed": 0
}
{
  "checks": {
    "no_account": true,
    "no_users": true,
    "no_rooms": true,
    "no_messages": true
  },
  "passed": 4,
  "failed": 0
}
```

```python
from pathlib import Path
import subprocess, tomllib
root = Path.cwd()
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','metadata','--locked','--format-version','1'], cwd=root/'rust', stdout=subprocess.DEVNULL, check=True)
print('cargo metadata --locked --format-version 1: exit 0')
d = tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(d)} unique; no duplicates')
subprocess.run(['git','merge-base','--is-ancestor','21a7332f','HEAD'], check=True)
print('Required base 21a7332f: ancestor; no new main merge')
print('Fresh clone HEAD: '+subprocess.check_output(['git','rev-parse','HEAD'], text=True).strip())
```

Raw lines:

```text
cargo metadata --locked --format-version 1: exit 0
Workspace dependency keys: 75 unique; no duplicates
Required base 21a7332f: ancestor; no new main merge
Fresh clone HEAD: cdb9b7578047ca7f5d44aa2740e74f4e6fcac054
```

## Fail-first and compiled regression checks

The runtime chrome, room-composer and full-sidebar request assertions failed against the previous adapters before those view changes. During these source slices the following owning-worktree commands compiled deliberate regressions, required actual assertion failures, and restored source bytes. Compiler/linker interruptions were retried with unchanged settings; they are not counted as detected regressions. No timing test failed in this continuation.

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py --only '^remaining-' > .scratch/remaining-discrimination.log 2>&1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py runtime-chrome-request-adapter-bypassed > .scratch/chrome-discrimination.log 2>&1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py --only '^room-composition-' > .scratch/room-composition-discrimination.log 2>&1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py --only '^full-sidebar-|^composed-sidebar-call-sections' > .scratch/full-sidebar-discrimination.log 2>&1
```

The actual pre-implementation failures were:

- `runtime_chrome_reads_twenty_three_recorded_rails_sound_and_drive_states`: indefinite DND returned `muted: false` instead of `true`.
- `room_composition_replaces_the_upstream_composer_with_rails_markdown`: missing `composer__textarea`.
- `full_sidebar_request_composes_workspace_destinations_and_profile_card_trigger`: missing `workspace-identity`.

Raw lines from this continuation, before the corresponding fixes:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 401 filtered out; finished in 0.12s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 403 filtered out; finished in 0.88s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 405 filtered out; finished in 0.59s
```

Ten compiled regressions detected across these slices, raw lines:

```text
remaining-stage-type-isolation-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.43s
remaining-internal-steady-check-writes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 400 filtered out; finished in 2.22s
remaining-personal-roster-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.39s
remaining-header-presence-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.42s
WS13 discrimination: 4 compiled regressions detected; sources restored
runtime-chrome-request-adapter-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.64s
WS13 discrimination: 1 compiled regressions detected; sources restored
room-composition-room-binding-corrupted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 404 filtered out; finished in 0.10s
room-composition-request-panels-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 404 filtered out; finished in 0.43s
WS13 discrimination: 2 compiled regressions detected; sources restored
full-sidebar-request-composition-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.98s
full-sidebar-menu-viewer-flags-corrupted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.16s
composed-sidebar-call-sections-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 407 filtered out; finished in 0.73s
WS13 discrimination: 3 compiled regressions detected; sources restored
```

## Exact remainder

- Full generic room-show composition: message-area/list wrappers and first-paint preloads; the current-user pending-message template and its attachment/card/thread affordances; welcome/invitation, unread-divider/jump-to-unread handling; per-viewer OOO notices and live subscriptions; and whole room/page byte comparisons using runtime chrome. The new composer/member/thread/poll fragments are integrated and complete, but do not prove the full surrounding page.
- Shared/direct row refresh and broadcast composition with WS8a's presenters; shared post-#163 application-layout/status-action reconciliation with WS8b-r2. The full sidebar request frame is complete, but full browser interaction is deferred.
- All **106 system declarations** await the end-to-end phase: 71 ordinary stubbed browser cases and 35 real LiveKit cases. All 35 remain individually inventoried: 31 in `test/system/huddles_test.rb` and four in `test/system/stage_test.rb`. Administrative-client recordings cannot express their browser WebRTC/media/reconnect/participant assertions; those need a real server and `LIVEKIT_SYSTEM_TESTS=1`. No browser/system declaration was reclassified as passed.
- WS13b's unmerged model/job/service work and WS17's policy/transport integration remain with those workers and the lead. Historical inventory counts on this branch are retained, not claims about their current separate branches.

Source work stops at four coherent pushed slices with the above remainder. The final report commit changes documentation only beyond the fresh-verified implementation SHA. No PR was opened.
