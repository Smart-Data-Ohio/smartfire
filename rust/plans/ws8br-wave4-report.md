# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`, assigned worktree `rust-ws8br`, Rails pin `d7c7de92`. Main `21a7332f` remains merged through `9f356e2f`; no additional merge was requested. Four coherent slices pushed through **`c5f445f7acdba40e9de4f905531d508bb62818d6`**. This report is a follow-up containing documentation, the inventory receipt wording and the 81-row merge-input clarification; it changes no Rust implementation. Earlier receipts remain in report history at `1841e075`.

## Delivered this continuation

- **`e6a4730e` — direct picker/settings.** `controllers/rooms/directs.rs` gathers active candidates, stable starred-first picker ordering, live viewer stars, agent association existence and the existing shared avatar presenter. It renders group name/add-member/leave/admin-delete controls; one-to-one and solo settings retain their proper controls. On invalid rename it supplies the normalized attempted name and record error attributes to the 422 page. `views/src/rooms.rs` adds defaulted `DirectEditView` fields and a `DirectPickerUser` input; `DirectsNew` now takes the people collection. No existing room/header/sidebar/ShowView field is removed or renamed. `views/templates/rooms/directs/{new,edit}.html` match 11 entire Rails body goldens for picker/starred picker, pair/member, group/admin/member, invalid/blank/escaped name, solo and all-members/no-candidates states.
- **`758b6d3d` — rename/access failure acceptance.** `controllers/rooms/direct_rename_tests.rs` compares 11 actual pinned Rails HTTP requests: nil, booleans, integer/float, escaped/trimmed strings, Unicode whitespace, array/hash values and 100/101 Unicode characters. It asserts status, redirect, next-request flash, persisted name, quiet notes, message/audit deltas and invalid field value/wrapper. An enqueue-rejecting trigger checks invalid rename without a queue-draining race. Named groups that shrink to a pair retain group controls and reject plain-member deletion; removed members and wrong room namespaces are rejected before nested name processing. The existing flash decoder in `direct_selection_tests.rs` is exposed only to its sibling test. Previously added tests were formatted. No domain validation or write algorithm changed in this slice.
- **`8abfe522` — reference file counts and owner integration instructions.** `reference-tools/rooms/check_controller_files.py` extracts the pin's tracked test archive into its own scratch, verifies every tested controller's source hash in a private reference container, runs 14 files separately with failure propagation, and checks all per-file zero-failure/zero-error/zero-skip summaries. Its wrong-hash injection fails before tests. `plans/ws8br-owner-integration.md` records exact existing APIs and missing native integration; no owner partial/policy was copied or changed.
- **`c5f445f7` — inventory receipts.** `reference-tools/rooms/deferred_inventory.py --rails-log` records actual Rails reference run/pass/assertion counts separately from Rust ports. `plans/ws8br-rails-cases.json` lists all 58 files and 512 declared case names/lines/hashes/owners. Eight inbound cases have individual current Rust pass receipts. Other exhaustive mappings remain deferred even where grouped Rust coverage exists.

New fixtures/tools: `vectors/{direct_forms,direct_rename}.json`, `reference-tools/rooms/{direct_forms,direct_rename}.rb`, both discrimination scripts, `views/tests/direct_forms.rs`, and `controllers/rooms/{direct_forms_tests,direct_rename_tests}.rs`. New Rust checks are six grouped app tests and one complete-byte view test; grouped checks are not a claim that 29 direct-controller declarations are individually ported.

## Owner inputs and remaining integration

The controller currently creates `ShowView` with default owner fragments. It does **not** call the missing WS8bm `Presenter::room_message_list` yet. Native message/composer/full-room parity is still partial. Exactly what is available/passed at the seam:

