# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`; worktree `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8br`; Rails reference `d7c7de92`. Main `4278cb1e7a4529d5e2ecee51fccf69be6ed45257` is merged with merge commit `8bca72a1`. Six coherent slices are pushed through implementation/test SHA **`e2c63fa0b06afdab99b1baefbb913a4067ff04ac`**. The final report commit changes documentation only; the final reply gives its pushed SHA.

This continuation completes the DM browser flow and adds individual Rust execution mappings for the largest unblocked room controller files. Members, pins refresh and full native room-page acceptance remain partial because their owning branches are not merged here. No owner policy/partial was copied, no fake offline/status/pin result was added, and no mask, allowlist or ignore was added or loosened. Previous delivered forms, shell/sidebar/header, CRUD, reads, categories, favorites, involvement and inbound-address work remains in this branch and ran in the fresh-clone suite; previous receipts are in report history at `8d921b27`.

## Changes by coherent slice and file

- **`8bca72a1` — merge WS9.** Merged main with two parents, preserving this worker's additive room views/routes and WS9's authentication, enrollment, sudo, profile/security, two-factor, session and audit code. Resolved conflicts in `controllers.rs`, `presenters/test_support.rs`, `views/src/users.rs` and `users/summary.rs`. WS9's `before_actions` is used directly; `boot_frozen` wrappers now call the merged WS9 `boot_with_clock` helpers. The existing soft-delete room guard remains. Locked metadata and all Cargo manifest duplicate-key checks passed after the merge. Initial seeded app verification was `470 passed; 0 failed; 3 ignored`.
- **`1a01c1dd` — actual DM browser acceptance.** Added `reference-tools/rooms/dm_browser.{py,mjs}`; reused an extracted `run_browser` in `inbound_browser.py`. `browser.Dockerfile` uses only tracked parity process configuration on the pinned Rails image: two worker queues rather than the old image's extra production processes. No Ruby application source changed. Chromium runs the same picker/create/settings flow against separately seeded Rails and native servers. It checks case-insensitive filtering, Enter, selection surviving hidden candidates, clear, real submitted member IDs, group creation, valid rename, overlong 422 with attempted value, reload, group reuse, the huddle-start redirect and Escape. The persisted settings member section is checked explicitly. A wrong submitted candidate ID must fail that acceptance. These are behavior checks, not a full room-page pixel claim.
- **`8783791e` — directs case mappings.** Added `controllers/rooms/directs_rails_cases.rs` and `directs_cases_discrimination.py`: 27 separately executed ports of the 29 source declarations. Covers candidate/star ordering, create/reuse/group cap, huddle location, active users, rename/add/leave/privacy/delete controls and the real durable destroy job. Test-only `rusqlite` feature `limits` in `crates/campfire/Cargo.toml` lowers the actual writer's SQLite variable limit: an uncapped HTTP candidate query then fails, proving the first-ten cap occurs before SQL. No database/domain behavior changed. Two message-owned cases remain named below.
- **`4eec7ab9` — generic rooms case mappings.** Added `rooms_rails_cases.rs` and `rooms_cases_discrimination.py`: 22 separately executed ports of 29 declarations. Covers last-room redirect/cookie, membership and open preview, joining, ordinary/group-DM leaving, HTML/JSON deletion/flash, sweep claims and job completion. A real authenticated cable subscription checks the exact global remove frame and no extra frame. Seven cases remain named below, including the fixed atomic-queue decision conflict.
- **`7a9dac83` — opens case mappings.** Added `opens_rails_cases.rs` and `opens_cases_discrimination.py`: all 15 declarations. Covers real global prepend/row/header streams, creation/admin gates, icon normalization/clearing/422 rollback, permitted params, closed-to-open conversion and direct-room exclusion. Open conversion asserts Rails' actual `User.active` grant callback plus retained existing members; the larger parity seed has an already-associated banned person, so the controller fixture's assumption that every person is active cannot be copied literally.
- **`e2c63fa0` — closeds case mappings.** Added `closeds_rails_cases.rs` and `closeds_cases_discrimination.py`: all 12 declarations. Reused a test-only signed cable helper from `opens_rails_cases.rs`. Checks one real prepend per new member, row and icon-bearing header replacements for each remaining member, outsider silence (including Bender), creation/update gates, membership revisions, open-to-closed conversion, unknown-icon rollback before membership revision, direct conversion exclusion and self-removal followed by access loss. The source's show test intentionally calls the open namespace for a closed room, and the Rust test preserves that behavior.

