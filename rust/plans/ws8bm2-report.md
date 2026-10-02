# WS8bm2 message features B — coherent checkpoint; wider scope partial

Branch: `rust/ws8bm2-message-features-b`, created from `origin/main` 056ab49acffd52007334356ada0bdba3774c0fe2 after #191 merged. Implementation commits: 7398a8d358b804fa1a069e63e537d7316ff6b965 (WS12 consumers) and 6281c29a0c5f2712aaa1e4614b4b9cbe376bac74 (populated provider preloads). Final verification source is 6281c29a0c5f2712aaa1e4614b4b9cbe376bac74; the last commit updates this report only. The pushed SHA is in the final reply.

Fresh-clone full workspace: **4268 passed, 0 failed, 14 existing ignores** across 61 summaries. Strict all-target clippy, the release-input binary build and locked metadata all exited 0. Both new Rails fixtures and all 28 existing oracle fixtures independently replayed byte-for-byte. This checkpoint is ready for review; stopping after the final report push.

At the checkpoint, fetched main remained 056ab49ac. `git merge --no-ff origin/main` returned `Already up to date.` Main is already an ancestor of this new branch, so there was no new merge commit to create. Locked metadata passed immediately after the merge attempt. No rebase, stash, deployment or PR creation.

## Complete in this checkpoint

**WS12 message consumers:** pinned Rails differential for 15 workflow steps (13 HTTP requests and two reminder dispatches). The fixture creates a board post through Rails' real `ChannelThread.create_board_post!` owner API, and Rust consumes those persisted owner rows through its real router. It covers board opener save/reminder, inbox open/handled, reminder re-arm and re-fire into the same item, pin, scheduled board-thread reply/send-now, `/me` in the board thread, wrong-stream scheduling validation, and dependent reminder-inbox deletion. Exact response status, Content-Type and JSON are compared. Four reminder responses exposed the wrong query-parameter order in Rust's source URL; the fixed URL puts `message_id` before `thread`, exactly like Rails.

**Work inbox owner integration:** replaced three raw WorkThreadEvent readers (destination, HTML presentation and JSON source payload) with request-local typed owner model preloads. The accessibility query runs before loading sources. Work events, threads, actors and rooms load in bounded batches; revoked membership produces no serialized work sources. The cost regression exercises the actual `/activity?type=threads` controller at four and sixteen distinct board posts, including reads on the writer connection used by destination resolution. Final Rust counts are 10/10 physical reads and 1/1 WorkThreadEvent reads. Rails counts are 12/24 physical reads and 1/1 WorkThreadEvent reads. Cached Active Record notifications and schema lookups are excluded from Rails' physical-read count.

**Populated provider preloads:** fixed per-message GitHub PR/discussion/account and calendar-event/organizer/venue queries in the production search preload/cache-key path. The warm HTTP search test now uses 51 reads for both four and sixteen messages, versus Rails 35/35, instead of Rust's former 86/206. Each message has both a populated GitHub card and a populated event card. Forty Rails-rendered card fragments compare byte-for-byte inside actual Rust HTTP search responses. These are complete card fragments, rather than assertions on selected text fields.

Shared rendering and owner policies remain in use: private GitHub cards expose only the permitted identity; discussion mappings are scoped to the referring room; calendar-event associations are restricted to the message's room in SQL before deserialization; cards retain Rails order; preloading does not schedule stale refreshes, while fragment misses retain the existing refresh policy. #179's explicit viewer zone and cache-stamp behavior are preserved. Root and quoted-source messages use the same page preload.

## Files and ownership