1. Reader connection, app, request host, verified request origin in `Presenter.cache_base_url`, and the app fragment-cache scope.
2. Root `Message` records from `room_shell::find_messages(conn, room.id, message_id)`: up to 40 last-page rows; a same-room root anchor selects up to 40 before + anchor + 40 after (81). Missing, foreign and thread anchors fall back to the last page. `Presenter::messages` produces the existing `MessageItem` collection.
3. Current membership `last_read_message_id`/`unread_at` produce optional divider record ID, unread count, optional scroll flag (in-page and count >5), or an out-of-page `/rooms/:id?message_id=:id` jump URL. There is no implicit read write from this presentation step.
4. `ShowView.room`: ID, STI-derived kind, persisted name, viewer display name, resolved header identity and involvement. `ShowView.user`: ID/name/title/fresh signed avatar URL. Also room `updated_at`, original-room invitation predicate, account join code, and the SHA1-derived signed room/messages stream name.
5. The shell calls `rooms::room_message_list(ctx, &ShowView)`, which emits `ShellComponents.message_list` verbatim when supplied and zero bytes otherwise. `message_template` and `composer` are separate optional trusted HTML slots; thread/pins/poll/huddle/OOO slots are trusted strings. The current composer fallback receives `room = &show.room` and request `ViewContext`; it has no explicit `user` local. The client message template separately receives `ShowView.user`.
6. Request `ViewContext` carries viewer/admin/bot/preferences, account, assets, base/request URLs/referrer/last-room, time zone, flash and chrome. Normal rendering supplies request CSRF and nonce. Detached broadcasts receive detached contexts and must remain session-free.

After the owner merge, set `ShellComponents.message_list = Some(presenter.room_message_list(&messages, divider.message_id, divider.count)?)` inside the app fragment-cache scope. Inspected WS8bm `68f6615b` exposes that adapter; its report still defers composer parity. WS8bm2 `f168c348` exposes `controllers::rooms::pins::list(conn, app, room)` and `pins::{CountPartial,ListPartial}`. Call those for refresh count/list streams; do not implement pin policy here. Current `RefreshView` has no pin fragments and a pin-only refresh is not complete. Existing region goldens lending real Rails owner fragments prove surrounding regions only, not a native owner/full-page implementation.

Members JSON needs WS17 `UserStatusSettings::{for_ids,effective_presence,status_text_display}` and `WorkspacePresenceLease::presence_by_user_id`, plus WS11 `Agent::{for_user,working_presence_text}` (inspected refs `1021be6a` / `8f338ac6`). These APIs are absent here. Never substitute offline/empty facts. The pending adapter must retain JSON 401, human/member/alive-room gates, 404 privacy, active `LOWER(name),id` ordering, viewer-scoped live stars, absolute fresh avatar URLs, exact agent/status precedence and no-store/no-shared-ETag behavior. The current picker reads only persisted star flags/agent association existence, not these owner policies.

No Rails source, owner partial internals, migrations/schema/dependencies/lockfile, masks/allowlists, ignored tests or production state changed. WS8br2 ownership remains users/profiles/avatars/cards/bans/time zones/accounts/public/welcome/first-run/tour/PWA/QR; those were not implemented here. No worker branch merge, stash, rebase or PR.

## Discrimination, actually rerun in the assigned worktree

```bash
python3 rust/reference-tools/rooms/direct_forms_discrimination.py > .scratch/direct-forms/discrimination.log 2>&1
python3 rust/reference-tools/rooms/direct_rename_discrimination.py > .scratch/direct-forms/rename-discrimination.log 2>&1
python3 rust/reference-tools/rooms/check_controller_files.py --inject-source-drift > .scratch/controller-pin-discrimination.log 2>&1
```

The five Rust mutants compile and fail actual HTTP assertions: leaked group-delete controls, lost viewer stars, byte-count validation, lost named-pair identity and bypassed membership scope. All sources restore in `finally`; the fresh positive suite ran afterwards on committed source. Raw lines:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.46s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.46s
Direct form discrimination: group-delete visibility and viewer-star mutants rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.46s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.59s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 380 filtered out; finished in 0.56s
Direct rename discrimination: Unicode length, named-pair identity and membership mutants rejected; sources restored
Rails controller source-pin injection: wrong hash rejected before tests
```

The first implementation was also demonstrated failing before the fix: two HTTP assertions and the complete 11-body byte corpus failed on the original implementation (not compiler failures). The current mutation receipts above are the rerun evidence.

## Fresh-clone verification

Independent `--no-local` clone of this branch. No seed, target, node_modules or local fixture was copied. It started at `e6a4730e`, generated both seeds from tracked scripts, compiled into its own newly created target and fast-forwarded to `c5f445f7` before the suite. All 56 test/doc targets ran: **1,415 passed, 0 failed, 10 existing ignores; no seed-dependent silent skips**. The app ran **378 passed, 0 failed, 3 ignored**. Ignored items are the app/cable reference recorders, app measurement and pre-existing WS11 `manages_bots`, three DB external-reference/export checks, mail export, and two kit example doctests. No ignore was added. The clone's generated scratch is untracked; committed source is clean.

Actually executed from the assigned worktree:

```bash
mkdir -p .scratch/fresh-continue6
git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-continue6/repo
mkdir -p .scratch/fresh-continue6/repo/.scratch
PARITY_NAMESPACE=ws8br-clean6 PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br .scratch/fresh-continue6/repo/rust/parity/bin/seed build default first_run
npm ci --prefix .scratch/fresh-continue6/repo/rust/parity
npm exec --prefix .scratch/fresh-continue6/repo/rust/parity -- playwright install chromium
```

```text
Cloning into '.scratch/fresh-continue6/repo'...
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
added 11 packages, and audited 12 packages in 468ms
found 0 vulnerabilities
```

Playwright install exit 0 (browser already cached). From the clone root:

```bash
git fetch origin rust/ws8br-rooms-http
git merge --ff-only origin/rust/ws8br-rooms-http
python3 rust/reference-tools/rooms/check_workspace.py
```

```text
Updating e6a4730e..c5f445f7
Fast-forward
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
```

From its `rust/`, exit 0 with no output:

```bash
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