`controllers/rooms.rs` registers the test modules only. `reference-tools/rooms/deferred_inventory.py` and `plans/ws8br-rails-cases.json` map individual source declarations to actual cargo pass receipts, separately from Rails baseline receipts. `plans/ws8br-owner-integration.md` documents the remaining adapter work and direct WS9 usage. Existing presenters, views and owner fragment hooks keep their fields/entry points; WS8br2's users/accounts/public/tour work is untouched.

## Exact shell inputs and owner seams

`controllers/rooms::render_show` currently selects and passes these facts into the existing `Presenter`/`ShowView`; it **does not yet invoke the unmerged WS8bm message-list factory**:

1. Reader connection, app state, request host, verified request origin `cache_base_url = c.url_for("")`, and `app.fragment_cache` around message presentation.
2. Root `Message` rows from `room_shell::find_messages(conn, room.id, message_id)`: 40 on the last page; around a same-room root anchor, up to 40 before + anchor + 40 after (81 total). Foreign, missing and thread anchors fall back to the last page.
3. Current membership `last_read_message_id`/`unread_at` yields `unread_divider_message_id: Option<i64>`, `unread_count: i64`, `scroll_to_unread_divider: bool` (in-page count above five) and an out-of-page `jump_to_unread_url`. These viewer facts must not enter shared message cache keys.
4. `ShowView.room`: ID, STI-derived kind, persisted name, viewer display name, resolved header identity and involvement. `ShowView.user`: ID, name, title and fresh signed avatar URL. Selected `MessageItem`s; room `updated_at`; invitation `Room::original == room && !Message::paged`; account join code; SHA1-keyed signed `[room_gid, "messages"]` stream name.
5. Request `ViewContext`: viewer/admin/bot/preferences, account, assets, verified URL/referrer/last-room, time zone, flash and chrome. Normal page rendering lends the request CSRF token and CSP nonce; broadcast contexts stay detached.

The inspected WS8bm entry point is `Presenter::room_message_list(&[Message], divider_id: Option<i64>, unread_count: i64) -> Result<String>`. The adapter should call it inside `fragment_cache::with(&app.fragment_cache, ...)` and assign the returned trusted bytes to `ShowView.shell.message_list = Some(...)`.

`ShellComponents` accepts `message_list`, `composer`, `message_template` as `Option<String>` (supplied empty output is distinct from missing output), plus trusted string `thread_panel`, `pins_panel`, `poll_builder`, `huddle_header`, `ooo_notices`. `campfire_views::rooms::room_message_list(ctx, show)` returns the supplied bytes verbatim or the authorized zero-byte Rails empty-room placeholder. Current HTTP creates `ShellComponents::default()`, so native full list/composer acceptance is unclaimed. The fallback composer receives `room = &show.room` and request `ViewContext`, with no separate `user` local. The client message template receives `user = &show.user`. A completed owner composer factory, with its request-bound tokens, still needs reconciliation at merge.

WS8bm2's inspected pins API is `controllers::rooms::pins::list(conn, app, room)` with its `CountPartial` and `ListPartial`. Refresh must wrap those bytes in replace streams for `pins_count_<room_param_key>_<id>` and `pins_list_<room_param_key>_<id>`, preserving STI keys. Current refresh selects new roots, updated roots excluding the new IDs, and returns 204 when both are empty and pins did not advance; pin-only refresh still has 200 with no count/list streams. No pin querying/order/excerpt/policy is owned here.

