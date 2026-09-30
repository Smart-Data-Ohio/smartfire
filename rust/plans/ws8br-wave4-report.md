WS8br wave 4 report — PARTIAL

Branch: `rust/ws8br-rooms-http`. Assigned base: `bb6c5d789f7504a198c9b6409a1730030fb8e215`. Rails oracle: `d7c7de92`. Implementation SHA: `43b56f726bdf7c12dae85e1c69787239e66ffa0e` (pushed). No PR opened. Work stayed in the assigned worktree; this external report is the requested exception.

This is a coherent room HTTP correctness slice, not completion of WS8br. It adds thirteen routed actions and eighteen regression tests. No template, browser capture, mask, pixel threshold, or parity allowlist was changed. Complete room-shell/sidebar HTML parity and the users/accounts/public/first-run re-diff remain unfinished.

Pushed implementation slices:

| SHA | Slice |
| --- | --- |
| `e71bfa10fd22572ace1b15f2cbfc013f736b11f0` | Room access scopes, group-delete authorization, durable deletion, first failing HTTP probes. |
| `86e90895d537701f17a96f42f53974d6776be1cc` | Per-user categories, favorites, involvement read clearing, inbound-address rotation. |
| `43b56f726bdf7c12dae85e1c69787239e66ffa0e` | Capped active direct-room selection, rename/add/leave writes, post-commit audit timing, reuse broadcast correction, final oracle and discrimination probes. |

**Changes by file**

All paths below are relative to `rust/`.

| File | Final change |
| --- | --- |
| `crates/campfire/src/concerns.rs` | RoomScoped endpoints reject soft-deleted rooms even if a stale membership remains. |
| `crates/campfire/src/controllers.rs` | Route category CRUD/index, category assignment, favorites CRUD, inbound-address creation, generic leave, direct update/add-members/leave; update route-wiring assertions. |
| `crates/campfire/src/controllers/rooms.rs` | Open/closed mutation scope excludes voice/stage/board; group deletion requires admin even through generic route; deletion uses WS8a begin_destroy; JSON success and HTML notice; shared leave helper; audits after domain commit. |
| `crates/campfire/src/controllers/room_categories.rs` | Own-user category index/create/update/destroy, append position, name/collapsed permitted parameters, Rails nonbang validation/redirect behavior. |
| `crates/campfire/src/controllers/rooms/categories.rs` | Own-user category assignment/unassignment on channel memberships; foreign categories 404, other room kinds 422. |
| `crates/campfire/src/controllers/rooms/favorites.rs` | Favorite/unfavorite/reorder the viewer's membership, idempotence, Ruby position coercion and model clamping. |
| `crates/campfire/src/controllers/rooms/inbound_email_addresses.rs` | Emailable-room and creator/admin checks, WS10 token rotation, exact Rails redirect and notice. |
| `crates/campfire/src/controllers/rooms/involvements.rs` | Require nonblank involvement, mark muted membership read, JSON head 200 rather than redirect. Existing visibility partials remain incomplete. |
| `crates/campfire/src/controllers/rooms/directs.rs` | Cap submitted IDs before querying, active-user filtering, reuse, capacity redirect, huddle query parameter, rename/add/leave model calls and Rails notices/alerts; only genuinely new rooms get creation audit/broadcast. |
| `crates/campfire/src/controllers/rooms/parity_tests.rs` | Eighteen seeded HTTP regressions with mandatory seed boot, database rows, actual rich-text note rendering, authorization, durable-queue and audit failure injection. |
| `crates/campfire/src/controllers/rooms/tests.rs` | Replace upstream synchronous-delete expectations with pending-deletion marker; missing involvement now 400 and leaves state unchanged. |
| `crates/campfire/src/channels/tests/hub_test.rs` | Existing broadcast test creates a genuinely new pair; reused DM explicitly produces no creation broadcast. Test fixture only; no cable production code changed. |
| `reference-tools/rooms/http.rb` | Actual HTTP requests against production-mode pinned Rails with its signed sessions and CSRF; forty-seven response/state cases; checks source hashes before probing. |
| `reference-tools/rooms/source-hashes.json` | SHA256 manifest of the pinned Rails controllers/models used by the oracle. |
| `reference-tools/rooms/discrimination.py` | Compile/run new tests with original bb6 controllers, then a deliberately incorrect same-transaction audit implementation; restore source in finally. |
| `vectors/rooms_http.json` | Generated only by the pinned Rails HTTP probe; regenerated and byte-compared on the final run. |