From clone root, both regenerated oracles compare byte-identically with committed fixtures; no masks or normalization:

```bash
PARITY_NAMESPACE=ws8br-clean6 PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/direct_forms.rb > ../direct-forms.json 2> ../direct-forms-oracle.log
cmp ../direct-forms.json rust/vectors/direct_forms.json
PARITY_NAMESPACE=ws8br-clean6 PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/direct_rename.rb > ../direct-rename.json 2> ../direct-rename-oracle.log
cmp ../direct-rename.json rust/vectors/direct_rename.json
```

```text
Rails direct forms: 11 complete body goldens; reference d7c7de92
Rails direct rename: 11 scalar, collection, Unicode and invalid HTTP cases; reference d7c7de92
```

Full positive commands, from clone root, each exit 0:

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > ../workspace-tests.log 2>&1
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > ../clippy.log 2>&1
TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo build --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire > ../build.log 2>&1
CARGO_TARGET_DIR="$PWD/rust/target" python3 rust/reference-tools/rooms/inbound_browser.py > ../browser.log 2>&1
```

All raw test summaries, then clippy/build/browser summaries:

```text
test result: ok. 378 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 36.77s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.02s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 53.60s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.84s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.53s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.64s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.52s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.45s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.07s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.17s
Inbound-email browser acceptance: 2 targets passed; create, confirm rotation, Rails 302-to-404 redirect, flash, reload and direct-room exclusion match
```

The browser rerun drives actual private Rails/Rust seeded servers and Chromium. Inbound creation, confirmed rotation, preserved Rails 302-to-missing-generic-edit/404 redirect, flash, explicit typed settings return, reload and direct-room exclusion match. It does not exercise delivery (WS10) or claim full room-page/pixel acceptance.

## Per-file Rails reference counts and deferred Rust mappings

Actually rerun from the clone root:

```bash
python3 rust/reference-tools/rooms/check_controller_files.py > ../rails-controller-files.log 2>&1
```

Every tested controller source hash was verified at the pin; all raw per-file lines:

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

These are **161 real Rails reference passes / zero failures/errors/skips**, not 161 completed Rust ports. Current inventory command, rerun from the assigned worktree against those fresh logs:

```bash
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log .scratch/fresh-continue6/workspace-tests.log --rails-log .scratch/fresh-continue6/rails-controller-files.log > .scratch/fresh-continue6/deferred.log
```

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; Rails reference executions recorded separately
Rails controller reference receipts: 14 files, 161 passes, 0 failures, 0 errors, 0 skips; reference only
Rails deferred inventory: 58 files, 512 source-declared cases; 161 Rails tests run, 161 Rails reference passes; Rust mappings separate
```

