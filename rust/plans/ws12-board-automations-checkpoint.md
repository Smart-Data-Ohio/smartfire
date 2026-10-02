# WS12 board automations: typed nudge and atomic claim checkpoint

**Partial, coherent checkpoint.** Item 1 is complete: BoardSlaNudge model facts, recorder integration, and the typed inbox reader replace the flagged BoardSlaNudge adapters. Item 4 has a tested atomic claim/notification building block. Items 2 and 3, and connecting item 4 to the recurring dispatcher, remain WS12-owned and unblocked. This is **not** an owner-blocked-only stop.

The branch is `rust/ws12-board-automations`, stacked on the reviewed `0442ed099f8a08944505852fca4029932e314417`. Implementation commit: `841d56bd2`. Main's #193 merge was brought in with merge commit `c5dfa5475b2c898a5c6802609851a58de999394d`; there were no conflicts or source differences in that merge. The reviewed `rust/ws12-boards` branch was not modified during review.

Code/merge HEAD validated below: `c5dfa5475b2c898a5c6802609851a58de999394d`. Latest fetched main: `033ab0ca93a567bb7ddaf239ca29db9d1253a9ff`. The final documentation commit adds this receipt only; the external report records the pushed SHA after publication.

## Changes by file

- `crates/db/src/models/board_sla_nudge.rs`: typed full-row read APIs, Rails validations and duplicate-crossing errors, microsecond elapsed minutes, immutable claim creation, recipient facts, dependent inbox destruction, and `claim_and_notify` with a source savepoint and typed job intent.
- `crates/db/src/models.rs`: exports `BoardSlaNudge` and `NewBoardSlaNudge`.
- `crates/db/src/models/activity_item/recorder.rs`: adds `ActivitySource::BoardSlaNudge(id)`. The stored recipient is the source fact; existing active-human, unknown-event, idempotency, read/handled state and broadcast rules remain the shared recorder's responsibility. Nudges do not participate in message/work-event grouping.
- `crates/campfire/src/controllers/presenters/activity.rs`: one page-wide batch of nudges, threads and rooms, reusing the existing room display-name cache. Removes both flagged nudge SQL readers. JSON keeps Rails' `source: null` whitelist behavior; paths, HTML, capitalization and age formatting remain Rails-compatible. Fixes elapsed-minute flooring at fractional-second boundaries.
- `crates/db/src/models/notification_push.rs`: WS17's existing `BoardNudgeSource` compatibility view now reads the typed model. Its policy, payload, tag, transport and job class remain WS17's implementation.
- `crates/db/src/tests/board_sla_nudge_test.rs` and its registration in `tests.rs`: three model/recorder/clock regressions consume the new Rails vector.
- `crates/campfire/src/controllers/activity_items/tests/board_nudge.rs` and its registration in `tests.rs`: five regressions cover unmasked inbox bytes/permissions, page query counts, atomic enqueue, source/inbox/queue failures, and competing independent SQLite writers. Queue tests use `TestApp::boot_frozen().without_job_runner()`.
- `reference-tools/board_automations/nudge.rb`, `source-hashes.json`, and `vectors/board_sla_nudge.json`: reproducible Rails producer, committed source ledger and generated model/recorder/response vectors.

## Reader API and transaction contract

The inbox/push consumer API is exported at `campfire_db::BoardSlaNudge`:

```rust
BoardSlaNudge::find_by_id(&Connection, id: i64) -> Result<Option<BoardSlaNudge>>
BoardSlaNudge::find(&Connection, id: i64) -> Result<BoardSlaNudge>
BoardSlaNudge::for_ids(&Connection, ids: &[i64]) -> Result<Vec<BoardSlaNudge>>
nudge.waited_minutes(now: Timestamp) -> i64
nudge.activity_recipient_ids() -> [i64; 1]
nudge.recipient_user_ids() -> [i64; 1]
```

