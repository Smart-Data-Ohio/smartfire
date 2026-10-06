# WS11 API next-5 checkpoint — partial

Branch: `rust/ws11api-next-5`. Verified code/evidence snapshot:
`599eeb849f15ec3f79ebb87bf1cab26b0deba098`, based on main
`f85fb4200e30d68f2a5e811b30fbe4d5bc1f397b`. Both requested main merge checks
returned `Already up to date.` No pushed history was rewritten.

## Complete versus remaining

All 16 newly available WS12 assignment declarations are closed against freshly
executed pinned Rails. The domain inventory now has **378 passed, 0 failed,
0 deferred** across 26 files. Each of the 16 has an activated producer mutation
rejected at its intended complete persisted-fact assertion.

The broader #215 assertion audit contains a separate 51 WS11-API declarations.
This checkpoint closes **20** with **31 fresh Rails HTTP/MCP responses**, exact
body bytes/status/selected headers and complete committed work/history/ledger/
handoff/audit/jobs. Each has its own activated negative control rejected at its
response or state assertion. Real set/clear and set/repeat writes replace the
previous direct pre-seeding shortcut. Valid foreign-owner payloads, other-room
grants, suspended receivers, real session/CSRF and authenticated bot keys are
isolated. No WS11-API observable behavior discrepancy was exposed in these cases.

**31 broader API declarations remain unblocked WS11-API evidence work. This is
partial; only owner-blocked items do not remain.** Exact names, original file/line,
reasons and the passed test/vector/control mappings are in
`ws11api-named-api-cases.json` and `ws11api-remaining-scope.md`.

The actual unmerged source integration remains with **WS12,
`rust/ws12-board-automations-4`**: default Recorder handling at
`rust/crates/db/src/models/activity_item/recorder.rs:377` requires an explicit
AgentBudgetNotice owner reader; its contract is at `recording_source.rs:47`.
The typed WS11 reader and presenter consumers are already installed. No WS12
board-automation file was changed. The queued #220 P3 remains with WS8.

## Files and design

- `case-ports.json`: map all 16 owner test functions in pinned source order.
  `check-case-ports.py` discovers `cases!` as well as `named_cases!`, with the
  word boundary retained; `test-case-ports.py` covers hiding the macro invocations.
- `next5_assignment.rb`: restore the exact archived test source into the disposable
  Rails image, which omits test files, then execute WS12's unchanged real-output
  producer. Source hashes and complete facts remain checked.
- `agent_recorder_cost_test.rs`, `next5_recorder.rb`, `agent_recorder_cost.json`:
  actual persisted recipients/source, transactions, commit frames and uncached
  SQL at 10/100 recipients. The only external broadcast sink records output.
- `agent_work_named_tests.rs`, `next5_work_named.rb`,
  `next5-work-named-inputs.json`, `agent_work_named_http.json`: focused named
  request sequences through the real router/services. The Rails wrapper changes
  harness inputs/sequencing only and retains the existing committed-write state
  extractor. Extra room-scoped grant fixture inputs live only in test preparation.
- `next5-*-controls.{py,json}` and `ws11api-next-5-controls.json`: producer recipes,
  activation/assertion receipts, per-test raw summaries and raw log SHA-256 values.
  Temporary producer changes happen only in the independent scratch clone and
  are restored byte-identically before gates. No vector or assertion is mutated.
- `check-named-api-cases.py`: verify the broader ledger's pinned names, generated
  test functions, real observations and receipts; optionally require cargo's
  actual passes. The existing CI seed checker invokes it.
  `write-case-status.py` preserves the separate broader pending count so an empty
  domain deferral list cannot claim all evidence complete.

No service grant, ledger, audit or transaction logic was reimplemented. No
production domain implementation changed; all new app modules are test-only.

## Failing-first and negative-control evidence

The new assignment-macro discovery regression fails against the original
checker, then passes after the fix:

```text
Ran 4 tests in 0.005s
FAILED (failures=1)
Ran 4 tests in 0.002s
OK
```

All producer selectors below run individual exact tests. Success alone is not
credited: each requires a producer hit, nonzero exit, one failing test and its
declared assertion label. Two initial controls were ineffective and received no
credit: board membership returned from an earlier REST boundary, and result length
had a second model guard. The replacements target that boundary and both length
guards. MCP denial controls also alter the original returned error text.

### 16 WS12 assignment declarations

| Case | Producer selector | Hits | Intended assertion |
| --- | --- | ---: | --- |
| `active` | `owner_active` | 1 | rejected |
| `legacy` | `owner_active` | 1 | rejected |
| `suspended` | `owner_post` | 1 | rejected |
| `nonmember` | `owner_nonmember` | 1 | rejected |
| `no_post` | `owner_post` | 1 | rejected |
| `outside_human` | `owner_human` | 1 | rejected |
| `plain_bot` | `owner_plain_bot` | 1 | rejected |
| `unavailable` | `owner_unavailable` | 5 | rejected |
| `status_note` | `status_note` | 1 | rejected |
| `status_inbox` | `status_inbox` | 3 | rejected |
| `tags` | `tags_string` | 1 | rejected |
| `invalid_fields` | `run_validation` | 1 | rejected |
| `result` | `result_actor` | 1 | rejected |
| `unowned_result` | `model_ownership` | 2 | rejected |
| `unowned_status` | `model_ownership` | 1 | rejected |
| `invalid_status_note` | `note_limit` | 1 | rejected |

