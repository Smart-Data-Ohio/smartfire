# WS8br PR #175 review fixes — complete; wider workstream partial

Verified source: `9abd1c14e1d8f961c6739e38af67da2719d3f06f`, based on reviewed `3f25dcc5105488c55834399caeba9c2578047f64`. Both P2 fixes and the injected-HTML audit are complete. The independent fresh clone passes **2805 workspace tests, zero failures, eleven existing ignores, 58 harness summaries**; its seeded app passes **1399, zero failures, two existing ignores**. Clippy passes with `--workspace --all-targets -- -D warnings`. No seed-dependent case silently skipped: CI=1 and independently built default/first_run seeds were used.

Pushed slices: `5d35e113` wires the real scheduling/GitHub providers and router-level regressions; `9abd1c14` replaces the shell substitution test with full HTTP acceptance and adds header-refresh coverage. This final documentation commit follows the verified application source. No new main merge was needed; both pushes succeeded without conflicts. Main providers already incorporated through the prior merge at `65ad0d39` are called directly.

## P2 fixes and failing-first evidence

**Thread content scheduling.** `channel_threads::content` renders the real `scheduled_messages::ComposerButton` inside the request ViewContext, with `room_id: composer.room_id` and `thread_id: Some(thread.id)`, then passes its Rust output to Conversation. The comparison now sends requests through the router for every successful existing Rails conversation window, plus the root room composer. The window/header/real CSRF ownership tests remain separate and unchanged. No Rails HTML is supplied to the Rust renderer.

Before changing either production handler, the new regressions ran against 3f25dcc5 plus test-harness additions. The content comparison failed at byte 256475: Rust 257305 bytes, Rails 259704. At that location Rails starts `<span class="schedule-send" data-controller="schedule-send"`; Rust goes directly to the send button. The existing thread-content golden was reproduced independently from Rails in the fresh clone.

**Standalone PR thread.** `render_thread_pull_request_header` calls main's unchanged `presenters::github::thread_header`. Its owner supplies the public PR card, files, signed stream, private/unknown lazy card frame and actions mount. The controller adapter records the authorized room/thread mapping's PR ID for main's `refresh_after_render` writer seam, including threads without starters. It reuses main's atomic claims, deduplication and graceful render-time queue failure behavior.

The seeded-header regression also failed before the production change: the HTTP page omitted the entire 1725-byte Rails header fragment. The reviewer's saved 3f25dcc5 main contains 549 bytes, while the fresh public Rails HTTP main contains 2279 bytes (header plus surrounding template whitespace). Four additional exact real-HTTP main comparisons cover public, private, unknown privacy and an unmapped thread. A real no-starter HTTP test verifies two requests enqueue once; an injected queue rejection leaves the page 200 and rolls back both claim and job.

Failing-first command in the original worktree, with CI=1, TMPDIR="$PWD/.scratch", CARGO_BUILD_JOBS=2, both Cargo debug profiles set to 0, CABLE_TEST_PORT_RANGE=52100-52149 and MAIL_TEST_PORT_RANGE=52100-52149:

```sh
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire rails_bytes_through_http -- --test-threads=4 --nocapture
```

Raw failures:

```text
test controllers::channel_threads::page_tests::seeded_pull_request_thread_header_matches_rails_bytes_through_http ... FAILED
test controllers::channel_threads::content_tests::conversation_and_room_composer_match_rails_bytes_through_http ... FAILED
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1396 filtered out; finished in 14.86s
```

## Injected-HTML audit and replacements

Seven reference-rendered fragment inputs were found across two comparison tests:

| Old comparison test | Rails HTML supplied as a Rust input | Replacement |
| --- | --- | --- |
| `channel_threads/content_tests.rs::conversation_and_room_composer_match_fixed_secret_rails_bytes` | `rows[*].composer_button` for the thread | Router GET, real schedule provider, complete conversation bytes |
| Same test | `room_schedule` for the root composer | Router room GET, real native root components, exact composer bytes |
| `views/tests/room_shell.rs::empty_room_shell_regions_match_rails` | `owner_fragments.pins_panel` | Four complete empty-room HTTP pages through native providers |
| Same test | `owner_fragments.thread_panel` | Same full HTTP pages |
| Same test | `owner_fragments.message_template` | Same full HTTP pages |
| Same test | `owner_fragments.composer` | Same full HTTP pages |
| Same test | `owner_fragments.poll_builder` | Same full HTTP pages |

The shell test's reference inputs are removed. Its seam and unread-jump unit checks remain. Full HTTP acceptance now covers empty channel, pair DM, group DM and open room pages, including every child and the application layout. The fixture explicitly preserves the seed room timestamp on both targets while removing messages through domain callbacks; this is a rendering fixture, not message-destruction timestamp coverage. Rails alone generated the four new full-page responses.

