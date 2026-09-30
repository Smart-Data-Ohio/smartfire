# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`; assigned worktree `rust-ws8br`; Rails pin `d7c7de92`. Main remains `21a7332f2d3c324f0862cdf448baf17a84395aa0`, already merged with merge commit `9f356e2f`. Latest pushed implementation is `213eca45dc010de5eadeeedb36e3d366d1686d01`, following the sidebar slice `7e037a45`. The following coverage/report commit includes the additional favorite-kind and unread-control goldens. Earlier HTTP/switcher/header work remains delivered. No PR or deployment.

**What changed this continuation, by file**

- `views/src/users/sidebar.rs`, `templates/users/sidebars/show.html`, `_room_categories.html`, `_room_menu.html`, `rooms/_shared.html`, `_direct_placeholder.html`, `_empty_venue_children.html`: full workspace/sidebar layout, navigation/profile tools, mixed favorites, channels/boards/voice/stage/direct sections, own categories with order/collapse/empty/forms, menu and placeholder DMs. Five complete Rails frames include every seeded favorite kind and muted/unread rows. Venue mount defaults match Rails' empty children even when LiveKit is unconfigured; configured children remain WS13 inputs.
- `campfire/src/controllers/presenters/accounts.rs`, `users/sidebars.rs`, `users/sidebars_tests.rs`: partition favorites before original sections; preload direct users in one membership-order query; preserve recipient role/cache facts. Three seeded database compositions match complete Rails frame bytes. Custom workspace icon lookups can still add per-row queries; constant overall query-count parity is not claimed.
- `campfire/src/controllers/rooms/involvements.rs`, `rooms.rs`, `presenters/page.rs`, `channels/tests/directory_test.rs`: shared visibility/mute rows now carry the recipient's membership/menu and correct optional unread local. Six actual HTTP changes match six actual socket frames, with another user's stream silent. Existing direct directory rename/clear/leave/add frame coverage is retained.
- `views/src/rooms.rs`, `templates/rooms/show.html`, `show/_nav.html`, `_header_overflow.html`, `_member_panel.html`, `_invitation.html`, `involvements/_bell.html`: room-workspace body/sidebar region, first-paint preloads, full unconfigured channel/DM header controls/overflow, member panel, accessible message-area wrapper, jump controls and corrected Smartfire invitation name. Four empty-room fixtures compare six regions each; both jump controls compare Rails bytes. Footer/thread owner regions are supplied Rails fragments, so their pass-through does not prove their Rust internals.
- `campfire/src/controllers/presenters/room_shell.rs`, `presenters.rs`, `rooms.rs`: root-only anchored pagination and read-only unread facts. Eight Rails cases cover read rooms, three/five/six unread, off-page boundaries, legacy stamps, deleted pointers and absent boundaries. More than five scrolls; five does not. No domain/schema change.
- `rooms/tests.rs`: preserve the 40/41 root-page assertions separately from the authorized empty-list placeholder, and exercise the legacy tokenless boost in the real pagination response. `messages/tests.rs`: minimal cross-owner test touch, moving only the forged-host fragment probe's URI to the actual pagination endpoint; every cache-poisoning assertion remains. No WS8b-m renderer internals were edited.
- `views/tests/{sidebar,room_shell}.rs`, `views/tests/golden/{sidebar,rooms}`, `vectors/room_shell_unread.json`, `reference-tools/rooms/{sidebar_page,involvement,shell,unread_shell}.rb`, discrimination scripts: pinned Rails full-byte oracles, compiled regressions and regeneration tools. No normalization or masks/allowlists changed.
- `plans/ws8br-rails-cases.json`, `reference-tools/rooms/deferred_inventory.py`: all 57 scoped Rails controller/system files grouped by file, with 495 source-declared names and lines, hashes, and explicit zero Rails execution/pass counts. Source declarations are not dynamically expanded Minitest counts.

**Seam and parity limits**

`rooms::room_message_list(ctx, &ShowView)` is the stable entry point. `ShowView.shell.message_list` accepts trusted owner-rendered HTML; absent it, the default is zero bytes, matching Rails' empty collection. The original message data and unread facts remain available to WS8b-m. `ShellComponents` carries page-local message-template, composer, thread, pin, poll, huddle-header and OOO inputs. These inputs are not read from request params or put into a shared cache/broadcast.