### 20 broader API declarations

| Case | Producer selector | Hits | Intended assertion |
| --- | --- | ---: | --- |
| `mcp_not_owner` | `named_work_not_found` | 2 | rejected |
| `mcp_suspended_receiver` | `receiver_denial` | 2 | rejected |
| `posts_nonmember` | `member_denial` | 2 | rejected |
| `posts_credentials` | `bearer_only` | 2 | rejected |
| `show_owned` | `payload_thread` | 1 | rejected |
| `show_not_owner` | `named_work_not_found` | 2 | rejected |
| `patch_not_owner` | `named_work_not_found` | 2 | rejected |
| `manage_other_room` | `manage_denial` | 1 | rejected |
| `ignore_assignment` | `ignored_assignment` | 1 | rejected |
| `tags_only` | `tags_string` | 1 | rejected |
| `blank_status` | `status_validation` | 1 | rejected |
| `note_only` | `missing_field` | 1 | rejected |
| `tags_no_manage` | `manage_denial` | 1 | rejected |
| `result_limit` | `result_limit` | 2 | rejected |
| `result_clear` | `result_blank` | 1 | rejected |
| `result_noop` | `result_noop` | 1 | rejected |
| `writes_credentials` | `bearer_only` | 2 | rejected |
| `show_other_read_room` | `read_access` | 1 | rejected |
| `rest_not_owner` | `named_work_not_found` | 2 | rejected |
| `rest_human_handoff` | `bearer_only` | 1 | rejected |

### Recorder facts and costs

| Case | Producer selector | Hits | Intended assertion |
| --- | --- | ---: | --- |
| `recorder_preflight` | `recorder_preflight` | 10 | rejected |
| `recorder_rows` | `recorder_rows` | 10 | rejected |

Reproduction from an independent source clone with its own target, using
`CI=1`, `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, and the existing rustc wrapper:

```sh
python3 rust/reference-tools/agents/next5-assignment-controls.py install --scratch ../next-5/assignment-controls
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db --no-run
python3 rust/reference-tools/agents/next5-assignment-controls.py run --scratch ../next-5/assignment-controls --binary rust/target/debug/deps/campfire_db-f678350f2be88232
python3 rust/reference-tools/agents/next5-assignment-controls.py restore --scratch ../next-5/assignment-controls
python3 rust/reference-tools/agents/next5-recorder-controls.py install --scratch ../next-5/recorder-controls
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db --no-run
python3 rust/reference-tools/agents/next5-recorder-controls.py run --scratch ../next-5/recorder-controls --binary rust/target/debug/deps/campfire_db-f678350f2be88232
python3 rust/reference-tools/agents/next5-recorder-controls.py restore --scratch ../next-5/recorder-controls
python3 rust/reference-tools/agents/next5-work-controls.py install --scratch ../next-5/work-controls-r3
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire --bin campfire --no-run
python3 rust/reference-tools/agents/next5-work-controls.py run --scratch ../next-5/work-controls-r3 --binary rust/target/debug/deps/campfire-51e126f6c4acd111
python3 rust/reference-tools/agents/next5-work-controls.py restore --scratch ../next-5/work-controls-r3
```

Each install/build/run/restore above was executed this round. Exact per-test
failing summary lines are preserved in `ws11api-next-5-controls.json`.
Raw control rollups:

```text
WS11 assignment producer controls: 16 activated; 16 rejected at intended assertions; 0 unsupported credits
WS11 recorder producer controls: 2 activated; 2 rejected at intended assertions; 0 unsupported credits
WS11 named API producer controls: 20 activated; 20 rejected at intended assertions; 0 unsupported credits
```

## Fresh Rails and read costs

Fresh Rails uses `ws11api-reference:d7c7de92`, one generator at a time, private
restored default-seed copies and a frozen `2026-03-02T16:00:00Z` clock. The executed
runner scripts are `next5_assignment.rb`, `next5_recorder.rb` and
`next5_work_named.rb`, via `rust/parity/bin/reference runner --storage ... --time
2026-03-02T16:00:00Z --freeze`. They generate actual output, not expected constants.
The captured assignment JSON is byte-identical to its owner golden after removing
only the separate stderr summary. Recorder/API JSON is byte-identical to the
committed fresh artifacts. No outcome masks were added.

For Recorder, model/adapter schema discovery is warmed equally before both sizes.
The Rails executor surrounds `uncached`; all recorded cache hits must be zero.
The measured operation includes transaction commit and callbacks; output rows and
frames are read after measuring. The source and user preflight stay at **1 + 1**,
with no service optimization invented here:

| Recorder phase | Rust SELECTs 10/100 | Fresh Rails SELECTs 10/100 | Rust/Rails growth |
| --- | ---: | ---: | ---: |
| Create | 22 / 202 | 22 / 202 | 180 / 180 |
| Repeat | 12 / 102 | 32 / 302 | 90 / 270 |

Raw fresh Rails:

```text
WS11_RECORDER_RAILS size=10 phase=create SELECTs=22 cache_hits=0
WS11_RECORDER_RAILS size=10 phase=repeat SELECTs=32 cache_hits=0
WS11_RECORDER_RAILS size=100 phase=create SELECTs=202 cache_hits=0
WS11_RECORDER_RAILS size=100 phase=repeat SELECTs=302 cache_hits=0
```

Raw Rust and unchanged nine write-path slopes below. The latter compare current
Rust with the existing pinned Rails per-size values, rather than claim a new Rails
HTTP cost recording this round. These fixed gaps remain owner service/rendering
costs, not a new optimization claim.

```text
WS11_RECORDER_READS size=10 phase=create Rust=22 Rails=22 source=1 users=1
WS11_RECORDER_READS size=10 phase=repeat Rust=12 Rails=32 source=1 users=1
WS11_RECORDER_READS size=100 phase=create Rust=202 Rails=202 source=1 users=1
WS11_RECORDER_READS size=100 phase=repeat Rust=102 Rails=302 source=1 users=1
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1361 filtered out; finished in 0.25s
WS12_AGENT_WORK_READS rest_create size=5/50 Rust=143/143 Rails=60/60
WS12_AGENT_WORK_READS mcp_create size=5/50 Rust=141/141 Rails=61/61
WS12_AGENT_WORK_READS rest_update size=5/50 Rust=63/63 Rails=33/33
WS12_AGENT_WORK_READS mcp_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS mcp_board_update size=5/50 Rust=63/63 Rails=34/34
WS12_AGENT_WORK_READS rest_result size=5/50 Rust=25/25 Rails=26/26
WS12_AGENT_WORK_READS mcp_result size=5/50 Rust=25/25 Rails=27/27
WS12_AGENT_WORK_READS rest_handoff size=5/50 Rust=68/68 Rails=55/55
WS12_AGENT_WORK_READS mcp_handoff size=5/50 Rust=68/68 Rails=56/56
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2742 filtered out; finished in 7.74s
```

Commands executed in the fresh clone:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire --bin campfire ws12_agent_work_service_reads_are_reduced_and_flat_at_two_sizes -- --test-threads=8 --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db ws11_recorder_real_output_and_read_growth_match_fresh_rails -- --test-threads=8 --nocapture
```