The audit inspected raw/Safe/Html construction, template field assignments and aliases across controller/app/view comparison tests. Layout-only tests in `views/tests/core.rs` and `views/tests/review/mod.rs` use literal dummy body/head/nav/chrome inputs to test layout primitives. The remaining message-list seam unit test uses a literal owner marker. Neither supplies Rails-rendered child output as an integration. Rich-text corpus HTML is domain input being transformed. No additional substitution of a Rails child into a Rust integration was found.

A test-only Tokio task-local fixes global/per-form tokens and nonce **before** the real router/controller render. It supplies no HTML, modifies no response, and leaves authentication and forgery verification on the real kit tokens. Separate tests use unmodified real request secrets. No parity allowlist or output mask was added or widened. Full-page comparison retains main's approved shared asset helper, which validates live local digests and compares all surrounding bytes.

To discriminate the replacement, the native root scheduling provider was temporarily replaced with empty Rust output. The complete HTTP comparison rejected it: empty-channel Rust 113483 bytes versus Rails 115827. The provider was restored before positive tests or commits. Raw rejection:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1400 filtered out; finished in 1.17s
Native root schedule mutation: rejected by full HTTP Rails bytes; provider restored
```

The before/after audit logs are `.scratch/review-fixes/injection-before.log` and `injection-after.log`; the latter has zero matches. The reviewer's probes, runner, security mutations and receipts under `/home/riels/.cache/rust-port/ws8brr/review/.scratch/` were read without modification.

## Changed files and owner boundaries

- `controllers/channel_threads.rs`: real schedule and PR header provider calls; PR refresh registration after room/thread authorization. No new route or authorization shortcut.
- `controllers/presenters.rs`: remember rendered header PR IDs in the existing shared refresh set.
- `presenters/test_support.rs`, `presenters/view_context.rs`: test-only deterministic rendering entropy, scoped around actual HTTP futures.
- `channel_threads/content_tests.rs`, `page_tests.rs`: actual router byte comparisons; public/private/unknown/unmapped PR matrix and real queue-failure integration.
- `rooms/full_page_tests.rs`, `empty_shell_http.json`: four complete native HTTP page comparisons, replacing reference-child composition.
- `views/tests/room_shell.rs`: remove all five injected owner fragments and move byte acceptance to the real HTTP test; retain isolated seam/jump checks.
- `reference-tools/rooms/{thread_review_http.rb,check_thread_review.py,empty_shell_http.rb,check_empty_shell_http.py}`, `vectors/messaging/pr-thread-http.json`: Rails-only HTTP recorders and independent reproducibility/source checks.
- `plans/ws8br-owner-integration.md`: exact thread and root seam inputs, provider ownership and new HTTP acceptance.

Message-list, composer, scheduling, pins and GitHub view/domain internals remain owner implementations. The new controller wires their stable entry points. Root and thread inputs are documented in the owner contract; no GitHub card renderer or pin logic was ported here. There was no browser pixel work and no browser/system run in this focused review slice.

## Positive worktree receipts

Executed sequentially with the same build/test environment as failing-first:

```sh
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire channel_threads:: -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire pull_request_thread_without_starter -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire empty_room_shell_and_all_owner_children -- --test-threads=4 --nocapture
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_views --test room_shell -- --test-threads=4 --nocapture
```

Raw summaries, respectively (the nineteen-thread run precedes the extra no-starter test):

```text
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 1380 filtered out; finished in 27.90s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1400 filtered out; finished in 1.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1400 filtered out; finished in 3.60s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## Independent fresh-clone verification

Fresh source: `.scratch/fresh15/repo`, exactly 9abd1c14. No target, seed, storage, node_modules or other untracked inputs were copied from the worktree. Cargo actions were sequenced so an executing test binary was never rebuilt or removed during its run.

Executed from the worktree:

```sh
git clone --no-local --single-branch --branch rust/ws8br-rooms-http . .scratch/fresh15/repo
```

The fresh clone's driver `.scratch/verify-review.py` applied this environment before the following actual commands: CI=1; TMPDIR at its own .scratch; CARGO_BUILD_JOBS=2; CARGO_PROFILE_TEST_DEBUG=0; CARGO_PROFILE_DEV_DEBUG=0; CABLE_TEST_PORT_RANGE=52100-52149; MAIL_TEST_PORT_RANGE=52100-52149; RUSTC_BOOTSTRAP=1; CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER at its tracked `rust/reference-tools/rooms/pinned_media_runner.py`; PARITY_NAMESPACE=ws8br-fresh15-seed; PARITY_OWNER=ws8br; PARITY_IMAGE=ws8br-reference-d7c7de92. The media runner preserves the storage version guard and byte assertions using the pinned media image; other tests run natively.

