# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`; worktree `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br`; Rails pin `d7c7de92`. Main `4278cb1e` was already merged in `8bca72a1`. This continuation pushes fifteen implementation/test/merge slices through Rust verification SHA **`b9cae0e00b3840712cf8e705c8b66a4fdf95a41d`**, followed by diagnostic-only **`c46d7f77eed9c6fd57f83ea0021f596277c8b370`**. The final report commit changes documentation only; its pushed SHA is in the final reply.

Members JSON and pin refresh now call the real owner implementations. The room HTTP page mounts the native owner list, composer, schedule control and pending template. **Full native byte acceptance remains partial: the strict comparator still rejects all nine complete component regions.** There are **146 of 161 controller declarations individually mapped and passed**, including 62 newly mapped this continuation; **15 remain**. The fresh-clone app suite has zero failures, and workspace tests and clippy pass. No browser/system acceptance was run this continuation; the requested end-to-end phase remains inventoried.

## Coherent pushed slices

| Commit | Change |
|---|---|
| `567425ff` | Merge WS17 `41dbe4bd` for real presence/status/OOO/reminder facts. |
| `7aad9674` | Merge WS11 `18c9219c` for actual agents and working presence. |
| `fc117fbc` | Merge WS8bm `b14759da` for list/composer/pending-template seams. |
| `4ce15b36` | Merge WS8bm2 `d24317e8` for pin, schedule and quote factories. |
| `39642d8f` | Reconcile app service fields, controller registration and test initializers across those owners. |
| `670b1cbe` | Register the one real WS17 saved-item reminder transport; remove the merged duplicate runtime placeholder. Retain the standalone M2 transport-boundary test module only under `cfg(test)`. |
| `4e7b4112` | Real members JSON, all 13 Rails declarations, bot denial and six complete Rails HTTP body/header goldens. |
| `fe682835` | Actual owner pin count/list refresh streams, all four Rails declarations and populated/empty rendering plus request-token ownership. |
| `27990da2` | Mount actual native list/composer/schedule/pending-template bytes in the shell; verify roots, around anchors, warm per-viewer dividers and request tokens. Add a strict failing Rails region comparison. |
| `3e779615` | Fix four composite integration failures: preload domain DM naming without queries, set search icons inside shared cache misses, use the actual quote-card broadcast renderer, and wait for only the quote job while asserting unrelated future retention work survives. |
| `286ee6eb` | Map five supported declarations from the largest remaining sidebar file; retain nine huddle/query cases explicitly. |
| `2900850f` | Map all eight involvement and seven reads declarations through HTTP and live per-user Cable. Correct voice/stage/board sidebar row identity to the full STI key. |
| `2a9f16c3` | Map all six favorites, six category CRUD, five category assignments, and four functional switcher declarations. |
| `e993b656` | Resolve four of the older nine deferred cases: DM note rendering and immutable edit/delete, in-page unread divider and off-page unread jump. All 29 direct declarations now map. |
| `b9cae0e0` | Move merged periodic tests after production items and compile the pin test regex once; no behavior/deadline change. |
| `c46d7f77` | Correct strict comparison diagnostics to measure UTF-8 byte lengths; keep the nine failures explicit. Rust source is unchanged from `b9cae0e0`. |

The four owner merges are merge commits, with locked metadata and duplicate-key checks after each. No WS8br2 branch or user/profile/avatar/account/public/tour implementation was added. Its layouts/presenters remain available with their existing entry points. Message/pin partial internals and pin policy were not ported into the shell.

## Members and refresh behavior

Members authenticate via WS9, emit empty JSON 401, deny bots, and scope alive rooms to the viewer. Active members are ordered by SQLite `LOWER(name), id`. WS17 `UserStatusSettings::for_ids` and `WorkspacePresenceLease::presence_by_user_id` supply status and effective presence; WS11 `Agent::for_user` and `working_presence_text` supply agent facts. Stars are live and viewer-scoped. The exact ordered fields are `id`, `name`, `avatar_url`, `bot`, `online`, `presence`, `status`, `starred`. Six actual Rails HTTP goldens cover three rooms and two viewers, including headers and full JSON bytes. Ruby's observed Rack ETag is retained alongside its no-store/no-cache headers; the controller's comment saying not ETagged does not describe the actual Rack response. No local fake offline/default-agent policy or presence TTL is introduced.

Refresh obtains pins only through the WS8bm2 list seam. Its real count/list factories supply streams after message append/replacements. Empty output is compared as complete Rails HTTP bytes. Populated rendering uses the same fixed request-secret instrumentation as Rails, while a separate actual HTTP Unpin token is accepted only for the current viewer. No response normalization is used.

