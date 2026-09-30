WS8br wave 4 report — PARTIAL, continued after WS19b

Branch: `rust/ws8br-rooms-http`. Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br`. Assigned base: `bb6c5d78`. Rails oracle: `d7c7de92`. Latest implementation SHA: `c11a0ac6b2a040fd416e2f062886b96464226af2`, pushed and synchronized with origin. No PR. This requested external report is the only worktree-location exception; the identical tracked mirror is `rust/plans/ws8br-wave4-report.md`.

The ordered work is still PARTIAL at priority 1. The switcher, shared/direct row partials, recipient header identity and group directory delivery are implemented and verified. The complete room shell, full header actions, sidebar section layout and message-list integration are unfinished. Prior room HTTP slices remain included. Priorities 2–5 have not been completed. No production, full-page, browser, pixel or complete Rails-suite parity claim.

**Pushed slices**

| SHA | Delivered slice |
| --- | --- |
| `e71bfa10` | Room access/group-delete authorization and durable begin_destroy. |
| `86e90895` | Own-user categories, favorites, involvement read clearing and inbound-token rotation. |
| `43b56f72` | Active capped group selection, rename/add/leave, post-commit audit timing and reuse behavior. |
| `26f598b9` | First partial report. Its old main/bot-failure status is superseded here. |
| `9f356e2fa00ff70ffdca3bd4aaf76916ce77557e` | Explicit merge commit of origin/main `21a7332f2d3c324f0862cdf448baf17a84395aa0` (WS19b). No conflicts. |
| `fd26f86980b61ce6107f4413c4f6c4a536d03c5b` | Scoped quick-switcher JSON and pinned Rails oracle. |
| `00c401dc`, `5f469aee10d5c3e9ff036b1748ed9e6db64437c3` | Rails sidebar rows and cache keys; immediate follow-up boxes the enlarged row enum to fix clippy. The first row commit was pushed before that clippy issue was fixed; the branch now passes. |
| `c11a0ac6b2a040fd416e2f062886b96464226af2` | Recipient header identity, model directory partial delivery, actual socket bytes, seeded group ordering and Ruby word splitting. |

**What changed, by file (paths relative to rust/)**

| Files | Behavior and evidence |
| --- | --- |
| `crates/campfire/src/concerns.rs` | Pending-deleted RoomScoped membership endpoints reject access. |
| `crates/campfire/src/controllers.rs` | Routes prior room/category/favorite/inbound/group actions and `switchers#show`; routing assertions include them. |
| `controllers/rooms.rs`, `rooms/{opens,closeds,directs,involvements}.rs` under `crates/campfire/src/` | Channel conversion scopes, group-delete guards, transactional durable destruction, JSON/redirect behavior, shared leave, active capped direct selection, rename/add/leave and audit after domain commit. Shared creation rows now render with ViewContext. |
| `controllers/room_categories.rs`, `rooms/{categories,favorites,inbound_email_addresses}.rs` | Own-user category CRUD/assignment, favorite ordering/idempotence and WS10 token rotation with Rails guards. |
| `controllers/rooms/{parity_tests,tests}.rs` | Eighteen discriminating room regressions and corrected pending-delete/required-parameter expectations. |
| `controllers/switchers.rs`, `controllers/presenters/switcher.rs` | Thin authorized JSON action; five scoped SQL queries; ordered typed JSON fields; current room visibility, people, existing pair URLs and latest fifteen threads. Full peer names in switcher group rows deliberately follow this controller's Ruby rather than the custom/sidebar group label. |
| `controllers/presenters/{accounts,rooms_directory}.rs`, `controllers/presenters.rs` | Row menu facts come from the membership's own viewer. Resolve room icons before rendering. Avatar order follows Membership order; label order follows Ruby naming. Header identities call WS8a's existing direct_display_name, including its SQL LOWER ordering. RoomView carries the header DTO. |
| `controllers/users/sidebars{,_tests}.rs` | Real HTTP role-cache invalidation, complete seeded-group row bytes and four Rails word-splitting cases. Full sidebar collection partition still needs porting. |
| `controllers/presenters/test_support.rs` | Frozen boot clock for byte goldens; existing ticking seed clock remains the normal test default. |
| `crates/views/src/users{,/sidebar,/summary}.rs`, `templates/users/sidebars/rooms/_{shared,direct}.html` | Full shared/direct row markup, icons, separate avatar card buttons, muted/unread/menu/favorite/category facts, solo/pair/group labels and first-three avatar preview. Cached direct page rows partition membership version, administrator role and participant IDs. Single-row broadcasts bypass the page collection cache, as Rails does. Huddle HTML has an explicit WS13 seam. |
| `crates/views/src/rooms{,/header}.rs`, `templates/rooms/show/_{header_identity,nav}.html` | Exact recipient header identity and real replacement DOM target in the room page. The rest of nav remains inherited and unfinished. |
| `crates/campfire/src/channels{,/sink,/rooms_directory}.rs` | Register only DirectSidebar and RoomHeader model partials; read committed data and render without actor/request/session state, then publish through WS7's guarded Turbo API. Unowned message/poll/pin partials remain at WS8b-m's seam. Template-free frames retain their existing path. |
| `crates/campfire/src/channels/tests/{hub_test,directory_test}.rs` | Actual HTTP writes and real sockets compare 31 complete Rails recipient frames; headers also occur verbatim on each member's room page. Check unrelated recipient silence, outsider/bot guards, newcomer prepend and leaver removal before disconnect. |
| `crates/views/tests/{sidebar,room_header}.rs`, `tests/support/context.rs`, `tests/golden/{sidebar/rows,rooms/directory}.json` | Fifteen complete row goldens and fourteen header goldens, with no HTML normalization. Administrator/participant cache mutations and nonce leaks are rejected. |
| `reference-tools/rooms/{http,switcher,sidebar,directory}.rb`, `source-hashes.json`, `vectors/{rooms_http,switcher}.json` | Actual pinned Rails responses/partials/callbacks. Relevant source hashes guard drift. Goldens come from Rails only; final outputs byte-match tracked files. |
| `reference-tools/rooms/{discrimination,sidebar_discrimination,directory_discrimination,switcher_discrimination,check_workspace}.py` | Compiled assertion-level wrong implementations with source restored in finally; duplicate TOML key validation. The original-controller probe has a tiny render-context API shim so old controllers compile against the new row DTO; original authorization/mutation logic is unchanged. |

