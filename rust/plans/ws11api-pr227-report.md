# PR 227 review fixes

Branch: `rust/ws11api-next-6`. Reviewed head: `dbab9218a6bd3cad971372a50c9ce955c863ad68`.
Verified source snapshot: `3c97cfecb`. Current fetched main is `7e35a5fd6`; the requested `git merge --no-ff origin/main` reports `Already up to date.` No history was rewritten. #225 was not on fetched main when verification began.

Both findings are fixed. The existing 51 named API declarations and 378 mapped domain cases remain credited; there are no newly deferred WS11-API cases. The separate AgentBudgetNotice Recorder integration still belongs to WS12 (`rust/ws12-board-automations-4`, PR #225), not this patch.

## Changes

- `campfire/src/integrations/agent_jobs/next6_named.rs` and its test-only exports: one recorder starts before the declaration's assignment/handoff producer and survives through the real durable runner. `deliver` asserts zero observed requests before any job draining. All three next-6 delivery declarations use it, including the no-read case. A per-database test fixture stores its numeric loopback address for mutation routing, avoiding process-global addresses when tests run concurrently. Neither the recorder nor its fixture is in the production build.
- `campfire/src/controllers/agent_next6_named_tests.rs`: initialize that recorder before the step loop and pass it to delivery. The real claim/prepare/sign/post/finish pipeline, outgoing bodies/signatures, durable jobs and committed facts remain asserted.
- `db/src/models/agent_event_polling.rs`: insert `work` before `thread_deleted` for retained snapshots whose thread has been deleted. No query, permission or persistence behavior changes.
- `agent_pr227_tests.rs`, `pr227_deleted_work.rb`, its inputs and `agent_pr227_deleted_work.json`: unmasked fresh-Rails bytes, status and selected headers for both REST and MCP at 5/50 deleted snapshots. Each response must contain every retained snapshot, with the exact ledger metadata unchanged. Both Rust SELECT counts stay flat; Rails caching is disabled at the executor boundary and zero cache hits are required.
- `pr227-inline-controls.py`, `pr227_inline_reference.rb` and delivery inputs: reproducible real-producer controls for all three delivery declarations. They add a synchronous configured-webhook POST while preserving the original ledger, queue and domain writes. No vector or assertion is changed by a control. Only the physical test dial is routed to the recorder; the configured URL, Host and path remain `http://93.184.216.34:8080/hook`.

No WS12 board-automation implementation, model server, timing threshold or concurrency setting was changed. No allowance or mask was added.

## Failing first and producer evidence

The original review installer was copied read-only into our scratch directory and pointed at our private clone. Against the reviewed producer and original declaration, its exact synchronous POST survived: one independently observed configured-webhook POST, one producer hit, and the unchanged test passed. Its source was then restored byte-identically.

```text
PR227_INLINE_SURVIVOR hits=1 observed_inline_posts=1 original_test_exit=0
PR227_INLINE_PROOF extra inline assignment POST observed; unchanged named delivery assertion still passes
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 1.06s
```

The new deleted-work regressions were installed against the reviewed production code before its key order was fixed. Both failed at their complete fresh-Rails byte assertions:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2776 filtered out; finished in 1.26s
```

The repaired recorder rejects that same producer defect at the pre-drain zero-request assertion. The control retains the reviewer's actual `record()` insertion and configured HTTP authority/path; its physical dial uses the early, per-test recorder rather than a separate parent-process observer. The same check is exercised for assignment without read permission and receiver handoff. Baselines still run the actual durable jobs and compare all committed facts.

```text
PR227_INLINE assignment_webhook baseline: hits=0; exit=0; test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.80s
PR227_INLINE assignment_webhook mutant: hits=1; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.51s
PR227_INLINE assignment_no_read baseline: hits=0; exit=0; test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.78s
PR227_INLINE assignment_no_read mutant: hits=1; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.52s
PR227_INLINE handoff_webhook baseline: hits=0; exit=0; test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.81s
PR227_INLINE handoff_webhook mutant: hits=1; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2777 filtered out; finished in 0.79s
PR227 inline controls: 3 baseline passes; 3 real producer POSTs rejected before job draining
```

Fresh Rails independently rejected each equivalent real-ledger producer mutation at `assignment blocked on external HTTP` (one producer hit, exit 1 per declaration). All three baseline declarations match the committed oracle: 4 HTTP responses, 3 durable producer outputs, zero differences. The freshly executed full next-6 oracle also matches exactly: 31 cases, 182 HTTP responses, 4 producer outputs, zero differences. Control activations, raw summaries, source hashes and log hashes are committed in `ws11api-pr227-controls.json`; temporary library instrumentation is restored before clean-source gates.

## Read counts

| Corpus/interface | Rust 5 / 50 | Fresh Rails 5 / 50 |
| --- | ---: | ---: |
| Deleted-work snapshots / REST | 21 / 21 | 18 / 63 |
| Deleted-work snapshots / MCP | 22 / 22 | 20 / 65 |
| Existing work polls / REST | 25 / 25 | 17 / 17 |
| Existing work polls / MCP | 26 / 26 | 19 / 19 |

There are four new fresh-Rails deleted-work responses, with zero query-cache hits. The pre-drain recorder does not add reads inside any measured HTTP request.

## Commands and verification

All commands below were executed in this round. Logs are under `.scratch/pr227/`. Source checks, strict clippy and the once-only workspace run use the independently cloned source at `.scratch/pr227-fresh`, with the three validated pinned seeds. One cargo invocation runs at a time; jobs are 2, test threads 8, and the machine rustc throttle is unchanged.

```sh
# Worktree; PARITY_OWNER=ws11api-pr227, PARITY_NAMESPACE=ws11api-pr227,
# PARITY_IMAGE=ws11api-reference:d7c7de92; private validated seed directory.
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/pr227_deleted_work.rb
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/next6_named.rb
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/next6_named.rb pr227-delivery-inputs.json
# Run serially for assignment_webhook, assignment_no_read, handoff_webhook:
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/pr227_inline_reference.rb "$key"

# Private clone: install, centrally compile, run, restore; binary path is the
# app executable printed by cargo. No concurrent cargo builds.
python3 rust/reference-tools/agents/pr227-inline-controls.py install --scratch ../pr227/inline-after
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire --no-run
python3 rust/reference-tools/agents/pr227-inline-controls.py run --scratch ../pr227/inline-after --binary rust/target/debug/deps/campfire-51e126f6c4acd111
# From the campfire crate directory, with producer controls disabled:
../../target/debug/deps/campfire-51e126f6c4acd111 pr227_deleted_work ws11_next6 --test-threads=8 --nocapture
python3 rust/reference-tools/agents/pr227-inline-controls.py restore --scratch ../pr227/inline-after

# Validate each seed serially with freshly executed pinned Rails:
bash rust/parity/bin/reference runner --seed "$seed" --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb "$seed"

mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
python3 rust/reference-tools/agents/check-case-ports.py
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
# Byte-exact media tests use the pinned libvips/ffmpeg, without masks.
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

Affected test summaries (the temporary producer branch was disabled for this run; clean-source workspace verification repeats these tests):

```text
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 2744 filtered out; finished in 10.07s
    Finished `dev` profile [unoptimized] target(s) in 1m 29s
WS11 next6 original assertion premises: 31 declarations; real positive selections, mutations, delivery and HTML/JSON projections; 0 missing clauses
WS11 broader named API assertions: 51 passed; 0 pending; owner WS11-API (unblocked)
WS11 source case files: 26 pinned Git files matched; 0 checkout mismatches
```

The pinned seed verifier ran independently for `default`, `first_run` and `agents_ui`, and all three validated. SQLite `integrity_check` returned `ok` for all three. There is one existing message-to-user foreign-key row in default/agents_ui, identical in the earlier seed copies; first_run has none. This work does not change those seed rows.

The additional checker/source checks also ran from the fresh clone:

```sh
python3 rust/reference-tools/agents/test-case-ports.py
python3 rust/reference-tools/agents/check-http-reference.py
```

```text
Ran 14 tests in 2.979s
OK
WS11-api reference sources: 93 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```

The clean-source workspace command exited 0. It ran exactly once, with these raw summaries:

```text
test result: ok. 2766 passed; 0 failed; 6 ignored; 0 measured; 6 filtered out; finished in 551.83s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2772 filtered out; finished in 2.40s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.97s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.99s
test result: ok. 1360 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 124.82s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.68s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.86s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.51s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.59s
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
WS11 workspace totals: 4861 passed; 0 failed; 15 ignored; 60 result summaries
WS11 missing-seed skips: 0
```

The final `git fetch origin` still found main at `7e35a5fd6`; `git merge --no-ff origin/main` again reported `Already up to date.` The post-merge named-case checker passed at 51/51.

Cleanup executed after all tests and cargo processes finished:

```sh
# Private clone, with CARGO_TARGET_DIR=$PWD/rust/target:
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml
test ! -d rust/target
```

```text
Removed 16884 files, 12.8GiB total
```

The created scratch target is absent. Evidence, seeds and source copies remain under our own `.scratch/`; no model-server process or another worker's target was touched.

Runtime named-case verification also executed on the clean workspace log:

```sh
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/pr227/workspace.log
python3 rust/reference-tools/agents/check-named-api-cases.py .scratch/pr227/workspace.log
```

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named comparisons: test/models/agent_test.rb: 41 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 40 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 23 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 15 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 9 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 378 passed; 0 failed; 0 deferred
WS11 next6 original assertion premises: 31 declarations; real positive selections, mutations, delivery and HTML/JSON projections; 0 missing clauses
WS11 broader named API assertions: 51 passed; 0 pending; owner WS11-API (unblocked)
```