The live page currently uses that requested empty main list and inherited composer/template fallbacks. WS8b-m has not supplied its list/composer/thread/pin/poll adapter. The full populated room page is therefore **not** byte/pixel accepted. Four shell fixtures lend Rails' owner fragments and explicitly choose empty OOO state; they prove the surrounding composition, not normal DM OOO behavior. WS17 must provide those viewer-specific notices; WS13 configured huddle/stage children and WS12 board pages remain their owners. The new header overflow is verified for channels/DMs; complete stage/board page integration is not claimed.

Sidebar rows/cache keys retain membership version, participant IDs and recipient administrator role; request forms remain uncached and keep request tokens. Broadcast fragments remain token/nonce free. The valid bot guard is end-to-end tested; valid agent-token acceptance/denial still needs WS9/WS11 integration. The shared `UserSummary::first_name` vertical-tab discrepancy remains for the users slice; own direct-row labels already match Ruby's whitespace split.

**Verification — commands rerun and raw summaries**

Commands below ran from the assigned worktree, except metadata from `rust/`. Rust 1.98.1, four jobs, its own target/temp directories and WS8br ports. Both previously built pinned `default` and `first_run` seeds are present. CI=1 requires real app seeds; no seeded test silently skipped. Three explicit ignores remain: reference recorder, push-latency measurement and `manages_bots` awaiting WS11. No new ignore.

```bash
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Exit 0, no output.

```bash
python3 rust/reference-tools/rooms/check_workspace.py > .scratch/metadata-duplicates.log
python3 rust/reference-tools/rooms/deferred_inventory.py > .scratch/deferred-inventory.log
```

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
Rails deferred inventory: 57 files, 495 source-declared cases; 0 Rails tests run, 0 Rails passes claimed
```

Each oracle below exited 0; its stdout was validated as JSON and byte-compared with its tracked vector/golden. The new oracle scripts assert matching source hashes. No Rails templates/helpers were patched.

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/http.rb > .scratch/rooms_http.json 2> .scratch/rooms_http-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/switcher.rb > .scratch/switcher.json 2> .scratch/switcher-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/sidebar.rb > .scratch/sidebar.json 2> .scratch/sidebar-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/directory.rb > .scratch/directory.json 2> .scratch/directory-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/sidebar_page.rb > .scratch/sidebar-page.json 2> .scratch/sidebar-page-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/involvement.rb > .scratch/involvement.json 2> .scratch/involvement-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/shell.rb > .scratch/shell.json 2> .scratch/shell-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/unread_shell.rb > .scratch/unread-shell.json 2> .scratch/unread-shell-oracle.log
```

```text
Rails room HTTP oracle: 47 cases; reference d7c7de92
Rails switcher oracle: 2 byte payloads, 2 auth responses; reference d7c7de92
Rails sidebar rows: 15 byte goldens; reference d7c7de92
Rails room directory: 14 header goldens, 31 recipient frames; reference d7c7de92
Rails sidebar page: 5 complete frame goldens; reference d7c7de92
Rails involvement: 6 HTTP transitions, 6 recipient frames; reference d7c7de92
Rails room shell: 4 empty-room region goldens; reference d7c7de92
Rails unread shell: 8 pointer and divider cases; reference d7c7de92
```

Failing-first evidence: the new complete-frame/shell comparisons initially failed before port corrections. The reproducible scripts below compile wrong implementations and require assertion failures, then restore source. They reject inherited room authorization/state, transactional auditing, unmapped switcher/bot guards, cache/nonce leaks, actor headers, association order, Unicode splitting, category collapse, favorite duplication, recipient-less broadcasts, visible empty-list markup, wrong unread threshold and altered jump labels. Script exit 0 means the wrong implementation was rejected; FAILED lines are deliberate.

```bash
python3 rust/reference-tools/rooms/discrimination.py > .scratch/discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/switcher_discrimination.py > .scratch/switcher-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/sidebar_discrimination.py > .scratch/sidebar-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/directory_discrimination.py > .scratch/directory-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/sidebar_page_discrimination.py > .scratch/sidebar-page-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/shell_discrimination.py > .scratch/shell-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 2 passed; 17 failed; 0 ignored; 0 measured; 316 filtered out; finished in 0.73s
WS8br discrimination: 17 HTTP regressions rejected bb6c5d78; 1 existing guard and 1 independent unread-presenter test passed; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.46s
WS8br audit discrimination: compiled transactional-audit regression rejected; source restored
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.38s
Switcher discrimination: compiled unmapped action rejected by all four regressions; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
Sidebar discrimination: compiled cache-key mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
Sidebar discrimination: compiled request-nonce mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.44s
Directory discrimination: compiled unscoped-access mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 1.69s
Directory discrimination: compiled actor-header mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.40s
Directory discrimination: compiled user-join-order mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.00s
Directory discrimination: compiled unicode-whitespace mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.01s
Sidebar page discrimination: compiled collapse mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.08s
Sidebar page discrimination: compiled favorite-partition mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.43s
Sidebar page discrimination: compiled recipient-membership mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
Room shell discrimination: compiled visible-list-placeholder mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.08s
Room shell discrimination: compiled scroll-threshold mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
Room shell discrimination: compiled unread-pill-label mutation rejected; source restored
```

Final seeded app suite (exit 0):

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```

