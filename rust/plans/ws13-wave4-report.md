# WS13 continuation report — partial

Fresh-verified source HEAD: `cce05c441cbf3a8f9e959c581b69ef7ee6decb4f` on `rust/ws13-huddles`. This continuation pushed the main merge and five coherent follow-up slices. The final report commit changes documentation only beyond that verified source SHA.

| Commit | Delivered |
| --- | --- |
| `e329e372` | Merge the fetched main snapshot `8952bed4` with a merge commit, including WS15e #166 and the shared asset-golden helper #168. Preserve both huddle and agent revocation, embed handlers, room composition and test support through the merge conflicts. |
| `3a3b8be0` | Adapt the huddle HTTP error/timeout and test-job shutdown helpers to main; use the shared asset-golden comparison for complete CRUD pages. |
| `ba5eb880` | Complete surrounding room composition: pending template, accessible message area/list, preloads, invitation, unread/jump controls and per-viewer live OOO notices. |
| `f34560a0` | Integrate shared/direct row refresh and WS8 callback rendering; preserve recipient menu/avatar state, exact group-DM headers, mute/read JSON responses and live OOO updates. |
| `29682e5f` | Match populated message collections and unread boundaries through whole-page Rails fixtures; retain two shared-message integration captures as deferred. |
| `cce05c44` | Isolate the inherited Fizzy matrix's subprocess origins after fresh verification exposed a shared-port collision. Test-only; all 17 cases pass. |

The main snapshot was fetched at the start of this continuation; no subsequent main merge was made. Later WS17 integration remains for the lead. Board drift #164/#165 was merged as reference input and remains WS12-owned. All implementation changes are under `rust/`. No root Rails, schema, dependency, lockfile or parity-mask change was made by the WS13 slices. No browser screenshot or pixel-diff work remains on this list.

## Delivered behaviour and byte parity

The native room page now has **36 complete HTML/content comparisons** against frozen Rails: 20 empty-timeline pages across Open/Closed/Direct/Voice/Stage, configuration and viewer role; plus 16 actual seed/unread scenarios. The actual seed cases load the real database timeline and native message presenter, and independently compare room/header/shell adapters with the recorded plain inputs. They include the original room's invitation and messages, a 40-message last page, pair/group DMs, actual seed Voice/Stage pages, an on-page seven-message unread boundary and an off-page jump. Rails renders each message collection in one insertion; the runtime now preserves those collection and divider whitespace boundaries. Fixtures are oracle data, never runtime HTML.

**38 complete shell renders** cover pending messages for both viewers, eleven first-paint preloads, all five message-area/list kinds, unread counts and jump variants, the actual original-room invitation, and manual/calendar/overlap/expired/invisible OOO states for both viewers. Request tests verify accessible live-log markup, restored pending attachment/card/thread affordances, viewer-specific subscriptions, escaped notes, invisibility suppression, no cached viewer state, cursor tie/deletion/off-page handling and the unchanged `> 5` scroll threshold. The OOO parent and callback use the same native line template.

**16 standalone sidebar rows and six group-DM headers** match complete Rails partials. Runtime create/refresh/involvement and the existing `DirectSidebar`, `RoomHeader` and `OooNotice` callback descriptions now render through native composition. Real signed Cable subscriptions verify ordered rename rows/headers, recipient-specific delete/leave flags and avatars, live OOO updates/clears, and absence of session-bound frame content. The request title and header use WS8a's existing direct-display-name method. JSON mute clears earlier unread state and returns empty HTTP 200, avoiding the Rails-described fetch redirect loop. WS8 callback/domain signatures are unchanged.

Earlier accepted coverage remains: 226/226 original controller/integration declarations; 400 Stage fragment renders; 28 complete headers; 14 complete CRUD pages; 30 composer/member/thread/poll fragments; 17 complete post-#163 sidebar frames; and 23 persisted runtime chrome states plus eight Picker configurations. The existing CSP additions remain verified in the fresh app suite: `wasm-unsafe-eval` and paired LiveKit WebSocket/HTTP origins.

