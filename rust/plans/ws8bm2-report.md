# WS8bm2 PR #191 callback rescue format fix — verified

The requested P2 slice is complete and PR-ready. Final fresh-clone workspace: **4147 passed, 0 failed, 14 existing ignores** across 61 summaries. Strict all-target clippy, locked metadata, the release-input build, 28 existing Rails oracle replays and both new review fixture replays passed. Stopping after push for the lead's review. The last commit changes this report only; the pushed SHA is in the final reply.


Verification source: 1b60775497ebcd1060818b0bfed5539641ba1a7d; production fix: a9a8c0029cba64c7fae8da4fcbf8949b8b42cfc5. Merge: 7425c8fe3d624ea5aa2c99aaa3fd60504165abb3, keeping origin/main 7c23b097885101b39e432e8628ab9ebb1251d9be (#187) and the message feature branch. No conflicts; locked metadata passed immediately after the merge. Final report commit will change documentation only.

Cause: kit finish used the request-aware head response for every rescue and then ran the JavaScript after-action check. Rails Rendering#process_action selects request formats inside AbstractController::Callbacks. Rescue#process_action runs outside that chain. Before-action exceptions use default HTML and unwind after-actions; action exceptions use the action's negotiated formats and also unwind after-actions. Autocomplete's method-local rescue returns normally and must still run the JavaScript check.

Files: kit/ctx.rs tracks entry into the action and handles this centrally. Polls (including set_poll), room pins, message pins and room slash commands mark entry after their callbacks. Saves and scheduling mark entry after prepare; their action-level lookups still negotiate request MIME. Autocomplete marks entry before its lookups and catches locally. Files/message-links use the same controller rescue instead of their old unconditional HTML catches. Shared module registers the regression. Rails review_rescue_formats.rb records the committed 270-response vector; the Rust test runs the actual router with real signed sessions and CSRF tokens. The test compares status, exact body, and seven headers including absence: Content-Type, Cache-Control, Pragma, X-Frame-Options, X-Content-Type-Options, X-Permitted-Cross-Domain-Policies and Referrer-Policy. No existing comparison mask/allowlist, browser timeout or concurrency changed.

The read-only Astra directory was only inspected. A separate clone at exact 99bdc96aa965ecc71e1bbd39c8b437464d83bae4 independently built the pinned default/first_run seeds. Only the new regression module, module inclusion and generated vector were added; no production fix. Original 33-response probe: 8 differences. Expanded matrix: 72 differences / 270 responses. It includes callback and action rescues, method-local rescue, HTML/JSON/Turbo/text-JavaScript/application-JavaScript, XHR and non-XHR, and 60 nonempty PublicExceptions controls. After the fix: zero differences. Four local-rescue non-XHR JS requests remain 422 with the exact Rails error body, their XHR peers are empty JS 404s, and controller-level action JS rescues remain 404s.

One intermediate compile reused baseline db/views artifacts at the common /src mount and therefore reported missing newly merged board APIs. Touching only those crate entry-point mtimes forced them to rebuild; no tracked source changed for that diagnosis. The focused test then passed. Its unused-import warning was removed before the committed fresh-clone checks. No timing retry or threshold change occurred.

Fresh clone independently builds default and first_run seeds. Only registry caches are copied; fixtures/code come from the committed branch. Metadata, release-input guard, strict clippy and workspace tests run sequentially with the configured rustc wrapper/slot file, CARGO_BUILD_JOBS=2, four test threads and owned port ranges. The usual owned native build cache is reused. Fresh Rails format fixture is byte-identical; 28 existing oracle fixtures independently replay byte-identically; source check covers 106 files and rejects both injected differences.

Wider workstream remains partial: populated-provider/older-window callback permutations, exceptional date/coercion matrices and further owner-API consumer integration. WS12 #187 is merged, so that API availability is no longer a blocker; completing its remaining consumer permutations is outside this requested P2 slice. WS11-API dependency remains named. Existing real WS17 transport, WS11 human-agent dispatcher and WS13 integrations, and #179 viewer-zone cache behavior remain. This is a verified coherent PR-ready fix, not a claim that only owner-blocked work remains. Stop after push.

Prior accepted browser inventory (not rerun in this request): polls 4/4, pins_saved 7/7, slash_commands 26/26, search_files 4/4, scheduled_messages 4/4: 45/45 on each app, accepted in Astra's review of 9dd82639 and rereview of 99bdc96a.

The initial full fresh-clone run completed at a9a8c0029: 4145 passed, 2 failed, 14 existing ignores across 61 summaries. Both failures were our stale structural HTTP tests asserting a chat composer and pin frame on the board index. The merged reviewed WS12 board page matches Rails rooms/show, which chooses rooms/boards/index and omits those controls. A new actual Rails HTTP five-STI probe confirms four chat surfaces and the separate filtered board index. Test-only commit 1b6077549 compares these facts from a generated vector and adds positive board-index/filter assertions; no scenario was removed, no mask added, no production board rendering changed. The shared composer partial goldens, including the Board param key, still pass independently. Final verification uses another newly created clone at this corrected commit, with freshly rebuilt seeds.

## Failing-first and focused raw receipts

Exact reviewed baseline 99bdc96a, exit 101 (all tracked production unchanged except test-module registration; independently built seeds):

```text
WS8bm2 original rescue format probe: 33 responses; 8 differences
WS8bm2 rescue format matrix: 270 responses; 7 headers and body per response; 60 nonempty public-exception controls; 72 differences
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2141 filtered out; finished in 1.04s
```

After the fix, exit 0:

```text
WS8bm2 original rescue format probe: 33 responses; 0 differences
WS8bm2 rescue format matrix: 270 responses; 7 headers and body per response; 60 nonempty public-exception controls; 0 differences
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2170 filtered out; finished in 1.40s
```

The non-XHR poll/pins JavaScript cases specifically failed with Rust 422 / 5408 body bytes against Rails 404 / zero bytes. JSON and Turbo callback cases had Rust JSON/Turbo Content-Type versus Rails text/html. The expanded matrix also discriminates the action-level message-link rescue's MIME choice. These are real router/controller responses, not response rewrites or sleeps.

## Commands run and final fresh-clone receipts

All commands cited here were executed this turn. The adapter is `.scratch/pr191-formats/ci-env.sh`; it retains the configured machine slot file and rustc wrapper, Docker CPU cap, CARGO_BUILD_JOBS=2, owned port ranges and native build cache. Only the final clone supplies source, fixtures and independently built seed state to the final tests.

Baseline setup was `git clone --quiet --no-hardlinks . .scratch/pr191-formats/baseline`, followed by `git -C .scratch/pr191-formats/baseline checkout --quiet --detach 99bdc96aa965ecc71e1bbd39c8b437464d83bae4`. The new test, module registration and vector were copied in; registry caches only were reused. Its exact regression command was:

```bash
source .scratch/pr191-formats/ci-env.sh
CARGO_TARGET_DIR=/native-target bash .scratch/pr191-formats/baseline/rust/ci/cargo.sh test --locked -p campfire -j4 controllers::message_features::rescue_format_tests -- --test-threads=4 --nocapture > .scratch/pr191-formats/before.log 2>&1
```

The focused fixed-code command, after forcing the db/views crate entry-point mtimes to rebuild the baseline artifacts, was:

```bash
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire -j4 controllers::message_features::rescue_format_tests -- --test-threads=4 --nocapture > .scratch/pr191-formats/after-rebuilt.log 2>&1
```

That intermediate stale-artifact compile had no tests run; its raw final diagnostic is retained in `.scratch/pr191-formats/after.log`:

```text
error: could not compile `campfire` (bin "campfire" test) due to 45 previous errors; 1 warning emitted
```

The initial fresh clone at a9a8c0029 ran the complete workspace, with the two stale board assertions described above and no other failures. Raw app summary:

```text
test result: FAILED. 2164 passed; 2 failed; 5 ignored; 0 measured; 0 filtered out; finished in 603.59s
```

Its full aggregate was 4145 passed / 2 failed / 14 ignored across 61 summaries; exit 101. Its exact command from `.scratch/pr191-formats/fresh/` was:

```bash
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 --nocapture > .scratch/ws8bm2/workspace.log 2>&1
```

The final clone was created after the test-only correction committed, at exact 1b60775497ebcd1060818b0bfed5539641ba1a7d. No local seeds or test data were copied into it:

```bash
git clone --quiet --no-hardlinks . .scratch/pr191-formats/final
mkdir -p .scratch/pr191-formats/final/.scratch/ws8bm2 .scratch/pr191-formats/final/rust/.cargo-home
cp -a .scratch/pr191/fresh/rust/.cargo-home/registry .scratch/pr191-formats/final/rust/.cargo-home/
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92
bash .scratch/pr191-formats/final/rust/parity/bin/seed build default first_run > .scratch/pr191-formats/final/.scratch/ws8bm2/seeds.log 2>&1
```

Raw seed lines (exit 0):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The required final commands ran sequentially from the assigned worktree, then the final clone:

```bash
source .scratch/pr191-formats/ci-env.sh
cd .scratch/pr191-formats/final
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > .scratch/ws8bm2/metadata.json 2> .scratch/ws8bm2/metadata.stderr
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/release-inputs.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings > .scratch/ws8bm2/clippy.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4 --nocapture > .scratch/ws8bm2/workspace.log 2>&1
```

All four exited 0; metadata stderr is empty and Cargo.lock unchanged. Release-input and strict clippy raw final lines respectively (terminal color only removed):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 34s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 12s
```

Full workspace raw summary lines, including vendored html5ever and doctests:

```text
test result: ok. 2166 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 717.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.04s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1254 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 220.81s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.85s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.24s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.80s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.78s
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

Existing ignored declarations (no new ignores; the two negative seed-discovery tests' expected failure/skip messages are not skipped feature tests):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

Final fresh-clone feature receipts from that same complete run:

```text
WS8bm2 STI composer HTTP: 5/5 Rails room surfaces; four chat composers and one board index
WS8bm2 STI pin HTTP: 3/3 Rails room surfaces; voice/stage pin panels and separate board index
WS8bm2 original rescue format probe: 33 responses; 0 differences
WS8bm2 rescue format matrix: 270 responses; 7 headers and body per response; 60 nonempty public-exception controls; 0 differences
WS8bm2 timer scheduled send: configured HTTPS origin and port delivered over cable
WS8bm2 Rust warm HTTP search: 4 results; 46 SELECT/WITH executions
WS8bm2 Rust warm HTTP search: 16 results; 46 SELECT/WITH executions
WS8bm2 role/room matrix: 520 responses; 480 byte comparisons; 0 differences
```

Independent Rails replays ran from the final clone. Each storage directory is a fresh copy of its newly built default seed. Both cmp commands exited 0, comparing exact committed fixture bytes:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92
mkdir -p .scratch/ws8bm2/rails-formats-db .scratch/ws8bm2/rails-surfaces-db
cp -a rust/parity/.seed/default/. .scratch/ws8bm2/rails-formats-db/
cp -a rust/parity/.seed/default/. .scratch/ws8bm2/rails-surfaces-db/
bash rust/parity/bin/reference runner --storage .scratch/ws8bm2/rails-formats-db --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/review_rescue_formats.rb /rails/storage/db/review_rescue_formats.json > .scratch/ws8bm2/rails-formats.log 2>&1
cmp .scratch/ws8bm2/rails-formats-db/db/review_rescue_formats.json rust/vectors/messaging/review_rescue_formats.json
bash rust/parity/bin/reference runner --storage .scratch/ws8bm2/rails-surfaces-db --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/review_room_surfaces.rb /rails/storage/db/review_room_surfaces.json > .scratch/ws8bm2/rails-surfaces.log 2>&1
cmp .scratch/ws8bm2/rails-surfaces-db/db/review_room_surfaces.json rust/vectors/messaging/review_room_surfaces.json
python3 rust/reference-tools/messaging/verify_oracles.py > .scratch/ws8bm2/oracles.log 2>&1
python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/ws8bm2/reference-check.log 2>&1
```

Raw Rails/source summaries:

```text
REVIEW Rails rescue format matrix: 270 responses; 33 original probe responses; status, 7 headers and body recorded
REVIEW Rails room surfaces: 5/5 HTTP room types; board index has filters, no chat composer or pin panel
WS8bm2 oracle replay: features.json byte-identical
WS8bm2 oracle replay: saved.json byte-identical
WS8bm2 oracle replay: scheduled.json byte-identical
WS8bm2 oracle replay: search.json byte-identical
WS8bm2 oracle replay: preloads.json byte-identical
WS8bm2 oracle replay: slash.json byte-identical
WS8bm2 oracle replay: links_files.json byte-identical
WS8bm2 oracle replay: reminder_push.json byte-identical
WS8bm2 oracle replay: quote_integration.json byte-identical
WS8bm2 oracle replay: root_cache.json byte-identical
WS8bm2 oracle replay: panels.json byte-identical
WS8bm2 oracle replay: date_inputs.json byte-identical
WS8bm2 oracle replay: review_saved_race.json byte-identical
WS8bm2 oracle replay: review_dates.json byte-identical
WS8bm2 oracle replay: date_compact_widths.json byte-identical
WS8bm2 oracle replay: providers.json byte-identical
WS8bm2 oracle replay: provider_edits.json byte-identical
WS8bm2 oracle replay: event_cards.json byte-identical
WS8bm2 oracle replay: date_coercions.json byte-identical
WS8bm2 oracle replay: composer.json byte-identical
WS8bm2 oracle replay: composer_sti.json byte-identical
WS8bm2 oracle replay: twitter_preloads.json byte-identical
WS8bm2 oracle replay: twitter_cards.json byte-identical
WS8bm2 oracle replay: twitter_text.json byte-identical
WS8bm2 oracle replay: provider_callbacks.json byte-identical
WS8bm2 oracle replay: agent_command.json byte-identical
WS8bm2 oracle replay: user_coercions.json byte-identical
WS8bm2 oracle replay: date_years.json byte-identical
WS8bm2 oracle replay: 28/28 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

## Cleanup and publication

The baseline has no scratch target remaining; the first fresh clone's generated target contained 32,806 file bytes and was deleted once its complete run exited. Final cleanup touches only the explicitly named clone/rust/target paths. Registry source directories and the normal owned native build cache remain untouched. No Rust test threshold/concurrency reduction, stash, rebase, force push, PR creation or deployment was performed. The Python model server and other workers' files/processes were untouched. All owned Cargo/reference executions have finished.

```text
No remaining scratch target: .scratch/pr191-formats/baseline/rust/target
No remaining scratch target: .scratch/pr191-formats/fresh/rust/target
Deleted completed scratch target: .scratch/pr191-formats/final/rust/target; 32806 file bytes
```

Raw receipts are `.scratch/pr191-formats/before.log`, `after.log`, `after-rebuilt.log`, the initial `fresh/.scratch/ws8bm2/` run, and final `final/.scratch/ws8bm2/` run. The tracked report and authorized wave4/ws8bm2-report.md mirror are identical. Push and stop for the lead's review.