```text
test result: ok. 336 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 28.16s
```

Views suite: 73 passed, zero failed/ignored; raw lines below (exit 0).

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views > .scratch/views-final.log 2>&1
```

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Exact workspace all-target clippy, including html5ever (exit 0):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.38s
```

No full workspace test run, full Rails Minitest run, browser/system/pixel or accessibility acceptance is claimed. The synthetic missing-seed unit test's deliberate local-skip log is not a skipped seeded app test.

**Verified Rust regressions grouped by file**

These are actual current passing tests; they are not complete Rails-file pass counts.

| Rust file | Passing tests |
| --- | ---: |
| `campfire/src/controllers/rooms/parity_tests.rs` | 19 |
| `campfire/src/controllers/rooms/tests.rs` | 16 |
| `campfire/src/controllers/users/sidebars_tests.rs` | 4 |
| `campfire/src/controllers/switchers.rs` | 4 |
| `campfire/src/channels/tests/directory_test.rs` | 3 |
| `views/tests/sidebar.rs` | 5 |
| `views/tests/room_shell.rs` | 3 |
| `views/tests/room_header.rs` | 1 |

**Deferred Rails cases grouped by file**

The complete case names and source lines are in `rust/plans/ws8br-rails-cases.json`. Zero full-file Rails execution/pass counts are claimed; equivalent Rust subsets are described below. Every scoped system/browser file remains deferred. Counts refer to source declarations, not inheritance or dynamic test expansion.

