# WS12: agent work services and tag assignment

Branch: `rust/ws12-boards`. This run completes the lead's item 1 and stops before new human pages/links, recorder/inbox integration or automation work. **Overall WS12 remains partial with unblocked owned work; it is not owner-blocked-only.** The callable boundary for WS11-API and WS8b-m is [ws12-agent-work-api.md](ws12-agent-work-api.md).

Production source is committed at `4ef0e5d0b3b56c9bec639667b5a391912f492abd`, following `f48dafeca57e35026a42841548c35398deee4d40` (services/rules/callbacks) and `4608783d699aeafdf85eb1f41ae3b0260a094e49` (entire stored result comparison and strict-clippy correction). Final verification uses a fresh local clone fast-forwarded to that source SHA, with the independently verified default and first_run seeds copied into it. The final report commit changes documentation only.

## Changes and design

| Files | Completed behavior |
| --- | --- |
| `crates/db/src/models/agent_work.rs` | Public typed board list/create, work list/show/update, result and handoff services. Fresh ownership/membership/read access yields the Rails 404; manage denial precedes field validation and yields the Rails 403. Filters precede the 100-row cap. Creates check the actual WS11 budget before validation. Denied outcomes commit budget notices; genuine database/queue failures roll back. |
| `models/channel_thread/agent_work.rs` | Agent status/tag/run URL/result writes with omitted versus explicit-null fields, Ruby `.to_s` semantics, Unicode limits, unchanged-result no-op behavior, fresh owner checks and one history row per real status change. Shared handoff writer creates the package, ownership change, history, audit and real WS11 ledger snapshots in one transaction. |
| `models/work_handoff.rs`, `models/work_thread_event.rs` | Rails package normalization, limits, HTTP(S) checks, shared receiver policy in its original order, history excerpt/counts and durable context snapshots. The agent service's `value || []` converts top-level false collections to empty; the direct model keeps `Array(value)` behavior. |
| `models/board_tag_assignment.rs` | Public rule create/update/destroy/read APIs, Rails validation/normalization/uniqueness/board/assignee checks and dirty-column updates. Rule configuration authorization remains with the controller. |
| `models/channel_thread/tag_assignment.rs`, `models/channel_thread.rs` | Pending tag replacement and added-tag callback. After the original post/tag transaction commits, only the first matching rule in lexical tag order may assign an unowned board post. Current activity, membership and agent post/read grants are rechecked. The separate assignment transaction atomically writes owner/history/ledger/jobs, with nil actor; its failure preserves the committed post/tags. |
| `models/channel_thread/work.rs`, `models/message.rs` | Board creation records the assignment ledger before the opener's real WS11 delivery, matching the actual Rails ledger order. Both delivery paths and their jobs remain inside the source transaction. Ordinary message creation keeps its existing public API and immediate delivery behavior. |
| `controllers/channel_threads/writes.rs` | Reload successful board JSON after the committed auto-assignment callback; complete Rails responses then contain the assigned owner. Ordinary thread creation remains #182's reviewed implementation. |
| `models.rs`, `tests.rs`, `controllers/channel_threads.rs` | Module exports and new test registration. `AgentWorkChanges`, `HandoffPackage`, rule and handoff models are available to peer callers. |
| `crates/db/src/tests/{agent_work_test,board_tag_assignment_test}.rs` | Five agent tests and nine rule tests: real Rails service/validation vectors, stale instances, two independent SQLite writers, access filtering before cap, no-op timestamps, Unicode boundaries, snapshots, grant revocation and rollback. |
| `controllers/channel_threads/agent_work_tests.rs` | Three seeded app tests using the real durable EventSink. Queue inspections use `TestApp::boot_frozen().without_job_runner()`. A rejected receiving-agent webhook enqueue rolls back the entire handoff and earlier sender enqueue; rejected auto-assignment enqueue preserves source post/tags while rolling back assignment. Four complete committed HTTP responses match Rails byte for byte. |
| `reference-tools/agents/{work_services_contract,tag_assignment_contract}.rb`, source hash ledger, `reference-tools/boards/tag_assignment_http.rb`, three vectors | Actual Rails service writes/denials (82), rule validations (18), and committed complete auto-assignment HTTP responses (4). Producers check the relevant Rails source hashes. No response masks. Service differentials compare persisted fields, full result bytes, status/error bodies, history metadata and ledger/context snapshots; dynamic handoff IDs are mapped between the two databases, not claimed as literal HTTP bodies. |
| `reference-tools/agents/work_discriminate.py` | Seven deliberately broken implementations are rejected at actual assertions, with sources restored in `finally`. Compilation/setup failures do not count as discrimination. |
| `plans/ws12-agent-work-api.md`, inventory tool and `plans/ws12-rails-cases.json` | API handoff notes and a per-original-declaration port/defer ledger. Domain implementation does not imply REST/MCP/browser closure. |

All clock-dependent model writes use the existing environment/FrozenClock. Domain code does not render HTML. No agent grant/budget/ledger/delivery/presence code was copied or duplicated: these writers call the merged WS11 APIs. No ignores, production includes outside crates, parity masks, pixel work, Rails edits or PR were added.

