# WS12: PR #193 query-count review fixes

Branch: `rust/ws12-boards`. Both reported P2s are fixed by `a9fe76bf736ba8ae7b31efa2bef28ac41bd25fab`. Final production source is merge commit `5593c6709d93046583358e0dc8eb4afa802a85d2`. The final report commit changes documentation only. This run adds no new feature work and stops after verification and push, as requested. Overall WS12 remains partial: recorder/inbox integration and automations remain unblocked owned work; this is not an owner-blocked-only stop.

The read-only Astra receipt at `/home/riels/.cache/rust-port/ws12r/smartfire/.scratch/pr193-review/summary.txt` was read and was never changed. Its already-confirmed concurrent-link and served-mutation checks were not reimplemented as unrelated review work.

## Changes and query counts

| Surface / fixture sizes | Rust before, at 2443816b | Rust after | Rails uncached SQL |
| --- | --- | --- | --- |
| HTML `/work?state=all`, 10 / 100 tracked threads sharing room and owner | 142 / 1,312 | 18 / 18 | 25 / 113 |
| Ordinary pane, 10 / 100 history entries by the same actor | 47 / 137 | 38 / 38 | 24 / 24 |
| Board pane, same history fixture | 55 / 145 | 46 / 46 | 27 / 27 |
| Unused index event-picker reads, 10 / 100 rows | 10 / 100 | 0 / 0 | Not needed by this row context |

- `controllers/presenters/work_threads.rs` builds the complete page's row facts: message counts, owners and links are loaded once; rooms, display names and owner availability are reused per room. It calls the existing owner policy once per room/owner set, preserving human and agent permissions. `controllers/work_threads.rs` uses this batch presenter for HTML. The JSON path is unchanged.
- `presenters/board_posts.rs` preloads all unique history actors once and keeps history order, names and missing-actor fallback. This shared path serves ordinary and board history. Link item presentation is split from the full panel so index rows do not load unused event-picker choices; the full panel retains its choices.
- `db/models/work_thread_link.rs` adds the page batch reader, retaining ascending link order within each thread. `channel_thread/board.rs` accepts borrowed groups for the existing owner map without cloning complete thread records. No writes, transactions, callbacks or cache invalidation change.
- `controllers/human_work_tests/query_tests.rs` adds two SQL-count regressions, each measuring 10 and 100 before asserting. They exercise actual authenticated HTTP, check rendered row/history counts, use FrozenClock and `TestApp::without_job_runner()`, and stop SQL capture after each request. The index stays within the actual Rails absolute counts and growth budget, with zero unused event-option reads. History assertions require constant counts for both surfaces.
- `reference-tools/work/query_counts.rb` records actual Rails counts and checks the existing 27-source hash ledger; `vectors/work_query_counts.json` stores its raw results. The initial failing index assertion used the same 25/113 Rails budget inline; the final assertion reads those verified values from the vector.

Rust counts are actual reader SQL traces. Rails counts exclude schema/transaction/PRAGMA statements and report uncached and cached queries separately. Response bytes are checked independently by all three complete oracle suites below.

## Fail first against the reviewed source

Before any production edit, both initial regressions were run with production code at `2443816b3a7472ecd0d8bf8097fac97f8d565d05`. The expected exit was 101: both tests reached query-count assertions after successful compilation, HTTP requests and rendered-content checks. They measured both sizes before asserting; these are not compile or fixture failures.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire human_work_tests::query_tests -- --test-threads=2 --nocapture > .scratch/logs/pr193-query-red.log 2>&1
```

```text
WS12 index rows=10: 142 reader SQL; 10 unused event-option queries
WS12 ordinary history=10: 47 reader SQL
WS12 board history=10: 55 reader SQL
WS12 index rows=100: 1312 reader SQL; 100 unused event-option queries
WS12 ordinary history=100: 137 reader SQL
WS12 board history=100: 145 reader SQL
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2134 filtered out; finished in 1.42s
```

## Final human-work and query regressions from the merged fresh clone

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr193-merged/source/rust/Cargo.toml --locked -p campfire human_work_tests -- --test-threads=2 --nocapture > .scratch/logs/pr193-merged-human-tests.log 2>&1
```