## Final gates

An independent clone was created with
`git clone --local --no-hardlinks --single-branch --branch rust/ws11api-next-5 .
.scratch/next-5-fresh`, then fast-forwarded to the verified snapshot before final
gates. All producer sources had been restored; only generated scratch files were
untracked. Fresh default/first_run/agents_ui seeds were built there.

The environment retains the machine-wide rustc throttle and uses one target:

```sh
export CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8
export CARGO_TARGET_DIR="$PWD/rust/target" TMPDIR="$PWD/.scratch/tmp"
export CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949
export MAIL_TEST_PORT_RANGE=52920-52949 GITHUB_TEST_PORT_RANGE=52920-52949
export PARITY_OWNER=ws11api-next5 PARITY_NAMESPACE=ws11api-next5-fresh PARITY_IMAGE=ws11api-reference:d7c7de92
```

Commands actually executed from `.scratch/next-5-fresh`:

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -j2 --bin campfire
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 rust/reference-tools/agents/summarize-tests.py ../next-5/workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py ../next-5/workspace.log
python3 rust/reference-tools/agents/check-named-api-cases.py ../next-5/workspace.log
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/test-case-ports.py
```

Raw workspace, runtime comparison and source summaries:

```text
WS11 workspace totals: 4825 passed; 0 failed; 14 ignored; 60 result summaries
WS11 missing-seed skips: 0
WS11 named comparison totals: 378 passed; 0 failed; 0 deferred
WS11 broader named API assertions: 20 passed; 31 pending; owner WS11-API (unblocked)
WS11-api new named sources: 4 pinned test files matched checkout; test sources are not shipped in the Rails image
WS11-api reference sources: 93 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```

Raw strict-clippy completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 50s
```

Raw Rust-only release-input completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 55s
```

All vector suites are included in the workspace run. Fourteen explicitly ignored
tests remain counted as ignored, not executed; no missing-seed skip occurred.
The existing media runner executes native tests first and the six byte-sensitive
app/media tests and storage vectors in the pinned runtime, sequentially. Sizes
and checksums remain compared; no timing thresholds or concurrency were lowered.

The sole created target directory was removed after all checks:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml
test ! -d rust/target
```

Raw cleanup:

```text
Removed 22440 files, 16.9GiB total
```

No Python model server, other worktree, peer branch or WS12 board-automation
implementation was modified. Scratch logs, seed copies and control backups remain
available; compiler targets and test processes do not.