Public snapshot fields are `id`, `room_id`, `channel_thread_id`, `recipient_id` (i64), `work_status`, `stage` (String), and `status_entered_at`, `created_at`, `updated_at` (Timestamp). `for_ids` costs one SELECT regardless of page size, and zero for an empty slice. It is a facts reader: callers must first select an authorized inbox page through the existing ActivityItem accessibility API. The presenter then batch-loads the associated ChannelThreads and Rooms. Both flagged BoardSlaNudge adapters are replaced; unrelated WS11 AgentBudgetNotice seams remain with WS11.

The new writer is `BoardSlaNudge::claim_and_notify(tx: &mut Tx<'_>, input: NewBoardSlaNudge, push: bool) -> Result<BoardSlaNudge>`. `NewBoardSlaNudge` has three i64 association IDs and optional `work_status: String`, `stage: String`, and `status_entered_at: Timestamp`, preserving Rails' missing-value validations.

Bare `create` writes only the claim, as Rails does. The opt-in building block writes the claim and the `work_sla` inbox record, and emits `BoardNudgeJob { nudge_id }` when `push` is true. Existing durable-event persistence inserts that job inside the same database transaction. Source, inbox or queue insertion failure leaves all three absent. Two independent writers competing for the same crossing yield one claim, one inbox item and one job. A caller can pass `push: false` for a subsequent stage whose authorized recipient already has a push in the same sweep; both stages still get their inbox items.

Recipient eligibility, board membership, agent-to-human fallback, deleted-board filtering, threshold checks, stale-thread rechecks and the per-sweep pushed-recipient set are dispatcher responsibilities and are **not yet implemented**. This API is not an HTTP authorization boundary. The model validates association existence, status/stage inclusion, entry presence and crossing uniqueness exactly as Rails; it does not add board-only, same-room or active-human validation Rails does not impose. In particular, `done` and a bot recipient are valid stored model claims, while the recorder rejects inactive/bot recipients; the future SLA rule/dispatcher must impose its own narrower rules.

App regressions use FrozenClock. Database regressions freeze TestClock at the same instant and advance it explicitly; minute/hour thresholds include one-microsecond-before, exact-boundary, future-entry and subsequent-crossing cases. Recurring sweep boundaries are deferred below.

## Failing-first receipts

At the unmodified production baseline `0442ed09`, the initial two inbox tests failed at runtime: distinct boards/sources amplified reads, and a status entry one microsecond short of a minute rendered `1 minutes` instead of Rails' `0 minutes`. The baseline test module contained the two reader tests; the three atomic tests were added after the new model API existed. No compile failure is counted as a regression proof.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire board_nudge -- --test-threads=2 --nocapture > .scratch/board-automations/logs/nudge-inbox-before.log 2>&1
```

```text
WS12 BoardSlaNudge inbox: 10 distinct boards/sources; 42 reader SQL
WS12 BoardSlaNudge inbox: 100 distinct boards/sources; 312 reader SQL
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2418 filtered out; finished in 2.89s
```

Because the model API did not exist at the baseline, its tests were proven with compiling, deliberately wrong implementations and then restored byte for byte. The local driver `.scratch/board-automations/prove-nudge-models.py` removes status inclusion and widens the nudge recipient fact, then introduces an elapsed-minute +1, then removes dependent inbox destruction. These are runtime assertion failures. The driver restores both source files in `finally`.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 python .scratch/board-automations/prove-nudge-models.py > .scratch/board-automations/logs/model-mutant-driver.log 2>&1
```

```text
Removed inclusion and widened recipient facts rejected by two runtime regressions.
Elapsed-minute off-by-one rejected by two runtime regressions.
Missing dependent destruction rejected by the recorder/destroy regression.
Restored both source files byte for byte.
```

Raw nudge-validation-recorder-mutant.log:

```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 1279 filtered out; finished in 0.64s
```

Raw nudge-wait-mutant.log:

```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 1279 filtered out; finished in 0.57s
```

Raw nudge-destroy-mutant.log:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1281 filtered out; finished in 0.08s
```

The job proof deliberately flips only `if push` to `if !push`, disabling the required enqueue. The success, rollback and competing-writer tests all reject it. The correct condition was restored before final checks.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire board_nudge -- --test-threads=2 --nocapture > .scratch/board-automations/logs/nudge-queue-mutant.log 2>&1
```