- `crates/db/src/models/work_thread_event.rs`, `models/channel_thread.rs`: bounded `for_ids` owner readers; existing writers/callbacks unchanged.
- `crates/campfire/src/controllers/presenters/activity.rs`: preloaded WorkThreadEvent facts, owner thread/actor/room associations, and exact reminder/message thread destination URLs. Existing ActivityItem authorization/filtering stays in the owner API.
- `crates/campfire/src/integrations/github/pull_requests.rs`, `github/threads.rs`: page association readers.
- `crates/db/src/models/calendar_event.rs`: room-restricted page event association reader.
- `crates/campfire/src/controllers/presenters/github.rs`, `presenters/events.rs`, `controllers/searches/preloads.rs`: reuse shared projections/renderers with batched persisted facts, explicit viewer zone, existing cache keys and refresh semantics.
- `controllers/message_features/{ws12_consumer_tests,provider_batch_tests}.rs` and module registration: actual HTTP differentials and production-path query regressions.
- `reference-tools/messaging/{ws12_consumers,provider_batch}.rb`, `vectors/messaging/{ws12_consumers,provider_batch}.json`: new independently regenerated pinned Rails vectors.

The four board N+1 files named by the lead are untouched: `presenters/boards.rs`, `models/channel_thread/board.rs`, `models/channel_thread/work.rs`, and `controllers/board_posts.rs`. WS12/WS14g retain ownership of those fixes. No comparison mask, ignored test, browser retry/timeout, timing threshold or test concurrency was changed.

## Remaining and flagged

This is a verified coherent checkpoint. **Wider scope remains partial, with unblocked callback and coercion work outstanding.**

1. Expand populated-provider permutations and older-window callbacks beyond the existing provider corpus and this GitHub/event search case. Existing provider callback/private-endpoint differentials still run, but this slice does not prove all updates/jobs for messages outside the current room window.
2. Expand exceptional scheduling/reminder/slash time grammar, multiparameter and array/hash parameter matrices, and additional DST gap/fold cases beyond the existing pinned corpus. No date parser or coercion production code changed here. WS11-UI's `origin/rust/ws11ui-agent-pages-b` input-casting work was inspected read-only; its shared-parser coordination remains pending. If grammar expansion needs code before that branch lands, coordinate a shared `rails_compat` parsing module with WS11-UI; do not add a second parser.
3. The named read-only `BoardSlaNudge` (WS12) and `AgentBudgetNotice` (WS11) source-reader seams in the activity presenter remain until their owners export typed reader APIs. WorkThreadEvent adapters are removed; #187/#188 API availability is no longer a blocker. Further WS11-API owner integration remains flagged in the inherited tests.
4. Keep the existing real WS17 push transport, WS11 human-agent dispatcher and WS13 integrations. This slice adds no stand-ins or owner-domain writers.

Prior accepted browser inventory, **not rerun for this request**: polls 4/4, pins/saved 7/7, slash commands 26/26, search/files 4/4, scheduled messages 4/4 = 45/45 on each app. Prior controller/deferred-case inventories and the PR #191 rescue-format checkpoint remain in the report history at 950e5de / main 056ab49ac. This checkpoint adds three Rust HTTP regressions backed by two Rails fixture generators; it does not claim new ports of deferred Rails test files or browser cases.

## Failing-first evidence

Work inbox and board consumer tests were run against production at 056ab49ac, with only the new tests/vector/module registration added. Both failed: per-item WorkThreadEvent reads grew with page size, and four reminder responses had a different source URL. The original baseline trace counted read-pool queries only; the final trace also includes destination resolver reads on the writer connection. Rails' baseline count included cached notifications; the final count excludes them. The baseline numbers below are the original raw receipt, not relabeled as final physical-read counts. The failing assertions (flat query cost and exact HTTP payloads) remained.

Baseline 056ab49ac, exit 101:

```text
WS8bm2 WS12 work inbox: 4 results; Rust 12 reads / 4 work reads; Rails 15 reads / 1 work reads
WS8bm2 WS12 work inbox: 16 results; Rust 36 reads / 16 work reads; Rails 39 reads / 1 work reads
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2279 filtered out; finished in 1.26s
```

Fixed code, exit 0; final physical-read counters:

```text
WS8bm2 WS12 work inbox: 4 results; Rust 10 reads / 1 work reads; Rails 12 reads / 1 work reads
WS8bm2 WS12 work inbox: 16 results; Rust 10 reads / 1 work reads; Rails 24 reads / 1 work reads
WS8bm2 WS12 message consumers: 15/15 Rails workflow steps; reminders, re-fire, pin, scheduled board reply, slash, wrong stream and dependent inbox deletion
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2279 filtered out; finished in 0.89s
```

