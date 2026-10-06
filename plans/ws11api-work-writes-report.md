# WS11-api: all nine WS12 work writes

Complete checkpoint on `rust/ws11api-work-writes`. Tested production source:
`264523a95e8a976034b9598af7de9e321775da57`. Reviewed parent remains
`928531fab99a253b04224a879408cae252879f20`; its branch was neither edited nor pushed.
Merge commit `5b96d0b24` brings that parent into this stacked branch and includes
current main `b98904b88032edfc8e06e2b765380a1d944e9afa` (#193 and #200).
The initial and final explicit `git merge --no-ff origin/main` checks both reported
already up to date. Final fetch: zero commits in HEAD..origin/main. Locked metadata passes.
The final report commit changes documentation only.

Saved WIP `e068d0893` was checked and amended, with the user's authorization, into
`07e6d94d9` (adapter preparation, not a claim of complete parity). Follow-up
`264523a95` completes parity and query/rollback controls. No stash or rebase was used.

## Per-path completion and query growth

Every row below passes raw response body/status/selected-header comparisons and
persisted-state/job comparisons against requests to pinned Rails `d7c7de92`.
The case counts include success, permission and validation cases, not just successes.
Each interface has owner/non-owner, revoked grants, room membership and scope,
inactive agent and revoked/expired credential coverage. The prior 99 work-validation
vectors also pass. Permission denials deliberately precede invalid field errors where
Rails does; budget notices remain committed when the service returns a denial.

| Path | Success HTTP status | New Rails cases | Rust SELECTs, 5 / 50 owned rows | Rails SELECTs, 5 / 50 |
| --- | --- | ---: | ---: | ---: |
| POST /rooms/:room_id/agents/posts | 201 | 33 | 209 / 209 | 60 / 60 |
| PATCH /agents/work/:id | 200 | 25 | 85 / 85 | 33 / 33 |
| PUT /agents/work/:id/result | 200 | 23 | 35 / 35 | 26 / 26 |
| POST /agents/work/:id/handoff | 201 | 28 | 113 / 113 | 55 / 55 |
| MCP create_board_post | 200 | 34 | 197 / 197 | 61 / 61 |
| MCP update_work | 200 | 24 | 85 / 85 | 34 / 34 |
| MCP update_board_post | 200 | 24 | 85 / 85 | 34 / 34 |
| MCP set_result | 200 | 24 | 35 / 35 | 27 / 27 |
| MCP handoff_work | 200 | 29 | 113 / 113 | 56 / 56 |

Total: 244 new Rails requests. All five MCP success envelopes have `isError:false`;
MCP transport status is 200, including operations whose shared service status is 201.
Error envelopes/codes and Content-Type, Cache-Control, Pragma, Retry-After and Location
are compared exactly. No response fields are masked.

Query counts include SELECTs on both the writer connection and reader pool, including
model callbacks and response rendering. A separate real SELECT control proves that both
capture mechanisms record their own connection and do not count each other. Rails counts
uncached SQL through sql.active_record. Both apps have zero growth from 5 to 50 rows on
all nine paths; Rust's fixed counts are higher. This is a slope regression, not a claim
of equal absolute counts.

## Files and design

- `crates/campfire/src/controllers/agents/work_writes.rs`: thin adapter to WS12's
  `agent_work::{create_board_post,update_work,set_result,handoff_work}` APIs. Only
  HTTP/MCP coercion, caller-owned board authorization, audit context and payload rendering
  live here. Recheck the current Agent under the writer lock and return model denials as
  successful database operations. Reload the committed thread after tag-assignment callbacks.
- `controllers/agents/pending.rs`, `controllers/agents.rs`: dispatch the nine paths to the
  adapter; delete the obsolete `agents/work_validation.rs` stand-in. Preserve REST nested
  parameters, top-level precedence and REST/MCP's different handoff summary coercions.
- `controllers/agent_work_writes_tests.rs`, `controllers.rs`: nine grouped request suites,
  two-size query regressions for every surface, query-capture control, and injected insert
  failures through every path. Each rollback control first proves its normal write succeeds.
- `controllers/agent_reads_tests.rs`: reuse fixture/request/wire checks, stop the background
  runner with main's without_job_runner(), and assert the work-write state projection.
- `reference-tools/agents/work_writes_http_contract.rb`, `vectors/agent_work_writes_http.json`:
  real committed requests, restoring a private seed between cases. No enclosing transaction
  suppresses Rails callbacks or jobs. Record thread metadata/tags/result timestamps, opener
  messages, handoff history, audit actor/IP/UA, ledger snapshots/hops/status, and queued job
  classes/arguments. Random ledger chain UUIDs are checked independently for validity and
  identity across the handoff; job representations compare logical class/arguments across
  the different queue implementations. These are explicit state projections, not response masks.
- `reference-tools/agents/{record-http-vectors,verify-http-vectors,check-http-reference}.py`:
  include the new oracle and four additional WS12 Rails sources; retain all parent R5 artifacts.
- `reference-tools/agents/work-writes-failing-first.txt`: immutable original nine-path red receipt.
- `plans/ws12-agent-work-api.md`: correct the documented opener-body coercion to Rails string
  column casting (`false` becomes `f`). This is a contract clarification, not a service rewrite.

No WS12 model, grant, ledger, audit or queue logic was reimplemented. WS12 supplies the
work writer, handoff backend and tag-assignment callbacks. Existing repository access stays
with its integration owner. The nine old pending seams no longer produce generic 500/-32603
for legitimate writes; genuine injected internal faults still produce those internal errors.

Rollback controls reject background_jobs inserts for create/handoff or work_thread_events
inserts for update/result. All rows in ten tables remain byte/value identical to their
pre-request snapshots: channel_threads, messages, thread_memberships, thread_tags,
work_thread_events, work_handoffs, agent_events, audit_logs, activity_items, background_jobs.
Successful vectors separately assert the precise committed source/history/audit/ledger/jobs.

## Failing-first evidence

Production baseline `6fc6cbe24` already had WS12's services but the nine API adapters were
pending. Commit `d588da210` introduced the nine endpoint regressions before the production
adapter changes. Every one failed on its success response: actual REST 500 (expected 201/200)
or actual MCP -32603 (expected the Rails success envelope). The saved raw receipt and original
log are retained; these are the earlier interrupted run's red results, not claimed fresh-clone
successes. Each new permission group shares that same failing-first positive control.

```text
test controllers::agent_work_writes_tests::agent_work_writes_rest_result ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_rest_create ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_rest_handoff ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_mcp_handoff ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_mcp_update ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_mcp_create ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_mcp_result ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_mcp_board_update ... FAILED
test controllers::agent_work_writes_tests::agent_work_writes_rest_update ... FAILED
test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 2526 filtered out; finished in 1.20s
```

The first resumed run then exposed two persisted-state failures: boolean opener body was
stored as `false` instead of Rails' `f` in both REST and MCP. The response bytes alone would
not have caught it. Correcting string-column coercion makes these state regressions pass.

```text
test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 2540 filtered out; finished in 22.01s
```

Final native targeted run, including all query and rollback controls:

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2540 filtered out; finished in 26.78s
```

## Fresh-clone verification

All commands below were actually run for this checkpoint, from the stated worktree.
The fresh clone was made at the tested production SHA, with --no-hardlinks and --no-local.
All thirteen local workspace packages were cleaned before rebuilding; dependency artifacts
were the only reused build output. Default, first_run and agents_ui seeds were rebuilt inside
the clone using the pinned reference image. Seed creation completed successfully:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-work-writes . .scratch/work-writes-final-fresh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/work-writes-final-fresh/rust/Cargo.toml --format-version 1 > .scratch/work-writes/fresh-metadata.json
```

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/work-writes-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/work-writes/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/work-writes-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/work-writes/logs/fresh-workspace.log 2>&1
python3 .scratch/work-writes-final-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/work-writes/logs/fresh-workspace.log > .scratch/work-writes/logs/fresh-workspace-summary.log 2>&1
```

Full workspace exit101: seven native media/version failures, exactly the seven on the
reviewed parent. They are counted as failures, not skipped or masked. The same freshly built
binaries pass every one in the pinned runtime below. Every raw workspace summary:

```text
    Finished `test` profile [unoptimized] target(s) in 4m 37s
test result: FAILED. 2539 passed; 6 failed; 7 ignored; 0 measured; 0 filtered out; finished in 617.23s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.93s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1282 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 130.06s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.47s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.74s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.48s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.17s
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
```

```text
WS11-api cargo totals: 4548 passed; 7 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

Native failures (libvips8.18.6/ffmpeg9.0.2 versus pinned libvips8.16.1/ffmpeg7.1.5):

- controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers
- controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect
- pipeline_matches_the_reference

Six are exact native media byte/size/checksum differences; one is the storage version guard.
There are zero actual seed skips. The one missing-seed notice comes from a deliberate unit
test of the missing-seed diagnostic. Sixteen pre-existing ignored tests remain counted;
the two polling HTTP comparisons are also executed explicitly below.

## Strict clippy and release-input build

Both commands exit0. Cargo uses two jobs, the configured machine-wide rustc throttle stays
intact, test runners use at most eight threads, and no timing thresholds were changed.

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/work-writes-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/work-writes/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/work-writes-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/work-writes/logs/fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 31s
```

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/work-writes-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/work-writes/tmp" mise exec rust@1.98.1 -- bash .scratch/work-writes-final-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/work-writes/logs/fresh-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 2m 01s
```

## Pinned-runtime tests and polling comparisons

These use the unchanged fresh-clone test binaries in ws11api-reference:d7c7de92.
The fresh clone is mounted read-only; the owned TMPDIR is the only writable bind mount.
Each test runner completes before the next begins. All commands exit0.

```bash
docker run --rm --name ws11api-work-writes-pinned --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/work-writes/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/work-writes-final-fresh:$PWD/.scratch/work-writes-final-fresh:ro" -v "$PWD/.scratch/work-writes/tmp:$PWD/.scratch/work-writes/tmp" --entrypoint "$PWD/.scratch/work-writes-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 agent_work_writes_ --nocapture --test-threads=8 > .scratch/work-writes/logs/pinned-work-writes.log 2>&1
```

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2540 filtered out; finished in 63.60s
```

```bash
docker run --rm --name ws11api-work-writes-pinned-pr192 --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/work-writes/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/work-writes-final-fresh:$PWD/.scratch/work-writes-final-fresh:ro" -v "$PWD/.scratch/work-writes/tmp:$PWD/.scratch/work-writes/tmp" --entrypoint "$PWD/.scratch/work-writes-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8 > .scratch/work-writes/logs/pinned-pr192.log 2>&1
```

```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 2521 filtered out; finished in 24.31s
```

```bash
docker run --rm --name ws11api-work-writes-pinned-logo --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/work-writes/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/work-writes-final-fresh:$PWD/.scratch/work-writes-final-fresh:ro" -v "$PWD/.scratch/work-writes/tmp:$PWD/.scratch/work-writes/tmp" --entrypoint "$PWD/.scratch/work-writes-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/work-writes/logs/pinned-logo.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2551 filtered out; finished in 1.10s
```

```bash
docker run --rm --name ws11api-work-writes-pinned-storage --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/work-writes/tmp" -v "$PWD/.scratch/work-writes-final-fresh:$PWD/.scratch/work-writes-final-fresh:ro" -v "$PWD/.scratch/work-writes/tmp:$PWD/.scratch/work-writes/tmp" --entrypoint "$PWD/.scratch/work-writes-final-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/work-writes/logs/pinned-storage.log 2>&1
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.89s
```

Raw two-size SELECT and injected-rollback results from the pinned work-write run:

```text
WORK_WRITE_QUERIES rest_create owned_rows=5/50 Rust_all_SELECTs=209/209 Rails_all_SELECTs=60/60
WORK_WRITE_QUERIES rest_update owned_rows=5/50 Rust_all_SELECTs=85/85 Rails_all_SELECTs=33/33
WORK_WRITE_QUERIES rest_result owned_rows=5/50 Rust_all_SELECTs=35/35 Rails_all_SELECTs=26/26
WORK_WRITE_QUERIES rest_handoff owned_rows=5/50 Rust_all_SELECTs=113/113 Rails_all_SELECTs=55/55
WORK_WRITE_QUERIES mcp_create owned_rows=5/50 Rust_all_SELECTs=197/197 Rails_all_SELECTs=61/61
WORK_WRITE_QUERIES mcp_update owned_rows=5/50 Rust_all_SELECTs=85/85 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_board_update owned_rows=5/50 Rust_all_SELECTs=85/85 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_result owned_rows=5/50 Rust_all_SELECTs=35/35 Rails_all_SELECTs=27/27
WORK_WRITE_QUERIES mcp_handoff owned_rows=5/50 Rust_all_SELECTs=113/113 Rails_all_SELECTs=56/56
WORK_WRITE_ATOMIC rest_create rejected_table=background_jobs all_10_tables_unchanged=true
WORK_WRITE_ATOMIC rest_update rejected_table=work_thread_events all_10_tables_unchanged=true
WORK_WRITE_ATOMIC rest_result rejected_table=work_thread_events all_10_tables_unchanged=true
WORK_WRITE_ATOMIC rest_handoff rejected_table=background_jobs all_10_tables_unchanged=true
WORK_WRITE_ATOMIC mcp_create rejected_table=background_jobs all_10_tables_unchanged=true
WORK_WRITE_ATOMIC mcp_update rejected_table=work_thread_events all_10_tables_unchanged=true
WORK_WRITE_ATOMIC mcp_board_update rejected_table=work_thread_events all_10_tables_unchanged=true
WORK_WRITE_ATOMIC mcp_result rejected_table=work_thread_events all_10_tables_unchanged=true
WORK_WRITE_ATOMIC mcp_handoff rejected_table=background_jobs all_10_tables_unchanged=true
```

```bash
env CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/work-writes/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/work-writes-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/work-writes/logs/polling-explicit.log 2>&1
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2550 filtered out; finished in 0.85s
```

## Rails source and vector suites

All 22 oracle artifacts were recaptured from private pinned Rails seeds for this checkpoint.
The new artifact has 244 requests; the existing 21 artifacts retain their bytes. The suite
contains 1,735 ordinary request vectors plus two approved media-difference cases and three
handled missing-representation scenarios (21 exact response sets). Approved differences
retain their explicit Rails and Rust outcomes; they are not falsely described as identical
Rails/Rust success statuses. Artifact bytes themselves match the fresh Rails recapture.
Source, verification and planted-mutation verifier checks are rerun from the fresh clone.

```bash
env PARITY_NAMESPACE=ws11api-work-writes-resumed python3 rust/reference-tools/agents/record-http-vectors.py .scratch/work-writes/resumed-oracles > .scratch/work-writes/logs/resumed-oracles.log 2>&1
python3 .scratch/work-writes-final-fresh/rust/reference-tools/agents/check-http-reference.py > .scratch/work-writes/logs/fresh-source-check.log 2>&1
python3 .scratch/work-writes-final-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/work-writes/resumed-oracles > .scratch/work-writes/logs/fresh-vector-verification.log 2>&1
python3 .scratch/work-writes-final-fresh/rust/reference-tools/agents/test-http-vector-verifier.py > .scratch/work-writes/logs/fresh-verifier-injections.log 2>&1
```

```text
WS11-api reference sources: 88 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 269 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 71 request/response pairs; byte-identical committed vectors
WS11-api fresh legacy bot/fanout/replacement oracle: 50 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh readers oracle: 192 request/response pairs; byte-identical committed vectors
WS11-api fresh pins oracle: 18 request/response pairs; byte-identical committed vectors
WS11-api fresh polls oracle: 168 request/response pairs; byte-identical committed vectors
WS11-api fresh polling oracle: 12 request/response pairs; byte-identical committed vectors
WS11-api fresh reactions oracle: 44 request/response pairs; byte-identical committed vectors
WS11-api fresh bot reactions oracle: 13 request/response pairs; byte-identical committed vectors
WS11-api fresh work validation oracle: 99 request/response pairs; byte-identical committed vectors
WS11-api fresh work writes oracle: 244 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 zones and ID shapes oracle: 59 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved JPEG difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved video difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 handled missing representations oracle: 3 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```

```text
.............
----------------------------------------------------------------------
Ran 13 tests in 0.013s

OK
```

## Cross-workstream boundaries and remaining items

All four REST and five MCP WS12 write adapters are complete; no service gaps were found.
The API boundary document has only the observed string-casting clarification. The shared
WS12 domain remains the authority for authorization, validation, histories and callbacks.
The deleted stand-in owns none of those operations any longer.

Pinned request differentials cover the write behavior of Rails agents/posts_controller_test,
agents/work_controller_test and agents/work_handoff_test, plus all five MCP aliases. The
writer/history/tag model tests remain WS12-owned; no claim is made that every Rails model
test was independently duplicated here. The former nine owner-blocked write paths are
no longer deferred. There are no unfinished items in this work-write slice.

Out of scope and still owner-flagged: WS12's /work.json query-growth fix in #201. This
branch does not touch work_threads.rs. Existing integration seams and approved media
differences from the reviewed parent remain intact. No PR or external message was sent.

All owned test/build/container processes finished. The only extra target directory was
deleted after verification; raw logs, fresh Rails vectors, seeds and fresh-clone source
are retained under .scratch/work-writes and .scratch/work-writes-final-fresh. The Python
model server was not touched. Cleanup receipt:

```text
11G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/work-writes-final-fresh/rust/target
WS11-api scratch target cleanup: complete; logs, vectors and fresh-clone source retained
```