## Exactly what the shell passes

`controllers/rooms::render_show` supplies the native owner entry points with:

1. A reader connection, actual merged app state, request host, verified request origin
   as `cache_base_url`, and the app's shared fragment store.
2. Root `Message` records from `room_shell::find_messages`: last 40, or up to 40 before
   a same-room root anchor plus the anchor plus 40 after (81 total). Foreign, missing,
   and thread anchors fall back to the last page. Their original IDs, timestamps,
   client IDs, creators, content and owner feature records are not replaced.
3. The current membership's `last_read_message_id` and `unread_at` produce
   `divider.message_id: Option<i64>` and `divider.count: i64`. The exact call is
   `presenter.room_message_list(&messages, divider.message_id, divider.count)` under
   `fragment_cache::with(&app.fragment_cache, ...)`. Its returned string is assigned
   unchanged to `ShowView.shell.message_list = Some(list)`. Viewer dividers remain
   outside the shared per-message cache.
4. `ShowView.room`: ID, owner `RoomKind` (Open/Closed/Direct; voice/stage/board currently
   use Closed until their screen-owner integration), persisted name, viewer display name, resolved
   header identity and involvement. `ShowView.user`: ID, name, title and fresh signed
   avatar URL. The same selected `MessageItem`s identify cached fragments. Other
   inputs are room `updated_at`, the original-room/unpaged invitation predicate,
   account join code, signed `[room_gid, "messages"]` stream name, divider scroll/jump
   facts, and WS17's real per-viewer OOO notice members.
5. Request `ViewContext`: viewer/admin/bot/preferences, account, assets, verified URL,
   referrer/last-room, time zone, flash and chrome. The layout lends actual request
   CSRF and CSP values. Broadcast contexts remain detached.

`presenter.composer_facts(&room, &viewer, None, drive_flow)` supplies WS8bm's root
composer with room ID/kind, viewer-relative domain display name, `thread: None`, the
static slash-command registry followed by room-scoped agent slash commands, and the
Drive flow. Drive consent scopes currently select `None` or `Metadata`; the unresolved
WS14 Picker availability input is explicitly `false`, so configured `Share` acceptance
is still pending.

Inside the request rendering scope, `room_native::components` renders:

- WS8bm2 `scheduled_messages::ComposerButton { ctx, room_id, thread_id: None }`, passed
  as the trusted `scheduled_control` argument to WS8bm `composer::Composer`.
- WS8bm `channel_threads::PendingTemplate { ctx, user: &show.user }`.

Those returned bytes become `shell.composer = Some(...)` and
`shell.message_template = Some(...)`. The actual HTTP page mounts all three native
components; it does not inject Rails message/composer fragments. Request tests check
selected roots, around-anchor roots, schedule controls, real viewer token ownership,
and different unread boundaries on warm shared fragments.

`ShellComponents` also retains `thread_panel`, `pins_panel`, `poll_builder`,
`huddle_header`, and `ooo_notices` slots. A full owner panel entry point is not present
for all of these. In particular, WS8bm2 currently supplies pin count/list factories,
not a complete pins-panel factory; WS8bm supplies no root thread-panel factory.
WS13 huddle and WS14 configured Picker facts also remain integration work. Do not
reimplement those partials or provider policies in this shell.

## Pin refresh

Root creation/update windows and pin-change timestamps are selected by the refresh
controller. A quiet refresh returns 204 before format negotiation. Only when pins
changed, it calls WS8bm2 `controllers::rooms::pins::list(conn, app, room)` and stores
that actual `pins::List` in `RefreshView.pins`.

`RefreshShow` renders the owner `CountPartial` and `ListPartial` with its current
`ViewContext`, after message append/replace streams. Targets are
`pins_count_<room_param_key>_<id>` and `pins_list_<room_param_key>_<id>`; full STI keys
are preserved. Ordering, excerpts, pin/unpin policy and durable events stay in WS8bm2.
The empty refresh is byte-identical to Rails. Populated owner rendering is compared
with fixed request secrets; the separate real HTTP response's Unpin token is verified
for its viewer and rejected for another viewer. No response bytes are normalized.

## Native acceptance still fails explicitly

