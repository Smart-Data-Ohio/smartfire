# WS8br Wave 4 — PARTIAL

Branch `rust/ws8br-rooms-http`, assigned worktree `rust-ws8br`, Rails pin `d7c7de92`. Main remains `21a7332f`, already merged through `9f356e2f`. This continuation received `7c32f0e9` and pushed five coherent slices through **`8f5fe0ee8219265b0a08b6718bb14f3e272107a9`**. No PR or deployment. Earlier detail and receipts remain in this report's history at `7c32f0e9`.

## Pushed slices and file changes

1. **`fba61d1f` — channel icons and invalid forms.** `campfire/controllers/rooms/{opens,closeds}.rs` permit/cast `icon_name`, call the transaction-local model seam, and render new/edit with 422 and the attempted normalized values on validation failure. Closed error pages use persisted membership selections; new forms keep Rails' unselected list/current-user hidden input. Invalid writes never convert the room, revise memberships, audit, or publish controller frames. `db/models/room.rs` adds `create_for_with_icon`, `update_with_icon`, and Ruby-compatible normalization; unchanged legacy unknown icons are not revalidated and no-op updates retain timestamps. Existing `create_for`/`update` callers remain stable.
2. **`31d1a161` — complete channel access forms.** `views/templates/rooms/{opens,closeds}/_{form,user}.html` match whole Rails partials for new, invalid, administrator, member and search states. `views/src/rooms.rs` concatenates user partial bytes like Rails collection rendering and calls the existing avatar helper for profile-card attributes. Ten complete partial fixtures include the >20 search branch (a test-only 21-entry collection). This does not claim a 21-user controller query or pixel matrix.
3. **`6f83d4d1` — inbound settings and browser workflow.** `views/rooms::InboundEmailSection`, its template and the edit layout render disabled, create and rotate states. `controllers/rooms.rs` supplies WS10's persisted token and configured domain as view facts. `presenters/test_support.rs` accepts test-local config without process environment mutation. Six complete Rails partial fixtures cover HTML escaping, non-admin and direct-room exclusion. `reference-tools/rooms/inbound_browser.{py,mjs}` creates private seeded reference/candidate servers and drives Chromium through create, confirmation/rotation, redirect/flash, explicit return to typed settings, reload and direct-room exclusion. Rails redirects to the absent generic `/rooms/:id/edit` action and gets 404; the browser test preserves that observed 302-to-404 behavior.
4. **`9ef072cf` — the inbound controller's entire declared case file.** `controllers/rooms/inbound_rails_cases.rs` has eight separately executed Rust tests, one per declaration in `test/controllers/rooms/inbound_email_addresses_controller_test.rb`. A non-admin creator is an additional guard discrimination. `inbound_discrimination.py` proves a compiled missing-admin-check mutant fails. `deferred_inventory.py` and `plans/ws8br-rails-cases.json` attach named Rust pass receipts while keeping Rails Minitest execution counts explicitly zero.
5. **`8f5fe0ee` — exact normalized icon lookup.** Additional real Rails probes reject Unicode-padded brand/emoji names and accept a custom workspace-icon metadata row created by each test/oracle itself. `richtext/markdown::IconCatalog::find_normalized` is a narrow exact-key data seam; the existing Markdown entry point retains its behavior. `campfire/rich_text.rs` uses this entry point for room validation. Expanded corpus: 36 creations, 10 updates, one legacy write and eight exact form-layout goldens.

Also changed `views/helpers/forms.rs`: optional record error attributes wrap labels/text fields in Rails' `field_with_errors` div. Default builders are unchanged. `views/rooms::FormRoom` gains defaulted icon/error/inbound inputs. The four existing construction sites were moved behind `controllers/rooms::form_room`; `RoomView`, `ShowView`, sidebar/header presenters and the message-list seam stay stable.

## Ownership and design notes

- WS8a touch: small Room icon write/validation APIs, same transaction/connection, with existing grants and after-commit semantics. No schema change, owner queue rewrite or HTML in the model.
- WS5 touch: reuse its icon catalog, add exact normalized-key lookup without changing Markdown behavior. Workspace metadata is loaded on the caller's connection, avoiding reader/writer deadlocks.
- WS6 touch: default-empty field-error builder input and reuse of its avatar/icon helpers. Other forms keep their original default behavior.
- WS10 touch: consume existing token/domain facts; no relay, delivery, ingestion or authentication change.
- WS8br2 retains users/profiles/avatars/cards/bans/time zones, account pages and their re-diff, public/welcome/first-run/tour/PWA/QR. None implemented here.
- WS8b-m retains message/list/composer/template/thread/pin/poll internals. WS13 owns venue/huddle facts and WS17/WS11 own presence/status/OOO and agents. No owner partial internals, Rails source, masks, allowlists, ignored-test declarations or lockfile were changed.

## Failing-first evidence