| File under `test/controllers/` | Rails reference passes | Rust status / remaining acceptance |
| --- | ---: | --- |
| `rooms_controller_test.rb` | 29 | Existing grouped scope/delete/join/layout/callback coverage; exhaustive per-declaration mapping, failed leave rescue and merged populated full room HTTP remain. |
| `rooms/opens_controller_test.rb` | 15 | Existing CRUD/icon/access/error/body subsets; complete admin edit composition needs WS15g subscription section, exhaustive params/format/audit/broadcast mappings remain. |
| `rooms/closeds_controller_test.rb` | 12 | Existing CRUD/membership/icon/access/error/body subsets; same admin/odd-input/stream mappings remain. |
| `rooms/directs_controller_test.rb` | 29 | Six new grouped app checks plus 11 whole bodies/11 HTTP input vectors; existing selection/cap/add/leave/delete broadcasts. Remaining: all individual source mappings, browser DM picker/settings, complete overflow/no-op/format/lifecycle matrices and owner-rendered quiet-note/full page acceptance. |
| `rooms/involvements_controller_test.rb` | 8 | Existing grouped involvement state/stream/auth checks; full source mappings and exhaustive flash/error/format/coercion states remain. |
| `rooms/refreshes_controller_test.rb` | 4 | Existing quiet-204/cursor/format/selection coverage; populated complete message streams and pin-only/count/list owner output remain. |
| `rooms/reads_controller_test.rb` | 7 | Existing cursor/auth/no-op callback differential; full individual mappings, browser unread effects and merged around/list integration remain. |
| `rooms/members_controller_test.rb` | 13 | Native endpoint/facts integration entirely deferred to WS8br with WS17/WS11 APIs and WS12 live stars; no Rust members pass claimed. |
| `rooms/categories_controller_test.rb` | 5 | Existing viewer/category/room assignment/privacy subsets; all individual mapping and full state/format/error matrices remain. |
| `rooms/favorites_controller_test.rb` | 6 | Existing viewer membership/favorite create/reorder/destroy subsets; all individual mapping and full state/format/error matrices remain. |
| `rooms/inbound_email_addresses_controller_test.rb` | 8 | All eight individually executed Rust ports passed; two-target browser passed. No declared case deferred. Delivery stays WS10. |
| `room_categories_controller_test.rb` | 6 | Existing CRUD/order/collapse/error/privacy subsets; full individual mappings and exhaustive matrix remain. |
| `switchers_controller_test.rb` | 5 | Existing full fragment/body/menu state goldens; full individual mappings, query bounds and browser combinations remain. |
| `users/sidebars_controller_test.rb` | 14 | Existing full category/favorite/type/direct/unread rows, page and per-viewer broadcasts; full individual mappings, configured WS13/WS17 facts and browser combinations remain. |

The JSON inventory names every remaining declaration, source line and owner, including transferred WS8br2 files. Room audit file has 17 mixed room/account declarations; room subsets remain covered, five account cases remain WS8br2. No whole-file Rust pass count is inferred from grouped tests. Owned system declarations (member selection/navigation/group DMs/header/sidebar/switcher/unread/mobile/keyboard/motion/CSP) still need merged owner APIs and actual browser/pixel runs. Split user/account/public/tour/PWA/QR/system cases remain WS8br2; message/composer/thread/pin cases remain WS8bm/WS8bm2, huddle WS13, status/OOO WS17, agents WS11 and board/star domain WS12.

## Precisely remaining, requested order

1. **Direct room forms/error completion:** delivered whole body parity and covered invalid/scalar/nested/Unicode/named-pair/removed-member paths. Finish the DM browser workflow and exhaustive overflow/no-op/format/failure permutations and individually map the 29 declared direct cases; message-owned quiet-note rendering stays WS8bm.
2. **Members JSON with owner facts:** implement its native adapter, auth/404/no-store, ordering, live per-viewer stars, absolute avatars and WS17/WS11 fact calls after those APIs merge. All 13 Rust equivalents remain unclaimed; no offline/status stubs added.
3. **Refresh/pins:** call WS8bm2's existing list/count seam after merge; mount count/list replacement streams for pin changes, preserve STI targets and finish populated message stream bytes. Pin policy/partial internals remain WS8bm2.
4. **Full room acceptance:** wire WS8bm's list adapter using the exact inputs above; reconcile a completed composer factory and template/thread/poll/pin/WS13/WS17 providers. Run full native room HTTP/layout/list/composer bytes and real browser/pixel matrix for the seed. Lent-fragment region goldens are not that acceptance.
5. **Deferred Rails cases:** finish the per-file Rust/source mappings and system runs listed above/JSON, including failed-leave rescue, configured owner facts and complete room CRUD/involvement/category/favorite/read/broadcast/error matrices. Reference baselines now pass in all 14 files but are not Rust completion.

No permission question, PR or deployment. Work stops at coherent pushed slices with the missing owner integration explicitly partial.
