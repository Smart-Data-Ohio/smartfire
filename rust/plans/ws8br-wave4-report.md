# WS8br Wave 4 continuation report — PARTIAL

Latest independently tested application source: `6dc741c9bd42922914d619f3d87889c63e5b839d`. Main is merged through `76e54ad5` (#169/#170/#171), including WS15e and the shared asset-golden helper. The final fresh-clone workspace passed **2726 tests, zero failures, eleven existing ignores across 58 harness summaries**. The seeded app passed 1319 tests with zero failures and two existing ignores. No missing-seed skips are accepted (`CI=1`). Clippy passed with `-D warnings`.

This continuation closes one of the ten original controller mappings, mounts the actual owner thread/poll/pin panels, connects public Picker configuration, and adds browser interaction acceptance. **152 of 161 original controller declarations now have individual Rust pass receipts; nine remain.** Full native page byte acceptance remains partial: eight of nine measured list/composer/template regions match, while one GitHub owner card is missing. Browser probes are separate from complete original system-file mappings. No screenshots or pixel differences are on the remaining list.

## Pushed coherent slices

| Commit | Result |
| --- | --- |
| `976a067b` | Merge origin/main at `2a7c8b9b` (#169), with a merge commit. Preserve the actual WS11 guarded webhook implementation and combined scheduler roster while adopting main's listener/readiness/paused-clock test fixes. Locked metadata and duplicate-key checks pass. |
| `507fc3da` | Mount the actual WS13 thread/poll templates and WS8bm2 pins panel; compare fourteen complete owner partials and real request token ownership. Map the collapsed work-thread guidance declaration. |
| `021c58ca` | Merge origin/main again at `76e54ad5` (#171), with a merge commit. Reconcile durable attachment analysis with the existing message/thread/agent domains. Locked metadata and duplicate-key checks pass again. |
| `91b816b0` | Connect the WS13 public Picker readiness adapter to layout metadata and root/thread composer inputs; reproduce eight complete root composers from pinned Rails. |
| `1630fc40` | Add native room keyboard/drawer/menu/thread/pin browser interactions; strengthen inbound create/cancel/rotate/valid-CSRF authorization/persistence acceptance. |
| `6dc741c9` | Fix the observed fixed-port collision in Fizzy HTTP fixture subprocesses by inheriting an already-bound assigned listener. Keep browser profiles under owned scratch. Production clients, authorization and timing limits are unchanged. |

The final report/inventory commit changes documentation and the executed inventory receipt tool; it does not change application code. The latest tested application source stays `6dc741c9`.

## Changes by file and design boundaries

- `views/src/rooms/panels.rs`, `views/templates/rooms/composition/_thread_panel.html`, `_poll_builder.html`, `views/templates/rooms/pins/_panel.html`, `views/src/pins.rs`, and their module registrations: mount owner factories. Thread/poll source is WS13 `66e4c66d`; pins is WS8bm2 `1fdf42a6`. Templates are copied unchanged. There is no local thread, poll, pin or huddle policy implementation.
- `controllers/rooms.rs`, `controllers/presenters/room_native.rs`, `views/src/rooms.rs`: additive/default shell fields `pins_count` and `thread_panel_name`; actual domain pin count; neutral DM names; render panels inside the request secret scope. Existing layout/presenter contracts stay stable for WS8br2.
- `rooms/owner_panel_tests.rs`, `owner_panels.json`, `owner_pins.json`, and `rooms_rails_cases.rs`: fourteen whole owner partial comparisons, actual viewer-scoped poll CSRF, neutral unnamed-DM title `David, Jason`, once-only panel mounting, and the original collapsed work-guide test. The owner composition corpus has thirty Rails captures, but this new Rust panel test compares its ten thread/poll captures plus four pin panels; it does not claim all thirty compositions as new Rust passes.
- `picker_configuration.rs`, `config.rs`, `main.rs`, `presenters/view_context.rs`, `controllers/rooms.rs`, `controllers/channel_threads.rs`: literal public readiness adapter from WS13 `abf4ecde`. All three of `GOOGLE_CLIENT_ID`, `GOOGLE_PICKER_API_KEY` and `GOOGLE_CLOUD_PROJECT_NUMBER` must be nonblank; Picker composer availability also requires a human. The adapter reads no account tokens or client secrets. Google transport/OAuth remains WS14's. Existing metadata-consent behavior is retained.
- `rooms/picker_config.json`, `owner_panel_tests.rs`, `reference-tools/rooms/picker_components.rb`, `check_picker_components.py`: eight configurations captured only from Rails, complete root-footer byte comparisons, and real HTTP metadata/controller/escaping assertions. No Rust output is used as a golden.
- Merge reconciliation in `controllers/messages.rs` and `presenters/attachments.rs`: use main's staged/existing/signed attachment assignment and durable analysis enqueue in the triggering write while retaining WS11 agent policy/ledger/replay/budget and M1 thread/root ownership. Delegating analysis and attachment-record touches uses main's `active_storage` implementation. The old ephemeral analysis path is not restored. `jobs/tests.rs` retains the future-job retention assertion alongside main's shutdown wait. The fresh workspace executes the durable atomicity/upload/update regressions.
- `integrations/test_support.rs`, `fizzy_connections/tests.rs`, `fizzy_message_cards/tests.rs`: subprocess fixtures use `ws15e_listener` and `CABLE_TEST_PORT_RANGE`. The parent holds the socket until its one-test child exits; an async-signal-safe child `fcntl` lets that socket survive exec. Each child consumes that inherited socket once. No release/rebind race, global environment mutation or production network substitution is used. The existing 27 matrix cases and assertions remain. Fifty-nine focused Fizzy tests pass.
- `reference-tools/rooms/room_browser.py/.mjs`, `inbound_browser.py/.mjs`, existing `dm_browser.py/.mjs`, and `plans/ws8br-browser-acceptance.json`: headless interaction probes on separate Rails and Rust seed copies, assigned ports 52160–52162, own TMPDIR and cleanup. No images or pixel comparison. The DM discriminator checks persisted members; the inbound discriminator substitutes an admin and must fail the non-admin denial assertion.
- `deferred_inventory.py`, `plans/ws8br-rails-cases.json`, `ws8br-owner-integration.md`: map actual raw passes, retain explicit owner deferrals, and exclude geometry/style-only work from the current phase. Historical source declaration counts remain historical counts; they do not imply every declaration is remaining behavior work.

Shared live-asset comparisons use main's `rust/test-support/asset_goldens.rs`. Identified asset URLs must match actual local digests; surrounding bytes stay exact. The separate strict native comparison below applies no masks. All new and reproduced golden HTML comes from our pinned Rails image, never Rust. Main's approved #163/#164/#165 drift remains in the respective owner boundaries. WS8br2's users/profiles/accounts/public/tour/PWA/QR work is neither implemented nor claimed here.

## Exact shell inputs to owner seams

The full contract is `rust/plans/ws8br-owner-integration.md`:

1. Reader connection, actual AppState, request host, verified origin/cache base URL, shared fragment store and unchanged root Message records: last forty, or forty before a same-room root anchor plus that root and forty after (81). Missing/foreign/thread anchors fall back to the last page.
2. Membership `last_read_message_id` and `unread_at` produce `divider.message_id: Option<i64>` and `divider.count: i64`. `room_native::message_list(&presenter, &records, divider.message_id, divider.count)` delegates to unchanged `Presenter::room_message_list`. The rooms/show caller contributes only `"\n    \n"`; viewer divider facts stay outside shared message fragments.
3. `composer_facts(&room,&viewer,None,drive_flow)` passes root thread `None`, real room ID/kind, viewer-relative name, static commands followed by real room agent commands, and actual `DriveFlow::{None, Metadata, Share}` input (Share is configured Picker). Open/Closed/Direct are native; Voice/Stage/Board retain the documented Closed composer fallback pending screen-owner integration. Header and stream STI keys remain complete.
4. Actual request ViewContext/account/preferences/chrome, viewer ID/name/title/fresh signed avatar, CSRF and CSP values. M2 `scheduled_messages::ComposerButton {ctx,room_id,thread_id:None}` is M1's trusted schedule control; M1 `PendingTemplate {ctx,user}` is mounted once. Root footer removes one initial owner source newline and adds two spaces; pending template retains the final newline.
5. WS13 ThreadPanel receives the room view and domain `room_display_name(&room,None)` including the viewer in unnamed DMs, with Rails' two-space content_for prefix. PollBuilder receives the same room ID and current request ViewContext. M2 PanelPartial receives room ID, full header STI param key and `MessagePin::count_for_room`. Pin count/list/order/excerpts/unpin policy stay M2's; refresh still calls its actual list/count seam.
6. Original invitation/join code, room updated timestamp, selected cached MessageItems, signed room message stream, unread jump/scroll facts, WS17 OOO/status/presence facts and main provider graph/pending-fetch facts. Provider jobs enqueue on the writer through the main owner seam. No list/composer internals or huddle grant policy are recreated.

Configured huddle header/sidebar facts and full surrounding-page acceptance remain unfinished. `google_drive_previews` and Google provider transport are not newly implemented by this public configuration adapter.

## Native byte result and GitHub owner delta

Fresh source `6dc741c9` emits the same strict result:

```text
FAIL room 654632876 message_list: Rust 287734 bytes, Rails 288823 bytes
PASS room 654632876 composer: 10373 exact bytes
PASS room 654632876 pending_template: 1449 exact bytes
PASS room 186869642 message_list: 19547 exact bytes
PASS room 186869642 composer: 10369 exact bytes
PASS room 186869642 pending_template: 1449 exact bytes
PASS room 699448329 message_list: 6559 exact bytes
PASS room 699448329 composer: 8596 exact bytes
PASS room 699448329 pending_template: 1464 exact bytes
Native room component acceptance: 8 exact matches; 1 differences; no masks
```

```text
github_pr_cards_message_dea0071a-bcd7-5548-a81a-8afdd467d12d: Rust 101 bytes; Rails 1190 bytes
Native residual inventory: 1 empty owner card slots explain the complete remaining difference; strict acceptance still fails
```

The Designers list is **Rust 287734 bytes versus Rails 288823 bytes**. The missing slot `github_pr_cards_message_dea0071a-bcd7-5548-a81a-8afdd467d12d` is Rust 101 bytes (empty target) versus Rails 1190 bytes (public PR card/discussion link), explaining all 1089 bytes. Complete Rails and Rust strings remain in `rust/plans/ws8br-native-residual.json`; that file is a diagnostic, not a golden, mask or accepted render. The strict checker deliberately exits **1**. Diff artifacts remain in the fresh clone's `.scratch/native-components-diff/`.

WS15g #167 supplies this renderer. Its owner branch `d59f18e7` has `controllers/presenters/github.rs::message_cards(conn,app,message)` calling actual PullRequest/public-card data and `github::cards`; `cache_stamp` incorporates linked PR and discussion timestamps. `MessageComponents.github_cards_html/github_cards_stamp` are the integration fields. This factory is not on the main version merged here, so, as requested, the 1089-byte gap is left for that owner merge. Reconcile those fields with M2 quote/preload and existing provider components. No substitute GitHub implementation is written.

Eight native regions and the separate fourteen panel/eight configured-composer comparisons are not a claim that the complete HTTP page is byte-identical. Full layout/shell/page capture and all surrounding differences still need acceptance after remaining owner integrations.

## Original controller mappings, largest files first

Pinned Rails: fourteen files, **161 actual runs/passes**, zero failures/errors/skips. Rust: **152 individual original mappings passed; nine deferred**. Two extra member regressions and one extra pins regression remain separate (155 named mapping tests including extras).

| Original Rails file | Rust passes | Deferred |
| --- | ---: | ---: |
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 0 |
| `test/controllers/rooms_controller_test.rb` | 28 | 1 |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 0 |
| `test/controllers/users/sidebars_controller_test.rb` | 6 | 8 |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 0 |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 0 |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 0 |
| `test/controllers/room_categories_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 0 |
| `test/controllers/switchers_controller_test.rb` | 5 | 0 |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 0 |

Exactly nine original declarations remain:

`test/controllers/rooms_controller_test.rb` — one:

- `destroy succeeds when the queue is down and the sweep recovers the room`. Rails commits soft deletion despite a Redis enqueue failure and recovers it after eleven minutes. Fixed decisions 1/2 and the explicit durable-job exception require atomic SQLite enqueue rollback. Existing native fault-injection rollback coverage is not mislabeled as this Rails success case. This requires a lead decision, not a local queue-policy change.

`test/controllers/users/sidebars_controller_test.rb` — eight, requiring WS13's real grant/presence/configuration/cache adapters and full-request query instrumentation:

- `channel row shows the live huddle stack with names and count`
- `board row shows the live huddle stack with names and count`
- `direct row shows the live huddle stack when the peer is in the call`
- `quiet rows keep an empty stack target with no visible presence`
- `direct row re-renders when a participant joins`
- `group direct rooms render member names and a huddle stack`
- `no channel or DM stacks without huddle configuration`
- `sidebar query count does not grow with quiet channels, DMs, boards, and stages`

The cached-row declaration requires Jason to remain after a rename until the actual grant sighting invalidates the collection, then render Jordan/live presence. The quiet-query declaration requires constant full-request SELECT counts and exactly one `huddle_grants` SELECT. No local grant issuance SQL or fake owner facts are substituted. The separate constant group-DM query declaration is already passed.

## Browser interaction acceptance and exact remaining scope

Fresh clone, independently built binary/node dependencies and separate seed copies:

```text
DM browser acceptance: 2 targets passed; filter, Enter, hidden selection, clear, real create, rename, invalid name, reload, reuse, huddle redirect and Escape match
Room browser acceptance: 2 targets passed; keyboard room/person jumps, recent rooms, option navigation, Escape, member drawer/focus trap, header menu keyboard/outside dismissal, thread focus return and native pins match
Inbound-email browser acceptance: 2 targets passed; create, cancel/confirm rotation, Rails 302-to-404 redirect, flash, reload, valid-CSRF non-admin denial and direct-room exclusion match
DM browser discrimination: wrong submitted member ID rejected by persisted settings acceptance
Inbound-email browser discrimination: submitting as an administrator fails the non-admin denial assertion
```

One **fresh room-probe timing failure** is retained in `.scratch/room-browser-initial.log`: after Rails completed, the Rust leg's immediate keyboard-menu focus assertion saw `null` rather than `menuitem` (`room_browser.mjs:67`, second-target call at line 115). The shared pinned `header_overflow_controller.js` schedules focus with requestAnimationFrame after setting menu visibility. An unchanged rerun passed both targets. No script assertion, timeout, test concurrency or app code was changed to make it pass. This is observed flakiness, not a claim of an inherited-main failure or resolved timing defect.

The inventory `ws8br-browser-acceptance.json` lists the exact probe interactions. These probes do not count as complete original Rails Minitest/system executions. `ws8br-rails-cases.json` retains exact per-declaration owners and phase exclusions.

Remaining behavior mappings: full original owned system coverage for cross-tab/sign-out/agent/member access; Turbo navigation/back/forward/cancel/loaders; remaining shortcut/modal/unread behavior; selection/range/long-press/presence re-render/menu actions; group add/leave/system-note and configured huddle flows; phone/tablet header actions/badges; favorites/categories/involvement/room-menu actions; unread divider/jump interactions; managed-profile launch and CSP flows. Relevant source files are `channel_members`, `channel_navigation`, `keyboard_shortcuts`, `member_select_mode`, interaction portions of `mobile_layout`/`motion`, `people_group_dms`, `quick_switcher`, `room_header`, `sidebar_organize`, `sidebar_room_menu`, `unread_divider`, `unread_rooms`, `browser_launch_profile`, `content_security_policy`. User/profile/account/tour/public/PWA/QR interactions remain WS8br2's where applicable. No screenshot, pixel, geometry, opacity or positioning acceptance is requested or inventoried as remaining work.

The inbound-email address brief is covered: create; cancel/confirm rotation; observed Rails generic-edit 302-to-404 redirect; flash; reload persistence; valid-CSRF ordinary-member denial preserving the address; direct-room exclusion. The pinned UI has Create/Rotate; nonexistent enable/disable actions are not invented.

## Commands rerun and raw receipts

Canonical cwd is the assigned worktree. Fresh cwd is `.scratch/fresh13/repo` below it. No target, database seed, untracked capture or node_modules was copied from canonical into the clone. The clone starts via:

```sh
git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh13/repo
PARITY_NAMESPACE=ws8br-fresh13-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 .scratch/fresh13/repo/rust/parity/bin/seed build default first_run
```

It was independently built, then fast-forwarded from `021c58ca` through `1630fc40` and `6dc741c9` before the final complete suite. Source receipt: `.scratch/tested-source.txt` in the clone. Raw seed receipt:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Fresh Rust environment (used for suite/clippy/build/native capture):

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149
export RUSTC_BOOTSTRAP=1
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire native_component_capture_matches_rails_root_selection -- --test-threads=4 --nocapture
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-capture.log
python3 rust/reference-tools/rooms/native_residual.py --capture-log .scratch/native-capture.log
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -p campfire
```

Locked metadata exits 0 with no output, including after both merge commits. Duplicate-key receipt:

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
```

The committed media runner invokes the storage/vector test executable in the pinned media image, using the clone's same absolute source/target path. Other executables run natively. rustc still uses the machine-wide throttle. Native host media versions differ from oracle versions; version checks are preserved, not relaxed. Debug assertions remain on; only debug symbols are disabled.

All 58 final workspace raw summary lines (empty harnesses/doctests and existing ignores are explicit):

```text
test result: ok. 1319 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 472.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.69s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 705 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 59.87s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.71s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.08s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.00s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.30s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.43s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
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

Fresh clippy/build and focused native-capture raw lines:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 18s
    Finished `dev` profile [unoptimized] target(s) in 1m 14s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1320 filtered out; finished in 0.82s
```

Fresh oracle commands:

```sh
python3 rust/reference-tools/rooms/check_controller_files.py
python3 rust/reference-tools/rooms/check_owner_panels.py
python3 rust/reference-tools/rooms/check_picker_components.py
```

Actual per-file Rails and panel/configuration receipts:

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
Rails panel source: 6 original files match d7c7de92
Rails owner panel oracle: 30 composition cases and 4 pin panels reproduced; all captured bytes unchanged
Rails Picker oracle: 8 public configurations and complete root composers reproduced; all captured bytes unchanged
```

Canonical inventory command, against final fresh raw logs:

```sh
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh13/repo/.scratch/workspace-final.log --rails-log .scratch/fresh13/repo/.scratch/controller-rails.log
```

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/directs_controller_test.rb: 29 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms_controller_test.rb: 28 Rust cases passed, 1 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/opens_controller_test.rb: 15 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/closeds_controller_test.rb: 12 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/members_controller_test.rb: 13 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/refreshes_controller_test.rb: 4 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/users/sidebars_controller_test.rb: 6 Rust cases passed, 8 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/involvements_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/reads_controller_test.rb: 7 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/favorites_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/room_categories_controller_test.rb: 6 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/categories_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/switchers_controller_test.rb: 5 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

Fresh browser prerequisites and executed commands:

```sh
npm ci --prefix rust/parity
rust/parity/node_modules/.bin/playwright install chromium
export TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target"
python3 rust/reference-tools/rooms/dm_browser.py
python3 rust/reference-tools/rooms/room_browser.py
python3 rust/reference-tools/rooms/inbound_browser.py
python3 rust/reference-tools/rooms/dm_browser.py --inject-selection-drift
python3 rust/reference-tools/rooms/inbound_browser.py --inject-admin-drift
```

Raw results are above; the unchanged room retry and initial timing failure are both retained.

## Failure-first evidence, intermediate failures and runtime

Before panel mounting, the new original guidance test failed at `new-thread panel`, in `.scratch/panel-guide-before13.log`; its final corresponding test passes in the complete fresh suite:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1303 filtered out; finished in 0.86s
```

The first fresh whole workspace, at `1630fc40`, found an actual fixture collision: the first Fizzy message case panicked binding port 51598 with `Os { code: 98, kind: AddrInUse, message: "Address already in use" }`. Raw initial app line:

```text
test result: FAILED. 1318 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 702.88s
```

A discarded attempt to inject the app's test network failed both HTTP matrices because the owners construct system clients. That adapter was removed, not shipped. The inherited-listener fix passes the focused suite and the subsequent complete fresh workspace. Canonical command (same CI/jobs/port/debug environment as above):

```sh
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire fizzy_ -- --test-threads=4
```

```text
test result: ok. 59 passed; 0 failed; 0 ignored; 0 measured; 1262 filtered out; finished in 81.49s
```

The panel other-viewer token setup initially lacked that browser's GET-created secret, and the configured-composer setup initially assumed the seed had no metadata consent. Those setup assertions were corrected to real seed facts; no production permission/consent logic was weakened. Earlier new-room browser development selected the seed's matching group room instead of the actual Kevin person; the probe now selects the `Open DM` person and verifies exactly one newly accessible room. These are not claimed as discovered production security bugs. Authorization coverage uses real request tokens and discriminator rejection; no new production authorization change is represented as failing first.

The final fresh app is 472.15 seconds versus 702.88 seconds in the initial collision run. No concurrency was lowered and no threshold widened. The slow tests below are mainly existing message-boundary matrices; the owned off-page unread case creates forty-one messages serially and is the slowest room mapping. Coverage is retained:

```text
Slowest app tests:
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <46.352s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <41.100s>
test controllers::messages::paging_tests::root_formats_and_destroy_side_effects_match_rails ... ok <40.991s>
test controllers::messages::paging_tests::validators_observe_related_rows_and_older_unpins_without_message_touches ... ok <38.122s>
test controllers::messages::paging_tests::page_anchors_require_alive_membership_and_a_root_message ... ok <36.509s>
test controllers::messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes ... ok <36.496s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <34.771s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <28.164s>
test controllers::message_features::slash_tests::slash_and_picker_responses_match_pinned_rails_exact_bytes ... ok <27.567s>
test integrations::fizzy::agent_requests::tests::ws15e_fizzy_agent_requests_match_pinned_service_results ... ok <24.102s>
Slowest owned room tests:
test controllers::rooms::rooms_rails_cases::show_keeps_the_last_page_when_the_first_unread_fell_off_it_and_links_the_pill_to_it ... ok <13.889s>
test controllers::rooms::direct_rename_tests::direct_rename_coercions_and_rejections_match_real_rails_requests ... ok <5.194s>
test controllers::rooms::owner_panel_tests::native_picker_configuration_reaches_layout_and_owner_composer ... ok <5.118s>
test controllers::rooms::direct_selection_tests::direct_selection_queries_match_rails_and_commit_notes_audits_and_flash ... ok <2.735s>
test controllers::rooms::directs_rails_cases::group_dm_notes_cannot_be_edited_or_deleted ... ok <2.481s>
```

## Exact remaining work and retained evidence

1. The nine named controller declarations above: eight WS13 integrations and one queue-decision conflict.
2. WS13 configured huddle header/sidebar presence, grant sighting/cache invalidation and configuration adapters through the actual owner code.
3. WS15g GitHub card integration after #167: the one exact 1089-byte native residual, plus quote/provider/preload reconciliation.
4. Complete native HTTP room-page byte acceptance for the pinned seed, including layout/shell/list/composer/panels. The tested regions are not full-page equality.
5. Remaining original system interaction mappings enumerated above and in the inventory, with per-file pass receipts. Existing DM/room/inbound probes are delivered; configured huddle and other owner flows remain separate. Geometry/pixel work is excluded.

Final logs/byte diagnostics are retained under `.scratch/fresh13/repo/.scratch/`: `workspace-final.log`, `workspace-initial.log`, `clippy.log`, `native-capture.log`, `native-comparison.log/status`, `native-residual.log`, `native-components-diff/`, `build.log`, `controller-rails.log`, panel/Picker reference logs, browser/discrimination logs, and the initial room focus failure. Canonical failure-first/development logs remain under `.scratch/`; `timings-final.log` and `inventory-final.log` are under `.scratch/fresh13/`. Required external report and tracked mirror are identical.

Verified cleanup receipt (`.scratch/fresh13/cleanup.log`):

```text
Fresh-clone scratch targets: none remaining; fresh13 target deleted, raw logs and byte diagnostics retained
Owned Cargo, rustc, app test/server and room browser fixture processes: none running
Owned ws8br Docker test/reference containers: none running
```