```sh
rust/parity/bin/seed build default first_run
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 rust/reference-tools/rooms/check_workspace.py
python3 rust/reference-tools/messaging/check-goldens.py thread-content
python3 rust/reference-tools/rooms/check_thread_review.py
python3 rust/reference-tools/rooms/check_empty_shell_http.py
mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=4 -Z unstable-options --report-time
mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

Seed, metadata/manifest and Rails oracle receipts:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
Cargo TOML duplicate-key check: all manifests parse; no duplicate workspace dependency keys
WS8bm thread-content oracle: 9 actual requests; anchor scope and fixed-secret conversation/composer bytes
WS8bm golden check: 1 Rails oracles re-run; 1 golden files byte-identical
Rails PR-thread HTTP oracle: 4 responses reproduced; 4 source files match d7c7de92; bytes unchanged
Rails empty-shell HTTP oracle: 4 full responses reproduced; 8 pinned/approved source files verified; bytes unchanged
```

Locked metadata exited 0; its JSON stdout was suppressed. The PR recorder verifies four original source hashes at d7c7de92. The empty-page recorder verifies seven original sources plus the approved 2e20b24c application layout. Newly regenerated vectors are exclusively Rails output; the fresh clone checks their reproduction without recording.

Every raw workspace summary line:

```text
test result: ok. 1399 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 632.68s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.10s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 705 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 98.57s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.45s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.69s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.96s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.52s
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

Clippy raw completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 2m 16s
```

The two app ignores are `channels::tests::golden::record_reference` (manual reference recording) and `jobs::tests::push_latency` (measurement). No new ignore was introduced. All 152 previously named original controller mappings have a matching fresh `test ... ok` receipt; none is missing. These are Rust mapping receipts, not new Rails Minitest executions.

The app took 632.68 seconds. Largest fresh app cases by reported duration:

```text
test controllers::github::write_tests::github_write_http_results_payloads_own_token_prompts_retry_text_and_streams_match_rails ... ok <43.649s>
test controllers::channel_thread_messages::tests::nested_reads_require_alive_membership_and_both_thread_and_message_scope ... ok <42.662s>
test controllers::github::webhooks::tests::webhook_http_status_body_selection_and_privacy_match_rails ... ok <41.858s>
test controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix ... ok <37.153s>
test integrations::fizzy::agent_requests::tests::ws15e_fizzy_agent_requests_match_pinned_service_results ... ok <32.529s>
test integrations::fizzy::agent_job::tests::ws15e_fizzy_agent_execution_rechecks_and_records_once ... ok <32.010s>
test controllers::messages::room_list_tests::room_list_places_unread_outside_shared_fragments_and_matches_rails_around_pages ... ok <31.998s>
test integrations::fizzy::agent_reads::tests::ws15e_fizzy_agent_reads_match_pinned_service_results ... ok <31.754s>
test controllers::github::connection_tests::github_connections_http_identity_flash_revocation_and_audits_match_rails ... ok <31.163s>
test controllers::messages::paging_tests::root_formats_and_destroy_side_effects_match_rails ... ok <30.423s>
```

New/regression full-path cases and existing full-page acceptance:

```text
test controllers::channel_threads::content_tests::conversation_and_room_composer_match_rails_bytes_through_http ... ok <14.981s>
test controllers::channel_threads::page_tests::pull_request_thread_pages_match_four_rails_http_responses ... ok <2.916s>
test controllers::channel_threads::page_tests::pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure ... ok <0.717s>
test controllers::channel_threads::page_tests::seeded_pull_request_thread_header_matches_rails_bytes_through_http ... ok <0.647s>
test controllers::rooms::full_page_tests::full_native_room_pages_match_four_complete_rails_pages ... ok <1.584s>
test controllers::rooms::full_page_tests::empty_room_shell_and_all_owner_children_match_rails_through_http ... ok <3.923s>
```

No timing threshold was widened, concurrency was not lowered from this branch's four-thread baseline, and coverage was not cut. The fresh workspace ran 2805 passes versus 2802 previously; no timing flake occurred.

## Auxiliary historical recorder limitation