**Design and observed reference behavior**

No domain model or schema was extended. Controllers authorize and call WS8a/WS10: Room::begin_destroy, find_or_create_direct_for, rename_direct, add_direct_members, leave_direct, Membership category/favorite/read methods, RoomCategory validation and deletion, and inbound-token regeneration. Group system-note persistence/rich-text/index behavior remains in WS8a. Recipient-specific directory events and their complete HTML delivery remain a rendering follow-up, not a claim of this slice.

Deletion marking, membership removal and durable job insertion share the existing WS8a transaction. An injected background_jobs insert failure rolls back that entire transaction. Rails records the audit AFTER the room operation commits; the controller now does the same. An injected room.destroy audit failure returns 500 while the deleted marker and queued job stay committed. Earlier slice e71 coupled audit to deletion; the final slice corrects it and proves the wrong coupling fails.

Rails generic room lookup redirects inaccessible/deleted rooms to root with an alert; RoomScoped nested endpoints return 404. Preserve that distinction rather than claiming every soft-deleted route returns 404. Non-admin group creators cannot delete group history, including through the generic route. Direct pair members retain the existing direct-namespace deletion permission.

Direct selection applies the first-ten cap BEFORE coercion/query, adds current user after that cap, filters active users and then checks unique active membership count. A reused member set does not emit another creation audit or creation broadcast. The huddle change is only the redirect query parameter; no WS13 huddle implementation was changed.

The reference inbound rotation redirects to `/rooms/:id/edit`, whose Rails action is currently missing. This controller preserves that redirect; the browser/view flow remains unfinished. Category boolean false as a NAME becomes the string `false` in the actual pinned app; its vector/test records that observed cast.

The Rails Redis-outage destroy/sweep test is not a byte-equivalent durable-SQL-queue test: WS3/WS8a and decisions.md require atomic enqueue. The new insertion-failure regression verifies that specified durable-queue contract. Rails Redis/sweep infrastructure semantics remain an explicit cross-workstream acceptance question, not a silently claimed port.

**Verification, rerun commands and raw summaries**

Commands ran from `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br`. Both reference-built `default` and `first_run` seeds were present. The reference image is `ws8br-reference-d7c7de92` (tagged from the existing pinned reference image); relevant source hashes are checked by the generator. Rust toolchain is 1.98.1, jobs are capped at four, target and temporary files are local, and ports are confined to 52100–52199.

Final Rails oracle generation (exit 0; stdout was valid JSON and byte-identical to the committed vector):

```bash
PARITY_NAMESPACE=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/http.rb > .scratch/rooms_http.json 2> .scratch/rails-room-http-summary.log
```

```text
Rails room HTTP oracle: 47 cases; reference d7c7de92
```

Failing-first/discrimination rerun (script exit 0; its two compiled cargo runs intentionally fail):

```bash
python3 rust/reference-tools/rooms/discrimination.py > .scratch/discrimination-summary.log 2>&1
```

```text
test result: FAILED. 1 passed; 17 failed; 0 ignored; 0 measured; 305 filtered out; finished in 0.78s
WS8br discrimination: 17 HTTP regressions rejected bb6c5d78; 1 existing guard passed; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 322 filtered out; finished in 0.44s
WS8br audit discrimination: compiled transactional-audit regression rejected; source restored
```