## Main and Rails reference

The latest checked origin/main remains `0681adc6894ba93d8e279f1590f9856165bdbfcd` (#182), already merged into the frozen base. It contains #176 and #181; their real APIs and reviewed dirty-column/snapshot implementations are retained. This slice does not modify ActivityItem or UserStar. PR #187's `rust/ws12b-board-writes` branch was not checked out or edited. #187 has not merged into the checked main, so there are no new #187 review versions to adopt in this run. When it merges, take those reviewed versions and preserve the new additions with a merge commit.

Pinned Rails is `d7c7de92` plus `_common.md`'s approved drift. The reference image `ws12-reference:boards-b908ebc2` uses origin/main's owned board drift files: `app/models/board_automations/nudge_pusher.rb` (#162 notification tag), `app/views/channel_threads/_board_post.html.erb` (#164 body class and #165 template/current-room meta), and `app/views/channel_threads/new.html.erb` (#164 body class). Other owned model/service sources remain pinned and are checked against the committed hash ledger.

## Original cases and remaining work

The regenerated inventory contains **488 original declarations: 148 ported, 3 existing peer tests, 337 deferred**. This run closes all 11 original rule model declarations, 10 auto-assignment model declarations and 13 handoff-package/receiver model declarations. It adds 82 actual Rails service cases and 18 validation cases without claiming the original REST/MCP/UI declarations are fully ported. Previously completed board/human-core/recorder-grouping receipts remain historical in the frozen board report; this run reruns their workspace tests and the 82-response board-write comparator.

Precisely remaining after item 1:

- **WS12 item 2:** human handoff controllers and receiver picker, handoff/link permissions and rendered audit/history behavior; WorkThreadLink writes and integration callbacks; work index/pages and ordinary work pane/history/owner-picker rendering. The shared handoff backend exists, but human endpoints and complete original human handoff cases remain.
- **WS12 item 3:** remaining recorder source matrix, generic recorder authorization/preferences/idempotency integration, remaining inbox APIs and replacement of WS11-UI's flagged adapters. This slice does not change the reviewed activity/stars state writers or broadcasts.
- **WS12 item 4:** SLA rule/nudge/digest domain and configuration/pages/jobs, exact FrozenClock boundaries, and `BoardNudgeJob { nudge_id }` committed atomically with its source write. Those automations are not implemented by this slice.
- **Peer integration:** WS11-API must wire its nine flagged board/work REST/MCP writes to the exposed functions and retain its surface authorization/throttling/rendering. WS8b-m has the model APIs for its ten work declarations; seven human-core declarations were previously covered, while the ordinary work pane/history/unavailable-owner/owner-picker render cases still need the WS12 page slice. No claim that all ten HTTP declarations are now closed. WS11-UI owns inbox HTTP/rendering; WS12 supplies its remaining domain integration. Presence is already WS11's API and is reused.

No owner question blocks the next WS12 slice. Stop after the item 1 push, as requested, so the lead can open its PR.

## Current verification receipts

All commands below ran in this session from the assigned worktree. Raw logs remain in `.scratch/logs/`. Build jobs were 2 under the configured machine-wide rustc throttle; tests used 4 threads. No missing-seed skips, new ignores or test processes are left running.

### Fresh checkout and locked metadata

```sh
git clone --local --no-hardlinks --single-branch --branch rust/ws12-boards . .scratch/agent-work-clean/source
mkdir -p .scratch/agent-work-clean/source/rust/parity/.seed .scratch/agent-work-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run .scratch/agent-work-clean/source/rust/parity/.seed/
git -C .scratch/agent-work-clean/source fetch origin
git -C .scratch/agent-work-clean/source merge --ff-only origin/rust/ws12-boards
git -C .scratch/agent-work-clean/source rev-parse HEAD
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/agent-work-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/agent-work-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/logs/agent-work-metadata.json
```

```text
4ef0e5d0b3b56c9bec639667b5a391912f492abd
```

The clone began at the first feature commit and was fast-forwarded after the two coherent corrections. Final metadata and every final workspace/clippy/release-input check use the SHA above. Metadata exits 0 and emits JSON without a textual summary.

### Rails seeds and oracle captures

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/logs/agent-default-seed-check.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/logs/agent-first-run-seed-check.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/agents/work_services_contract.rb" > rust/vectors/agents_work_services_contract.json 2> .scratch/logs/agent-work-rails.log
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/agents/tag_assignment_contract.rb" > rust/vectors/board_tag_assignments_contract.json 2> .scratch/logs/tag-assignment-rails.log
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/boards/tag_assignment_http.rb" > rust/vectors/board_tag_assignment_http.json 2> .scratch/logs/tag-assignment-http-rails.log
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/agent-old-board-write-oracle.json 2> .scratch/logs/agent-old-board-write-oracle.log
cmp .scratch/agent-old-board-write-oracle.json rust/vectors/boards_write.json
```

```text
  "passed": 29,
  "failed": 0
```

```text
  "passed": 4,
  "failed": 0
```

```text
Rails agent work service oracle: 82 cases; real writes and denials; 0 masks
```

```text
Rails board tag assignment oracle: 18 validation cases
```

```text
Rails tag auto-assignment HTTP oracle: 4 complete committed responses; 0 masks
```

```text
Rails board write oracle: 82 complete HTTP responses; no masks
```

All exit 0. Default seed: 29 checks passed; first_run: 4 passed. The original 82-response write vector is reproduced byte for byte. The service producer was strengthened to compare the entire result, then extended from 80 to 82 cases for explicit false handoff collections; the final log above is the 82-case capture. The other two producers were also independently recaptured and matched their committed JSON with `cmp` in this run.

### Failed before correction / broken implementations

```sh
python3 rust/reference-tools/agents/work_discriminate.py > .scratch/logs/agent-work-discrimination.log 2>&1
```

```text
stale-owner: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.08s
sender-manage: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.15s
receiver-read: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.23s
rule-read: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.37s
rule-dirty-columns: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.10s
opener-order: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.15s
atomic-handoff-job: actual assertion rejected broken implementation
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1915 filtered out; finished in 0.72s
WS12 agent work discriminators: 7 broken implementations rejected at actual assertions; 0 compile/setup failures; sources restored
```

These are actual assertion failures, with successful compilation/setup. Each source was restored. The demonstrated production guards are unchanged by the later full-result/falsy-collection corrections; the final workspace below reruns every positive test on the corrected committed source.

The complete committed auto-assignment HTTP comparator also failed before reloading the post after commit: Rust returned a null owner while Rails returned Kevin. Its raw summary was:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1915 filtered out; finished in 1.52s
```

The new actual Rails false-links case failed before service normalization (Rust 422 `Links must be http(s) URLs`, Rails 201 with an empty collection). The same aggregate then passed after the correction:

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db ws12_agent_work_writes_and_denials_match_rails_services -- --test-threads=4 > .scratch/logs/agent-work-false-first.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire_db ws12_agent_work_writes_and_denials_match_rails_services -- --test-threads=4 > .scratch/logs/agent-work-false-fixed.log 2>&1
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.90s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1216 filtered out; finished in 0.57s
```

The first broad attempt at `4608783d` was deliberately interrupted for that uncovered Rails edge case and is retained as `agent-work-workspace-interrupted.log`; it is not a passing workspace receipt. Initial strict clippy found one redundant closure in a new test, corrected before the final checks.

### Full workspace on final committed source

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/agent-work-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/agent-work-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/logs/agent-work-workspace-test.log 2>&1
```

```text
test result: ok. 1913 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 620.93s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.76s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1213 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 114.16s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.48s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.33s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.80s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.96s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.59s
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

Exit 0. Sum of all 60 raw summaries: **3842 passed, 0 failed, 12 existing ignored**; includes vendored html5ever. All 17 new tests ran. The literal-response tests in this full run compare the original 82 write responses, 29 board read responses and four new committed auto-assignment responses. Selected raw test lines:

```text
test controllers::channel_threads::agent_work_tests::failed_auto_assignment_job_keeps_the_post_and_tags_but_rolls_back_assignment ... ok
test controllers::channel_threads::agent_work_tests::handoff_package_owner_history_ledger_audit_and_webhook_jobs_are_atomic ... ok
test controllers::channel_threads::agent_work_tests::tag_auto_assignment_matches_rails_committed_http_response_bytes ... ok
test controllers::channel_threads::board_read_tests::board_post_forms_pages_and_panes_match_complete_rails_http_responses ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test tests::agent_work_test::ws12_agent_work_writes_and_denials_match_rails_services ... ok
test tests::board_tag_assignment_test::ws12_tag_assignment_validations_match_rails ... ok
```

Existing ignored declarations (none added):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
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

### Strict clippy and production-only inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/agent-work-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/agent-work-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/logs/agent-work-clippy.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/agent-work-clean/.scratch" CI=1 mise exec rust@1.98.1 -- .scratch/agent-work-clean/source/rust/ci/with-release-inputs.sh cargo check --locked -p campfire --bin campfire > .scratch/logs/agent-work-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.54s
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.23s
```

Both exit 0 on `4ef0e5d0`. Strict clippy has zero warnings. The release guard checks the production binary target with only crates/manifests and separate allowed asset inputs, without test vectors, parity seeds or reference tools; this receipt is `cargo check`, not a linked release build.

### Inventory

```sh
python3 rust/reference-tools/users/ws12_inventory.py
```

```text
WS12 Rails inventory: 488 declarations; 337 deferred; 3 existing peer tests; 148 ported
```


## Cleanup and stop

All test, build and reference processes have completed. The WS12 Docker-name and 53400–53499 listener checks are empty. Removed the measured 22G `.scratch/target` and the 40K diagnostic-only `.scratch/agent-work-clean/source/rust/target` using deletion bounded by explicit resolved paths and expected contents, after Cargo clean refused their missing cache tags. Both are verified absent and no target directory remains under `.scratch`. Logs, source checkouts, reference images, media tools and seeds are retained. The Python model server was not touched. The final report commit is documentation only; no feature work begins after item 1. **WS12 remains partial with unblocked owned work, not owner-blocked-only.**