Whole-page fixtures use frozen `d7c7de92`. The sidebar oracle additionally uses only the tracked, hash-checked #163 sidebar source from `2e20b24c`; shared #163 application-layout/status-action reconciliation is still open. The shared #168 helper verifies current logical assets and their real digests before comparing markup. No new mask was added.

## Files and design notes

Paths are relative to `rust/`.

| Files | Change |
| --- | --- |
| `crates/campfire/src/controllers/rooms.rs`, `rooms/{shell,call_navigation}.rs` | Load plain message-shell/unread/private-notice/header data; integrate native shared rows. |
| `controllers/rooms/{directs,involvements}.rs`, `controllers/users/sidebars/composition.rs` under `crates/campfire/src/` | Bind neutral or recipient membership composition; use correct JSON mute/read effects and direct labels. |
| `controllers/presenters.rs`, `presenters/accounts.rs`, `presenters/runtime_chrome.rs` | Reuse domain direct-display names and existing persisted chrome adapters. |
| `crates/campfire/src/channels/{sink,room_composition}.rs` | Render the three existing room/OOO callback partial descriptions on the committing thread. |
| `crates/views/src/{lib,rooms}.rs`, `rooms/{shell,navigation}.rs`, `users/sidebar.rs` | Add plain render data, context cloning, standalone row/identity/OOO line rendering and complete message collections. Views perform no SQL. |
| `crates/views/templates/rooms/{show.html,shell/*,show/_invitation.html}`, `templates/messages/_template.html`, `templates/accounts/_invite.html`, sidebar composition templates | Transcribe exact native markup/whitespace and restore Smartfire invitation branding. |
| `controllers/rooms/{room_shell_tests,full_room_tests,row_broadcast_tests}.rs` and corresponding vectors | Complete component/page oracles and real request/Cable security and effect assertions. |
| `reference-tools/huddle_{room_shell,full_rooms,row_broadcasts}.rb`, `ws13_verify_*.py`, `huddle_discrimination.py` | Real Rails recordings, frozen provenance/catalogue checks, complete regeneration and compiled regression probes. |
| `controllers/presenters/test_support.rs`, `huddle.rs`, `huddle/tests.rs`, `rooms/call_page_tests.rs` under `crates/campfire/src/` | Reconcile main's runner/HTTP variants and shared asset-golden helper. |
| `crates/campfire/src/controllers/fizzy_message_cards/tests.rs` | Test-only private origin per subprocess; no production Fizzy change. |
| `plans/ws13-deferred-tests.md`, `plans/ws13-presence-slice.md`, this report | Retain original titles/counts, exact shared-view/E2E deferrals and current acceptance. |

## Original Rails declarations by file

No additional model/job/service Rails declaration was ported in this continuation. The catalogue retains **548 titles / 33 files: 263 passed, 285 open**. WS13b's historical domain counts on this branch remain 37/216 passed and 179 open; these are not claims about its separate branch. All 106 system declarations remain open by instruction.

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

## Stable seams

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

WS17 retains policy/transport registration; the push/ring jobs remain durable unknown-handler failures on this snapshot until its handlers are integrated. No delivery success is fabricated. WS13b domain interfaces and WS8 enum/presenter signatures are unchanged. Additive view seams include `ShowView.shell`, `ViewContext: Clone`, `Navigation::identity`, `Row::render_fragment`, and the native OOO line.

## Fresh-clone acceptance

Final clone command (no seed, target, Node modules or untracked source fixtures inherited):

```sh
git clone --single-branch --branch rust/ws13-huddles --no-local . .scratch/fresh-final-ws13
```

All following final acceptance commands ran from that clone at `cce05c441cbf3a8f9e959c581b69ef7ee6decb4f`. Cargo used jobs 2, test threads 4 and the machine-wide rustc throttle. No timing threshold, test concurrency or ignore was changed. Common Cargo environment prefixes:

```sh
CI=1 TMPDIR="$PWD/.scratch" CAMPFIRE_REFERENCE="$PWD" CARGO_TARGET_DIR="$PWD/rust/target"
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
CABLE_TEST_PORT_RANGE=52300-52349 MAIL_TEST_PORT_RANGE=52350-52399
```

Reference commands use `PARITY_NAMESPACE=ws13 PARITY_IMAGE=ws13-reference:d7c7de92 PARITY_OWNER=ws13 PARITY_CPUS=2`.

```sh
rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The first clone's app run exposed `AddrInUse` on the inherited Fizzy test's shared port 51598. The isolated test-only origin fix passed every one of the existing 17 subprocess cases, was pushed, and the final clone was created from that commit. Its ordinary host app run passed 777/0/4. Host workspace execution subsequently rejected mismatched media versions (libvips 8.18.6 / ffmpeg 9.0.2 versus pinned 8.16.1 / 7.1.5). The same eight storage vector tests passed in the pinned image, then the complete workspace was rerun there. No storage vector or version gate was changed. These were environment/isolation failures, not timing flakes; no timing assertion failed in this continuation.

Final whole-workspace command, including its self-contained pinned runtime runner (no untracked helper dependency):

```python
import json, os, subprocess
from pathlib import Path
root = Path.cwd()
runner = ['docker','run','--rm','--name','ws13-pinned-workspace-test',
          '--label','com.smartfire.rust-parity.owner=ws13','--cpus','2',
          '--network','none','--user',f'{os.getuid()}:{os.getgid()}',
          '--volume',f'{root}:{root}','--workdir',str(root),'--env','CI=1',
          '--env',f'TMPDIR={root}/.scratch','--env','CABLE_TEST_PORT_RANGE=52300-52349',
          '--env','MAIL_TEST_PORT_RANGE=52350-52399','--entrypoint','/usr/bin/env',
          'ws13-reference:d7c7de92']
env = dict(os.environ, CI='1', TMPDIR=str(root/'.scratch'),
           CAMPFIRE_REFERENCE=str(root), CARGO_TARGET_DIR=str(root/'rust/target'),
           CARGO_BUILD_JOBS='2', CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0')
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','--config',
                'target.x86_64-unknown-linux-gnu.runner='+json.dumps(runner),
                'test','--locked','-j2','--manifest-path','rust/Cargo.toml',
                '--workspace','--no-fail-fast','--','--test-threads=4'],
               env=env, check=True)
```

Exit zero: **1,907 passed, zero failed, 13 existing ignores across 48 raw summaries**. Seeded app 777/0/4; DB 474/0/4. Raw summaries:

```text
test result: ok. 777 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 277.33s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.56s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 474 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 78.52s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.68s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.37s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.75s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.42s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.92s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.97s
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
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

Exit zero, raw final line:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 23s
```

```sh
NPM_CONFIG_CACHE="$PWD/.scratch/npm-cache" mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=4
```

The gateway's own 16 Node tests execute against real Rust endpoints; pinned dependencies were installed in this clone's own scratch directory. Ordinary responses are not fabricated; only the original explicit outage/stall/malformed injections remain. Raw lines:

```text
ℹ tests 16
ℹ suites 0
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 5746.693032
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 780 filtered out; finished in 6.74s
```

```sh
python3 rust/reference-tools/ws13_verify_reference.py
python3 rust/reference-tools/ws13_verify_declarations.py
python3 rust/reference-tools/ws13_verify_corpora.py
```

All exit zero. Complete raw summaries (the full_rooms file additionally retains two deferred Designers captures):

```text
Reference identity: 86 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Rails declaration catalogue: 548 titles retained; 263 passed; 285 partial/deferred; 33 files; source titles match
Sidebar reference: tracked #163 source SHA256 verified; isolated image built
row_broadcasts: reference rerun byte-identical; 16 cases
full_rooms: reference rerun byte-identical; 36 cases
room_shell: reference rerun byte-identical; 38 cases
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
WS13 corpora: 28 complete files regenerated byte-identically
```

```sh
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb default
rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb first_run
```

Both exit zero. Raw verification outputs:

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
subprocess.run(['mise','exec','rust@1.98.1','--','cargo','metadata','--locked',
                '--format-version','1'], cwd=root/'rust',
               stdout=subprocess.DEVNULL, check=True)
print('cargo metadata --locked --format-version 1: exit 0')
deps = tomllib.loads((root/'rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'Workspace dependency keys: {len(deps)} unique; no duplicates')
subprocess.run(['git','merge-base','--is-ancestor','8952bed4','HEAD'], check=True)
print('Required fetched main 8952bed4: ancestor via merge commit e329e372')
print('Fresh clone HEAD: '+subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip())
```

