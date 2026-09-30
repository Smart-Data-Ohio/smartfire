# WS11-api wave 4 — PARTIAL continuation, 2026-09-30

Verified code head: `0a6b47eec4b014c8214365dd3119e8fa4d45ac27`. The assigned branch is `rust/ws11api-rest-mcp`; origin and the fresh clone match this code head. No main merge was made. This report replaces the earlier report at `66aaa3b4`; that commit retains the earlier slices and evidence.

This continuation completes **five REST success actions and all nine Fizzy MCP tools**, using WS15e's published services. It adds **186 distinct raw HTTP/MCP vectors** (54 reads, 132 approval requests), matching Rails response bytes, status, and the presence/absence of Content-Type, Cache-Control, Pragma, Retry-After and Location. It checks outbound read targets and authentication, stored executable approval JSON and credential/account identity, inbox recipients, token disconnection, replay, expiry and budget notices. Approval requests make no DNS or network call.

The full workspace passed from a fresh clone: **1484 passed, 0 failed, 9 existing ignores**, with both freshly built seeds and no actual missing-seed skips. Workspace clippy passed with `-D warnings`. The first fresh-clone attempt failed the unchanged OpenGraph one-second timing assertion at 1.116366518 seconds; the complete retry passed without a source change, test-thread override, or widened timing threshold. This observed timing failure is retained below.

Overall API work remains partial: **23 of 35 REST actions and 24 of 38 MCP tools have selected success coverage**. The remaining **12 REST and 14 MCP successes** are listed below. There are **681 committed HTTP/MCP pairs, 679 asserted distinct pairs**; the two deferred base pairs remain list_rooms/list_work. **299 distinct pairs assert runtime raw response bytes** (47 bot, 66 conversation/lifecycle, 186 Fizzy). Older base/surface vectors still compare decoded JSON and selected headers, with exact MCP content text where present; fixture regeneration alone does not prove their outer runtime serialization byte-identical.

## Pushed coherent slices

| Commit | Work |
| --- | --- |
| `04f7aff9` | Shared Fizzy reads, four REST/four MCP successes, 54 raw vectors, remote errors/502 status mapping, published account-deactivation hook and owned test listeners |
| `c5f57de2` | Shared approval requests, one REST/five MCP successes, 88 initial raw vectors, tool field slices, credential/identity snapshots, replay/expiry, budgets and inbox writes |
| `35a5cee0` | 44 further wire cases: AR boolean/String casts, flattened array/NULL replay, Hash exceptions, REST deep munge, local-day budgets and inbox preferences |
| `0a6b47ee` | Three compiled mutation proofs for read/write workspace grants and owner-account disconnection |

Every slice was pushed with the GPT-6.1 Sol coauthor. The report-only commit follows the verified code head.

## Changes by file and design

Paths below are relative to `rust/`.