**Design and boundaries**

No model or schema changes. Controllers reuse WS8a/WS10. Durable deletion is atomic with enqueue; audit happens after that commit, exactly as the actual Rails HTTP failure probe observes. Redis/sweep equivalence remains a separate WS3/WS8a acceptance question.

The reference generic/direct inaccessible-room guard redirects to root with an alert (302); RoomScoped nested endpoints use 404. The new directory oracle confirms anonymous 302, valid bot-key 403 and signed-in outsider 302 to `http://campfire.test/`. Do not replace these with a blanket 404 claim. The sidebar pair-delete menu intentionally follows Rails' creator/admin helper even where its pair-delete endpoint allows any member.

The seeded four-person group exposed different user-join versus Membership association orders. The rendering adapter preserves Rails Membership order for avatars, using membership id order; direct labels sort separately. The WS8a descriptor's member_ids select members, while the renderer orders them through the fresh Membership association. WS8a's descriptor-level ordering remains an owner follow-up if it is intended as a fully ordered public contract; no domain implementation was edited. Ruby word splitting includes vertical tab and excludes NBSP; the new labels match the four Rails vectors.

Only group directory frames are claimed: rename, clear name, leave and add, including new-member prepend and removal-before-disconnect. Rendered system-note message append delivery remains WS8b-m. Header identity is byte-tested for channel, voice, stage, board, icon, pair, solo, group and ten-person group cases; full WS13/WS12 pages and header controls are not claimed.