Populated provider cost test against production at 7398a8d35, with only the new regression/vector/module registration added; exit 101. Card comparisons passed before the cost assertion exposed the per-message queries:

```text
WS8bm2 populated provider search: 4 messages; Rust 86 reads; Rails 35 reads
WS8bm2 populated provider search: 16 messages; Rust 206 reads; Rails 35 reads
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2281 filtered out; finished in 1.19s
```

After batching, exit 0:

```text
WS8bm2 populated provider search: 4 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider search: 16 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider containers: 40/40 byte-identical GitHub/event containers in actual HTTP search responses
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2281 filtered out; finished in 1.54s
```

Final focused message-feature suite before the fresh clone, exit 0:

```text
WS8bm2 populated provider search: 4 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider search: 16 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider containers: 40/40 byte-identical GitHub/event containers in actual HTTP search responses
WS8bm2 WS12 message consumers: 15/15 Rails workflow steps; reminders, re-fire, pin, scheduled board reply, slash, wrong stream and dependent inbox deletion
WS8bm2 WS12 work inbox: 4 results; Rust 10 reads / 1 work reads; Rails 12 reads / 1 work reads
WS8bm2 WS12 work inbox: 16 results; Rust 10 reads / 1 work reads; Rails 24 reads / 1 work reads
WS8bm2 role/room matrix: 520 responses; 480 byte comparisons; 0 differences
test result: ok. 174 passed; 0 failed; 0 ignored; 0 measured; 2108 filtered out; finished in 41.75s
```

## Verification setup and executed commands

All final gates run against the committed implementation in a fresh local clone, with freshly built `default`, `first_run` and `agents_ui` seeds. Only the Cargo registry cache is copied; test state and seeds are rebuilt. The owned normal native build cache is reused. CI mode is set so missing seeds fail instead of skipping. The configured machine rustc wrapper and slot file remain mounted; the adapter uses two Cargo build jobs, four unchanged test threads, a four-CPU container cap and owned ports 52500–52599.

Assigned-root setup:

```bash
git clone --no-hardlinks --branch rust/ws8bm2-message-features-b "$PWD" .scratch/ws8bm2-b/fresh
mkdir -p .scratch/ws8bm2-b/fresh/rust/.cargo-home
cp -a rust/.cargo-home/registry .scratch/ws8bm2-b/fresh/rust/.cargo-home/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 bash .scratch/ws8bm2-b/fresh/rust/parity/bin/seed build default first_run agents_ui
```

Raw seed summaries, exit 0:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

Commands from `.scratch/ws8bm2-b/fresh`, using the assigned-root adapter; output is retained under the assigned root's `.scratch/ws8bm2-b/`. Gates run sequentially.

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-b/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture > "$WS8BM2_ROOT/.scratch/ws8bm2-b/fresh-workspace.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings > "$WS8BM2_ROOT/.scratch/ws8bm2-b/fresh-clippy.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > "$WS8BM2_ROOT/.scratch/ws8bm2-b/fresh-release-inputs.log" 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > "$WS8BM2_ROOT/.scratch/ws8bm2-b/fresh-metadata.json" 2> "$WS8BM2_ROOT/.scratch/ws8bm2-b/fresh-metadata.err"
```

## Independent Rails replays

The two new vectors were replayed from separate storage copies of the final clone's newly built default seed, and exact `cmp` checks exited 0. The shell used `set -e`, so the byte-identical receipts were printed only after each successful comparison. The commands were run from `.scratch/ws8bm2-b/fresh` with `PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92`:

```bash
bash rust/parity/bin/reference runner --storage /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-b/final-rails-ws12 --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/ws12_consumers.rb /rails/storage/db/ws12_consumers.json
cmp rust/vectors/messaging/ws12_consumers.json /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-b/final-rails-ws12/db/ws12_consumers.json
bash rust/parity/bin/reference runner --storage /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-b/final-rails-provider --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/messaging/provider_batch.rb /rails/storage/db/provider_batch.json
cmp rust/vectors/messaging/provider_batch.json /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-b/final-rails-provider/db/provider_batch.json
python3 rust/reference-tools/messaging/verify_oracles.py
python3 rust/reference-tools/messaging/features-reference-check.py
```

Raw new vector summaries, both exit 0:

```text
WS8bm2 WS12 consumer Rails: 15 workflow steps; work inbox 4 items=12 reads/1 work reads; 16 items=24 reads/1 work reads
WS8bm2 provider Rails oracle: 11 GitHub containers; 7 embed/LinkedIn containers; 8 fixture tables
WS8bm2 populated provider Rails: 4 messages=35 reads; 16 messages=35 reads; 40 GitHub/event containers
WS8bm2 final WS12 oracle replay: byte-identical
WS8bm2 final populated-provider oracle replay: byte-identical
```

The existing 28 vectors also replay byte-for-byte; raw individual replay and source-check summaries, exit 0:

```text
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