| Files | Change |
| --- | --- |
| `crates/campfire/src/integrations/fizzy/{accounts,client,error_body,agent_reads,agent_action,agent_requests}.rs` and their selected tests | Imported WS15e production modules and service/model vectors from `a9e14853`; reads and approval requests are shared by REST and MCP. Request replay now uses WS11's existing client-ID casting helper, adds NULL binding and Rails Hash exceptions. The reads body helper has an explicit unused-helper annotation; execution methods remain staged with WS15e's approval job. |
| `crates/campfire/src/integrations/fizzy/mod.rs`, `app.rs` | Minimal module wiring and per-app network/base configuration; production uses the system network and configured Fizzy origin, tests inject a transport without changing process-global environment. |
| `crates/campfire/src/controllers/agents/integrations.rs`, `mcp.rs` | Call the published shared read/request services; remove duplicate Fizzy policy/validation preflight. Preserve the REST controller's fixed field set and each tool's narrower field set, credential and request-local zone. Map 502 to MCP bad_gateway. GitHub preflight and its flagged seam remain. |
| `crates/campfire/src/controllers/agent_fizzy_tests.rs`, `agent_fizzy_action_tests.rs`, `controllers.rs` | Full-router wire, state, network/no-network and header assertions for 186 distinct cases. The replay-type function intentionally repeats its cases as a focused regression check. |
| `crates/campfire/src/controllers/agent_http_tests.rs`, `controllers/presenters/test_support.rs` | Reuse the existing credential fixture with a per-app injected Fizzy transport. Published service tests use the existing job-shutdown helper. |
| `crates/campfire/src/integrations/net/http.rs`, `net/http/deadline_tests.rs` | WS15e's write-timeout/closed-connection primitive and connection-budget tests. Preserve all existing client open/read limits and test timing thresholds; default write limit is Rails' 60 seconds. |
| `crates/campfire/src/integrations/{webhook,web_push}.rs`, `opengraph/fetch.rs` | Supply the new write-timeout field; their existing open/read limits are unchanged. |
| `crates/campfire/src/integrations/test_support.rs` | Expose fixture networking to HTTP tests and own/abort each fake server's accept task when dropped. This fixes listener accumulation across case loops without changing test concurrency or port ranges. TLS tests use the certificate's configured test origin and retain TLS verification. |
| `crates/db/src/events.rs`, `models/user.rs`, `crates/campfire/src/jobs.rs` | Published WS15e account-disconnection hook, adapted to WS11's existing suspension path; installs Fizzy disconnection during real user deactivation. Other connected-account hooks remain with their owners. |
| `crates/db/src/slash_commands/time_parser.rs` | Public accessor for the existing Rails zone resolver; no new parsing/domain logic. Used for integration budgets and flagged for WS11 reconciliation. |
| `reference-tools/agents/fizzy_http_contract.rb`, `fizzy_action_http_contract.rb`; `vectors/agent_fizzy*_http.json` | Production Rails wire captures on private disposable seed copies. Cases commit and reset their fixtures, preserving actual after-commit inbox fanout and avoiding an encrypted-record rollback artifact. Record Net::HTTP target bytes before WebMock's CGI query normalization. |
| `vectors/ws15e_fizzy_agent_{reads,action,requests}.json` | Published WS15e service/model oracles; exercised by imported tests. These were imported, not independently regenerated in this continuation. |
| `reference-tools/agents/check-fizzy-http-mutations.py` | Compiles three broken guards, requires assertion failures (compilation failures do not count), and restores source files. |
| `reference-tools/agents/{check-http-reference,verify-http-vectors}.py` | Pin-check the Fizzy client and regenerate/compare both new wire artifacts with the five existing HTTP artifacts and scheduler. |

REST/MCP adapters perform presentation and caller field selection. Services own active-agent/workspace authorization, owner token usability, replay, validation, daily budgets, approval persistence and inbox effects. A request creates approval; it does not execute a remote write. Live private-PR resolution remains the explicit WS15g seam, per the user's direction concerning PR #167. No GitHub access implementation or main merge was added here.

## Precisely remaining / owner seams

- **12 REST successes:** pins create/destroy (2); polls create/show (2); board posts index/create (2); work index/show/update/result/handoff (5); GitHub pull-request-action create (1).
- **14 MCP successes:** list_rooms, read_messages, react; pin_message, unpin_message; create_poll, get_poll; list_board_posts, create_board_post, update_board_post; set_result; list_work, update_work; handoff_work.
- **Domain owners:** WS11 room/read/reaction/pin/poll services remain unavailable on this branch; board/work/result/handoff require WS11/WS12 integration. Their auth/grant/selected validation/error paths remain ported behind flagged success seams. WS15g owns live private-PR resolution and GitHub actions. WS15e owns execution/approval-job integration; its agent_job was not imported here. Existing request-success coverage does not prove the remote approval execution flow.
- **Exhaustive validation/errors:** expand all remaining field shapes, raw coercions, numeric/length/date/expiry boundaries, pagination/cursors, race and callback precedence for the older API families; convert older decoded comparisons to raw body comparisons. The 186 Fizzy cases are substantial selected coverage, not a claim of exhaustive input coverage.
- **Attachments and bots:** REST Posting still needs multipart/signed attachment staging through the existing storage path; full bot/root/thread attachment delivery, persisted state, analysis/fanout and enqueue rollback; human work-thread viewer/cache parity with WS8b-m/WS12; complete pagination/cursor cases. Forty-seven bot wire cases remain selected coverage, not complete file-equivalent ports.
- **Security/delivery:** broader credential transitions and lifecycle/race cases, stream finalization callback exceptions, attachment enqueue rollback, actual SSRF/private-IP/DNS-rebinding tests with domain owners, and full delivery/callback file ports. Prior named authentication/rate/header/version fail-first proof is inherited from earlier slices; this continuation reran the three new Fizzy mutations below.
- **WS11 follow-ups:** published peer-callback/presenter and finalization-exception commits `73beb1fe`, `d41b5399` and prerequisites remain for reconciliation. Other connected-account deactivation hooks remain with their integrations.

No approval or clarification is pending. Next work depends on owner service integration and the listed API validation/attachment slices.

## Commands actually rerun and raw evidence

All paths are in the assigned worktree unless noted. Rust commands use mise Rust 1.98.1, `-j4`, CI=1, the private `.scratch/clean-target`, and assigned ports 52900–52949. No RUST_TEST_THREADS / --test-threads override was supplied.