```text
WS12 ordinary history=10: 38 reader SQL
WS12 board history=10: 46 reader SQL
WS12 ordinary history=100: 38 reader SQL
WS12 board history=100: 46 reader SQL
WS12 index rows=10: 18 reader SQL; 0 unused event-option queries
WS12 index rows=100: 18 reader SQL; 0 unused event-option queries
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 2236 filtered out; finished in 50.86s
```

## Rails reference and complete response recapture

Pinned Rails is `d7c7de92` plus `_common.md`'s approved drift. The reference image is `ws12-reference:boards-b908ebc2`. Board drift uses origin/main's approved Rails files: #162 nudge tags, #164 body classes and #165 message/current-room metadata. The human work sources remain pinned, and the 27-source ledger verifies their inputs. No new Rails application edit, response mask, whitespace normalization or pixel work is involved.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/pr193/human-merged-recaptured.json 2> .scratch/logs/pr193-merged-human-oracle.log
cmp .scratch/pr193/human-merged-recaptured.json rust/vectors/human_work_http.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/pr193/link-model-merged-recaptured.json 2> .scratch/logs/pr193-merged-link-model-oracle.log
cmp .scratch/pr193/link-model-merged-recaptured.json rust/vectors/work_link_model.json
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/pr193/boards-merged-recaptured.json 2> .scratch/logs/pr193-merged-boards-oracle.log
cmp .scratch/pr193/boards-merged-recaptured.json rust/vectors/boards_write.json
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
Rails work link model oracle: 29 validation/persistence cases; 0 masks
Rails board write oracle: 96 complete HTTP responses; no masks
```

All three producers and all three `cmp` checks exit 0. The complete committed responses are unchanged, including status, body, location, content type and cache-control. The matching Rust tests are part of the final full workspace run.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/query_counts.rb" > .scratch/pr193/rails-query-counts.json 2> .scratch/logs/pr193-rails-query-counts.log
cmp .scratch/pr193/rails-query-counts.json rust/vectors/work_query_counts.json
```

```text
Rails query oracle index size=10: 25 SQL; 14 cached
Rails query oracle json-index size=10: 38 SQL; 50 cached
Rails query oracle ordinary-history size=10: 24 SQL; 12 cached
Rails query oracle board-history size=10: 27 SQL; 17 cached
Rails query oracle index size=100: 113 SQL; 104 cached
Rails query oracle json-index size=100: 308 SQL; 500 cached
Rails query oracle ordinary-history size=100: 24 SQL; 102 cached
Rails query oracle board-history size=100: 27 SQL; 107 cached
```

The JSON index measurement is recorded for completeness; this review fix does not change its presentation or query strategy.

## Verified seeds, fresh checkout and locked metadata

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/logs/pr193-merged-default-seed.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/logs/pr193-merged-first_run-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```text
  "passed": 4,
  "failed": 0
```

```sh
git clone --local --no-hardlinks --single-branch --branch rust/ws12-boards . .scratch/pr193-merged/source
mkdir -p .scratch/pr193-merged/source/rust/parity/.seed .scratch/pr193-merged/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run .scratch/pr193-merged/source/rust/parity/.seed/
cp -a rust/parity/.seed/agents_ui .scratch/pr193-merged/source/rust/parity/.seed/
git -C .scratch/pr193-merged/source rev-parse HEAD
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-merged/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/pr193-merged/source/rust/Cargo.toml --locked --format-version 1 > .scratch/logs/pr193-merged-metadata.json
```

```text
5593c6709d93046583358e0dc8eb4afa802a85d2
```

The default and first_run seed verifiers and locked metadata exit 0. Metadata emits JSON without a textual summary. The fresh local clone has independent Git/source files; it uses this worktree's build target and verified media tools under the existing machine-wide compiler throttle.


## Main merge and the added agent-UI seed