Huddle rows are verified against the unconfigured reference. Cache separation accepts owner-supplied trusted participant HTML/IDs, but production configured-huddle participant loading/rendering remains WS13 integration work. No message/list/composer/thread/pin/poll/search template internals, frontend assets, masks, allowlists or pixel thresholds were edited.

**Verification: final reruns and raw summary lines**

Commands ran from the worktree root, except metadata from its `rust/` directory. Rust 1.98.1, four jobs, local target/temp directories, WS8br Docker prefix and ports 52100–52199. Both default and first_run reference seeds are present. `CI=1` makes missing real app seeds fail; seeded app tests had zero failures. The synthetic `missing_seed_may_skip_locally` unit test intentionally prints a local-skip note for an empty temporary test directory; it is not a skipped seeded app test.

After the merge, locked metadata was rerun from `rust/` (exit 0, intentionally no output):

```bash
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > /dev/null
```

All fifteen Cargo manifests parse without duplicate keys, including workspace dependency keys. No Cargo.lock or Cargo.toml changes.

```bash
python3 rust/reference-tools/rooms/check_workspace.py > .scratch/metadata-duplicates.log
```

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
```

Actual Rails oracle reruns (all exit 0; each output was validated and byte-compared with the tracked JSON):

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/http.rb > .scratch/rooms_http.json 2> .scratch/rails-room-http-summary.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/switcher.rb > .scratch/switcher.json 2> .scratch/switcher-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/sidebar.rb > .scratch/sidebar.json 2> .scratch/sidebar-oracle.log
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/directory.rb > .scratch/directory.json 2> .scratch/directory-oracle.log
```

```text
Rails room HTTP oracle: 47 cases; reference d7c7de92
Rails switcher oracle: 2 byte payloads, 2 auth responses; reference d7c7de92
Rails sidebar rows: 15 byte goldens; reference d7c7de92
Rails room directory: 14 header goldens, 31 recipient frames; reference d7c7de92
```

Failing-first/discrimination reruns (scripts exit 0; nested compiled cargo tests intentionally fail assertions, not compilation):

```bash
python3 rust/reference-tools/rooms/discrimination.py > .scratch/discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/switcher_discrimination.py > .scratch/switcher-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/sidebar_discrimination.py > .scratch/sidebar-discrimination-summary.log 2>&1
python3 rust/reference-tools/rooms/directory_discrimination.py > .scratch/directory-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 1 passed; 17 failed; 0 ignored; 0 measured; 314 filtered out; finished in 0.72s
WS8br discrimination: 17 HTTP regressions rejected bb6c5d78; 1 existing guard passed; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.39s
WS8br audit discrimination: compiled transactional-audit regression rejected; source restored
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 332 filtered out; finished in 0.39s
Switcher discrimination: compiled unmapped action rejected by all four regressions; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
Sidebar discrimination: compiled cache-key mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
Sidebar discrimination: compiled request-nonce mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.41s
Directory discrimination: compiled unscoped-access mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 1.72s
Directory discrimination: compiled actor-header mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.40s
Directory discrimination: compiled user-join-order mutation rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 335 filtered out; finished in 0.00s
Directory discrimination: compiled unicode-whitespace mutation rejected; source restored
```

Fourteen row byte comparisons rejected the inherited templates before porting. The header seam initially failed all fourteen goldens; the final full-byte comparisons and compiled actor-header mutation provide the header parity evidence. The actual directory socket regression compiled and failed because its partial handler was missing, while the existing guards passed. A new seeded-group HTTP assertion failed against the user-join adapter; Ruby word-splitting failed against the inherited first-name implementation. The reproducible mutations above re-prove the security/cache/name/order failures on the final implementation. The inherited unmapped switcher fails all four final tests, including the valid bot-key guard. No compiler error is treated as failing-first evidence.