The original-controller failures include group deletion authorization, conversion scopes, deleted membership access, own-user category/favorite access, inbound guards and group writes. The existing closed/direct access guard is the single passing test. This is assertion-level discrimination, not compiler failures, placeholder JSON or mocked network calls. The audit mutation is a second compiled, assertion-level failure.

Final room controller suite (exit 0):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire controllers::rooms:: -- --nocapture > .scratch/rooms-final.log 2>&1
```

```text
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 289 filtered out; finished in 1.59s
```

Final seeded app suite on this branch (exit 101):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/app-final.log 2>&1
```

```text
test result: FAILED. 320 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 25.39s
```

Only failure: `controllers::presenters::accounts::tests::manages_bots`, the existing WS11 stale bot-key assertion. Two explicit ignores: `channels::tests::golden::record_reference` (reference recorder) and `jobs::tests::push_latency` (measurement). No `skipping:` lines in this log; first-run setup actually ran. No blanket green-suite claim.

Final exact requested clippy command, including ALL workspace members and targets (exit 0):

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.37s
```

Fresh main comparison: the first scratch snapshot was `e5b2a29c0def437fd90fb2744861a7e0c03bf81e` (same Rust as assigned bb6 base). It has BOTH seeds, its own manifest/target placeholder, and uses this worktree as CAMPFIRE_REFERENCE. Re-run:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD" CABLE_TEST_PORT_RANGE=52150-52199 MAIL_TEST_PORT_RANGE=52150-52199 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/baseline/rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/baseline-tests.log 2>&1
```

```text
test result: FAILED. 302 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 24.34s
```