```text
cargo metadata --locked --format-version 1: exit 0
Workspace dependency keys: 75 unique; no duplicates
Required fetched main 8952bed4: ancestor via merge commit e329e372
Fresh clone HEAD: cce05c441cbf3a8f9e959c581b69ef7ee6decb4f
```

```sh
CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml --target-dir rust/target
```

Final fresh target removed, raw line:

```text
     Removed 6278 files, 2.1GiB total
```

The initial failed clone's target was also removed (4,209 files, 1.9 GiB), and the older fresh-sidebar target was removed before this continuation's new build. No scratch fresh-clone target directory remains populated.

## Fail-first and compiled regression evidence

The actual pre-fix assertions caught absent recipient row delivery, JSON mute's 302 instead of 200, and a renamed group-DM request's stale page title. Populated whole-page captures caught collection whitespace; the complete Designers mismatches identify the two shared integration gaps below. These are genuine observable assertions, not merely title mappings.

The following owning-worktree commands were rerun in this continuation, required compiled assertion failures, and restored source bytes before fresh acceptance:

```sh
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py --only '^room-shell-|^room-row-'
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 python3 rust/reference-tools/huddle_discrimination.py room-populated-collection-rendering-bypassed
```

Raw summaries:

```text
room-row-callback-rendering-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 780 filtered out; finished in 0.20s
room-shell-invisible-notice-leaks: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 780 filtered out; finished in 0.09s
room-shell-scroll-threshold-corrupted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 780 filtered out; finished in 0.71s
WS13 discrimination: 3 compiled regressions detected; sources restored

room-populated-collection-rendering-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 780 filtered out; finished in 7.95s
WS13 discrimination: 1 compiled regressions detected; sources restored
```

## Exact remainder

- Two complete Designers fixtures: `seed_654632876_false` and `seed_654632876_true`, retained under `full_room_vectors.json.deferred`. The shared native message renderer currently emits empty GitHub PR-card and message-link-card containers where Rails emits the PR article and lazy message-link frame. Both HTML/content outputs remain deferred with that precise reason; the lead reconciles the owning shared integration branches. These captures are not counted among the 36 native acceptance scenarios.
- Shared post-#163 application-layout/status-action composition with WS8b-r2. The exact post-#163 sidebar frame is integrated; whole-page goldens otherwise target the frozen layout. Board #164/#165 belongs to WS12.
- All **106 browser system declarations** await the authorized end-to-end phase: 71 ordinary browser cases and 35 real LiveKit cases. All 35 stay individually inventoried (31 huddles, four Stage). Recorded administrative-client responses cannot express their browser WebRTC/media/reconnect/participant assertions; those need the real server and `LIVEKIT_SYSTEM_TESTS=1`. No browser/system declaration was reclassified as passed, and no screenshot/pixel item is retained as work.
- WS13b's model/job/service reconciliation and WS17's policy/transport integration remain with those workers and the lead. Their signatures were preserved. Historical domain inventory counts on this branch remain historical.

No open question requires user input. Source work stops at coherent pushed slices with the above precise remainder; final acceptance is complete and no PR was opened.