Final seeded app suite (exit 0):

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```

```text
test result: ok. 333 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 28.51s
```

Three explicit ignores: `channels::tests::golden::record_reference` (reference recorder), `jobs::tests::push_latency` (measurement), and `controllers::presenters::accounts::tests::manages_bots` (WS11, inherited from the requested WS19b merge). No new ignore or acceptance relaxation. The old one-failure bot status in the first report is superseded.

Final views suite (exit 0; 36 unit, 28 shared-core, one header and four sidebar tests, plus zero doctests):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire_views > .scratch/views-final.log 2>&1
```

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Exact workspace all-target clippy, including html5ever (exit 0):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.83s
```

Full workspace tests, complete Rails Minitest suites, browser/screenshots, accessibility/pixel comparisons and full response error-body/flash/Turbo acceptance were not run. These commands were rerun; the lines above are copied from their final logs.

**Rails test inventory: ported subsets and precise deferred ownership**

Every path below is under `test/controllers/`. “Subset” means related HTTP/state assertions were ported into the room HTTP probes and new switcher/sidebar/directory regressions or corrected existing tests; it does not mean the whole file is complete. No unchanged upstream test passing is treated as proof of the Smartfire fork's complete behavior.

| Rails file | Delivered subset; deferred work and owner |
| --- | --- |
| `rooms_controller_test.rb` | Subset: pending deletion/job/JSON, generic group deletion guard, leave/JSON, retained room, access after deletion. Header identity and its per-recipient target are now byte-tested; defer the rest of show/shell, nonmember open-room join preview and join, notices byte checks, job execution/sweep infrastructure and full audit/broadcast UI — WS8br, with WS8a/WS3 queue contract review. Link-preview/message-body assertions — WS8b-m/WS15e. |
| `rooms/opens_controller_test.rb` | Subset: cannot convert voice/stage/board. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `rooms/closeds_controller_test.rb` | Subset: cannot convert voice/stage/board and inaccessible private room guard. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `rooms/directs_controller_test.rb` | Subset: active capped creation/reuse/huddle redirect, >10 rejection, rename/add/leave note text, last leave, pair widening/nonmember guards, group deletion permissions. Directory rename/clear/leave/add frames are now byte-tested; defer starred/client-filter picker, settings/delete-button HTML, invalid rename 422 form state, add overflow/no-op/query-count assertions, removed-member read and exact stream/flash bytes — WS8br. Immutable/rendered system-note message assertions — WS8b-m. |
| `rooms/involvements_controller_test.rb` | Subset: required parameter, muted read state and JSON success. Defer exact visibility streams/header/sidebar HTML and full enum/format cases — WS8br; voice/stage rendering — WS13; boards — WS12. |
| `rooms/refreshes_controller_test.rb` | Deferred entire fork re-diff, including empty 204 and response JSON — WS8br; pins/message facts — WS8b-m. |
| `rooms/reads_controller_test.rb` | Deferred unread/read endpoints and broadcasts — WS8br, coordinating notification/presence facts with WS17. |
| `rooms/members_controller_test.rb` | Deferred fork members JSON/UI — WS8br; presence facts WS17, bot/agent facts WS11. |
| `rooms/categories_controller_test.rb` | Subset: assignment/unassignment, own category, foreign category/membership access, non-channel 422. Defer exhaustive malformed params/formats and byte acceptance — WS8br. |
| `rooms/favorites_controller_test.rb` | Subset: append/idempotence/remove/reorder/clamp/private membership. Defer exhaustive malformed params/formats and sidebar browser/byte acceptance — WS8br. |
| `rooms/inbound_email_addresses_controller_test.rb` | Subset: token rotation, emailable and creator/admin/member guards, exact redirect. Defer missing relay-domain UI, edit form, browser flow and full notice bytes — WS8br on WS10 domain. |
| `room_categories_controller_test.rb` | Subset: scoped ordered index, create append, update collapse/rename, invalid name no-op, destroy/unassign, foreign ownership; extra actual-app boolean-name cast. Defer broader type-coercion matrix and sidebar HTML — WS8br. |
| `public_pages_controller_test.rb` | Deferred entire fork re-diff and golden acceptance — WS8br. |
| `first_runs_controller_test.rb` | Deferred entire fork re-diff/acceptance — WS8br; first_run seed did run upstream integration coverage. |
| `welcome_controller_test.rb` | Deferred entire fork re-diff/acceptance — WS8br. |
| `switchers_controller_test.rb` | Re-diffed the fork controller. Subset: signed-in/bot guards, room/people/thread scopes, kinds, flags, latest-15 limit, group labels and two complete JSON byte goldens. Defer browser quick-switcher acceptance and the exhaustive input/format matrix — WS8br. |
| `users_controller_test.rb` | Deferred entire fork re-diff/acceptance — WS8br. |
| `users/sidebars_controller_test.rb` | Subset: 15 full row goldens, own-recipient menu permissions, administrator/participant cache separation, HTTP seeded-group association order, Ruby whitespace labels and 31 directory frames. Defer full page/section bytes, category collapse/order, favorite partition, all involvement/read streams and constant page query-count tests — WS8br; configured huddle stacks — WS13. |
| `users/tours_controller_test.rb` | Deferred first-run tour endpoints and browser acceptance — WS8br. |
| `users/profiles_controller_test.rb` | Deferred profile fork re-diff/acceptance — WS8br; sudo/session gates coordinate with WS9. |
| `users/avatars_controller_test.rb` | Deferred upload/initials SVG/default avatar fork re-diff and byte acceptance — WS8br. |
| `users/cards_controller_test.rb` | Deferred user cards fork re-diff/acceptance — WS8br; presence/status facts WS17. |
| `users/bans_controller_test.rb` | Deferred ban fork re-diff/acceptance — WS8br. |
| `users/time_zones_controller_test.rb` | Deferred timezone fork re-diff/acceptance — WS8br. |
| `accounts_controller_test.rb` | Deferred account fork re-diff/acceptance — WS8br; auth/sudo gates WS9. |
| `accounts/users_controller_test.rb` | Deferred admin user list/edit fork re-diff/acceptance — WS8br; bots/agents WS11. |
| `accounts/icons_controller_test.rb` | Deferred icon library/admin UI fork re-diff/acceptance — WS8br. |
| `accounts/audit_logs_controller_test.rb` | Deferred audit-log filtering and page fork re-diff/acceptance — WS8br (WS8a domain reused). |
| `accounts/custom_styles_controller_test.rb` | Subset: non-admin denied existing admin page. Defer full CSS/admin page fork re-diff/acceptance — WS8br; sudo gates WS9. |
| `accounts/logos_controller_test.rb` | Deferred logos fork re-diff/acceptance — WS8br. |
| `accounts/join_codes_controller_test.rb` | Deferred join-code fork re-diff/acceptance — WS8br; sudo gates WS9. |
| `workspace_icons_controller_test.rb` | Deferred workspace-icon fork re-diff/acceptance — WS8br. |
| `pwa_controller_test.rb` | Deferred PWA fork re-diff/acceptance — WS8br. |
| `qr_code_controller_test.rb` | Deferred QR fork re-diff/acceptance — WS8br. |
| `unfurl_links_controller_test.rb` | Deferred existing unfurl controller fork re-diff — WS8br; new embeds and their behavior remain WS15e, untouched here. |

Scoped system files below are ALL deferred; no system file was ported or run by this worker. Paths are under `test/system/`:

| File | Owner / boundary |
| --- | --- |
| `audit_log_test.rb` | WS8br audit UI. |
| `channel_members_test.rb` | WS8br member UI; WS17 presence, WS11 bots. |
| `channel_navigation_test.rb` | WS8br room shell/navigation. |
| `first_run_tour_test.rb` | WS8br. |
| `icons_test.rb` | WS8br. |
| `keyboard_shortcuts_test.rb` | WS8br shell/switcher; WS8b-m composer/message shortcuts. |
| `member_select_mode_test.rb` | WS8br group/channel selection. |
| `mobile_layout_test.rb` | WS8br shell/sidebar; WS8b-m message/composer internals. |
| `motion_test.rb` | WS8br shell; WS6 shared templates/assets. |
| `people_group_dms_test.rb` | WS8br group settings/member flow; WS8b-m rendered system notes. |
| `quick_switcher_test.rb` | WS8br. |
| `room_header_test.rb` | WS8br; WS13 huddle/voice/stage integrations. |
| `service_worker_test.rb` | WS8br PWA; shared assets WS6. |
| `sidebar_organize_test.rb` | WS8br categories/favorites. |
| `sidebar_room_menu_test.rb` | WS8br. |
| `starred_people_test.rb` | WS8br starred picker/profile presentation. |
| `timezone_detection_test.rb` | WS8br. |
| `unread_divider_test.rb` | WS8br room shell; WS8b-m list insertion. |
| `unread_rooms_test.rb` | WS8br sidebar/header; WS17 notification facts. |
| `workspace_icons_test.rb` | WS8br. |
| `browser_launch_profile_test.rb` | Shared browser/UI coverage WS6, integrate with WS8br pages. |
| `content_security_policy_test.rb` | Shared security WS4, integrate with WS8br pages. |

Message/composer/thread/pin/poll/search system files remain WS8b-m; huddle/voice/stage remain WS13; boards WS12; events WS14; status/DND/notification settings remain WS17; auth/sudo/2FA/sessions WS9; bots/agents WS11; embed-specific system files WS15e/WS15g. This slice changes none of their internals.

**Precise remaining work, in the requested order**

1. Finish priority 1 before users/account/public work: the room-workspace body/head/preloads, complete nav/actions/overflow/bell, member panel, unread divider/jump/scroll state, invitation and safe nonmember open-room preview/join. Wire WS8b-m's actual message-list/composer/thread/pin/poll rendering through its API without editing internals. That message-list renderer is not yet present on this branch, and the shell still uses inherited list/composer integration. Finish full sidebar workspace navigation/profile tools/room menu, favorite section mixing every room kind, categorized/uncategorized channel partition, collapse/order/empty/category forms, placeholder DMs and all visibility/involvement/unread/read states. Shared visibility/read broadcasts still need the membership/unread locals. Constant page query-count parity is not claimed: the current sidebar adapter still performs per-room reads. Coordinate actual voice/stage/huddle/board row renderers and configured participant/cache facts with WS13/WS12. Add full-page/state goldens and browser acceptance; current row/header bytes and directory frames are only subsets.
2. Re-diff and finish users, profiles, avatar upload/default initials SVG, cards, bans and time zones. Port the controller/browser cases in the inventory, including banned states and sudo/session gates with WS9. Presence/status facts remain WS17.
3. Re-diff every inherited account controller and finish the admin user list, audit-log filtering/page, custom styles, logos, icon library/workspace icons and join codes. Coordinate sudo with WS9 and bot/agent facts with WS11. Existing upstream app tests are not full fork acceptance.
4. Finish public pages, welcome, first run, first-run tour, inbound-address browser acceptance and existing PWA/QR/unfurl re-diff. The pinned inbound controller redirects to `/rooms/:id/edit` although its generic Rails edit action is missing; preserve the observed response and raise that reference browser-flow gap with the lead rather than silently fixing Rails.
5. Finish deferred Rails cases and the input/format/coercion/error-body/flash/Turbo matrix. Open/closed CRUD icon/emoji/invalid forms/audit/callbacks, direct picker/starred/invalid rename/overflow/no-op/member removal/query counts, refresh/read/member endpoints, leave RecordNotDestroyed rescue and durable job execution/sweep acceptance remain. The inherited directs#show redirect still differs from the pinned Rails missing-@room error and needs explicit re-diff under the frozen-behavior rule. Prove valid agent-token denial after WS11/WS9 authentication is integrated; Before::default alone is not end-to-end proof. Run every deferred browser/system file and complete the full acceptance matrix.

No deploy or cutover claim. Stop point: coherent pushed recipient header/directory slice, with the full scoped work explicitly partial.
