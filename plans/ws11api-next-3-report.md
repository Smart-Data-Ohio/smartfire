# WS11 API next-3 checkpoint

Tested source: `808f212b2ee84f6974aa80022aad5ee3451533c8`. Fresh clone: `.scratch/next-3-fresh`, made with `git clone --no-hardlinks --single-branch` from this worktree. Its application, workspace-library and vendored html5ever packages were rebuilt there; only registry build cache was reused. All three Rails seeds were regenerated. Main `c6c37fb8f` (#210, #208 and #211) is included by merge commits; #210/next-2 history was not modified. The final report commit only changes documentation. The final fetch confirms zero commits in HEAD..origin/main. Strict clippy first exposed an eight-argument helper; grouping its preloaded actor facts resolved the lint without suppressions, and the complete fresh suite was rerun on that revision.

## Requested items

| Item | Result |
| --- | --- |
| A: nested candidate IDs | Fixed. Multi-candidate nested arrays serialize independently; singleton/null compaction and primary-key selection match Rails. Alpha/Beta real HTTP cases are pinned. |
| B: mixed objects/reaction IDs and sweep | Fixed. Replay creates then reuses the reaction. Shared casting/scoped scalar adapters cover every array-ID lookup found in REST/MCP, nested event-ack members, current membership, deleted rooms and service-specific scopes. 119 cases / 125 responses compare literal body/status/selected headers and full projected rows/jobs. Thirty direct casting cases include decimal underscores and Unicode whitespace. One JSON bind bounds SQL parameters, including the existing 40,000-candidate test. |
| C: uncached recorder | Fixed. Cache is disabled after the request executor's AR hook; cached or cache-enabled SELECTs cause failure. All 24 regenerated cases have zero cache hits. Historical descriptions are corrected. |
| 1: named-case checker | Fixed. Pinned delivery order, nine named macro tests, and stale deferrals corrected. Scanner negative controls pass. CI seed preparation executes checker and controls. |
| 2: avoidable WS11 reads | Trimmed. REST-create preflight 12→2; whole-request create/handoff savings and owner boundaries below. Fresh authorization and private-owner sealing remain. Required WS11 reads are explicitly retained; the larger remaining service/render callback costs belong to WS12. |
| 3: race classification | Documented as approved differences, without changing behavior. Astra's `behavior-race-results.md`: 13/34 prewriter and 4/10 private-owner races retain Rust's stricter current-state authorization/redaction. |
| 4: typed AgentBudgetNotice | Fixed. Exported typed owner reader, one-bind batch lookup, cap labels/limits and owner/admin recipient policy. All three activity presenter SQL seams replaced. Twelve Rails reader rows and existing inbox HTML/JSON goldens remain exact; HTML/JSON preloads are 3/3 reads at 5/50 notices. WS12 owns Recorder producer integration. |
| 5: peer-owned manifest names | Pending peer evidence. Current main has not added WS8's 35 or WS12's 16 named comparisons; no mapping invented. Two WS11-UI names also remain with that owner. |

All WS11-owned named comparisons are complete. **Only peer-owned named evidence remains:** WS8 35 built-in slash commands; WS12 16 assignment eligibility/mutation names; WS11-UI 2 rendered badge/directory and secret-leak broadcast cases. The ledger remains 325 mapped/pass comparisons and 53 deferred. WS12 owns its separate ActivityItems::Recorder integration and fixed service/callback read costs, not missing WS11 reader APIs. `work_threads.rs`, `presenters/boards.rs` and WS12 work services are untouched by this branch.

## Files and boundaries

- `controllers/agents/{reads,reactions,id_args,mcp,pending,work_writes,approvals}.rs` and GitHub agent controller: shared candidate casting, bounded scoped adaptation, and repeated REST preflight removals.
- DB `agent_access`, `agent_delivery`, `agent_work_events` and `bot_webhook_fanout`: current capability batches, stored-event RETURNING, one-query webhook eligibility, and reuse of writer-local actor/hop facts.
- `models/agent_budget_notice.rs`, `models/agent.rs`, root model exports, and `presenters/activity.rs`: typed reader and bounded notice/agent/author preloads.
- `reference-tools/agents`, three new vectors and focused controller/DB regressions: real Rails coercion/state matrix, query-cache boundary, typed reader, named scanner and negative controls.
- `parity/bin/ci-seed`: existing CI preparation now runs the named checker and scanner controls. Plans correct prior cached-count explanations and record the approved race decisions.

## Read counts

Whole-request physical SQLite SELECTs, both reader and writer lanes, at 5/50 owned rows. Before counts are this round's actual pre-trim run after main updates and typed preloads, not substituted older review counts.

| Path | Before Rust 5/50 | After Rust 5/50 | Rails 5/50 |
| --- | --- | --- | --- |
| REST create | 189/189 | 169/169 | 60/60 |
| MCP create | 177/177 | 167/167 | 61/61 |
| REST update | 83/83 | 83/83 | 33/33 |
| MCP update / update_board_post | 83/83 | 83/83 | 34/34 |
| REST result | 33/33 | 33/33 | 26/26 |
| MCP result | 33/33 | 33/33 | 27/27 |
| REST handoff | 105/105 | 94/94 | 55/55 |
| MCP handoff | 105/105 | 94/94 | 56/56 |

Room and live capability preflight facts are loaded once per boundary; writer checks remain fresh. Ledger INSERT RETURNING preserves stored defaults and removes immediate rereads. Webhook eligibility combines current policy facts; event UPDATE mutates the result only when successful. Message hop and assignment actor identity reuse facts from the same writer connection. Durable jobs retain their transactional source-write boundary.

Four authentication/ban reads, current adapter identity, repository candidate selection and committed work payload reads remain WS11-owned and required. WS12 owns repeated checks in `agent_work::find_owned/writable/update_work/set_result/handoff_work`, ChannelThread work producers, board/tag preloads and after-commit history/activity. These services are named rather than modified. See `ws11api-write-read-trim.md` and Astra's read-only `review-205-r1/query-cost-attribution.md`.

Eight array-reader size controls remain flat at 5/50 IDs: MCP thread 23/23, before 24/24, after 24/24, context-message 24/24, context-thread 23/23, react 11/11; REST context-message 25/25 and context-thread 24/24. These use the new shared matrix fixture; they are not substituted for Astra's cached 26/27 figures. Correctly uncached Rails oracle: thread 29/29, cursors 30/30, all zero cache hits.

## Failing-first evidence

The named checker first failed on `test/jobs/agent/delivery_job_test.rb`; after order correction, it failed discovering `agent_assignment_cases_test.rs`'s nine macro tests. Both failures are retained in `checker-before.log` and `checker-order-before.log`. Controls then confirm actual macro invocation discovery and rejection of undiscovered macro syntax.

The first shared 52-case matrix on the starting implementation had 18 strict observable differences and the direct cast test also failed. Expanded ack-member cases had 15 differences before adaptation. The fresh-clone alive-room regression failed all six deleted-room cases against the prior production code. The final string extension failed against `8f7f3bd8c`: Unicode whitespace chose Alpha instead of Beta and decimal underscores missed Alpha. Raw failing summaries:

shapes-before-retry.log
```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2622 filtered out; finished in 34.94s
```

ack-before.log
```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2623 filtered out; finished in 86.48s
```

alive-scope-before.log
```text
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 2622 filtered out; finished in 78.68s
```

string-before.log
```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 2622 filtered out; finished in 63.74s
```

The pre-trim cost threshold fails before the read removals:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2623 filtered out; finished in 10.34s
```

The unfixed uncached recorder rejects its real request boundary:

```text
/work/reference-tools/agents/array_read_contract.rb:79:in 'block (3 levels) in <top (required)>': Uncached recorder observed 29 cached/enabled SELECTs (RuntimeError)
```

The committed typed-reader negative control changes the board-post label, requires the Rust regression to fail and restores the exact oracle bytes:

```text
WS11 typed budget reader negative control: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1307 filtered out; finished in 0.15s
WS11 typed budget reader: wrong label rejected; exact oracle bytes restored
```

## Fresh-clone commands and raw results

`CI=1`, `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_PROFILE_DEV_DEBUG=0`, owned `CARGO_TARGET_DIR=$PWD/rust/target`, and the existing rustc throttle were retained. Test port ranges: cable 52900–52919; integration/mail/GitHub 52920–52949. Rails image: `ws11api-reference:d7c7de92`. The existing pinned-media runner executes the six native-version-sensitive media tests and storage vectors in the pinned runtime; no byte/size/checksum assertions are removed or masked. Other application tests run natively at eight threads.

Commands below ran from the fresh clone. Source/oracle Python checks ran from the primary worktree using newly regenerated Rails artifacts. One large Rails generator ran at a time. No stash, target outside scratch or model-server changes occurred.

```sh
rust/parity/bin/ci-seed prepare
rust/parity/bin/seed build default first_run agents_ui
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire ws11_next3_ -- --nocapture --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_work_writes_ -- --nocapture --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -j2 --bin campfire
python3 rust/reference-tools/agents/check-budget-reader-negative.py /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/next-3/logs
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db ws11_next3_ -- --test-threads=8
```

CI preparation and regenerated seeds:

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-a84a88b580bc609efb524959436495899ce3982988f435cc509b6ea4e51a7007
seed_key=rust-parity-seed-v1-297ed8903d9f24799ddc69f1654e63e1ad014aa45a96dfbdc2ba854cdf2c52e9
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

Full workspace, including all library/integration/doc-test summaries:

```text
test result: ok. 2613 passed; 0 failed; 7 ignored; 0 measured; 6 filtered out; finished in 494.70s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2620 filtered out; finished in 2.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.62s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1304 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 120.00s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.64s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.90s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.55s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
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
WS11 next3 fresh workspace totals: 4652 passed; 0 failed; 16 ignored; 60 raw summaries
```

Focused coercion/budget preload and nine work-write vector suites:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 2623 filtered out; finished in 60.40s
```

```text
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2614 filtered out; finished in 22.48s
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1306 filtered out; finished in 2.03s
```

Whole-request read receipts:

```text
WORK_WRITE_QUERIES rest_create owned_rows=5/50 Rust_all_SELECTs=169/169 Rails_all_SELECTs=60/60
WORK_WRITE_QUERIES rest_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=33/33
WORK_WRITE_QUERIES rest_result owned_rows=5/50 Rust_all_SELECTs=33/33 Rails_all_SELECTs=26/26
WORK_WRITE_QUERIES rest_handoff owned_rows=5/50 Rust_all_SELECTs=94/94 Rails_all_SELECTs=55/55
WORK_WRITE_QUERIES mcp_create owned_rows=5/50 Rust_all_SELECTs=167/167 Rails_all_SELECTs=61/61
WORK_WRITE_QUERIES mcp_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_board_update owned_rows=5/50 Rust_all_SELECTs=83/83 Rails_all_SELECTs=34/34
WORK_WRITE_QUERIES mcp_result owned_rows=5/50 Rust_all_SELECTs=33/33 Rails_all_SELECTs=27/27
WORK_WRITE_QUERIES mcp_handoff owned_rows=5/50 Rust_all_SELECTs=94/94 Rails_all_SELECTs=56/56
```

Strict clippy and release-input guard/build:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 01s
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 45s
```

Source/oracle and executed-name checks:

```sh
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/test-case-ports.py
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/test-proxy-headers.py
python3 rust/reference-tools/agents/verify-http-vectors.py .scratch/next-3/fresh-vectors
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/next-3/logs/fresh-workspace.log
```

reference-check.log
```text
WS11-api new named sources: 4 pinned test files matched checkout; test sources are not shipped in the Rails image
WS11-api reference sources: 93 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```

named-checker-controls.log
```text
..
----------------------------------------------------------------------
Ran 2 tests in 0.001s

OK
```

proxy-controls.log
```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.001s

OK
```

verify-fresh-wire-final.log
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
WS11-api fresh array lookups oracle: 24 request/response pairs; byte-identical committed vectors
WS11-api fresh blob proxy all headers oracle: 4 request/response pairs; byte-identical committed vectors
WS11-api fresh agent_id_casting.json: 30 groups; byte-identical committed vector
WS11-api fresh agent_budget_notice_reader.json: 4 groups; byte-identical committed vector
WS11-api fresh agent_array_shapes.json: 119 groups; byte-identical committed vector
WS11-api uncached array oracle: 24 cases; zero query-cache hits
WS11-api all-header oracle: 25 responses; every header name/value/cardinality; config.ru HTTP/1.1; only 6 named security additions and 3 per-request names approved
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```

named-pass-counts.log
```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 13 passed; 0 failed; 16 deferred
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
WS11 named comparison totals: 325 passed; 0 failed; 53 deferred
```

Cleanup: the sole owned fresh-clone target was removed after verification; no other worker targets or protected evidence were touched.
