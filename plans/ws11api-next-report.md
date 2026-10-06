# WS11 API next checkpoint (partial)

Branch: `rust/ws11api-next`, stacked on approved #202 (`05bdf0f7`) and merged
with main through #202, #199 and #203 (`b573dd24c`). Protected #202 and #203 branch
heads were not edited or pushed. No WS12 writer, handoff, tag, ledger producer,
audit or permission implementation was replaced. The Rails pin is `d7c7de92`.

## Changes and boundaries

- `controllers/agents/work_writes.rs` uses canonical capability batches for the two
  board checks, in their original order, and the live owner-sealed repository batch
  for response serialization. It still reloads the committed thread after tag callbacks.
- `integrations/agent_repositories.rs` loads bounded private-link candidates before
  identity/account facts. Empty/private-free requests skip unused owner/account reads.
  Actual private links retain the current-owner/account validation, including rendering
  revalidation. Existing disconnect/relink, mixed-link and permission regressions pass.
- `db/models/agent_work_events.rs` uses the same canonical one-statement capability
  batch for webhook eligibility and skips the impossible null-owner lookup. Ledger,
  history, webhook eligibility ordering, atomic queue writes and after-commit behavior
  are unchanged. This is WS11's callback, not a WS12 service rewrite.
- `controllers/agent_work_writes_tests.rs` retains the 5/50 flat-growth and complete
  response comparisons and adds lower fixed-query ceilings for all nine surfaces.
- Three new named Rust test files and two pinned Ruby recorders/goldens compare fifteen
  deferred model/job cases: reply token room/expiry/membership/deactivation, self-assigned
  work hops, kill-switch owned-work preservation, serialized rate check/insertion, and
  nine assignment ledger/history/outer-commit/rollback cases using installed WS12 producers.
- `case-ports.json`, `deferred-domain-cases.json` and `ws11api-remaining-scope.md` reconcile
  exactly those names. No broader API vector is credited as a missing model comparison.

## Read counts

All counts include reader and writer SELECTs plus callbacks and response rendering.
Both sizes are 5/50 owned rows. The baseline is the worker's approved-#202 fixtures;
Astra's independent create fixtures have one additional SELECT (210/198 rather than
209/197), explicitly recorded in its review. No independent post-change count is claimed.

| Surface | Approved #202 worker baseline | WS11-trimmed on main #202 | Rails |
| --- | ---: | ---: | ---: |
| REST create | 209/209 | 197/197 | 60/60 |
| MCP create_board_post | 197/197 | 185/185 | 61/61 |
| REST update | 85/85 | 83/83 | 33/33 |
| MCP update_work | 85/85 | 83/83 | 34/34 |
| MCP update_board_post | 85/85 | 83/83 | 34/34 |
| REST result | 35/35 | 33/33 | 26/26 |
| MCP set_result | 35/35 | 33/33 | 27/27 |
| REST handoff | 113/113 | 105/105 | 55/55 |
| MCP handoff_work | 113/113 | 105/105 | 56/56 |

Remaining repeated service reads belong to WS12: `agent_work::find_owned` loads
thread/membership/read capability; `writable` loads manage capability again; `update_work`
and `set_result` reload the thread returned by the model; `handoff_work` loads sender and
receiver and reloads the result. `ChannelThread::{create_board_post,update_work,update_result}`
validate/load owners and repeat fresh locked-row policy, then schedule full board preloads
for rendering callbacks. Those reviewed service boundaries remain untouched. The main fixed
cost gap remains named with WS12; the API adapters no longer duplicate these policy services.
`work_threads.rs` and `presenters/boards.rs` were not edited.

The first ceilings failed on all nine #202 surfaces before adapter changes. The tighter
callback ceilings failed on create/handoff against first-trim head `690ee97b2` before
callback changes. Raw red receipts are committed in `next-read-trim-failing-first.txt`.

## Named comparisons and negative controls

Fresh Rails model projections preserve row IDs, owners/actors, types, complete metadata,
hop/outcome/details/webhook state, work history and logical event-job arguments. Generated
chain IDs are checked as UUIDs in both apps; they are not replaced with a broad mask.
No existing response/header/body/size/checksum allowlist or mask was changed.

The rate test observes Rails' real rate SELECT inside its lock/transaction. Rust uses
independent SQLite writer connections on the same file for two concurrent requests at
19 eligible deliveries: exactly one additional eligible row/job and one suppression,
both source writes inside transactions. This substitutes the real serialization boundary
for Rails' method spy. No production code is mocked. A first draft attempted blocking
fixture setup inside a Tokio runtime; it failed in setup and was corrected to synchronous
setup plus a current-thread runtime for the two async writers. That harness failure is
retained in `.scratch/next/logs/named-final.log`; it is not counted as a domain defect.