Fresh-clone native root-selection capture passes, then the separate strict comparison exits **1**. No Rails owner HTML is substituted into the native page; no mask, allowlist, ignored app case or widened deadline hides these differences.

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1017 filtered out; finished in 0.60s
FAIL room 654632876 message_list: Rust 286281 bytes, Rails 288823 bytes
FAIL room 654632876 composer: Rust 10372 bytes, Rails 10373 bytes
FAIL room 654632876 pending_template: Rust 1448 bytes, Rails 1449 bytes
FAIL room 186869642 message_list: Rust 19541 bytes, Rails 19547 bytes
FAIL room 186869642 composer: Rust 10368 bytes, Rails 10369 bytes
FAIL room 186869642 pending_template: Rust 1448 bytes, Rails 1449 bytes
FAIL room 699448329 message_list: Rust 6553 bytes, Rails 6559 bytes
FAIL room 699448329 composer: Rust 8595 bytes, Rails 8596 bytes
FAIL room 699448329 pending_template: Rust 1463 bytes, Rails 1464 bytes
Native room component acceptance: 0 exact matches; 9 differences; no masks
```

The Designers list lacks the unmerged WS15 GitHub/Fizzy/LinkedIn/link-embed provider integration and has caller whitespace differences. The pair/group list differences are caller whitespace; composer/pending-template regions also differ in owner caller whitespace/final newline. Artifacts are under `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh-continue8/repo/.scratch/native-components-diff/`, with actual, expected and unified diffs per complete region. Reconciliation belongs in the owner entry points/callers; partial internals remain untouched. Complete pins/thread panels, poll builder, WS13 huddle/venue inputs and WS14 configured Picker availability also remain integrations; the current venue `RoomKind` fallback is stated above. Full-page/browser/pixel parity is not claimed.

## Per-file controller receipts

These counts are source declarations individually mapped to named Rust tests, separate from the actual Rails Minitest runs. Two extra members regressions and one extra pin-refresh regression also pass; they do not inflate source mapping counts.

| File | Declarations | Rust mapped passes | Remaining | Rails reference runs |
|---|---:|---:|---:|---:|
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 29 | 0 | 29 |
| `test/controllers/rooms_controller_test.rb` | 29 | 24 | 5 | 29 |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 15 | 0 | 15 |
| `test/controllers/users/sidebars_controller_test.rb` | 14 | 5 | 9 | 14 |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 13 | 0 | 13 |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 12 | 0 | 12 |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 8 | 0 | 8 |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 8 | 0 | 8 |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 7 | 0 | 7 |
| `test/controllers/room_categories_controller_test.rb` | 6 | 6 | 0 | 6 |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 6 | 0 | 6 |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 5 | 0 | 5 |
| `test/controllers/switchers_controller_test.rb` | 5 | 4 | 1 | 5 |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 4 | 0 | 4 |
| **Total** | **161** | **146** | **15** | **161** |

`rust/plans/ws8br-rails-cases.json` records each source name, line, hash, native selector/pass or exact deferral reason. The inventory validates fresh-clone cargo receipts including libtest timings; it does not infer a pass from a source declaration. The broader 58-file/512-declaration browser/controller inventory retains ownership handoffs to WS8br2 and other workers.

## Exactly what remains

`test/controllers/users/sidebars_controller_test.rb`:

- channel row shows the live huddle stack with names and count — WS13 huddle grant/presence integration and full-request query instrumentation.
- board row shows the live huddle stack with names and count — WS13 huddle grant/presence integration and full-request query instrumentation.
- direct row shows the live huddle stack when the peer is in the call — WS13 huddle grant/presence integration and full-request query instrumentation.
- quiet rows keep an empty stack target with no visible presence — WS13 huddle grant/presence integration and full-request query instrumentation.
- direct row re-renders when a participant joins — WS13 huddle grant/presence integration and full-request query instrumentation.
- group direct rooms render member names and a huddle stack — WS13 huddle grant/presence integration and full-request query instrumentation.
- no channel or DM stacks without huddle configuration — WS13 huddle grant/presence integration and full-request query instrumentation.
- sidebar query count does not grow with quiet channels, DMs, boards, and stages — WS13 huddle grant/presence integration and full-request query instrumentation.
- sidebar query count does not grow with group DMs, named or not — WS13 huddle grant/presence integration and full-request query instrumentation.

`test/controllers/rooms_controller_test.rb`:

- show renders collapsed work-thread guidance in the new-thread panel — WS8bm thread-panel rendering.
- show renders a link preview written by hand without its off-scheme image and link — WS8bm message renderer and WS15e embed provider.
- show renders a link preview written by hand without its image pointed at this Smartfire — WS8bm message renderer and WS15e embed provider.
- show renders an unfurled link preview — WS8bm message renderer and WS15e embed provider.
- destroy succeeds when the queue is down and the sweep recovers the room — Lead decision 2 requires atomic queue rollback; native fault-injection coverage is separate.

`test/controllers/switchers_controller_test.rb`:

- show costs a constant number of queries as rooms, people and threads grow — Full-request query instrumentation; existing pure read-model budget tests do not claim this HTTP case.

Of the original nine deferred declarations, four are resolved and five remain in `rooms_controller_test.rb`. The queue-down Rails declaration conflicts with fixed lead decision 2 requiring atomic queue rollback; existing native queue fault-injection coverage remains separate and is not mislabeled as a port of that source case.

Browser/system acceptance waits for the end-to-end phase. Keep DM picker/create/settings/huddle flows, inbound-email address reveal/enable/regenerate/disable and non-admin denial, full room list/composer/panels, per-viewer unread state, navigation/header/sidebar/switcher, accessibility, motion, responsive layout and configured huddle/venue states inventoried. The previously delivered DM and inbound browser probes remain tracked but are not claimed as rerun here.

## Failing-first discrimination and integration failures

Before implementation, the meaningful members route run rejected 14 tests (including authorization); pin refresh rejected its two new pin-dependent regressions; native list/composer mounting rejected three tests. Canonical raw logs are `.scratch/members-before.log`, `.scratch/pin-refresh-before.log` and `.scratch/native-before.log`. The new controller mappings additionally execute compiled dispatch-removal regressions, restored in `finally`. Their seven groups reject all 41 newly mapped declarations in those groups. The four newly unblocked cases reject compiled missing adapters as well.

Commands actually executed in this continuation, with the two profile variables set to `0` (compiler jobs and harness threads remain four):

```sh
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py sidebars
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py involvements
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py reads
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py favorites
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py room_categories
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py categories
python3 rust/reference-tools/rooms/controller_mapping_discrimination.py switchers
python3 rust/reference-tools/rooms/deferred_cases_discrimination.py
```

Raw summaries:

```text
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 973 filtered out; finished in 1.73s
Controller mapping discrimination: sidebars compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 985 filtered out; finished in 2.93s
Controller mapping discrimination: involvements compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 986 filtered out; finished in 1.74s
Controller mapping discrimination: reads compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 1008 filtered out; finished in 1.79s
Controller mapping discrimination: favorites compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 1008 filtered out; finished in 1.60s
Controller mapping discrimination: room_categories compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 1009 filtered out; finished in 1.11s
Controller mapping discrimination: categories compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 1010 filtered out; finished in 1.28s
Controller mapping discrimination: switchers compiled dispatch removal rejected; source restored
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1016 filtered out; finished in 0.88s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1016 filtered out; finished in 1.17s
Deferred case discrimination: four compiled missing-adapter regressions rejected; source restored
```

The first complete composite app run exposed four failures, all resolved before the fresh clone. They were merge/test integration issues, not claimed inherited or timing flakes. The quote test's 10-second wait erroneously required an unrelated job scheduled an hour ahead to disappear; the corrected predicate waits only for the quote class and still asserts that future retention job is present. Its timeout was not widened. The fresh-clone run has no failure or timing flake to report.

## Fresh-clone verification and reproducibility

Fresh clone `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh-continue8/repo` was created with `git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-continue8/repo`. It received the pushed lint-only commit before any seed/build; later it fast-forwarded only the UTF-8 diagnostic script. All Rust source remains exactly the tested `b9cae0e0` tree. No canonical seed, build target, node_modules, captured response or scratch fixture was copied in. The seeds were independently built from tracked tooling and the pinned Rails image. The only untracked checkout state afterwards is generated `.scratch/` output. Failure artifacts and prior logs are retained in canonical scratch; only its regenerable compiler target was cleaned to make space.

From that fresh clone, these commands were rerun:

```sh
PARITY_NAMESPACE=ws8br-fresh8-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
CI=1 TMPDIR="$PWD/.scratch" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 RUSTC_BOOTSTRAP=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire native_component_capture_matches_rails_root_selection -- --test-threads=4 --nocapture
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-capture.log
```

Workspace tests exit 0; clippy exits 0; metadata exits 0 with intentionally no stdout; the final strict diagnostic exits 1 as reported above. Rust is still stable `1.98.1`; `RUSTC_BOOTSTRAP` enables libtest's timing report, without changing implementation features, deadlines or concurrency. Debug symbols alone are disabled for disk space. Aggregate of the 58 raw harness summaries is 2360 passes, zero failures, ten existing ignores. The app's two ignores remain the reference recorder and push-latency measurement; WS11's bot-management test now executes.

Raw seed/manifest/clippy output:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
    Finished `dev` profile [unoptimized] target(s) in 38.33s
```

