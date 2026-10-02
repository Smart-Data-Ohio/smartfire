# WS11-api wave 4 — PARTIAL continuation, 2026-10-02

Code head: `c8e7357541c0ef7b4478d301f95d86454af74b04`, pushed on `rust/ws11api-rest-mcp`; a report-only commit follows. Four coherent slices were pushed. Selected successes remain **31/35 REST actions and 33/38 MCP tools**. The nine WS12 writes remain flagged. Main is retained through `59ad94de` (#183); no main or WS12 merge, stash or rebase occurred.

**Not only owner-blocked items remain.** This is a partial handoff; the precise remaining API work is listed below. No approval or clarification is pending.

## Changes and design

| Commit | Slice |
| --- | --- |
| `2eba70d4` | 117 additional reader validation vectors; Rails nested ID-list lookup, cursor/limit ordering and board-filter whitespace |
| `538ceb2d` | 50 legacy/bot/fanout/replacement vectors and two complete-router concurrent posting checks |
| `b25e8c83` | 15 public/private/unknown-repository work and board payload comparisons |
| `c8e73575` | Five compiled negative proofs for current grants, self-fanout, hop bounds, private details and atomic budgets |

Files below are relative to `rust/`.

- `crates/campfire/src/controllers/agents/{reads,pending}.rs`: history resolves the authorized room association or thread, checks the current grant in the presentation operation, converts the limit, then resolves cursors within that conversation. Nested ID lists use Rails IN-list selection rather than `Array#to_i`. Board room lookup uses the authorized association; filter validation uses Ruby's ASCII/NUL strip behavior. The obsolete history preflight was removed, avoiding a second policy implementation. No WS11/WS12 domain service was duplicated.
- `crates/campfire/src/controllers/agent_legacy_bot_tests.rs` and `controllers.rs`: legacy bots are created with main's real User API, without an Agent row. HTTP response bodies/statuses and selected headers are exact; persisted message/attachment fields, ledger outcomes/hops and durable job arguments are asserted. Cases cover UTF-8/blank input, raw bodies, reply-only tokens, ownership/system-note denials, legacy repeat behavior, membership/board gates, duplicate/self mentions, inactive and nonmember recipients, configured/absent webhooks, hops 0–3, and direct-room fanout. Replacement covers text editing, clearing, invalid signatures, retaining/replacing signed blobs and deletion, for both legacy and agent-backed bots. Both application and Active Storage jobs are captured from Rails; Rust fixtures use `TestApp::without_job_runner()` and assert actual committed jobs. These are queue assertions, not claims of external HTTP delivery or completed asynchronous purge.
- The same file runs two overlapping full-router requests: one remaining daily budget slot yields one 201 and one exact Rails 429; the same client ID yields two identical 201 responses while saving, posting to the ledger and queuing push once. The overflow notice is committed once. No test concurrency or timing threshold changed.
- `crates/campfire/src/controllers/agent_reads_tests.rs`: fixture PR links exercise work show/list and REST/MCP board reads with public, private and unknown visibility. Complete responses prove private/unknown title and branch redaction with no connected account. Main's WS15g repository resolver stays unchanged; no live/private GitHub access was implemented.
- `reference-tools/agents/{reads_http_contract,legacy_bot_http_contract}.rb` and `vectors/agent_{reads,legacy_bot}_http.json`: the reader corpus is now 192 cases; the new bot corpus has 50. The complete wire registry has **1,432 cases across 17 artifacts**, up 182 cases. All HTTP bytes and selected status/header values remain unmasked. The persisted-state projection deliberately asserts explicit row/job fields; it is separate from complete response-byte comparison.
- `reference-tools/agents/{record-http-vectors,verify-http-vectors,check-http-reference,check-permission-media-mutations,check-legacy-security-mutations}.py`: oracle registration, eight additional source pins and the current reader mutation anchor. The new five-mutation runner restores original bytes in `finally` and requires compilation plus a failed runtime assertion.

Cross-workstream production code is unchanged this continuation; the only production fixes are transport/reader adapters. Main's existing User, Message, attachment, fanout, ledger and budget APIs are used. WS14g's previously pushed polling HTTP shape remains available.

## WS12 contract and owner seams

Read the full published `rust/plans/ws12-agent-work-api.md` from `de084e9a55d52dcef63609b6e2af82c8b25f518d` without merging `rust/ws12-boards`. The future replacement maps board creation to `agent_work::create_board_post(BoardPostInput)`, both work/board updates to `update_work(AgentWorkChanges)`, result writes to `set_result`, and handoff to `handoff_work(HandoffPackage, audit context)`. Denied outcomes must commit budget notices, genuine DB/job failures roll back, and board responses must reload after tag auto-assignment. Actual adapter wiring waits for #187 and WS12's services on main; no stand-in was added.

The unchanged `agent_api_pending::execute` seam covers REST board create, work update/result/handoff and MCP create_board_post, update_board_post, update_work, set_result, handoff_work. WS15g's installed repository-access seam is retained exactly.

## Failing-first and selected positives

The new history room-list case returned Room not found before the fix; a nonbreaking-space board status returned 200 instead of Rails 422. Both actual assertions failed before implementation. The first bot fixture failure was a setup mismatch (board validation), and a later purge-job mismatch exposed an oracle adapter setup error; those are not claimed as production failures. The capture was corrected to set ActiveJob::Base.queue_adapter so storage jobs are included.


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_reads -- --nocapture
```


```text
test result: FAILED. 5 passed; 2 failed; 0 ignored; 0 measured; 1862 filtered out; finished in 38.31s
```


```bash
python3 rust/reference-tools/agents/check-legacy-security-mutations.py
```


```text
WS11-api current_history_grant mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1872 filtered out; finished in 1.21s
WS11-api legacy_self_fanout mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1872 filtered out; finished in 7.61s
WS11-api legacy_hop_gate mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1872 filtered out; finished in 3.65s
WS11-api private_work_details mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1872 filtered out; finished in 11.80s
WS11-api concurrent_budget mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1872 filtered out; finished in 1.33s
WS11-api legacy/security mutations: 5 broken guards rejected; sources restored
```

All five mutations compiled and failed at runtime, with sources restored. Private-detail and concurrent-budget proofs change main model guards only temporarily; no model change was committed.


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_reads -- --nocapture
```


```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1862 filtered out; finished in 77.96s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_legacy_bot -- --nocapture
```


```text
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1869 filtered out; finished in 26.33s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_reads -- --nocapture
```


```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1866 filtered out; finished in 69.62s
```

The first reader positive precedes the 15 PR vectors; the last reader positive includes all 192. The bot selector matches the module, so it runs both wire groups and both concurrency tests. Restored behavior is rebuilt and verified again below.

## Fresh-clone suite and strict clippy

The clone was initially made from pushed b25e8c83, then fast-forwarded to c8e73575 (the security-tool-only commit). Both seeds were rebuilt from the pin. After owned build/test processes exited, the single target cache was moved into the clone; changed source/reference paths force recompilation. Cargo used two jobs, the existing rustc throttle and unchanged default test concurrency.


```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/fresh9
PARITY_NAMESPACE=ws11api-fresh9 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh9/rust/parity/bin/seed build default first_run
# In .scratch/fresh9:
git fetch origin
git merge --ff-only origin/rust/ws11api-rest-mcp
# Back at the assigned worktree:
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh9-tmp" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh9/rust/Cargo.toml --format-version 1
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh9/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture
python3 rust/reference-tools/agents/summarize-http-tests.py .scratch/continue9/fresh-workspace.log
```

Locked metadata exited 0. Raw seed and aggregate lines:


```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)

WS11-api cargo totals: 3713 passed; 2 failed; 12 ignored; 58 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

Every native target summary (the native workspace exits 101):


```text
test result: FAILED. 1869 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 499.10s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.76s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.85s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.83s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 162.45s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.18s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.96s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.65s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.50s
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

The native workspace has two failures: the account-logo PNG byte comparison and storage's pinned-media version gate. Both exact fresh-clone targets pass unchanged in the pinned runtime below. No baseline run is re-claimed here. The richtext hardening target passes unchanged in this run; no timing threshold, test concurrency, ignore or assertion changed.


```bash
vips --version
ffmpeg -version
docker run --rm --name ws11api-check9-vips-version --network none --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"'
docker run --rm --name ws11api-check9-ffmpeg-version --network none --entrypoint /usr/bin/ffmpeg ws11api-reference:d7c7de92 -version
```


```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```


```bash
docker run --rm --name ws11api-fresh9-pinned-app --network none --user "$(id -u):$(id -g)" -e CI=1 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/fresh9-tmp" -v "$PWD/.scratch/fresh9:$PWD/.scratch/fresh9" -v "$PWD/.scratch/fresh9-tmp:$PWD/.scratch/fresh9-tmp" --entrypoint "$PWD/.scratch/fresh9/rust/target/debug/deps/campfire-abe1b035fe2b45bc" ws11api-reference:d7c7de92 --nocapture
```


```text
test result: ok. 1870 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 440.72s
```


```bash
docker run --rm --name ws11api-fresh9-pinned-storage --network none --user "$(id -u):$(id -g)" -e CI=1 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/fresh9-tmp" -v "$PWD/.scratch/fresh9:$PWD/.scratch/fresh9:ro" -v "$PWD/.scratch/fresh9-tmp:$PWD/.scratch/fresh9-tmp" --entrypoint "$PWD/.scratch/fresh9/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture
```


```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.68s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh9-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh9/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```


```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 04s
```

## Fresh Rails oracle and controller files


```bash
PARITY_NAMESPACE=ws11api-cont9-final python3 rust/reference-tools/agents/record-http-vectors.py .scratch/continue9/oracles
python3 rust/reference-tools/agents/verify-http-vectors.py .scratch/continue9/oracles
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/test-http-vector-verifier.py
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
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.003s

OK
```

Nine reference files reran unchanged: 169 runs, 734 assertions. They establish the pinned behavior and are not a claim of one-for-one Rust file coverage.


```bash
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agents/polls_controller_test.rb test/controllers/agents/contexts_controller_test.rb test/controllers/agents/work_controller_test.rb test/controllers/agents/posts_controller_test.rb test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb test/controllers/messages/by_bots_controller_test.rb test/controllers/messages/boosts/by_bots_controller_test.rb
```


```text
test/controllers/agents/polls_controller_test.rb: 12 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/contexts_controller_test.rb: 13 runs, 50 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
```


| Rails files | Selected Rust coverage; deferred owner |
| --- | --- |
| agents/contexts, work, posts | Existing context/owned-work reads, warmed authority changes; new history/board coercions and 15 PR visibility responses. Context ID/input permutations and remaining viewer/race cases remain API; nine work writes/associated callbacks remain WS12. |
| agent_capability, revocation_endpoints, owner_deactivation | Existing auth/grant/owner transitions plus compiled current-history guard proof; overlapping budget/replay requests now covered. Concurrent revocation versus writes/finalization and delivery failure interactions remain API/domain owners. |
| messages/by_bots | 71 prior bot cases plus 50 new legacy/fanout/replacement/race reference cases. Multi-hop legacy bounce/reply chains, unshared blob purge execution, remaining MIME/representation permutations remain API/domain owners. |
| messages/boosts/by_bots | Existing 13 grouped reaction and bot-vector cases rerun; legacy bots without Agent rows and broader mutation/deletion permutations remain API. |
| agents/polls | Existing 168 REST/MCP responses rerun; compact/partial dates, broader zone/DST and remaining ID/input permutations remain API. |
| Other API/MCP files | Full wire registry and fresh Rust suite rerun; older Rails-file runner counts are not re-claimed. |

## Precisely remaining

- **WS12 owner-blocked:** four REST and five MCP writes listed above; success responses and remaining service-dependent owner/context-package validation/callbacks wait for #187 and WS12's agent work services on main. The published contract was read, never merged.
- **Unblocked exhaustive validation:** remaining field/ID/array/hash/coercion, length and callback-precedence matrices for the other endpoint/tool families; context limit/ID permutations, remaining work/filter/cursor cases, compact/partial dates and broader user-zone/DST grammar. The 192 reader vectors are selected, not an exhaustive claim for all actions/tools.
- **Unblocked bot/media:** legacy boosts without an Agent, multi-hop agent/legacy bounce and reply-source chains, remaining root/thread replay/attachment callback interactions, unshared replacement/purge execution and further MIME/representation/viewer/cache permutations. The new replacement checks prove exact HTTP and committed purge jobs, not execution of every purge.
- **Unblocked/dependent permissions:** concurrent revocation versus writes/finalization, the remaining viewer and credential/account snapshot cases, and delivery/finalization failure interactions. Daily budget and idempotency races plus private/unknown PR redaction are now covered.
- **Integration owners:** live private-PR repository access/network behavior remains WS15g through main's installed seam; full external delivery/network execution remains domain/integration work. No GitHub access implementation or Python model server changes occurred.

No assertions, goldens, masks, skips, test concurrency or timing bounds were relaxed. This remains partial, with unblocked work outstanding. Logs, oracles and clones are retained under the assigned worktree's .scratch; owned targets were deleted after all processes exited.


```text
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```