All fifteen new assertions were shown failing against deliberate broken implementations:
ledger disabled (nine failures), token verifier bypass (two), membership bypass (one),
rate check bypass (one), self-assignment hop filter removed (one), kill-switch ownership
cleared (one). Original source was restored after every mutation. Mutation descriptions,
exact commands and raw lines are committed in `next-named-discrimination.txt`.
The correct production behavior already matched the pinned Rails projections; no new
model mismatch was found. Twelve DB comparisons and three application reply comparisons were executed
successfully; mapping alone is never credited as a passing comparison.

## Verification

Both complete fresh workspace runs report 4,573 passed, seven accepted native-media
failures and sixteen ignored, with zero actual seed skips. All fifteen added comparisons
pass, and the merged tree retains the measured nine-path counts above. The first complete
run predates #203; its source/logs are retained separately. The final run after #203 and
repeated clippy/release/oracle/pinned checks are recorded below. The earlier clone was interrupted during
compilation, before tests started, when #199 reached main. It is retained with its logs;
it is not presented as a passing run. The final clone is `.scratch/next-fresh`, after the
merge. Only dependency cache was retained; all thirteen local workspace packages were
cleaned. CI=1, two cargo jobs, unchanged rustc throttle, eight test threads and assigned
port ranges are used. Seeds default/first_run/agents_ui are rebuilt at the pinned Rails
image. No other worker checkout, target or Python model process was touched.

## Remaining scope

**Partial: 312 mapped named comparisons; 66 names remain across six files.**
This is not an only-owner-blocked checkpoint. WS11 still owns four complete streaming
projections, three bot factory/reset/queued-delivery comparisons, and four further
assignment hop/deletion/root callbacks, plus two budget comparisons using WS12
activity-viewer/capped-handoff services. WS12 owns sixteen assignment eligibility/work
mutation names. Changes inside those producer/viewer services stay with WS12.
WS8 owns thirty-five built-in slash-command names; WS11-ui owns two rendered badge/directory names. Every
name and reason is in `ws11api-remaining-scope.md` and the deferred manifest.

All nine API work writes are on main through #202; no pending seam remains in them.
#203 merged during validation; its reviewed proxy-header correction and nine-header
oracle are retained through the final main merge. The final fresh clone is rerun after
that merge. The only conflict was the scope inventory: the current 66-name ledger is
kept and reconciled with #203's now-completed proxy work; no production fix was discarded.
Existing approved media status/state differences remain unchanged.
No service gap or newly unported REST/MCP/token/webhook behavior was identified here.

## Completed final verification receipts