```text
test result: FAILED. 2 passed; 3 failed; 0 ignored; 0 measured; 2418 filtered out; finished in 2.68s
```

Correct-source focused results:

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db board_sla_nudge -- --test-threads=2 --nocapture > .scratch/board-automations/logs/nudge-model-test.log 2>&1
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1279 filtered out; finished in 13.21s
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire board_nudge -- --test-threads=2 --nocapture > .scratch/board-automations/logs/nudge-app-test.log 2>&1
```

```text
WS12 BoardSlaNudge inbox: 10 distinct boards/sources; 15 reader SQL
WS12 BoardSlaNudge inbox: 100 distinct boards/sources; 15 reader SQL
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 2418 filtered out; finished in 3.23s
```

| Full HTML inbox probe | Baseline SELECTs | Typed reader SELECTs |
|---|---:|---:|
| 10 distinct boards, threads and nudge sources | 42 | 15 |
| 100 distinct boards, threads and nudge sources | 312 | 15 |

Counts come from the actual read-connection SQL trace after authentication is warmed. All rows are rendered. No Rails query total was measured for this new probe; the regression enforces flat growth. Dispatchers are still absent, so no dispatcher query-parity claim is made.

## Rails pin and byte comparisons

Reference: `d7c7de92` plus approved drift #162/#164/#165. **origin/main's Rails is the reference for the affected board/work/activity files.** All four entries in the new committed source ledger match origin/main and are independently checked against the actual reference image by the producer. The approved #162 nudge-push tag belongs to the existing WS17 payload code. No Rails source was edited.

The new producer generates 24 model cases, 6 recorder cases, and 5 complete JSON responses plus 5 detached HTML list fragments. It compares recipient access, revoked membership, wrong user, escalation and fractional-minute output without masks. Detached list rendering supplies a token-free partial directly, without rewriting a served response. Full HTTP HTML query tests additionally use the actual controller and presenter; the existing inbox vectors retain their original coverage.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/board_automations/nudge.rb > .scratch/board-automations/nudge-regenerated.json 2> .scratch/board-automations/logs/nudge-rails-final.log
```

```text
Rails BoardSlaNudge oracle: 24 model cases; 6 recorder cases; 5 complete JSON responses plus HTML list fragments; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/board-automations/human_work_http.json 2> .scratch/board-automations/logs/human-oracle.log
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/board-automations/work_link_model.json 2> .scratch/board-automations/logs/link-oracle.log
```

```text
Rails work link model oracle: 29 validation/persistence cases; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/board-automations/boards_write.json 2> .scratch/board-automations/logs/board-oracle.log
```

```text
Rails board write oracle: 96 complete HTTP responses; no masks
```

```sh
cmp rust/vectors/board_sla_nudge.json .scratch/board-automations/nudge-regenerated.json
cmp rust/vectors/human_work_http.json .scratch/board-automations/human_work_http.json
cmp rust/vectors/work_link_model.json .scratch/board-automations/work_link_model.json
cmp rust/vectors/boards_write.json .scratch/board-automations/boards_write.json
```

All four commands exit 0, with no output. No response masks changed. The fresh-clone full suite runs the new consumers, all three existing consuming oracles, #188's inbox vectors and WS17's board push contracts.

## Fresh clone and final gates

All commands run from `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12`. The clone is new; only this worker's regenerable Cargo cache and three verified parity seeds are shared. No package is excluded. Rustc's machine-wide throttle stays configured; no additional build jobs are requested. Focused tests use two test threads and the full suite uses eight.

```sh
git clone --local --no-hardlinks --single-branch --branch rust/ws12-board-automations . .scratch/board-automations-clean/source
mkdir -p .scratch/board-automations-clean/source/rust/parity/.seed .scratch/board-automations-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run rust/parity/.seed/agents_ui .scratch/board-automations-clean/source/rust/parity/.seed/
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/board-automations/logs/default-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/board-automations/logs/first_run-seed.log 2>&1
```

```text
  "passed": 4,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/board-automations/logs/agents_ui-seed.log 2>&1
```