Raw workspace test summary lines, including doc-test harnesses:

```text
test result: ok. 1016 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 161.99s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.64s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 697 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 63.83s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.65s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.43s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.34s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.18s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.65s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.55s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.92s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.34s
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

Logs: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br/.scratch/fresh-continue8/repo/.scratch/workspace-tests.log`, `workspace-clippy.log`, `seed-build.log`, `native-capture.log` and `native-comparison.log`.

## Runtime inventory

The fresh-clone app harness took **161.99 s** with four test threads. This is verification under the current shared-host load, not a controlled speed comparison with the previously reported 588 s. The slowest app tests are:

- `channels::tests::golden::replays_reference_frames`: 20.917 s.
- `app::admin_two_factor_tests::self_service_limits_use_ip_and_user_windows_and_remembered_actions_share_a_bucket`: 13.492 s.
- `app::round_four_security_tests::profile_zone_case_and_alias_validation_matches_rails_without_partial_writes`: 12.153 s.
- `channels::tests::support::until_closed_bounds_a_socket_that_keeps_pinging`: 10.124 s.
- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`: 9.586 s.

The slowest owned room tests are:

- `controllers::rooms::closeds_rails_cases::updating_the_icon_replaces_sidebar_rows_and_headers_for_members_only`: 2.172 s.
- `controllers::rooms::involvements_rails_cases::update_involvement_sends_turbo_update_when_becoming_visible_and_when_going_invisible`: 1.880 s.
- `controllers::rooms::involvements_rails_cases::updating_involvement_does_not_send_turbo_update_changing_visible_states`: 1.606 s.
- `controllers::rooms::involvements_rails_cases::updating_involvement_does_not_send_turbo_update_for_direct_rooms`: 1.554 s.
- `controllers::rooms::closeds_rails_cases::create_case`: 1.446 s.

None of the room tests individually dominates the app run. Outside the app harness, the existing Cable `concurrent_cutoff_drops_later_publications` test took 42.639 s, and DB slash-command Ruby differentials took about 34 s each. Coverage, test concurrency and timing thresholds were not reduced or widened.

## Pinned Rails and inventory commands

From the canonical worktree, both were rerun after the implementation slices:

```sh
python3 rust/reference-tools/rooms/check_controller_files.py
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh-continue8/repo/.scratch/workspace-tests.log --rails-log .scratch/controller-reference-current.log
```

The reference tool verifies all fourteen controller source hashes at `d7c7de92`, runs each actual Rails file separately, and preserves its failure exit status. These are reference executions, not a claim of full Rust case completion. Raw per-file Rails summaries:

```text
test/controllers/rooms_controller_test.rb
29 runs, 153 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/opens_controller_test.rb
15 runs, 55 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/closeds_controller_test.rb
12 runs, 68 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/directs_controller_test.rb
29 runs, 167 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/involvements_controller_test.rb
8 runs, 63 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/refreshes_controller_test.rb
4 runs, 24 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/reads_controller_test.rb
7 runs, 25 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/members_controller_test.rb
13 runs, 57 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/categories_controller_test.rb
5 runs, 13 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/favorites_controller_test.rb
6 runs, 21 assertions, 0 failures, 0 errors, 0 skips
test/controllers/rooms/inbound_email_addresses_controller_test.rb
8 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/room_categories_controller_test.rb
6 runs, 26 assertions, 0 failures, 0 errors, 0 skips
test/controllers/switchers_controller_test.rb
5 runs, 33 assertions, 0 failures, 0 errors, 0 skips
test/controllers/users/sidebars_controller_test.rb
14 runs, 76 assertions, 0 failures, 0 errors, 0 skips
WS8br Rails controller reference: 14 files passed; reference counts only
```

Raw mapping/inventory receipts:

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/directs_controller_test.rb: 29 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms_controller_test.rb: 24 Rust cases passed, 5 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/opens_controller_test.rb: 15 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/closeds_controller_test.rb: 12 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/members_controller_test.rb: 13 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/refreshes_controller_test.rb: 4 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/users/sidebars_controller_test.rb: 5 Rust cases passed, 9 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/involvements_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/reads_controller_test.rb: 7 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/favorites_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/room_categories_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/categories_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/switchers_controller_test.rb: 4 Rust cases passed, 1 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

Report mirror: `rust/plans/ws8br-wave4-report.md`. Required external report: `/home/riels/Projects/SD-Labs/Campfire/.claude/delegation/rust-port/wave4/ws8br-report.md`.