Its only failure was also manages_bots, with no silent skips. During verification origin/main advanced to `21a7332f2d3c324f0862cdf448baf17a84395aa0` (WS19b #160). A separate fresh archive of that exact commit, with both seeds and restored unmodified source, was built and tested:

```bash
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD" CABLE_TEST_PORT_RANGE=52150-52199 MAIL_TEST_PORT_RANGE=52150-52199 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path .scratch/main-21a7332f/rust/Cargo.toml -p campfire --bin campfire -- --test-threads=4 --nocapture > .scratch/current-main-tests.log 2>&1
```

```text
test result: ok. 306 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 24.50s
```

The third explicit ignore on latest main is manages_bots, with its WS11 reason. It also contains the presence-helper fix discussed below. No silent skips. Latest main was inspected and compared, not merged into this assigned branch; integration with WS19b is for the lead's merge direction.

One intermediate branch run additionally failed `workspace_presence_unsubscribing_deletes_only_this_connections_lease` at `support.rs`'s `expect("listed")`. The old helper reads lease IDs, then separately reads each row while the connection deletes a lease. To distinguish this from room code, a private e5 scratch copy was instrumented with a 25ms pause between those unchanged reads. The bounded reproduction script restores the scratch source in finally:

```bash
python3 .scratch/reproduce-presence-race.py > .scratch/presence-race-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 304 filtered out; finished in 0.16s
Baseline presence helper race reproduced at attempt 1 with 25ms between its unchanged two reads
```

This is an instrumented baseline reproduction, not an unmodified-main failure claim. The final branch run passed that test. WS19b's new main commit replaces the two-read helper; no production or helper change was made here. The scratch reproduction script/logs are local evidence, not tracked deliverables.

Full workspace tests, complete Rails controller/system suites, browser screenshots, accessibility/pixel checks and full error-body/Turbo/flash byte comparisons were NOT run. The Rails tool uses actual production-mode requests, not a claim that all Rails Minitest suites ran.

**Rails test inventory: ported subsets and precise deferred ownership**

Every path below is under `test/controllers/`. “Subset” means related HTTP/state assertions were ported into the eighteen Rust probes or corrected existing tests; it does not mean the whole file is complete. No unchanged upstream test passing is treated as proof of the Smartfire fork's complete behavior.

| Rails file | Delivered subset; deferred work and owner |
| --- | --- |
| `rooms_controller_test.rb` | Subset: pending deletion/job/JSON, generic group deletion guard, leave/JSON, retained room, access after deletion. Defer show/shell, nonmember open-room join preview and join, notices byte checks, job execution/sweep infrastructure and full audit/broadcast UI — WS8br, with WS8a/WS3 queue contract review. Link-preview/message-body assertions — WS8b-m/WS15e. |
| `rooms/opens_controller_test.rb` | Subset: cannot convert voice/stage/board. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `rooms/closeds_controller_test.rb` | Subset: cannot convert voice/stage/board and inaccessible private room guard. Defer full CRUD, icon/emoji inputs, invalid forms, membership/audit/broadcast and HTML assertions — WS8br. |
| `rooms/directs_controller_test.rb` | Subset: active capped creation/reuse/huddle redirect, >10 rejection, rename/add/leave note text, last leave, pair widening/nonmember guards, group deletion permissions. Defer starred/client-filter picker, settings/delete-button HTML, invalid rename 422 form state, add overflow/no-op/query-count assertions, removed-member read and exact stream/flash bytes — WS8br. Immutable/rendered system-note message assertions — WS8b-m. |
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
| `switchers_controller_test.rb` | Deferred entire fork re-diff/acceptance — WS8br. |
| `users_controller_test.rb` | Deferred entire fork re-diff/acceptance — WS8br. |
| `users/sidebars_controller_test.rb` | Deferred full fork sidebar categories/favorites/unread/group rows, collapse/order/cache keys and recipient-specific streams — WS8br. |
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

**Remaining and cross-workstream integration**

1. Complete room-shell/header/sidebar/switcher rendering, including recipient-specific WS8a broadcasts, cache keys, categories/favorites collapse/order, unread markers and group rows. Register/capture byte goldens and browser screenshots for every scoped page/state. No acceptance goldens for those pages were added here. Coordinate missing collapsed-category, group-of-ten, soft-deleted and banned-user parity seed states with WS19 tooling.
2. Complete room join and open-room nonmember preview, refresh/read/member endpoints, open/closed CRUD icons/emoji/audit/header callbacks and invalid forms. Direct new/edit picker and invalid rename HTML use upstream templates; complete fork markup and submitted-value/error state. The inherited directs#show redirect differs from the Rails missing-@room error and remains a re-diff decision.
3. Complete users/profiles/avatars/cards/bans/timezones/tour and admin/account styles/logos/icons/join-codes/audit-log/workspace-icons pages. Complete public/welcome/first-run/PWA/QR/existing-unfurl re-diff, routing and HTML/JSON/Turbo acceptance. Existing upstream coverage is not a substitute for these checks.
4. Complete inbound-address edit/browser flow on WS10. Preserve the observed redirect while deciding with the lead whether the missing reference edit route remains the frozen behavior.
5. Complete malformed JSON/scalar/array/hash/coercion and format/error body/flash/Turbo coverage. In particular leave's RecordNotDestroyed 422 rescue is not ported: WS8a exposes database/model errors, while huddle-dependent callbacks involve WS13. Do not claim its failure response matches yet.
6. Restore valid agent authentication with WS11/WS9, then show actual valid agent tokens denied on these human-only actions. New actions use Before::default, but valid-agent-token denial was not end-to-end proven on this base. Existing non-admin/nonmember/deleted/group guards were proven.
7. Merge current main only under the lead's integration direction, rerun seeded suite and exact workspace clippy, resolve any conflicts with other workers, then finish the deferred acceptance matrix. Latest WS19b already fixes the old presence test race and marks the known WS11 bot test explicitly ignored; this branch has not copied either fix.

No deployment or production parity claim. This report's commands were rerun; raw summaries above are from those runs, including failures and ignores.