An additional worktree command `PARITY_IMAGE=ws8br-reference-d7c7de92 python3 rust/reference-tools/messaging/check-goldens.py thread-content thread-pages` failed the historical thread-pages file comparison because that corpus deliberately uses the approved #163 layout/assets. A fresh-clone rerun with `PARITY_IMAGE=ws8br-reference-status-2e20b24c` reproduced all twenty-eight complete response rows identically, but the existing byte-file checker still rejects the extra `layout_reference` annotation already in the committed vector and absent from its recorder output. No HTTP response byte differs in the approved-layout reproduction. No expectation or checker was altered. This existing recorder-metadata mismatch is inventoried for WS8b-m; the fresh seeded suite and new oracles above pass.

Raw auxiliary lines:

```text
WS8bm thread-content oracle: 9 actual requests; anchor scope and fixed-secret conversation/composer bytes
WS8bm thread-pages oracle: 28 actual Rails requests; state lists, standalone HTML/JSON, latest replies and deleted starter
AssertionError: thread-pages.json: golden bytes differ
Approved-layout thread-pages diagnostic: 28 complete response rows identical; checker differs only on existing layout_reference metadata
```

## Exact remaining work

No requested P2 or injected-child integration fix remains. The wider workstream stays partial:

- The nine original controller deferrals below remain flagged; eight wait for WS13 and one conflicts with fixed queue decision 2.
- Configured huddle header/sidebar/full-page participant/grant/stage adapters remain WS13 integration work.
- Original system behavior declarations still unmapped in `plans/ws8br-rails-cases.json` and `ws8br-system-mappings.json` remain inventoried; the prior browser receipts were not rerun in this review slice. This includes `muted rooms dim and stay quiet until mentioned`, configured header cases, the twelve supplemental room-audit declarations without individual original-case receipts, and the inbound negative browser mutation that previously stopped at Rails setup ERR_ABORTED. WS8br2 owns users/profiles/accounts/tour/public/PWA/QR; stars belong to WS12 and call/agent domains retain WS13/WS11.
- Additional full-page viewer/anchor/unread combinations and configured voice/stage/board pages remain beyond the measured eight page captures and existing strict component regions.
- The auxiliary historical thread-pages recorder metadata mismatch above remains for WS8b-m. It is separate from runtime byte acceptance.

Original mappings re-verified by the fresh Rust log:

| Original file | Passing Rust mapping receipts | Deferred |
| --- | ---: | ---: |
| `test/controllers/rooms_controller_test.rb` | 28 | 1 |
| `test/controllers/rooms/opens_controller_test.rb` | 15 | 0 |
| `test/controllers/rooms/closeds_controller_test.rb` | 12 | 0 |
| `test/controllers/rooms/directs_controller_test.rb` | 29 | 0 |
| `test/controllers/rooms/involvements_controller_test.rb` | 8 | 0 |
| `test/controllers/rooms/refreshes_controller_test.rb` | 4 | 0 |
| `test/controllers/rooms/reads_controller_test.rb` | 7 | 0 |
| `test/controllers/rooms/members_controller_test.rb` | 13 | 0 |
| `test/controllers/rooms/categories_controller_test.rb` | 5 | 0 |
| `test/controllers/rooms/favorites_controller_test.rb` | 6 | 0 |
| `test/controllers/rooms/inbound_email_addresses_controller_test.rb` | 8 | 0 |
| `test/controllers/room_categories_controller_test.rb` | 6 | 0 |
| `test/controllers/switchers_controller_test.rb` | 5 | 0 |
| `test/controllers/users/sidebars_controller_test.rb` | 6 | 8 |

Exact original deferrals:

- `test/controllers/rooms_controller_test.rb` — **destroy succeeds when the queue is down and the sweep recovers the room**: Lead decision 2 requires atomic queue rollback; native fault-injection coverage is separate.
- `test/controllers/users/sidebars_controller_test.rb` — **channel row shows the live huddle stack with names and count**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **board row shows the live huddle stack with names and count**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **direct row shows the live huddle stack when the peer is in the call**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **quiet rows keep an empty stack target with no visible presence**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **direct row re-renders when a participant joins**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **group direct rooms render member names and a huddle stack**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **no channel or DM stacks without huddle configuration**: WS13 huddle grant/presence integration and full-request query instrumentation.
- `test/controllers/users/sidebars_controller_test.rb` — **sidebar query count does not grow with quiet channels, DMs, boards, and stages**: WS13 huddle grant/presence integration and full-request query instrumentation.

There are no screenshot/pixel remaining items. Raw baseline/fixed/mutation logs are retained in `.scratch/review-fixes/`; independent suite/oracle/clippy receipts in `.scratch/fresh15/repo/.scratch/`. Failure-byte diagnostics from the canonical tests remain under its normal rust/target output directory. The fresh-clone target is deleted after verification, with no fresh-clone processes left. The external report and tracked mirror contain identical bytes.