```text
  "passed": 40,
  "failed": 0
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-automations-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/board-automations-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/board-automations/logs/fresh-metadata.json
```

Exit 0; the merged lockfile and fresh clone resolve without modification.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-automations-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/board-automations-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8 > .scratch/board-automations/logs/workspace-test.log 2>&1
```

```text
test result: ok. 2416 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 363.45s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.91s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1278 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 118.47s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.87s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.18s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 29.50s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.58s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.43s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
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

Workspace aggregate: 4431 passed; 0 failed; 16 ignored; 61 target summaries. `CI=1`; no silent missing-seed skip. No new ignore was added.

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
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

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-automations-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/board-automations-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/board-automations/logs/clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 41s
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/board-automations-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/board-automations-clean/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/board-automations/logs/release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 20s
```

The release-input check builds production with development/reference inputs removed, enforcing the approved crates-only production include rule. It is a cargo-check gate, not a linked release-binary build.

## Remaining scope and Rails tests

Completed model contracts are derived from `app/models/board_sla_nudge.rb`. The new nudge-specific cases extend the recorder/access/formatting contracts alongside `test/services/activity_items/recorder_test.rb`, `test/helpers/activity_items_helper_test.rb` and `test/controllers/activity_items_controller_test.rb`. Their surrounding inbox and recorder coverage already exists on main and ran in the full gate; this checkpoint adds the nudge cases and clock boundaries rather than claiming a new wholesale port of those test files. Atomicity and queue failure tests are additional Rust regressions under the approved durable-queue rule.

Still WS12-owned, unblocked and explicitly flagged for the next slice:

1. `BoardSlaRule` model, settings controllers/pages, Rails permission checks and settings audits. Port `test/models/board_sla_rule_test.rb` plus the relevant settings/system interactions.
2. `BoardAutomations::SlaDispatcher`: bulk-load rules/boards/threads/owners/agents/memberships/claims, authorize human recipients and agent fallbacks, handle deleted/done/untracked/stale rows, exact threshold crossings, one claim per crossing/stage, per-sweep push dedupe and isolated failures. Wire the implemented atomic API into this sweep and cover across-many-board query counts and FrozenClock boundaries. Port `test/models/board_automations/sla_dispatcher_test.rb`.
3. `BoardDigestDelivery` and recurring digest dispatcher: stale selection, owner/creator recipients, local-hour/timezone/weekend and per-day behavior, preference/quiet-hour decisions, claim races and failures, batched reads and FrozenClock boundary coverage. Port `test/models/board_automations/digest_dispatcher_test.rb` and `test/jobs/room/destroy_job_board_automations_test.rb` where cleanup extends to those new sources.
4. Register board SLA nudges (five minutes) and board stale digests (one hour) in `crates/campfire/src/jobs/periodic.rs`; port the remaining `test/system/board_automations_test.rb` settings/sweep interactions. No dispatch task was silently registered as a no-op in this checkpoint.

`test/models/board_automations/nudge_pusher_test.rb` is WS17's existing area; its Rust push/policy tests ran and this slice preserves its public source/job contract. The new reader closes WS8b-m2's BoardSlaNudge read seam, and the shared ActivityItem recorder/inbox remain the WS12 domain under WS11-UI's controllers. No ownership approval or external dependency blocks the remaining WS12 work. No pixel phase is proposed.

No open design question is needed for this checkpoint. The approved atomic queue exception is applied to the new opt-in notification operation; generic bare claim creation keeps Rails' callback behavior.

## Cleanup and publication

Owned Cargo targets were measured and removed after compiler/test processes finished. Seeds, native media inputs, the fresh source clone and evidence logs remain. No stash, rebase, unrelated worktree edit, python-model-server operation or external message was performed.

```text
Verified: no active WS12 cargo, rustc or test executable.
40K	.scratch/board-automations-clean/source/rust/target
27G	.scratch/target
Removed .scratch/board-automations-clean/source/rust/target
Removed .scratch/target
Owned scratch targets remaining: 0
```

The pushed SHA and remote readback are appended to the authorized external report after publication. This checkpoint is partial; remaining scope is owned and unblocked.