New tests compiled and failed before their corresponding production fixes. The six raw lines below are, in order: original icon writes, shared form layout, complete channel access forms, missing inbound section, Unicode lookup regression, and missing authorization mutant. The last is an intentional compiling mutation, restored in `finally`, followed by the passing eight-case run. The first five logs are this continuation's pre-fix runs; none is a compiler-failure receipt.

```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 362 filtered out; finished in 0.42s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 365 filtered out; finished in 0.39s
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 372 filtered out; finished in 0.57s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 374 filtered out; finished in 0.42s
```

Original worktree mutation command, actually rerun:

```bash
python3 rust/reference-tools/rooms/inbound_discrimination.py > .scratch/inbound-discrimination-summary.log 2>&1
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 374 filtered out; finished in 0.42s
Inbound authorization discrimination: compiled missing-admin check rejected; source restored
```

## Fresh-clone verification

Fresh independent clone, created with `--no-local` from this branch; no seed, target, node_modules or fixture was copied from the original worktree. It initially checked `7c32f0e9`, then fetched/fast-forwarded committed slices through `8f5fe0ee`. Both seeds were built there from tracked seed scripts using the named reference runtime; oracle source hashes verify the accepted Rails pin. The reference runtime image was provisioned before this continuation. Test-created workspace-icon metadata and private browser storage are generated inside the tests/tools. The clone's only untracked artifact is its generated `.scratch/` directory.

The report is a documentation-only follow-up to the code SHA above. All commands below were actually run; all positive checks exit 0. Raw outputs are under the original worktree's `.scratch/fresh-clone/`.

Initial clone, from the assigned worktree:

```bash
mkdir -p .scratch/fresh-clone
git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh-clone/repo > .scratch/fresh-clone/clone.log 2>&1
```

```text
Cloning into '.scratch/fresh-clone/repo'...
```

Then from `.scratch/fresh-clone/repo`:

```bash
mkdir -p .scratch
git fetch origin rust/ws8br-rooms-http
git merge --ff-only origin/rust/ws8br-rooms-http
PARITY_NAMESPACE=ws8br-fresh PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/seed build default first_run > ../seed-build.log 2>&1
npm ci --prefix rust/parity > ../npm-ci.log 2>&1
npm exec --prefix rust/parity -- playwright install chromium > ../playwright-install.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
added 11 packages, and audited 12 packages in 463ms
found 0 vulnerabilities
Chrome Headless Shell 153.0.8010.12 (playwright chromium-headless-shell v1243) downloaded to /home/riels/.cache/ms-playwright/chromium_headless_shell-1243
```

From the clone's `rust/`:

```bash
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > /dev/null
```

Exit 0, no output. From the clone root:

```bash
python3 rust/reference-tools/rooms/check_workspace.py > ../duplicate-keys-final.log
```

```text
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
```

All three oracles were rerun in the clone and compared byte for byte with their tracked vectors. The expanded icon oracle was rerun after the last code slice:

```bash
PARITY_NAMESPACE=ws8br-fresh PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/icons.rb > ../icons-final.json 2> ../icons-final-oracle.log
cmp ../icons-final.json rust/vectors/room_icons.json
PARITY_NAMESPACE=ws8br-fresh PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/forms.rb > ../forms.json 2> ../forms-oracle.log
cmp ../forms.json rust/vectors/room_access_forms.json
PARITY_NAMESPACE=ws8br-fresh PARITY_IMAGE=ws8br-reference-d7c7de92 PARITY_OWNER=ws8br rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze -e RAILS_LOG_LEVEL=fatal rust/reference-tools/rooms/inbound_section.rb > ../inbound.json 2> ../inbound-oracle.log
cmp ../inbound.json rust/vectors/inbound_section.json
```

```text
Rails room icons: 36 creations, 10 updates, 1 legacy write, 8 exact form-layout goldens; reference d7c7de92
Rails channel forms: 10 complete partial goldens (new, invalid, admin, member, search); reference d7c7de92
Rails inbound-email section: 6 complete partial goldens; reference d7c7de92
```

Full workspace suite after the final code slice, including the seeded app, views, domain models, integrations and doctests:

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52100-52149 MAIL_TEST_PORT_RANGE=52100-52149 mise exec rust@1.98.1 -- cargo test --locked -j 4 --manifest-path rust/Cargo.toml --workspace -- --test-threads=4 > ../workspace-tests-final.log 2>&1
```

**1,408 passes, zero failures, ten existing ignores across 55 test/doc targets**, calculated from these raw result lines. App: 372 passes, three ignores. Views: 77 passes. CI=1 forbids silent seeded skips; both seeds exist. No new ignore was added.

```text
test result: ok. 372 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 37.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.75s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 47.68s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.15s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.24s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.28s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.49s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
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

Explicit existing ignores:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::presenters::accounts::tests::manages_bots ... ignored, WS11: resetting Bender's bot key leaves the original seeded key visible in the account bot list
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