### Selected success, validation and failure-first checks

From the assigned root, with `CARGO_TARGET_DIR=$PWD/.scratch/clean-target` and `INTEGRATION_TEST_PORT_RANGE=52920-52949`:

```bash
CI=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire fizzy_read_wire_successes -- --nocapture
CI=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire action_ -- --nocapture
CI=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire fizzy_action_replay_types -- --nocapture
CI=1 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire --bin campfire fizzy -- --nocapture
CI=1 mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml -p campfire --all-targets -- -D warnings
CARGO_TARGET_DIR=$PWD/.scratch/clean-target python3 rust/reference-tools/agents/check-fizzy-http-mutations.py
```

The first three commands ran against the missing service / wrong replay implementation and failed for assertions. The success test initially returned REST 500; both action success groups returned the pending seam; the replay test found stored true instead of t. The final positive run includes all 132 approval and 54 read vectors. Raw lines:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.79s
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 403 filtered out; finished in 12.14s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.78s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 394 filtered out; finished in 71.59s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.68s
WS11-api Fizzy read_workspace mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 17.42s
WS11-api Fizzy write_workspace mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 8.71s
WS11-api Fizzy disconnect_owner mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.81s
WS11-api Fizzy mutations: 3 broken guards rejected; sources restored
```

Sources were restored after every mutation. The fresh clone contains the restored implementations.

### Rails files, grouped by file, with pass counts

```bash
python3 rust/reference-tools/agents/run-controller-reference.py
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb
```

All 38 reference files: **602 runs, 4138 assertions, zero failures/errors/skips**. The MCP file ran 71 tests with its default 20 processes; Docker init owns Redis. These are reference pass counts, not claims that all cases have equivalent Rust ports.

Selected Rust coverage and deferral, grouped by source file:

| Rails file/group | Ported / deferred |
| --- | --- |
| fizzy/boards_controller, cards_controller, card_actions_controller; mcp_fizzy | All success actions/tools plus 186 wire/state vectors and published model/service tests; further input/lifecycle/execution cases: API/WS15e |
| agents_controller; steps_controller; slash_commands_controller; approvals_controller | Selected successes, policy/coercion/budget/expiry cases; exhaustive field/race/callback coverage: API/WS11 |
| contexts_controller; messages_controller; dms_controller; streaming_messages_controller; mcp_streaming | Existing context/post/DM/stream successes and 66 raw conversation/lifecycle vectors; attachments and full callbacks/races: API/WS11/storage |
| pins_controller; polls_controller; mcp_slash_polls | Existing policy/selected errors; success and complete datetime/input variants: WS11/API |
| posts_controller; work_controller; work_handoff; mcp_handoff | Existing policy/selected errors; success/result/handoff and full fields: WS11/WS12/API |
| github/pull_request_actions_controller; events_controller | Existing policy/account/cursor/event/selected errors; success and live private access: WS15g/WS11/API |
| mcp_controller | All dispatch/protocol/boundary checks, 24 selected successful tools; 14 successes and exhaustive byte/error matrix: owners/API |
| messages/by_bots; messages/boosts/by_bots | Existing runtime tests and 47 raw wire vectors; complete attachments, human work viewer/cache and callback variants: API/WS11/WS8b-m/WS12 |
| api_throttle; budgets_endpoints; concerns/agent_authentication; agent_capability; agent_revocation_endpoints; agent_owner_deactivation | Existing boundary/mutation proof and selected lifecycle cases; wider credential/lifecycle/security transitions: API/WS11 |
| approval_delivery; drive_attachments_delivery; github_action_delivery; slash_command_delivery; work_delivery; fizzy/action_delivery | Published domain/runtime tests execute; full file-equivalent delivery/SSRF/callback ports: respective owners |
| directory_controller | Reference passed; HTML/system equivalent: WS11-ui |

Raw file summaries:

```text
test/controllers/agents/approval_delivery_test.rb: 5 runs, 26 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/approvals_controller_test.rb: 24 runs, 85 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/api_throttle_test.rb: 13 runs, 1088 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/budgets_endpoints_test.rb: 8 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/directory_controller_test.rb: 6 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/contexts_controller_test.rb: 13 runs, 50 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/drive_attachments_delivery_test.rb: 3 runs, 6 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/dms_controller_test.rb: 18 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/action_delivery_test.rb: 3 runs, 14 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/boards_controller_test.rb: 13 runs, 28 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/events_controller_test.rb: 43 runs, 206 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/card_actions_controller_test.rb: 11 runs, 43 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/cards_controller_test.rb: 8 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github_action_delivery_test.rb: 9 runs, 51 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github/pull_request_actions_controller_test.rb: 21 runs, 82 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_fizzy_test.rb: 28 runs, 333 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_controller_test.rb: 71 runs, 660 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_slash_polls_test.rb: 14 runs, 60 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_handoff_test.rb: 8 runs, 95 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_streaming_test.rb: 11 runs, 71 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/messages_controller_test.rb: 15 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/pins_controller_test.rb: 10 runs, 29 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/polls_controller_test.rb: 12 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/slash_command_delivery_test.rb: 2 runs, 4 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/slash_commands_controller_test.rb: 15 runs, 38 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/steps_controller_test.rb: 12 runs, 38 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/streaming_messages_controller_test.rb: 17 runs, 77 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_delivery_test.rb: 9 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents_controller_test.rb: 10 runs, 30 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_handoff_test.rb: 13 runs, 106 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/concerns/agent_authentication_test.rb: 13 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