## Full fresh-clone workspace results

WS8bm2 fresh workspace aggregate: 4268 passed; 0 failed; 14 ignored; 61 summaries; exit 0

The complete workspace command exited 0, including vendored html5ever and doctests. All 61 raw summary lines follow; no new ignores were added:

```text
test result: ok. 2277 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 686.29s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.87s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1254 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 190.42s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.90s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.72s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.50s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.83s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.37s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.39s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.05s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.11s
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

Existing ignored declarations (tests skipped versus run):

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

Relevant raw feature receipts from that same full fresh-clone run:

```text
WS8bm2 populated provider search: 4 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider search: 16 messages; Rust 51 reads; Rails 35 reads
WS8bm2 populated provider containers: 40/40 byte-identical GitHub/event containers in actual HTTP search responses
WS8bm2 original rescue format probe: 33 responses; 0 differences
WS8bm2 rescue format matrix: 270 responses; 7 headers and body per response; 60 nonempty public-exception controls; 0 differences
WS8bm2 timer scheduled send: configured HTTPS origin and port delivered over cable
WS8bm2 Rust warm HTTP search: 4 results; 46 SELECT/WITH executions
WS8bm2 Rust warm HTTP search: 16 results; 46 SELECT/WITH executions
WS8bm2 WS12 work inbox: 4 results; Rust 10 reads / 1 work reads; Rails 12 reads / 1 work reads
WS8bm2 WS12 work inbox: 16 results; Rust 10 reads / 1 work reads; Rails 24 reads / 1 work reads
WS8bm2 WS12 message consumers: 15/15 Rails workflow steps; reminders, re-fire, pin, scheduled board reply, slash, wrong stream and dependent inbox deletion
WS8bm2 role/room matrix: 520 responses; 480 byte comparisons; 0 differences
```

## Final lint, release-input and metadata receipts

Strict clippy (`--workspace --all-targets -D warnings`), exit 0:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.20s
```

Release-input binary build, exit 0:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 59.63s
```

Locked metadata exited 0 with 0 bytes on stderr. `git diff --exit-code -- rust/Cargo.lock` in the final clone also exited 0. The release-input guard built with only Dockerfile release inputs, without external vectors, parity files or reference tools.

## Cleanup

The build reused the owned normal `/native-target` cache. The fresh clone also produced four JSON exports under `rust/target`, totaling 40K. A generic recursive deletion command was rejected before execution. From the fresh clone, `CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clean --target-dir /src/rust/target` then exited 101 because the export directory had no Cargo cache marker; it removed nothing. Raw diagnostic:

```text
error: cannot clean `/src/rust/target`: missing or invalid `CACHEDIR.TAG` file
  |
  = note: cleaning has been aborted to prevent accidental deletion of unrelated files
```

After inspecting the complete directory, the four generated files were removed by exact name with ordinary file deletion, and the empty directory with `rmdir`. `test ! -e .scratch/ws8bm2-b/fresh/rust/target` then passed. No extra scratch target remains. The release guard removed its temporary release-input directory. No WS8bm2 test/reference container remains running. Evidence logs, seeds and the clone remain under the owned `.scratch/ws8bm2-b/`. The configured rustc throttle and Python model server were untouched.
