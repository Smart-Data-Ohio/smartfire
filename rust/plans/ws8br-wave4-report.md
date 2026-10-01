# WS8br Wave 4 continuation report — PARTIAL

Latest tested application/browser source: `ad4ffef14a6a5249707c12e2feddf45c7b81fa5b`. Main is merged through `65ad0d39` (#167), with #170, #171 and #173 included. The final independent fresh clone passes **2802 workspace tests, zero failures, eleven existing ignores across 58 harness summaries**. The seeded app passes **1395 tests, zero failures, two existing ignores**. `CI=1` rejects missing seeds. Clippy passes with `-D warnings`.

Four complete native room pages match the Rails oracle, with the approved shared live-asset helper. All nine standalone list/composer/pending-template regions match strictly, with zero differences and no masks. Main's unchanged WS15g factory fills the former 1089-byte GitHub gap; WS8br did not write a card renderer. Ten original browser declarations now have separate Rails/Rust interaction receipts. The fourteen owned controller files retain 152 named original Rust passes and nine explicit deferrals. Browser receipts are not Rails Minitest runs.

## Pushed coherent slices

| Commit | Change |
| --- | --- |
| `2912fe66` | Merge origin/main at `65ad0d39`, preserving message/thread/agent/attachment behavior and both sides' assertions; adopt main's held-listener helper. Locked metadata and duplicate-key checks pass. |
| `57983370` | Capture four complete pages from Rails only; use the real HTTP fact factory for native acceptance; fix shell whitespace, brand-icon and Drive-consent metadata; reconcile main GitHub factories with existing M1/M2 preloads and broadcasts. |
| `23de26cd` | Verify atomic HTTP rollback plus recovery of a persisted Rails tombstone after eleven minutes. The original queue-success assertion remains a policy conflict. |
| `20f88bbc` | Add original QuickSwitcher declaration/source receipts and actual global Room.count checks on both server databases. |
| `edbba098` | Execute five original sidebar organization declarations with menus, ordering, category forms and reload persistence. |
| `132928c2` | Give the five original switcher declarations an independent runner; retain the complete default drawer/header/thread/pins probe unchanged. |
| `ad4ffef1` | Resolve clippy's checked-unwrap and item-order errors without lint allowances, altered branches or assertion changes. |

The final documentation/inventory commit follows this tested source. It changes no application or browser assertions.

## Changes by file and owner boundaries

- `controllers/rooms.rs`, `presenters/room_native.rs`: extract `NativePage` loading into the same read-only factory used by the HTTP controller and full-page acceptance. Render actual owner components inside the request token/CSP scope. Add Rails' exact empty-room placeholder, populated collection prefix and thread-panel content_for suffix; leave message-list/composer internals intact.
- `rooms/full_page_tests.rs`, `full_pages.json`, `reference-tools/rooms/full_pages.rb`, `check_full_pages.py`: four entire Rails captures with real controller.show assigns, including unread and OOO facts. Deterministic global/per-form tokens and nonce are renderer inputs on both sides. No output masks or Rust-generated goldens. The checker verifies pinned room/message sources at d7c7de92 and approved #163 application layout at 2e20b24c. Real HTTP viewer token ownership remains separately tested.
- `presenters.rs`, `presenters/view_context.rs`: mount the unchanged WS13 `498aa4e6` read-only client-icon-name helper, using the already vendored registry. Factor the existing Drive scope predicate into `google_drive_consent`, shared by composer selection and layout preferences. It selects scopes only, never credentials or tokens. This is no Google transport or configuration-policy implementation.
- `views/templates/rooms/show.html`, `views/src/lib.rs`: align native component/OOO whitespace and permit cloning the rendering context for deterministic page inputs. The four complete pages include the actual layout, shell, head, list, composer, pending template and thread/poll/pin panels.
- `presenters.rs`, `searches/preloads.rs`, `messages/rendered.rs`, `rooms/native_integration_tests.rs`: call main's unchanged GitHub message_cards/cache_stamp factories, share refresh IDs across child presenters, preload persisted linked-message facts and retain empty facts, and drain refresh IDs through main's writer seam in the real message-edit path. Preserve query-free quoted rendering and constant search-query counts. Remove the prior Designers exception from the native test. M1 should reconcile these adapter fields/refresh sets with its in-flight branch; no GitHub template or pin policy is ported here.
- Merge reconciliation in `channels.rs`, `channels/sink.rs`, `controllers/messages.rs`, `rooms/{opens,closeds,refreshes}.rs`, config/app/jobs and user lifecycle: retain M1/M2 features, WS11 guarded agent behavior and combined periodic roster while adding main's GitHub handlers/status callbacks and staged/signed/existing attachment analysis. Open/closed forms use main's subscription section in the real new/edit/error branches. WS11's actual suspension primitive remains authoritative; main's linked-account disconnect runs in the existing transaction.
- `integrations/test_support.rs`, Fizzy HTTP matrices: use main's held-listener OwnedFd/stdin protocol in both matrices. Remove the obsolete fixed-port helper. Preserve every matrix and assertion; no test concurrency reduction or timeout relaxation.
- `rooms/queue_recovery_tests.rs`, `native_acceptance_discrimination.py`: reject durable RoomDestroyJob insertion through the real HTTP route and assert rollback of deletion/claim/memberships; create the legacy persisted tombstone state; prove quiet-before-grace, exactly-once reenqueuing and actual destruction after eleven minutes. Separate production mutations break the heading bytes and recovery cutoff and must fail the tests.
- `reference-tools/rooms/{quick_switcher_browser,room_browser,sidebar_organize_browser,inbound_browser}.*`, `plans/ws8br-system-mappings.json`: real Playwright interactions on separately copied Rails/Rust seeds. Original synthetic HTML5 drag events drive the same browser handlers; geometry is input for the drop, never a comparison. Restore fixture state through actual controllers between sidebar declarations. Count the real global rooms table before/after switcher interactions.
- `deferred_inventory.py`, browser/controller inventories, `ws8br-owner-integration.md`, `ws8br-native-residual.json`: validate browser receipts against literal original declarations and pinned source hashes; distinguish individual native tests, browser mappings and actual Ruby Minitest counts. Diagnostic residual now has equal 288823-byte Designers lists and zero unmatched regions. Shared user/account/public/tour/PWA/QR scope remains WS8br2's.

All goldens come from Rails at d7c7de92 plus approved _common drift. Main's asset_goldens helper validates actual compiled digests and compares every surrounding byte. Standalone component comparisons apply no masks. No screenshot or pixel work was done.

## Exact shell inputs to message/composer/panel seams

The detailed contract is `rust/plans/ws8br-owner-integration.md`.

1. `room_native::load(conn, app, room, user, message_id, request_host, cache_base_url)` receives the reader, actual AppState, verified request host/origin and real records. It returns ShowView, ComposerFacts and link/Twitter/GitHub pending IDs. Selected roots are the last forty, or forty before + the same-room root anchor + forty after. Missing, foreign and thread anchors fall back to the last page.
2. The viewer's membership supplies last_read_message_id/unread_at. `room_native::message_list(&presenter,&messages,divider.message_id,divider.count)` calls unchanged `Presenter::room_message_list`. Populated collections get rooms/show's `"\n    \n"`; empty roots still call the owner seam, then use Rails' `"\n"` placeholder. Shared per-message fragments exclude viewer divider facts.
3. Root composer facts receive real room ID/kind, viewer-relative display name, `thread: None`, static slash commands followed by room agent commands, and existing DriveFlow. Public Picker needs three nonblank public settings and a human. Drive consent uses persisted scopes. Voice/Stage/Board retain the documented Closed composer fallback pending their screen owners.
4. The real request ViewContext carries account/preferences/chrome, signed viewer avatar, time zone, assets, verified URL, last-room/recent searches, actual HTTP CSRF/CSP and flash. M2 scheduled ComposerButton gets ctx, room_id and thread_id None and is passed to M1 Composer. M1 PendingTemplate gets ctx and the viewer once. Root footer removes one initial owner newline and adds two spaces; pending template preserves its source newline.
5. ThreadPanel receives the real room view plus neutral room_display_name(room,None), including the viewer in unnamed DMs, with two leading spaces and a trailing newline. PollBuilder gets the same room and request ctx. M2 PanelPartial gets room ID, full header STI param key and real MessagePin count. Pins refresh delegates to M2's actual list/count; no list/order/excerpt/unpin policy is duplicated.
6. Remaining ShowView facts include room updated_at, original-room invitation predicate, join code, original selected MessageItems, signed room stream, unread jump/scroll and WS17 OOO members. HTTP code enqueues link/Twitter requests and GitHub refreshes through main's writer seams after the reader closes.

Configured huddle integration remains unfinished: merged main has only the readiness boolean, not WS13's Config, HuddleGrant::participants_for, call_navigation::model or stage/participant callbacks. Their inspected owner source is 498aa4e6. ShellComponents.huddle_header remains stable and empty; LayoutChrome.huddle_configured remains false. The eight sidebar cases below wait for WS13 as instructed; no grant-policy imitation is added.

## Byte receipts and original declarations

Complete Rails fixture sizes (native comparison permits only validated live asset digests):

| Room | Viewer | Rails UTF-8 bytes |
| --- | --- | --- |
| 654632876 | 127326141 | 404649 |
| 186869642 | 127326141 | 133574 |
| 699448329 | 712064548 | 117266 |
| 201306877 | 773523953 | 111766 |

Standalone strict native receipt:

```text
PASS room 654632876 message_list: 288823 exact bytes
PASS room 654632876 composer: 10373 exact bytes
PASS room 654632876 pending_template: 1449 exact bytes
PASS room 186869642 message_list: 19547 exact bytes
PASS room 186869642 composer: 10369 exact bytes
PASS room 186869642 pending_template: 1449 exact bytes
PASS room 699448329 message_list: 6559 exact bytes
PASS room 699448329 composer: 8596 exact bytes
PASS room 699448329 pending_template: 1464 exact bytes
Native room component acceptance: 9 exact matches; 0 differences; no masks
Native residual inventory: 0 empty owner card slots explain the complete remaining difference; strict acceptance still passes
```

Named original controller declarations, largest files first; extras are counted separately by the inventory:

| Pinned original file | Named Rust passes | Deferred |
| --- | --- | --- |
| `test/controllers/rooms_controller_test.rb` | 28 | 1 |
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 0 |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 0 |
| `test/controllers/users/sidebars_controller_test.rb` | 6 | 8 |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 0 |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 0 |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 0 |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 0 |
| `test/controllers/room_categories_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 0 |
| `test/controllers/switchers_controller_test.rb` | 5 | 0 |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 0 |

Exactly nine original deferrals in those fourteen files:

- `test/controllers/rooms_controller_test.rb` — **destroy succeeds when the queue is down and the sweep recovers the room**: Lead decision 2 requires atomic queue rollback; native fault-injection coverage is separate.
- `test/controllers/users/sidebars_controller_test.rb` — **channel row shows the live huddle stack with names and count**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **board row shows the live huddle stack with names and count**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **direct row shows the live huddle stack when the peer is in the call**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **quiet rows keep an empty stack target with no visible presence**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **direct row re-renders when a participant joins**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **group direct rooms render member names and a huddle stack**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **no channel or DM stacks without huddle configuration**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **sidebar query count does not grow with quiet channels, DMs, boards, and stages**: WS13 huddle grant/presence integration and full-request query instrumentation.

Original browser mappings and still-unmapped behavior source declarations are below. Existing DM/drawer/header/inbound probes are additional coverage, not substitutes for full original file mappings. Header tests' original configured-huddle setup is not reproduced by the unconfigured probe. People/profile/tour/agent/call parts retain their owners. Literal declarations and phase/owner flags are in the JSON inventory.

| Original system file | Browser mappings passed | Behavior declarations still unmapped | Owner boundary |
| --- | --- | --- | --- |
| `test/system/audit_log_test.rb` | 0 | 2 | WS8br2 |
| `test/system/channel_members_test.rb` | 0 | 4 | WS8br |
| `test/system/channel_navigation_test.rb` | 0 | 7 | WS8br |
| `test/system/first_run_tour_test.rb` | 0 | 4 | WS8br2 |
| `test/system/icons_test.rb` | 0 | 4 | WS8br2 |
| `test/system/keyboard_shortcuts_test.rb` | 0 | 14 | WS8br |
| `test/system/member_select_mode_test.rb` | 0 | 18 | WS8br |
| `test/system/mobile_layout_test.rb` | 0 | 2 | WS8br drawer destinations; WS8br2 pages outside the workspace |
| `test/system/motion_test.rb` | 0 | 9 | WS8br room/member/drawer interactions; WS8br2 people-directory interactions; style assertions excluded |
| `test/system/people_group_dms_test.rb` | 0 | 19 | WS8br DM/member shell; WS8br2 people/profile; WS13 configured calls; WS11 agents |
| `test/system/quick_switcher_test.rb` | 5 | 0 | WS8br |
| `test/system/room_header_test.rb` | 0 | 9 | WS8br |
| `test/system/service_worker_test.rb` | 0 | 2 | WS8br2 |
| `test/system/sidebar_organize_test.rb` | 5 | 1 | WS8br |
| `test/system/sidebar_room_menu_test.rb` | 0 | 16 | WS8br |
| `test/system/starred_people_test.rb` | 0 | 4 | WS12 with WS8br2/WS8br presentation |
| `test/system/timezone_detection_test.rb` | 0 | 2 | WS8br2 |
| `test/system/unread_divider_test.rb` | 0 | 5 | WS8br |
| `test/system/unread_rooms_test.rb` | 0 | 2 | WS8br |
| `test/system/workspace_icons_test.rb` | 0 | 1 | WS8br2 |
| `test/system/browser_launch_profile_test.rb` | 0 | 1 | WS8br |
| `test/system/content_security_policy_test.rb` | 0 | 5 | WS8br |

The supplemental `test/controllers/audit_log/rooms_audit_test.rb` inventory also retains twelve room declarations without individual original-case receipts (the existing room-audit matrices pass separately); five account/icon declarations belong to WS8br2. Unfurl controller rendering uses main WS15e directly; its nine original declarations remain WS15e's mapping responsibility.

## Fresh-clone commands and raw receipts

Independent clone: `.scratch/fresh14/repo`, initially 20f88bbc. Only committed browser-only slices were fast-forwarded during the initial workspace run. After clippy reported three lint errors, the clone fast-forwarded to ad4ffef1 and ran final metadata, clippy/build, browsers and the complete workspace again. No target, seed, storage or node_modules directories were copied from the working tree. The fresh seeds are built from the pinned reference; all app fixtures are committed.

```sh
git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh14/repo
PARITY_NAMESPACE=ws8br-fresh14-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 .scratch/fresh14/repo/rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Commands below ran from the independent clone unless explicitly labelled canonical. The machine-wide rustc throttle remains active; no extra jobs were set.

```sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0
export CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149
export RUSTC_BOOTSTRAP=1
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/rooms/pinned_media_runner.py"
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
mise exec rust@1.98.1 -- cargo build --locked --manifest-path rust/Cargo.toml -p campfire
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire native_component_capture_matches_rails_root_selection -- --test-threads=4 --nocapture
python3 rust/reference-tools/rooms/native_components_check.py --capture-log .scratch/native-capture-final.log
python3 rust/reference-tools/rooms/native_residual.py --capture-log .scratch/native-capture-final.log
```

Locked metadata exits zero silently. Raw TOML, clippy, build and native-capture lines:

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
    Finished `dev` profile [unoptimized] target(s) in 42.74s
    Finished `dev` profile [unoptimized] target(s) in 13.42s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1396 filtered out; finished in 1.44s
```

All 58 final workspace summary lines, including empty harnesses and explicit existing ignores:

```text
test result: ok. 1395 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 758.09s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 45.34s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.83s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s
test result: ok. 705 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 114.18s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.99s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.67s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.19s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.50s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.77s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
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

The seeded app's two explicit ignores are the reference recording helper and the push-latency measurement. Media/vector checks use the committed pinned_media_runner in the pinned Rails media image at the same absolute clone path. No vector/version assertion was weakened. Debug assertions remain on; only debug symbols are disabled.

Fresh Rails reproduction commands:

```sh
python3 rust/reference-tools/rooms/check_controller_files.py
python3 rust/reference-tools/rooms/check_owner_panels.py
python3 rust/reference-tools/rooms/check_picker_components.py
python3 rust/reference-tools/rooms/check_full_pages.py
```

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
Rails full-page oracle: 4 complete pages reproduced; pinned rooms/messages and approved #163 layout verified; bytes unchanged
```

Fresh browser commands:

```sh
npm ci --prefix rust/parity
rust/parity/node_modules/.bin/playwright install chromium
export TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target"
python3 rust/reference-tools/rooms/quick_switcher_browser.py
python3 rust/reference-tools/rooms/sidebar_organize_browser.py
python3 rust/reference-tools/rooms/dm_browser.py
python3 rust/reference-tools/rooms/room_browser.py
python3 rust/reference-tools/rooms/inbound_browser.py
python3 rust/reference-tools/rooms/dm_browser.py --inject-selection-drift
python3 rust/reference-tools/rooms/inbound_browser.py --inject-admin-drift
```

```text
QuickSwitcher browser acceptance: 2 targets passed; all five original interactions match
QuickSwitcher original mapping: 5 passed on Rails; 5 passed on Rust; 0 failed; exact global Room.count delta +1 on each
QuickSwitcher pinned source SHA256: 37760d50e96277757bbc68562dfadc689c51ae9a28bff33c1219ec85404c79d3
SidebarOrganize original mapping: 5 passed on Rails; 5 passed on Rust; 0 failed; muted delivery case remains unexecuted
SidebarOrganize pinned source SHA256: db6a1bba4f7576a3ace15dd368e3b44e195bc2b45c82d32bc9aa7a4b7fc76286
DM browser acceptance: 2 targets passed; filter, Enter, hidden selection, clear, real create, rename, invalid name, reload, reuse, huddle redirect and Escape match
Room browser acceptance: 2 targets passed; keyboard room/person jumps, recent rooms, option navigation, Escape, member drawer/focus trap, header menu keyboard/outside dismissal, thread focus return and native pins match
QuickSwitcher original mapping: 5 passed on Rails; 5 passed on Rust; 0 failed; exact global Room.count delta +1 on each
QuickSwitcher pinned source SHA256: 37760d50e96277757bbc68562dfadc689c51ae9a28bff33c1219ec85404c79d3
Inbound-email browser acceptance: 2 targets passed; create, cancel/confirm rotation, Rails 302-to-404 redirect, flash, reload, valid-CSRF non-admin denial and direct-room exclusion match
DM browser discrimination: wrong submitted member ID rejected by persisted settings acceptance
```

The last command failed during **Rails setup**, before candidate mutation acceptance:

```text
page.goto: net::ERR_ABORTED at http://127.0.0.1:52160/rooms/opens/201306877/edit
```

Retained in `.scratch/inbound-discrimination.log`; no retry, test-concurrency reduction or threshold widening. The positive valid-CSRF denial/persistence acceptance passed on both targets. The complete default room probe passed unchanged this run.

Canonical receipt-generation command, using the final fresh raw logs:

```sh
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh14/repo/.scratch/workspace-final.log --rails-log .scratch/fresh14/repo/.scratch/controller-rails.log --system-log .scratch/fresh14/repo/.scratch/quick-switcher-browser.log --system-log .scratch/fresh14/repo/.scratch/sidebar-organize-browser.log
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
Original system mapping receipts: test/system/quick_switcher_test.rb: 5 Rails/Rust browser cases passed, 0 behaviour cases remaining; Rails Minitest execution count stays zero
Original system mapping receipts: test/system/sidebar_organize_test.rb: 5 Rails/Rust browser cases passed, 1 behaviour cases remaining; Rails Minitest execution count stays zero
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

## Failures first, fixes and runtime

The first merged app run exposed five GitHub refresh/preload integration defects, plus two subprocess NoSuchFile failures caused by my rebuilding the executing test binary. The refresh/preload adapters were fixed; subsequent Cargo builds and tests were sequenced. The final fresh suite passes those actual owner tests, without removing cases or changing assertions. Initial raw app line:

```text
test result: FAILED. 1386 passed; 7 failed; 2 ignored; 0 measured; 0 filtered out; finished in 618.64s
```

Clippy then rejected two checked unwraps and the icon helper after the test module. The if-let/item-order slice fixes all three without allowances. The first fresh workspace, before those lint-only fixes, had this raw app result:

```text
test result: ok. 1395 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 700.92s
```

Canonical mutation command (executed before the lint-only slice at functional source 20f88bbc):

```sh
python3 rust/reference-tools/rooms/native_acceptance_discrimination.py
```

```text
full-page-header: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1396 filtered out; finished in 1.38s
stuck-room-recovery: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1396 filtered out; finished in 1.19s
Native acceptance discrimination: 2 broken production paths rejected; source restored
```

Both source files were restored. Broken header bytes and a recovery cutoff delayed by an hour each fail the corresponding real native test. This continuation introduces no local authorization policy; merged owner guards and existing nonmember/deleted/bot/CSRF boundaries rerun through the suite, and the DM browser discriminator rejects a valid wrong member ID. The failed inbound discriminator setup is reported separately above.

No coverage was cut, no test concurrency lowered, and no timing threshold widened. The fresh final timings below show the slow cases; the new four-page and queue tests are included in the complete app suite.

```text
Slowest app tests:
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <46.953s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <45.660s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <44.748s>
test controllers::github::write_tests::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails ... ok <40.069s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <39.930s>
test integrations::fizzy::agent_requests::tests::ws15e_fizzy_agent_requests_match_pinned_service_results ... ok <39.516s>
test integrations::fizzy::agent_job::tests::ws15e_fizzy_agent_execution_rechecks_and_records_once ... ok <38.814s>
test controllers::messages::paging_tests::root_formats_and_destroy_side_effects_match_rails ... ok <37.635s>
test controllers::messages::paging_tests::page_anchors_require_alive_membership_and_a_root_message ... ok <35.585s>
test controllers::messages::paging_tests::validators_observe_related_rows_and_older_unpins_without_message_touches ... ok <35.432s>
Slowest owned room tests:
test controllers::rooms::rooms_rails_cases::show_keeps_the_last_page_when_the_first_unread_fell_off_it_and_links_the_pill_to_it ... ok <23.548s>
test controllers::rooms::owner_panel_tests::native_picker_configuration_reaches_layout_and_owner_composer ... ok <13.513s>
test controllers::rooms::direct_rename_tests::direct_rename_coercions_and_rejections_match_real_rails_requests ... ok <7.732s>
test controllers::rooms::direct_selection_tests::direct_selection_queries_match_rails_and_commit_notes_audits_and_flash ... ok <4.154s>
test controllers::rooms::refreshes_rails_cases::refresh_includes_pin_and_unpin_changes_since_the_last_sync ... ok <3.899s>
New acceptance tests:
test controllers::rooms::full_page_tests::full_native_room_pages_match_four_complete_rails_pages ... ok <2.851s>
test controllers::rooms::queue_recovery_tests::queue_decision_keeps_atomic_http_failure_and_recovers_a_rails_tombstone ... ok <1.516s>
```

## Exact remaining work

1. **Queue decision:** the single original queue-down-success declaration remains deferred under decision 2. Atomic HTTP500 rollback and persisted Rails tombstone recovery are verified separately; matching the original HTTP success would violate that fixed decision.
2. **Configured WS13 integration:** real header/sidebar participant/grant/stage/configuration/callback adapters and configured full-page captures. All eight named original sidebar declarations above stay flagged for WS13.
3. **Original systems:** all still-unmapped behavior declarations in the system table/JSON, including SidebarOrganize's literal `muted rooms dim and stay quiet until mentioned`. Profile/people/account/tour/PWA/QR portions remain WS8br2, stars WS12, call/agent domains WS13/WS11. Supplemental original room-audit per-declaration receipts remain distinct from existing passing matrices.
4. **Acceptance breadth:** the four full-page captures and nine strict component regions pass. Additional room/viewer/anchor/unread combinations and configured voice/stage/board pages are not claimed as complete-page captures; use the owning screen APIs as they land.
5. **Inbound negative browser probe:** repeat its unchanged mutation assertion when Rails setup navigation succeeds; this run stopped at ERR_ABORTED before that assertion. Positive inbound behavior/authorization/persistence passed.

There is no remaining GitHub byte difference in these measured regions, and no pixel/screenshot items remain. Raw logs, diffs and receipts are retained in `.scratch/fresh14/repo/.scratch/`, initial/final workspace logs separately; canonical mutations are in `.scratch/native-acceptance-discrimination/`. The report's external copy and tracked mirror are identical.

Verified cleanup:

```text
Fresh-clone scratch targets: none remaining; fresh14 target deleted, raw logs and byte diagnostics retained
Owned Cargo, rustc, app test/server and room browser fixture processes: none running
Owned ws8br Docker test/reference containers: none running
```