### Pinned source and verifier checks

```bash
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/test-http-vector-verifier.py
```

```text
WS11-api reference sources: 58 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.006s

OK
```

### Fresh clone and full workspace

From the assigned root:

```bash
git clone --no-hardlinks --single-branch --branch rust/ws11api-rest-mcp . .scratch/fresh5
mkdir -p .scratch/fresh5-tmp .scratch/fresh5-oracles
PARITY_NAMESPACE=ws11api-fresh5 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh5/rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

The first workspace command used `--manifest-path .scratch/fresh5/rust/Cargo.toml` from the assigned root. The complete retry and clippy ran from `.scratch/fresh5/rust`. All used these environment values:

```text
CI=1
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target
TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/fresh5-tmp
CABLE_TEST_PORT_RANGE=52900-52919
MAIL_TEST_PORT_RANGE=52920-52949
INTEGRATION_TEST_PORT_RANGE=52920-52949
```

```bash
mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path .scratch/fresh5/rust/Cargo.toml --workspace --exclude html5ever -- --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j4 --workspace --exclude html5ever -- --nocapture
mise exec rust@1.98.1 -- cargo clippy --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings
```

First attempt: exit 101, unchanged OpenGraph timing guard. Raw failure and result:

```text
1.116366518s for <meta property="og:title" content="x" a0000000 a0000001 a000…
test result: FAILED. 409 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 334.32s
```

Complete retry: exit 0. All 46 raw summaries:

```text
test result: ok. 410 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 198.61s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.15s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 459 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 48.31s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.10s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.12s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.57s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.41s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.37s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.11s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.71s
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

```bash
python3 rust/reference-tools/agents/summarize-http-tests.py .scratch/fresh5-workspace-retry.log
```

```text
WS11-api cargo totals: 1484 passed; 0 failed; 9 ignored; 46 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 34.25s
```

Nine existing ignores remain (seven runtime and two doctests); none was added. Conditional storage media byte checks retain their existing version gate, separate from those ignores:

```text
skipping byte comparisons that depend on versions: vectors have libvips "8.16.1" / "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", local libvips 8.18.6 / ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
```

### Fresh Rails wire regeneration

From the assigned root, every capture runs from the fresh clone with a disposable default seed copy:

```bash
PARITY_NAMESPACE=ws11api-fresh5 PARITY_IMAGE=ws11api-reference:d7c7de92 python3 - <<'CAPTURE'
from pathlib import Path
import subprocess
root=Path('.scratch/fresh5').resolve()
out=Path('.scratch/fresh5-oracles').resolve()
for script,filename in [('http_contract.rb','agent_http.json'),('mcp_contract.rb','agent_mcp.json'),('surface_contract.rb','agent_surface.json'),('bot_http_contract.rb','agent_bot_http.json'),('conversation_http_contract.rb','agent_conversation_http.json'),('periodic_contract.rb','agent_periodic.json'),('fizzy_http_contract.rb','agent_fizzy_http.json'),('fizzy_action_http_contract.rb','agent_fizzy_action_http.json')]:
 with (out/filename).open('w') as stdout,(out/(script+'.log')).open('w') as stderr:
  result=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','bin/rails','runner','/work/reference-tools/agents/'+script],cwd=root,stdout=stdout,stderr=stderr)
 assert result.returncode==0,(script,result.returncode)
 print('Captured '+filename,flush=True)
CAPTURE
python3 .scratch/fresh5/rust/reference-tools/agents/verify-http-vectors.py .scratch/fresh5-oracles
```

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 265 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 47 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh scheduler oracle: 19 Rails tasks captured; all 8 implemented Rust tasks asserted in Rails order
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 82 asserted vectors; 2 explicitly deferred list success vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```

Logs and fresh clone are retained under the assigned worktree's `.scratch/`. No production app or Rails source was changed.