`origin/main` was initially `7c23b097885101b39e432e8628ab9ebb1251d9be`, already merged. It moved to `573762b5987522edadbdb855532c5dc14a88b526` when #188 merged during the gates. Merge commit `5593c6709d93046583358e0dc8eb4afa802a85d2` retains both sides without conflicts. The WS12 batch readers and history preload remain intact; #188's real ActivityItem callers and removed adapters are adopted through the merge. No further recorder or automation feature was added.

The pre-merge fresh workspace passed 4,127 tests with 14 existing ignores, strict clippy and release-input checks; those logs remain separate. Final checks below are rerun on a second fresh clone at the merge commit, with the new required `agents_ui` seed built and independently verified.

```sh
git fetch origin main
git merge --no-ff origin/main -m 'Merge WS11 UI main into PR193 query fixes' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>'
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/seed build agents_ui > .scratch/logs/pr193-agents-ui-seed-build.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/logs/pr193-merged-agents-ui-seed.log 2>&1
```

```text
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

```text
  "passed": 40,
  "failed": 0
```

All commands exit 0. The frozen #187 branch was never checked out or changed.

## Final full workspace tests from the merged fresh clone

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr193-merged/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/logs/pr193-merged-workspace-test.log 2>&1
```

Exit 0. The aggregate below is calculated from all 61 Cargo result lines, pasted unmodified underneath it. No package or test target was excluded. Jobs remained 2 under the configured machine-wide rustc throttle; the test harness used 4 threads.

```text
Workspace aggregate: 4245 passed; 0 failed; 14 ignored; 61 raw test-result summaries
```

```text
test result: ok. 2239 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 825.28s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.57s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1269 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 312.01s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.40s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 13.04s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.04s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 15.39s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.13s
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

The three response/model comparators all passed in this merged run:

```text
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

No ignore was added by this review slice. Existing ignored declarations include the two peer-owned pending WS11-API Google consumers from merged #190:

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

## Final strict clippy and production-only inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/pr193-merged/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/logs/pr193-merged-clippy.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-merged/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/pr193-merged/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/logs/pr193-merged-release-inputs.log 2>&1
```

Clippy raw summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 17s
```

Release-input raw summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.89s
```

Both commands exit 0 on the merged production source. Strict clippy has zero warnings. The release-input guard copies only crates/manifests and the explicitly allowed separate asset inputs: no vectors, reference tools or parity seeds are available to production code. This is a production `cargo check`, not a linked release-profile build.

## Final main check, cleanup and stop

The final fetch still reports `573762b5987522edadbdb855532c5dc14a88b526`, already an ancestor of the merged source. `git diff --check` passes. The fresh clone has no tracked edits; only its regenerable `.scratch/` remains untracked.

```sh
git fetch origin main
git rev-parse origin/main
git merge-base --is-ancestor origin/main HEAD
git diff --check
docker ps --filter name=ws12- --format '{{.Names}}'
ss -ltn '( sport >= :53400 and sport <= :53499 )'
python3 .scratch/pr193/cleanup_targets.py > .scratch/logs/pr193-target-cleanup.log 2>&1
```

```text
573762b5987522edadbdb855532c5dc14a88b526
State Recv-Q Send-Q Local Address:Port Peer Address:Port
```

The Docker-name check is empty. Cleanup measured and removed the 26G build target plus the two 40K diagnostic targets. It validates explicit resolved paths inside this worktree, checks the build markers, and permits only the four verified regenerable JSON filenames in each diagnostic directory. It checks active compiler/test processes rather than matching shell command text. Raw cleanup summary:

```text
WS12 active cargo/rustc/test processes: 0
Removed /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target; exists=False
Removed /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/pr193-clean/source/rust/target; exists=False
Removed /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/pr193-merged/source/rust/target; exists=False
Remaining target directories under WS12 .scratch: 0
```

Logs, oracle captures, fresh checkouts, native media tools and all three verified seeds are retained. No test listener or WS12 reference container remains. No stash was used, the Python model server was not touched, and no message was sent to another person or PR.

This completes the two PR #193 query-count requests. No new feature work started. Overall WS12 remains partial: the remaining recorder/inbox sources and automations are unblocked owned work, so this is **not an owner-blocked-only stop**. The report commit contains documentation only; the source checked by the final gates is the merge commit above.