```bash
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > ../clippy-final.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo build --locked -j 4 --manifest-path rust/Cargo.toml -p campfire --bin campfire > ../build-final.log 2>&1
CARGO_TARGET_DIR="$PWD/rust/target" python3 rust/reference-tools/rooms/inbound_browser.py > ../browser-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 42.04s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 29.69s
Inbound-email browser acceptance: 2 targets passed; create, confirm rotation, Rails 302-to-404 redirect, flash, reload and direct-room exclusion match
```

The browser runner starts/stops only its own servers on 52150–52152 and deletes its generated private storage. No full room-page browser/pixel matrix is claimed by this inbound flow.

## Rails cases grouped by file

```bash
python3 rust/reference-tools/rooms/deferred_inventory.py --test-log ../workspace-tests-final.log > ../case-receipts-final.log
git diff --exit-code -- rust/plans/ws8br-rails-cases.json
```

```text
Rails case port receipts: test/controllers/rooms/inbound_email_addresses_controller_test.rb: 8 Rust cases passed, 0 deferred; 0 Rails Minitest executions
Rails deferred inventory: 58 files, 512 source-declared cases; 0 Rails tests run, 0 Rails passes claimed
```

The table counts individually named source-case ports with execution receipts, not inferred coverage from combined Rust tests. Other source files already have passing Rust subsets (prior report and the new icon/form matrices), but their exhaustive per-case mapping remains deferred. Every full Rails Minitest execution/pass count is zero. The machine-readable inventory preserves every deferred case name/line/hash, including transferred files.

| Owned Rails controller file | Source declarations | Individually mapped Rust case passes | Mapping still deferred |
| --- | ---: | ---: | ---: |
| `rooms_controller_test.rb` | 29 | 0 | 29 |
| `rooms/opens_controller_test.rb` | 15 | 0 | 15 |
| `rooms/closeds_controller_test.rb` | 12 | 0 | 12 |
| `rooms/directs_controller_test.rb` | 29 | 0 | 29 |
| `rooms/involvements_controller_test.rb` | 8 | 0 | 8 |
| `rooms/refreshes_controller_test.rb` | 4 | 0 | 4 |
| `rooms/reads_controller_test.rb` | 7 | 0 | 7 |
| `rooms/members_controller_test.rb` | 13 | 0 | 13 |
| `rooms/categories_controller_test.rb` | 5 | 0 | 5 |
| `rooms/favorites_controller_test.rb` | 6 | 0 | 6 |
| `rooms/inbound_email_addresses_controller_test.rb` | 8 | 8 | 0 |
| `room_categories_controller_test.rb` | 6 | 0 | 6 |
| `switchers_controller_test.rb` | 5 | 0 | 5 |
| `users/sidebars_controller_test.rb` | 14 | 0 | 14 |
| `unfurl_links_controller_test.rb` | 9 | 0 | 9 |

`audit_log/rooms_audit_test.rb` has 17 mixed room/account declarations: room creation/revision/outage subsets remain verified here, now including invalid-icon no-write/no-audit checks; five account cases belong to WS8br2. This continuation does not infer a 17-case pass count.

## Precisely remaining, requested order

1. **Remaining room forms/members/error paths:** direct picker/settings whole-partial acceptance and invalid rename/overflow/no-op/removed-member cases; `rooms/members#index` auth/response/no-store integration with WS17 lease/effective-presence/status and WS11 agent facts (plus the live per-viewer starred-ID provider); nonquiet refresh's complete message/pin-count/list output through WS8b-m; leave `RecordNotDestroyed` rescue, exhaustive exact flash/errors/format/coercion cases for involvement/categories/favorites and room CRUD. Open/closed icon handling, invalid-form layout and whole access-form bytes are now done. Complete admin settings-page composition still needs WS15g's GitHub subscription section/provider.
2. **Owner rendering and full seeded room-page acceptance:** integrate WS8b-m list/composer/template/threads/pins/polls, WS17 per-viewer OOO facts and WS13 configured venue/header children; run live full-page byte/browser/pixel acceptance. `rooms::room_message_list(ctx, &ShowView)` still emits the authorized zero-byte empty-room placeholder until its owner adapter lands. Existing goldens lending real Rails owner fragments prove surrounding shell regions only. Member/presence defaults are not claimed as completed policy.
3. **Inbound-email address browser acceptance is done** on real Rails and Rust seeded servers, including the preserved broken generic-edit redirect. No relay delivery acceptance is added here; WS10 owns it.
4. **Remaining owned Rails file/system cases:** finish the case mappings shown above and their per-file receipts, query-count acceptance, configured-owner states and valid agent-token gates when WS11 authentication lands. Remaining system-file declarations and split-off WS8br2 ownership are in `plans/ws8br-rails-cases.json`.

No new permission request or unresolved user preference. Remaining integration facts require their owning branches; the lead merges those branches. Work stops at a coherent pushed code/test slice with final fresh-clone verification and this documentation follow-up.