Members still resolve to the existing 501 route pending the actual WS17/WS11 adapter. It must call WS17 `UserStatusSettings::for_ids`, `WorkspacePresenceLease::presence_by_user_id`, `effective_presence` and `status_text_display`, plus WS11 `Agent::for_user`/`working_presence_text`. Then emit only Rails' member keys, live viewer stars and absolute fresh avatars with no-store/auth/room-access behavior. Do not substitute empty status or all-offline defaults. Exact adapter instructions and the inspected owner refs are in `ws8br-owner-integration.md`; those refs are inspection evidence, not merged implementations. A dependency-merge question was left pending; no permission was inferred from elapsed time.

## Rails files and native pass receipts

The 14 pinned Rails controller files ran independently with source SHA guards: **161 runs/passes, zero failures/errors/skips**. Those are reference executions, not Rust completion. The committed inventory lists all 58 relevant controller/system files, 512 declared source cases, exact names/lines/hashes, ownership and supplied run receipts.

| Rails file | Rails reference passes | Individually mapped Rust passes | Explicit deferred cases |
|---|---:|---:|---:|
| `rooms/directs_controller_test.rb` | 29 | 27 | 2 |
| `rooms_controller_test.rb` | 29 | 22 | 7 |
| `rooms/opens_controller_test.rb` | 15 | 15 | 0 |
| `rooms/closeds_controller_test.rb` | 12 | 12 | 0 |
| `rooms/inbound_email_addresses_controller_test.rb` | 8 | 8 | 0 |

Nine explicit named deferrals in those mapped files:

- Directs: “a member can rename the group and everyone sees the compact system note” (WS8bm list rendering; rename/domain already covered); “group DM notes cannot be edited or deleted” (WS8bm message authorization).
- Rooms: collapsed work-thread guidance (WS8bm); three link-preview render cases (WS8bm with WS15e); two full unread-divider/list cases (WS8bm integration, WS8br cursor/page facts already covered); “destroy succeeds when the queue is down and the sweep recovers the room” (lead decision 2 instead requires atomic enqueue rollback, already exercised through native HTTP fault injection).

Next unmapped room controller files, largest first: `users/sidebars` 14 (WS13 live huddle/participants and other owner facts); `rooms/members` 13 (WS17/WS11); `rooms/involvements` 8; `rooms/reads` 7; `rooms/favorites` 6; `room_categories` 6; `rooms/categories` 5; `switchers` 5; `rooms/refreshes` 4 (WS8bm/WS8bm2 integration). These are **68 source declarations in nine files**, still unclaimed as individual ports even where grouped HTTP coverage exists.

Also remaining: the 17 mixed room/account audit declarations (room subset WS8br, account subset WS8br2); existing unfurl-controller re-diff coordinated with WS15e; the full owned system-file matrix in the JSON. Largest system files are group DMs 19, member selection 18, sidebar room menu 16, keyboard 14, motion/header 9 each, navigation 7, sidebar organization 6, mobile/switcher/unread/CSP 5 each, channel members 4, unread rooms 2 and browser launch 1. The new real DM/inbound browser probes are not claimed as full Minitest system-file executions. Split users/profiles/avatars/cards/bans/time zones/accounts/public/welcome/first-run/tour/PWA/QR remain WS8br2. Message/composer/thread logic remains WS8bm, pins/polls WS8bm2, huddle WS13, status/OOO WS17, agents WS11, board/star policy WS12.

## Reproducible fresh-clone verification

Fresh clone `.scratch/fresh-continue7/repo` was created with `git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-continue7/repo`, with no copied target, node_modules, seeds or worker scratch fixtures. Its own default and first-run seeds were built using the tracked seed tooling from our pinned Rails image; Playwright dependencies/browser were installed there. The clone was fast-forwarded to the pushed closed-room slice before the final suite. CI=1 makes missing seeds fail. All test/build commands below use this clone's own target and TMPDIR, with test sockets in 52100–52149 and browser servers 52150/52151 (internal 52152). The final documentation-only commit can be fast-forwarded without changing the tested Rust/fixtures.

From the fresh-clone root:

```sh
PARITY_NAMESPACE=ws8br-fresh7-seed PARITY_OWNER=ws8br PARITY_IMAGE=ws8br-reference-d7c7de92 rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo build --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire
npm ci --prefix rust/parity
npm exec --prefix rust/parity -- playwright install chromium
CARGO_TARGET_DIR="$PWD/rust/target" python3 rust/reference-tools/rooms/dm_browser.py
CARGO_TARGET_DIR="$PWD/rust/target" python3 rust/reference-tools/rooms/dm_browser.py --inject-selection-drift
CARGO_TARGET_DIR="$PWD/rust/target" python3 rust/reference-tools/rooms/inbound_browser.py
PARITY_IMAGE=ws8br-reference-d7c7de92 python3 rust/reference-tools/rooms/check_controller_files.py
PARITY_IMAGE=ws8br-reference-d7c7de92 python3 rust/reference-tools/rooms/check_controller_files.py --inject-source-drift
```

Each command was run this continuation; metadata exited 0 with intentionally no stdout. npm ci reported no vulnerabilities, Chromium installation exited 0 using Playwright's Ubuntu fallback (host OS warning). Raw seed/check/browser/build/clippy summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
found 0 vulnerabilities
BEWARE: your OS is not officially supported by Playwright; downloading fallback build for ubuntu24.04-x64.
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.96s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.81s
DM browser acceptance: 2 targets passed; filter, Enter, hidden selection, clear, real create, rename, invalid name, reload, reuse, huddle redirect and Escape match
DM browser discrimination: wrong submitted member ID rejected by persisted settings acceptance
Inbound-email browser acceptance: 2 targets passed; create, confirm rotation, Rails 302-to-404 redirect, flash, reload and direct-room exclusion match
Rails controller source-pin injection: wrong hash rejected before tests
Fresh-clone workspace: 56 targets; 1634 passed; 0 failed; 11 ignored
```

One repeated DM browser run at `e2c63fa0` timed out on the **Rails reference** while waiting for the reused-group Start huddle POST, before native acceptance began. The same script passed unchanged on retry, including native acceptance and wrong-ID rejection. No timeout, assertion or app code was relaxed. This remains observed browser-harness intermittency, not a claimed fix; the complete system-file acceptance is still deferred. The failed execution is retained in `.scratch/fresh-continue7/dm-browser-final-first.log`:

```text
page.waitForResponse: Timeout 30000ms exceeded while waiting for event "response"
```

All 56 raw workspace target summaries (including zero-test doctest targets):

```text
test result: ok. 546 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 588.26s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.16s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.49s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 198.92s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.54s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.36s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.26s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 14.25s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.93s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.13s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.72s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 14.93s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.53s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All 11 ignores are pre-existing, listed explicitly below; no seed-dependent test silently skipped:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::presenters::accounts::tests::manages_bots ... ignored, WS11: resetting Bender's bot key leaves the original seeded key visible in the account bot list
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

The initial fresh-clone full run with four test threads hit the unchanged richtext hardening test's five-second wall-clock threshold (5.636s); an isolated retry also took 5.677s. No source, threshold or ignore changed. A separate freshly seeded clone detached at exactly main `4278cb1e` passed that same isolated test in 1.91s; the unchanged worker clone then passed in 1.91s. The final serial full suite above passed it as well. This is an observed timing failure/recovery, not a claim that main fails. Logs remain under `.scratch/fresh-continue7/{workspace-before-opens,hardening-retry,hardening-final-retry}.log` and `.scratch/fresh-main-4278/hardening.log`.