| Rails file | Declared cases | Rails passes | Delivered subset / deferred owner |
| --- | ---: | ---: | --- |
| `test/controllers/rooms_controller_test.rb` | 29 | 0 | Delivered Rust subsets: deletion/job/leave/access guards, shell regions with owner inputs, root pagination, eight unread-pointer cases, jump pills and header/directory bytes. Deferred: open nonmember preview/join, full live-page bytes with WS8b-m/WS17 inputs, unread/read browser flows, error/flash matrices and remaining controller cases — WS8br; message internals — WS8b-m. |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 0 | Subset: cannot convert voice/stage/board. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 0 | Subset: cannot convert voice/stage/board and inaccessible private room guard. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 0 | Subset: active capped creation/reuse/huddle redirect, >10 rejection, rename/add/leave note text, last leave, pair widening/nonmember guards, group deletion permissions. Directory rename/clear/leave/add frames are now byte-tested; defer starred/client-filter picker, settings/delete-button HTML, invalid rename 422 form state, add overflow/no-op/query-count assertions, removed-member read and exact stream/flash bytes — WS8br. Immutable/rendered system-note message assertions — WS8b-m. |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 0 | Delivered Rust subsets: parameter/mute/read/JSON behavior plus six exact per-user HTTP-to-socket transitions. Deferred: exhaustive enum/format cases, every venue transition and browser acceptance — WS8br with WS13/WS12. |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 0 | Deferred entire fork re-diff, including empty 204 and response JSON — WS8br; pins/message facts — WS8b-m. |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 0 | Deferred unread/read endpoints and broadcasts — WS8br, coordinating notification/presence facts with WS17. |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 0 | Deferred fork members JSON/UI — WS8br; presence facts WS17, bot/agent facts WS11. |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 0 | Subset: assignment/unassignment, own category, foreign category/membership access, non-channel 422. Defer exhaustive malformed params/formats and byte acceptance — WS8br. |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 0 | Delivered Rust subsets: scoped/idempotent/reordered writes and full favorite section bytes for all six room kinds. Deferred: malformed formats/params and browser acceptance — WS8br. |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 0 | Subset: token rotation, emailable and creator/admin/member guards, exact redirect. Defer missing relay-domain UI, edit form, browser flow and full notice bytes — WS8br on WS10 domain. |
| `test/controllers/room_categories_controller_test.rb` | 6 | 0 | Delivered Rust subsets: scoped mutations and full category form/ordering/collapse/empty byte states. Deferred: broader coercions/formats and browser acceptance — WS8br. |
| `test/controllers/public_pages_controller_test.rb` | 18 | 0 | Deferred entire fork re-diff and golden acceptance — WS8br. |
| `test/controllers/first_runs_controller_test.rb` | 4 | 0 | Deferred entire fork re-diff/acceptance — WS8br; first_run seed did run upstream integration coverage. |
| `test/controllers/welcome_controller_test.rb` | 2 | 0 | Deferred entire fork re-diff/acceptance — WS8br. |
| `test/controllers/switchers_controller_test.rb` | 5 | 0 | Re-diffed the fork controller. Subset: signed-in/bot guards, room/people/thread scopes, kinds, flags, latest-15 limit, group labels and two complete JSON byte goldens. Defer browser quick-switcher acceptance and the exhaustive input/format matrix — WS8br. |
| `test/controllers/users_controller_test.rb` | 20 | 0 | Deferred entire fork re-diff/acceptance — WS8br. |
| `test/controllers/users/sidebars_controller_test.rb` | 14 | 0 | Delivered Rust subsets: complete frames, every favorite room kind, ordered/empty/collapsed categories, restricted creation, own permissions, direct/group rows, recipient involvement frames and cache separation. Deferred: constant SQL query-count acceptance, configured WS13 children and browser/system acceptance — WS8br/WS13. |
| `test/controllers/users/tours_controller_test.rb` | 5 | 0 | Deferred first-run tour endpoints and browser acceptance — WS8br. |
| `test/controllers/users/profiles_controller_test.rb` | 57 | 0 | Deferred profile fork re-diff/acceptance — WS8br; sudo/session gates coordinate with WS9. |
| `test/controllers/users/avatars_controller_test.rb` | 4 | 0 | Deferred upload/initials SVG/default avatar fork re-diff and byte acceptance — WS8br. |
| `test/controllers/users/cards_controller_test.rb` | 7 | 0 | Deferred user cards fork re-diff/acceptance — WS8br; presence/status facts WS17. |
| `test/controllers/users/bans_controller_test.rb` | 8 | 0 | Deferred ban fork re-diff/acceptance — WS8br. |
| `test/controllers/users/time_zones_controller_test.rb` | 4 | 0 | Deferred timezone fork re-diff/acceptance — WS8br. |
| `test/controllers/accounts_controller_test.rb` | 4 | 0 | Deferred account fork re-diff/acceptance — WS8br; auth/sudo gates WS9. |
| `test/controllers/accounts/users_controller_test.rb` | 3 | 0 | Deferred admin user list/edit fork re-diff/acceptance — WS8br; bots/agents WS11. |
| `test/controllers/accounts/icons_controller_test.rb` | 6 | 0 | Deferred icon library/admin UI fork re-diff/acceptance — WS8br. |
| `test/controllers/accounts/audit_logs_controller_test.rb` | 15 | 0 | Deferred audit-log filtering and page fork re-diff/acceptance — WS8br (WS8a domain reused). |
| `test/controllers/accounts/custom_styles_controller_test.rb` | 3 | 0 | Subset: non-admin denied existing admin page. Defer full CSS/admin page fork re-diff/acceptance — WS8br; sudo gates WS9. |
| `test/controllers/accounts/logos_controller_test.rb` | 6 | 0 | Deferred logos fork re-diff/acceptance — WS8br. |
| `test/controllers/accounts/join_codes_controller_test.rb` | 2 | 0 | Deferred join-code fork re-diff/acceptance — WS8br; sudo gates WS9. |
| `test/controllers/workspace_icons_controller_test.rb` | 7 | 0 | Deferred workspace-icon fork re-diff/acceptance — WS8br. |
| `test/controllers/pwa_controller_test.rb` | 5 | 0 | Deferred PWA fork re-diff/acceptance — WS8br. |
| `test/controllers/qr_code_controller_test.rb` | 1 | 0 | Deferred QR fork re-diff/acceptance — WS8br. |
| `test/controllers/unfurl_links_controller_test.rb` | 9 | 0 | Deferred existing unfurl controller fork re-diff — WS8br; new embeds and their behavior remain WS15e, untouched here. |
| `test/system/audit_log_test.rb` | 2 | 0 | WS8br audit UI. |
| `test/system/channel_members_test.rb` | 4 | 0 | WS8br member UI; WS17 presence, WS11 bots. |
| `test/system/channel_navigation_test.rb` | 7 | 0 | WS8br room shell/navigation. |
| `test/system/first_run_tour_test.rb` | 4 | 0 | WS8br. |
| `test/system/icons_test.rb` | 4 | 0 | WS8br. |
| `test/system/keyboard_shortcuts_test.rb` | 14 | 0 | WS8br shell/switcher; WS8b-m composer/message shortcuts. |
| `test/system/member_select_mode_test.rb` | 18 | 0 | WS8br group/channel selection. |
| `test/system/mobile_layout_test.rb` | 5 | 0 | WS8br shell/sidebar; WS8b-m message/composer internals. |
| `test/system/motion_test.rb` | 9 | 0 | WS8br shell; WS6 shared templates/assets. |
| `test/system/people_group_dms_test.rb` | 19 | 0 | WS8br group settings/member flow; WS8b-m rendered system notes. |
| `test/system/quick_switcher_test.rb` | 5 | 0 | WS8br. |
| `test/system/room_header_test.rb` | 9 | 0 | WS8br; WS13 huddle/voice/stage integrations. |
| `test/system/service_worker_test.rb` | 2 | 0 | WS8br PWA; shared assets WS6. |
| `test/system/sidebar_organize_test.rb` | 6 | 0 | WS8br categories/favorites. |
| `test/system/sidebar_room_menu_test.rb` | 16 | 0 | WS8br. |
| `test/system/starred_people_test.rb` | 4 | 0 | WS8br starred picker/profile presentation. |
| `test/system/timezone_detection_test.rb` | 2 | 0 | WS8br. |
| `test/system/unread_divider_test.rb` | 5 | 0 | WS8br room shell; WS8b-m list insertion. |
| `test/system/unread_rooms_test.rb` | 2 | 0 | WS8br sidebar/header; WS17 notification facts. |
| `test/system/workspace_icons_test.rb` | 1 | 0 | WS8br. |
| `test/system/browser_launch_profile_test.rb` | 1 | 0 | Shared browser/UI coverage WS6, integrate with WS8br pages. |
| `test/system/content_security_policy_test.rb` | 5 | 0 | Shared security WS4, integrate with WS8br pages. |