Final verified source head: `25f35cda573313bfc2e6796a94c742e05a80dbc2`, the merge
of main `b573dd24c` (#203). The following report-only commit changes no executable
source or test vectors. Both protected branch heads are retained unchanged.

Commands are run from the assigned worktree. Each cited check was executed in this
round; raw output is retained under `.scratch/next/logs`. The first complete pre-203
run is retained as `pre-203-*`; the lines below are the final merged-tree run.

Common environment for all Cargo checks:

```bash
export CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/next-fresh/rust/target"
export CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8
export TMPDIR="$PWD/.scratch/next/tmp" CABLE_TEST_PORT_RANGE=52900-52919
export INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949
unset CAMPFIRE_REFERENCE
```

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-next . .scratch/next-fresh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > .scratch/next/metadata-final.json
mise exec rust@1.98.1 -- cargo clean --manifest-path .scratch/next-fresh/rust/Cargo.toml --target-dir "$CARGO_TARGET_DIR" -p campfire -p campfire_assets -p campfire_cable -p campfire_db -p campfire_jobs -p campfire_kit -p campfire_mail -p campfire_richtext -p campfire_routes -p campfire_storage -p campfire_views -p html5ever -p rails_compat
env PARITY_NAMESPACE=ws11api-next-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/next-fresh/rust/parity/bin/seed build default first_run agents_ui
```

```text
Removed 12539 files, 9.8GiB total
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

```bash
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/next-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8
```

```text
Finished `test` profile [unoptimized] target(s) in 4m 51s
test result: FAILED. 2551 passed; 6 failed; 7 ignored; 0 measured; 0 filtered out; finished in 590.77s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.87s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1295 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 164.57s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.69s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.64s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 10.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.84s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.39s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.77s
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
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.13s
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

Native exit status 101: the six reviewed application media-byte comparisons and
`campfire_storage::vectors::pipeline_matches_the_reference` fail with the host media
versions. Every one is rerun successfully in the pinned image below, retaining every
byte/size/checksum assertion. No additional failures occurred. Native failures are not
misrepresented as native passes. Sixteen ignored tests remain explicitly counted; the
two agent polling tests are additionally executed explicitly below.

```bash
python3 .scratch/next-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/next/logs/fresh-workspace.log
```

```text
WS11-api cargo totals: 4573 passed; 7 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

```bash
python3 .scratch/next-fresh/rust/reference-tools/agents/named-case-pass-counts.py .scratch/next/logs/fresh-workspace.log
```

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 9 passed; 0 failed; 20 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 19 passed; 0 failed; 4 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 12 passed; 0 failed; 3 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 9 passed; 0 failed; 2 deferred
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
WS11 named comparison totals: 312 passed; 0 failed; 66 deferred
```

```bash
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/next-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
Finished `dev` profile [unoptimized] target(s) in 2m 02s
```

```bash
bash .scratch/next-fresh/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --workspace --bins -j2
```

```text
Finished `dev` profile [unoptimized] target(s) in 1m 12s
```

Final raw flat SELECT counts and atomic rollback controls:

```text
WORK_WRITE_QUERIES rest_create owned_rows=5/50 Rust_all_SELECTs=197/197 Rails_all_SELECTs=60/60
WORK_WRITE_QUERIES rest_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=33/33
WORK_WRITE_QUERIES rest_result owned_rows=5/50 Rust_all_SELECTs=33/33 Rails_all_SELECTs=26/26
WORK_WRITE_QUERIES rest_handoff owned_rows=5/50 Rust_all_SELECTs=105/105 Rails_all_SELECTs=55/55
WORK_WRITE_QUERIES mcp_create owned_rows=5/50 Rust_all_SELECTs=185/185 Rails_all_SELECTs=61/61
WORK_WRITE_QUERIES mcp_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_board_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_result owned_rows=5/50 Rust_all_SELECTs=33/33 Rails_all_SELECTs=27/27
WORK_WRITE_QUERIES mcp_handoff owned_rows=5/50 Rust_all_SELECTs=105/105 Rails_all_SELECTs=56/56
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
env PARITY_NAMESPACE=ws11api-next-wire python3 .scratch/next-fresh/rust/reference-tools/agents/record-http-vectors.py .scratch/next/fresh-oracles
python3 .scratch/next-fresh/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/next-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/next/fresh-oracles
python3 .scratch/next-fresh/rust/reference-tools/agents/test-http-vector-verifier.py
```

```text
WS11-api reference sources: 88 pinned files matched; 0 image or checkout mismatches (d7c7de92)

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

..............
----------------------------------------------------------------------
Ran 14 tests in 0.008s

OK
```

The 22 HTTP/MCP artifacts comprise 1,735 ordinary vectors, two explicitly approved
media-difference cases, and three handled missing-representation scenarios. The #203
oracle now checks nine headers, including Content-Transfer-Encoding; its verifier
mutation control remains intact. Artifact bytes match fresh Rails recapture; approved
Rails/Rust status/state differences keep their explicit outcomes and rationale.

```bash
env PARITY_NAMESPACE=ws11api-next-next_named_cases PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/next-fresh/rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/next_named_cases.rb > .scratch/next/fresh-next_named_cases.json
env PARITY_NAMESPACE=ws11api-next-assignment_named_cases PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/next-fresh/rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/assignment_named_cases.rb > .scratch/next/fresh-assignment_named_cases.json
```

```text
WS11 new named oracle: agents_next_named.json: 6 cases; byte-identical committed vectors
WS11 new named oracle: agents_assignment_named.json: 9 cases; byte-identical committed vectors
```

Named-case recapture is checked as full file bytes against the committed vector by the
postcheck driver; all fifteen production assertions also passed in the full fresh suite.

```bash
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/next/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/next-fresh:$PWD/.scratch/next-fresh:ro" -v "$PWD/.scratch/next/tmp:$PWD/.scratch/next/tmp" --name ws11api-next-pinned-pr192 --entrypoint "$PWD/.scratch/next-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8
```

```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 2533 filtered out; finished in 20.44s
```

```bash
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/next/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/next-fresh:$PWD/.scratch/next-fresh:ro" -v "$PWD/.scratch/next/tmp:$PWD/.scratch/next/tmp" --name ws11api-next-pinned-logo --entrypoint "$PWD/.scratch/next-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2563 filtered out; finished in 0.78s
```

```bash
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/next/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/next-fresh:$PWD/.scratch/next-fresh:ro" -v "$PWD/.scratch/next/tmp:$PWD/.scratch/next/tmp" --name ws11api-next-pinned-storage --entrypoint "$PWD/.scratch/next-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.97s
```

```bash
env CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/next/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/next-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2562 filtered out; finished in 0.68s
```

## Cleanup and delivery

All owned Cargo, test and reference-container work has finished. The sole additional
build target is removed after verification; raw logs, private seeds, oracle captures
and fresh-clone source are retained. The Python model server and other workers'
targets/worktrees were not touched. No new allowlist, response mask or timing threshold
was introduced; neither test concurrency nor the configured rustc throttle was reduced.
No PR or external message was sent.

```text
12G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/next-fresh/rust/target
WS11-api scratch target cleanup: complete; no owned test/build/container processes; logs, seeds, vectors and fresh-clone source retained
```