The isolated comparison command was run in both clones:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire_richtext --test hardening deeply_nested_content_attachments_render_quickly -- --exact --test-threads=1
```

Raw chronological timing receipts:

```text
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.82s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 5.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 1.91s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 1.91s
```

## Failing-first discrimination and per-file raw receipts

From the canonical worktree, each committed discrimination script compiled a deliberately incorrect real controller, exercised HTTP/SQLite/cable, required a test failure and restored its exact source in `finally`. Mutation checks were sequential; normal verification followed restoration. An accidentally overlapping normal check was stopped and rerun after restoration; it is not counted as a pass. A broad formatter invocation was reverted before final verification, leaving only this slice's intended test changes.

```sh
python3 rust/reference-tools/rooms/directs_cases_discrimination.py
python3 rust/reference-tools/rooms/rooms_cases_discrimination.py
python3 rust/reference-tools/rooms/opens_cases_discrimination.py
python3 rust/reference-tools/rooms/closeds_cases_discrimination.py
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire rails_cases -- --test-threads=1
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/ws9-rails-case-ports-final.log --rails-log .scratch/ws9-rails-controller-files.log
```

The mutations relaxed group/room creation and update/private-room authorization, removed the first-ten cap before a real SQL query, and accepted unknown icons. All eight were rejected by the named native tests. The browser ID mutation and wrong Rails source hash were also rejected. Raw discrimination summaries:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 499 filtered out; finished in 0.54s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 499 filtered out; finished in 0.50s
Direct Rails case discrimination: group authorization and uncapped real SQL rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 521 filtered out; finished in 0.72s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 521 filtered out; finished in 0.53s
Room Rails case discrimination: administrator and private-room membership mutants rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 536 filtered out; finished in 1.88s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 536 filtered out; finished in 2.11s
Open-room Rails case discrimination: creation authorization and unknown-icon validation mutants rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.40s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 548 filtered out; finished in 0.41s
Closed-room Rails case discrimination: creation and update authorization mutants rejected; source restored
```

Raw normal cargo/per-file mapping receipts:

```text
test result: ok. 84 passed; 0 failed; 0 ignored; 0 measured; 465 filtered out; finished in 19.94s
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/directs_controller_test.rb: 27 Rust cases passed, 2 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms_controller_test.rb: 22 Rust cases passed, 7 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/opens_controller_test.rb: 15 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails case port receipts: test/controllers/rooms/closeds_controller_test.rb: 12 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

Raw pinned Rails file receipts (again, reference counts only):

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

Final raw logs are under `.scratch/fresh-continue7/`: `seed.log`, `workspace-keys-final.log`, `workspace-tests-final.log`, `clippy-final.log`, `build-final.log`, `npm-ci-final.log`, `chromium-final.log`, `dm-browser-final.log`, `dm-browser-discrimination-final.log`, `inbound-browser-final.log`, `rails-controller-files-final.log`, `rails-source-injection-final.log`. Canonical discrimination/normal/inventory logs use the filenames cited above. Log files are evidence, never test fixture dependencies; the JSON pass map is committed. Only the inactive canonical `rust/target` cache was cleaned afterward to free space; sources, seeds, logs and the fresh-clone target were preserved.

## Precisely remaining, requested order

1. **Members JSON:** merge actual WS17/WS11 APIs, implement the native route and verify all 13 individual Rails cases, ordering/presence/status/agent precedence/stars/absolute avatars/no-store/authorization. Currently 501; no fabricated facts.
2. **Pins refresh:** call WS8bm2's list/count partial seams; assert populated and pin-only refresh streams with exact STI targets. Pin logic and partial internals stay with WS8bm2.
3. **DM completion:** the real browser workflow and 27 owned direct-controller cases are delivered; the two message-owned note-render/edit/delete cases and the complete 19-case group-DM/18-case member-selection system matrices remain. Additional overflow/no-op/format/fault permutations remain unclaimed as exhaustive acceptance.
4. **Full native room page:** wire the actual WS8bm list and completed composer/template factories using the exact inputs above, mount the WS8bm2/WS13/WS17 owner fragments, then run the full seeded HTTP/layout/list/composer and browser/pixel matrix for each viewer/unread/around state. Existing lent-owner-fragment shell goldens prove only the surrounding shell.
5. **Remaining mappings:** 68 declarations across the nine controller files listed largest-first, nine named deferrals, mixed audit and the listed system matrix. Users/accounts/public/tour/PWA/QR remain transferred to WS8br2; existing unfurl re-diff needs WS15e coordination.

No PR or deployment. Work stops at six coherent pushed slices with owner integration explicitly partial.