**Precisely remaining, in the requested order**

1. Priority 1 remains partial: integrate WS8b-m's list/composer/template/thread/pin/poll output; WS17 OOO and WS13 configured venue/huddle/header facts; WS12 board pages. Finish owned nonmember open-room preview/join, full live room-page/sidebar byte and browser acceptance, query-count checks, every read/unread/visibility route/stream and applicable venue state. The full sidebar's requested layout/organization and tested recipient rows are delivered; configured features and browser behavior are not fully accepted.
2. Users/profiles/avatars/default initials SVG/cards/bans/time zones: re-diff and complete all fork controllers/pages and browser cases, including banned users and the shared first-name split; coordinate WS9 sudo/session and WS17 status/presence. None of this priority's fork acceptance was newly delivered here.
3. Account pages and re-diff: admin users, audit-log filtering/page, custom styles, logos, account/workspace icon libraries, join codes and every inherited controller. Coordinate WS9 and WS11. Existing upstream passes are not complete fork acceptance.
4. Public pages, welcome, first run/tour, inbound-email address browser acceptance; PWA/QR/existing unfurl re-diff. The pinned inbound controller's `/rooms/:id/edit` redirect reaches a missing generic Rails action; keep the observed response and report that browser-flow gap rather than changing Rails.
5. The inventory's unclaimed Rails cases: complete open/closed CRUD/forms/icons/audit/broadcasts, direct picker/starred/settings/invalid rename/overflow/no-op/removed-member behavior, refresh/read/member endpoints, leave failure rescue, exact error bodies/flash/formats/Turbo responses and durable destroy execution/sweep acceptance. Re-diff inherited directs#show against the pinned missing-@room behavior. Prove valid agent-token denial when WS9/WS11 auth lands. All 22 scoped browser/system files remain deferred.

Stop point: two pushed implementation slices plus their expanded state coverage and this report. Full WS8br scope is PARTIAL; no broad parity or cutover claim.
