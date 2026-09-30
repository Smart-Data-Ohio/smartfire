# WS11-api Wave 4 report — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws11api-rest-mcp`.
Assigned worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api`.
Reference: Rails `d7c7de92`. Starting base: WS11 `35b4bff4`, including main `21a7332f`.
**Main was not merged. No test concurrency was reduced and no timing threshold was widened.**
Final fresh-clone code/vector verification head: `cf82d476d9146b43a8fe83fd10cf2b3f0bc3dd4e`.
The following report commit changes documentation only. The requested external report and this tracked copy are identical. No PR or deployment was created.

## Delivered in this continuation

Seven API/integration slices were committed and pushed, alongside four selected WS11 domain commits. The REST and MCP adapters now share WS11's context, Posting, DirectMessages and Streaming services. Selected successful responses for six additional REST actions and six additional MCP tools are asserted against actual Rails response bytes, HTTP status and header presence/absence. Posting covers HTML/Markdown, root/thread replies, Drive IDs, locked threads, budget denial and replay precedence. DM vectors cover nested/top-level fields, input filtering and room/message presentation. Streaming vectors cover start, append/replace, finalization, repeated finalization and selected malformed inputs.

Bot vectors now assert raw response bodies and selected headers across 47 Rails requests, including work-thread status/owner data and Unicode/HTML/line-separator escaping. The inherited JSON update response uses Rails' render-json serialization; the listing and boost Jbuilder responses retain their Jbuilder escaping. Work-thread metadata is read from existing membership/access policy rather than omitted.

Seven owner lifecycle HTTP vectors exercise the real shared User deactivate/ban methods, credential and legacy bot-key refusal, MCP credential refusal and unrelated-owner controls. Both authorization guards were deliberately broken and each corresponding test failed before source restoration. Separate REST/MCP streaming tests inject a durable trailing-job enqueue failure and assert the message write rolls back; dropping the enqueue makes both fail.

REST strong parameters and MCP argument slicing now remain distinct. MCP ignores nested REST message data; top-level DM/MCP reply fields are ignored as Rails does; false reply IDs are blank; array/hash Markdown coercions, malformed REST message values and selected root-reply errors match the captured Rails responses. Event polling calls the explicitly flagged **WS15g live private-PR access seam**. It grants no private access; no GitHub access implementation was added here.

The first fresh-clone run found three failures: two owner-ban row-count races with asynchronous cleanup and an older scheduler roster. Lifecycle HTTP tests now preserve the pending durable cleanup job while comparing the synchronous request, matching the Rails oracle's absent worker. Scheduler task order now follows Rails; the golden capture contains all 19 Rails tasks and the test positively checks all eight implemented Rust tasks, removing the previous two-task filter. Rails controller references run at their normal parallelism: Docker init owns the Redis daemon so the 71-case MCP file can start and finish with 20 processes. Rust test thread counts and Rails worker counts are not overridden.

## Acceptance boundary and precise remaining work

**Partial acceptance.** Selected successful paths now exist for **18/35 REST actions and 15/38 MCP tools**. Every action/tool is dispatched, and existing boundary/error/throttle matrices remain exercised. Authorized success for the following actions still reaches the explicit no-success domain seam. These must not be treated as finished endpoint/tool ports:

- **17 REST actions:** pins create/destroy; polls create/show; board posts index/create; work index/show/update/result/handoff; Fizzy boards index/show, cards search/show, card actions create; GitHub pull-request action create.
- **23 MCP tools:** `list_rooms`, `read_messages`, `react`, `pin_message`, `unpin_message`, `create_poll`, `get_poll`, `list_board_posts`, `create_board_post`, `update_board_post`, `set_result`, `list_work`, `update_work`, `handoff_work`, and all nine Fizzy tools (`list_fizzy_boards`, `get_fizzy_board`, `search_fizzy_cards`, `get_fizzy_card`, `create_fizzy_card`, `comment_on_fizzy_card`, `move_fizzy_card`, `close_fizzy_card`, `reopen_fizzy_card`).
- **WS11-api validation/errors:** complete every raw type/coercion/status/header variant for profiles, steps, slash commands, approvals, conversation fields, polling/cursors, polls, board/work/handoff and integrations. Remaining notable cases include false/array/hash client-message-ID replay semantics in the newly exposed services, all datetime/expiry conversions, tags/status/note/result/URL lengths and field-shape precedence. Boundary matrices and selected failures do not prove exhaustive 401/403/404/422/429 coverage.
- **Bot parity:** multipart and signed attachment request/response/state/callback differentials; full bot/root/thread attachment delivery and rollback; human-viewer work-thread permission/cache behavior (WS8b-m/WS12 presenter seam); all pagination/cursor edge cases; broader enqueue-failure tests. The mock-only Rails Boost save-false case has not been reproduced through a mock. Forty-seven wire vectors are selected coverage, not the complete two-file port.
- **REST attachment staging:** the newly exposed Posting adapter does not yet stage/assign a submitted REST attachment. It must reuse the existing storage/app path with WS11 preflight and transaction policy; attachment analysis/fanout remains a flagged app/domain integration boundary.
- **Security:** broader HTTP revocation/credential transitions; full streaming mutation races, callback-failure finalization behavior and posting/DM/bot attachment enqueue rollback; actual SSRF/private-IP/DNS-rebinding failure-first integration cases with the domain owners. The brief's original named auth/rate/header/version guards, both owner guards and trailing enqueue rejection have compiled failure-first proof, but these additional security scenarios remain deferred.
- **Domain ownership:** WS11 supplies remaining agent room/read/reaction/pin/poll services and complete callback/lifecycle integrations; WS12 owns board/work/result/handoff and work authorization; WS15e owns Fizzy read/action clients and remote failures; **WS15g owns live private-PR resolution and GitHub client/action access**. The WS15g seam must be installed after that work lands. Private fields are withheld until then.
- **WS11 follow-up integration:** later published peer-callback/presenter and finalization-exception commits (`73beb1fe`, `d41b5399`) and their dependencies are not imported here. Coordinate them with the API presenter instead of replacing it; this branch currently imports the earlier context/DM/lifecycle/stream service slices only. No main merge or domain reimplementation was used to bypass ownership.

There are **495 committed HTTP/MCP request/response pairs, 493 asserted distinct pairs**: 33 base REST, 82 of 84 base MCP, 265 surface, 47 bot and 66 conversation/lifecycle. The two unasserted base pairs are deliberately deferred `list_rooms` and `list_work` successes. **113 vectors assert runtime raw response bytes** (47 bot + 66 conversation); the older base/surface tests compare decoded JSON and selected headers, with exact MCP content text where present. Regenerating every fixture byte-identically confirms the oracle artifacts; it does not establish byte-identical outer serialization for every older runtime response. Scheduler and imported WS11 domain captures are additional fixtures, not extra HTTP coverage.

## Pushed slices

| Commit | Change |
| --- | --- |
| `1940cf0e` | WS11 context service, selected cherry-pick of `f153b1fa` |
| `b5c230c2` | WS11 Posting/DirectMessages, selected cherry-pick of `0d194f3f` |
| `c4e4edb8` | Shared context/post/DM REST and MCP adapters with Rails wire vectors |
| `7026465f` | WS11 lifecycle/quiet finalization, selected cherry-pick of `8f338ac6` |
| `9b2388ff` | WS11 streaming/trailing queue/sweep, selected cherry-pick of `63852938` |
| `692a70ff` | Shared streaming adapters and HTTP enqueue rollback proof |
| `fc521e7a` | Bot raw response serialization and work-owner/status payloads |
| `e7743d9c` | REST/MCP field/coercion/error parity and flagged WS15g resolver |
| `7fd221fe` | Owner deactivation/ban HTTP vectors and two guard mutations |
| `4ae8f1e4` | Docker init for default-parallel Rails reference execution |
| `cf82d476` | Fresh-suite lifecycle isolation, complete implemented scheduler roster/order |

The WS11 cherry-picks adapted only module/oracle rosters to omit unavailable work-event files; those domain implementations were taken from WS11. A failed attempt to take streaming before its lifecycle prerequisite was aborted, then the prerequisite and streaming commits were taken in order. Later fixes to app scheduler integration and test isolation are described above. Every slice was pushed with the required GPT-6.1 Sol coauthor.

## Changes by file

Paths below are relative to `rust/`; grouped bare filenames share the first file's directory. Rails application sources were not edited.

| Files | Change / ownership |
| --- | --- |
| `crates/campfire/src/controllers/agents/conversations.rs`, `crates/campfire/src/controllers/agents/pending.rs`, `crates/campfire/src/controllers/agents.rs`, `crates/campfire/src/controllers.rs` | Shared REST/MCP context, post, DM and stream service calls; canonicalization, separate strong-param/argument rules, payload presentation and Rails error adaptation |
| `crates/campfire/src/controllers/agents/repository_access.rs` | Explicit no-private-access WS15g resolver seam called by event polling |
| `crates/campfire/src/controllers/presenters/agent_payload.rs`, `crates/campfire/src/controllers/messages/by_bots.rs` | Rails payload key order, permalink query order, Drive field order, reply fields, work metadata and render-json/Jbuilder escaping |
| `crates/campfire/src/controllers/bot_http_tests.rs`, `crates/campfire/src/controllers/agent_conversation_tests.rs` | Forty-seven bot and 66 conversation/lifecycle Rails wire cases; row/state assertions; separate REST/MCP enqueue rollback and owner guard tests |
| `crates/campfire/src/controllers/presenters/test_support.rs` | Test-only optional job-runner ownership so lifecycle HTTP checks can keep durable cleanup pending; other seeded apps retain their normal workers |
| `crates/db/src/models/agent_context.rs`, `crates/db/src/models/agent_direct_messages.rs`, `crates/db/src/models/agent_lifecycle.rs`, `crates/db/src/models/agent_streaming.rs`, `crates/db/src/models/agent_posting.rs`, `crates/db/src/models/agent_service.rs` | Imported WS11 services; no separate REST/MCP domain implementations |
| `crates/db/src/models/message.rs`, `user.rs`, `models.rs` | Imported WS11 streaming finalization and owner lifecycle wiring, with module roster integration |
| `crates/db/src/tests/agent_context_test.rs`, `agent_direct_messages_test.rs`, `agent_lifecycle_test.rs`, `agent_streaming_test.rs`, `tests.rs` | Imported WS11 service differential/security/atomicity tests |
| `crates/campfire/src/integrations/agent_streaming.rs`, `crates/campfire/src/integrations/jobs.rs`, `crates/campfire/src/integrations.rs` | Imported WS11 trailing broadcast runtime integration and tests |
| `crates/campfire/src/jobs/periodic.rs`, `crates/campfire/src/jobs/tests.rs`, `crates/campfire/src/ws8_runtime_vectors.json` | Streaming sweep integration; Rails order for all eight implemented tasks; strict roster assertion |
| `reference-tools/agents/conversation_http_contract.rb`, `bot_http_contract.rb`, `vectors/agent_conversation_http.json`, `vectors/agent_bot_http.json` | Production Rails capture and raw wire artifacts; fresh Rails executor per lifecycle case |
| `reference-tools/agents/periodic_contract.rb`, `vectors/agent_periodic.json` | Actual 19-task Rails roster and implemented-task projection for the eight Rust services |
| `reference-tools/agents/check-owner-http-mutations.py`, `check-stream-http-mutation.py` | Compile broken guards/enqueue implementation, require assertion failures, restore sources; compilation failure does not count |
| `reference-tools/agents/run-controller-reference.py` | Default Rails worker count and two-file concurrency; Docker init prevents inherited Redis-child wait |
| `reference-tools/agents/check-http-reference.py`, `verify-http-vectors.py` | Fifty-seven pinned source checks; five regenerated HTTP/MCP artifacts plus scheduler capture and roster agreement |
| `reference-tools/agents/{context,direct_messages,lifecycle,streaming,stream_trailing}_contract.rb`, related `verify-*-mutations.py`, `verify-contracts.py`, `check-reference.py`, `vectors/agents_*_contract.json` | Imported WS11 domain captures and mutation/verification roster additions; API commands use this worker's image/namespace instead of their inherited default WS11 settings |

## Rails controller cases, grouped by file

All 38 pinned reference files ran again with zero failures/errors/skips: **602 runs, 4138 assertions**. These are Rails source-case counts, independent of Rust functions or vector counts. Full source-file-equivalent Rust ports remain partial.

| Rails source file/group | Selected Rust coverage and deferred owner |
| --- | --- |
| `agents_controller_test` | Profile/auth/error vectors; remaining field/session boundaries: WS11-api |
| `concerns/agent_authentication_test` | Unknown/revoked/expired tokens, human-route refusal, bot reply scope; broader credential transitions: WS11-api/WS11 |
| `agents/api_throttle_test`, `budgets_endpoints_test` | Digest/action/minute buckets, credential isolation, shared MCP/REST counts, overflows and Retry-After; exhaustive callback/order/budget variants: WS11-api/WS11 |
| `agents/events_controller_test` | Poll/ack shapes, cursor/replay/grants; full payloads/callbacks and live private PR: WS11/WS15g with API vectors |
| `agents/steps_controller_test`, `slash_commands_controller_test` | Create/update/register/replay/denials/raw duration and rates; all coercions/delete/parent variants: WS11-api/WS11 |
| `agents/approvals_controller_test` | Four REST actions/two tools, selected success/replay/expiry/budget/grants; exhaustive expiry/status filtering/races: WS11-api/WS11 |
| `agents/contexts_controller_test` | Root/thread/trigger wire success, membership/grants/required/mismatch; all limit/type variants: WS11-api |
| `agents/dms_controller_test` | Owner DM success, nested/top fields, target eligibility, Drive/empty/error paths; attachment/callback/type/replay matrix: WS11-api/WS11 |
| `agents/messages_controller_test` | Root/thread success, HTML/Markdown/Drive/reply/replay/budget and selected errors; multipart staging and complete raw types/callback matrix: WS11-api/WS11/storage |
| `agents/streaming_messages_controller_test`, `mcp_streaming_test` | Start/update/finalize wire success, repeat/errors/locked/author policy and durable trailing rollback; race/callback-exception coverage: WS11-api/WS11 |
| `agents/pins_controller_test`, `polls_controller_test`, `mcp_slash_polls_test` | Policy/selected validators/throttles; pin/poll successes and full poll datetime/coercions: WS11 domain + API |
| `agents/posts_controller_test`, `work_controller_test`, `work_handoff_test`, `mcp_handoff_test` | Policy/selected fields/errors/throttles; successes and full board/work/handoff validation: WS12/WS11 + API |
| `agents/fizzy/boards_controller_test`, `cards_controller_test`, `card_actions_controller_test`, `mcp_fizzy_test` | Grants/accounts/IDs/query/action validators/rate/budget/replay; full success/remote failure/coercion matrices: WS15e + API |
| `agents/github/pull_request_actions_controller_test` | Selected policy/account/action/budget/replay/errors; client/action success and live private access: WS15g + API |
| `agents/mcp_controller_test` | Four versions, metadata, errors, Origin/headers, all explicit dispatches and 15 selected successful tools; remaining 23 tools and complete wire/error matrix: named domain owners + API |
| `messages/by_bots_controller_test`, `messages/boosts/by_bots_controller_test` | Forty-seven raw request vectors plus existing runtime tests; attachment/human work-viewer/cache/full-file cases: WS11-api/WS8b-m/WS12/WS11 |
| `agent_capability_test`, `agent_revocation_endpoints_test`, `agent_owner_deactivation_test` | Named auth/grant/revocation guards; seven real owner lifecycle HTTP cases plus inherited domain assertions; complete source-equivalent lifecycle scenarios: WS11/API |
| `agents/approval_delivery_test`, `drive_attachments_delivery_test`, `github_action_delivery_test`, `slash_command_delivery_test`, `work_delivery_test`, `fizzy/action_delivery_test` | Existing imported domain/runtime tests execute; complete source-equivalent delivery/callback/SSRF ports remain WS11/WS15g/WS15e/WS12 |
| `agents/directory_controller_test` | Source file passed; HTML/directory/admin/system port is WS11-ui |

Raw per-file summaries, each exit 0:

```text
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/api_throttle_test.rb: 13 runs, 1088 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/approval_delivery_test.rb: 5 runs, 26 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/approvals_controller_test.rb: 24 runs, 85 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/budgets_endpoints_test.rb: 8 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/contexts_controller_test.rb: 13 runs, 50 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/directory_controller_test.rb: 6 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/dms_controller_test.rb: 18 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/drive_attachments_delivery_test.rb: 3 runs, 6 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/events_controller_test.rb: 43 runs, 206 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/action_delivery_test.rb: 3 runs, 14 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/boards_controller_test.rb: 13 runs, 28 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/card_actions_controller_test.rb: 11 runs, 43 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/cards_controller_test.rb: 8 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github/pull_request_actions_controller_test.rb: 21 runs, 82 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github_action_delivery_test.rb: 9 runs, 51 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_controller_test.rb: 71 runs, 660 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_fizzy_test.rb: 28 runs, 333 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_handoff_test.rb: 8 runs, 95 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_slash_polls_test.rb: 14 runs, 60 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_streaming_test.rb: 11 runs, 71 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/messages_controller_test.rb: 15 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/pins_controller_test.rb: 10 runs, 29 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/polls_controller_test.rb: 12 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/slash_command_delivery_test.rb: 2 runs, 4 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/slash_commands_controller_test.rb: 15 runs, 38 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/steps_controller_test.rb: 12 runs, 38 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/streaming_messages_controller_test.rb: 17 runs, 77 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_delivery_test.rb: 9 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_handoff_test.rb: 13 runs, 106 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents_controller_test.rb: 10 runs, 30 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/concerns/agent_authentication_test.rb: 13 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

## Commands and raw evidence rerun in this continuation

### Reference source files

From the assigned worktree root:

```bash
python3 rust/reference-tools/agents/run-controller-reference.py > .scratch/rails-init-current.log 2>&1
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb > .scratch/rails-security-init-current.log 2>&1
```

The raw file lines above come from those two runs. The 71-case MCP file's log reports `Running 71 tests in parallel using 20 processes`. No `PARALLEL_WORKERS` override is supplied. The first run without Docker init hung before tests; it was stopped, the runner fixed, and every file rerun.

### Compiled fail-first checks

From the assigned worktree root:

```bash
python3 rust/reference-tools/agents/check-http-mutations.py > .scratch/security-current.log 2>&1
python3 rust/reference-tools/agents/check-owner-http-mutations.py > .scratch/owner-final-mutants.log 2>&1
python3 rust/reference-tools/agents/check-stream-http-mutation.py > .scratch/stream-final-mutant.log 2>&1
```

All three scripts exit 0 only when broken implementation tests fail for assertions and source restoration completes. Raw output:

```text
WS11-api mutation human_token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.70s
WS11-api mutation mirror_headers: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.65s
WS11-api mutation revoked_credential: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.86s
WS11-api mutation expired_credential: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 1.76s
WS11-api mutation missing_grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.88s
WS11-api mutation nonmember_hidden: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.73s
WS11-api mutation reply_create_only: test result: FAILED. 1 passed; 5 failed; 0 ignored; 0 measured; 384 filtered out; finished in 1.07s
WS11-api mutation rate_overflow: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.86s
WS11-api mutation budget_overflow: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.72s
WS11-api mutation mcp_version: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.63s
WS11-api mutation duration_input: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.63s
WS11-api mutations: 11 broken guards rejected; all source files restored
WS11-api owner baseline: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 388 filtered out; finished in 2.16s
WS11-api owner credentials mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 1.07s
WS11-api owner bot_keys mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 389 filtered out; finished in 0.67s
WS11-api owner lifecycle mutations: 2 broken guards rejected; source restored
WS11-api HTTP enqueue mutation: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.66s
WS11-api HTTP enqueue mutation: dropped job rejected; source restored
```

The new malformed-field/wire tests also failed before the corresponding implementation corrections (checkpoint logs retained in `.scratch/`):

```text
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 381 filtered out; finished in 8.40s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 6.28s
```

They subsequently pass in the full fresh-clone app run below. The first failed fresh clone (before the final integration slice) reported:

```text
test result: FAILED. 385 passed; 3 failed; 2 ignored; 0 measured; 0 filtered out; finished in 77.14s
```

Those were the two owner row-delta races and the scheduler roster described above. No test was ignored, filtered out of the final workspace command, slowed by a new worker override, or given a wider threshold to obtain the successful rerun.

### Fresh clone, new seeds and full Rust suite

From the assigned worktree root:

```bash
git clone --no-hardlinks --single-branch --branch rust/ws11api-rest-mcp . .scratch/fresh4
mkdir -p .scratch/fresh4-tmp .scratch/fresh4-oracles
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh4/rust/parity/bin/seed build default first_run > .scratch/fresh4-seed.log 2>&1
```

The clone checked out `cf82d476d9146b43a8fe83fd10cf2b3f0bc3dd4e`. Its seeds were built anew rather than copied. Only this worker's private target cache is reused. Raw seed output:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

From `.scratch/fresh4/rust/`:

```bash
CI=1 CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/fresh4-tmp CABLE_TEST_PORT_RANGE=52900-52919 MAIL_TEST_PORT_RANGE=52920-52949 INTEGRATION_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j4 --workspace --exclude html5ever -- --nocapture > ../../fresh4-workspace.log 2>&1
CI=1 CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/fresh4-tmp mise exec rust@1.98.1 -- cargo clippy --locked -j4 --workspace --exclude html5ever --all-targets -- -D warnings > ../../fresh4-clippy.log 2>&1
```

Both commands exit 0. No `--test-threads` argument is supplied. Every raw cargo summary (including doctests):

```text
test result: ok. 388 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 72.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.46s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 459 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 146.60s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.63s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 7.12s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.43s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.91s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.09s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.86s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.73s
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

From the assigned worktree root:

```bash
python3 rust/reference-tools/agents/summarize-http-tests.py .scratch/fresh4-workspace.log > .scratch/fresh4-test-summary.log
```

```text
WS11-api cargo totals: 1462 passed; 0 failed; 9 ignored; 46 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.94s
```

The nine existing ignores are shown explicitly:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

The intentional missing-seed helper unit emits one local-skip notice and passes; there are zero actual missing-seed skips. The existing storage fixture condition skipped version-dependent media byte comparisons because the host versions differ from the fixture versions. This is not full pinned-media-byte validation:

```text
skipping byte comparisons that depend on versions: vectors have libvips "8.16.1" / "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", local libvips 8.18.6 / ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
```

### Fresh Rails HTTP/MCP and scheduler captures

From `.scratch/fresh4/` (each command exited 0; output files were compared only after completion):

```bash
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/http_contract.rb > ../fresh4-oracles/agent_http.json 2> ../fresh4-oracles/http_contract.rb.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/mcp_contract.rb > ../fresh4-oracles/agent_mcp.json 2> ../fresh4-oracles/mcp_contract.rb.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/surface_contract.rb > ../fresh4-oracles/agent_surface.json 2> ../fresh4-oracles/surface_contract.rb.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/bot_http_contract.rb > ../fresh4-oracles/agent_bot_http.json 2> ../fresh4-oracles/bot_http_contract.rb.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/conversation_http_contract.rb > ../fresh4-oracles/agent_conversation_http.json 2> ../fresh4-oracles/conversation_http_contract.rb.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/periodic_contract.rb > ../fresh4-oracles/agent_periodic.json 2> ../fresh4-oracles/periodic_contract.rb.log
python3 rust/reference-tools/agents/verify-http-vectors.py ../fresh4-oracles > ../fresh4-oracle-summary.log 2>&1
python3 rust/reference-tools/agents/check-http-reference.py > ../fresh4-source.log 2>&1
python3 rust/reference-tools/agents/test-http-vector-verifier.py > ../fresh4-verifier-tests.log 2>&1
```

Raw results:

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 265 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 47 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh scheduler oracle: 19 Rails tasks captured; all 8 implemented Rust tasks asserted in Rails order
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 82 asserted vectors; 2 explicitly deferred list success vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 57 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.002s

OK
```

### Imported domain captures from the final fresh clone

From `.scratch/fresh4/`, each command exited 0:

```bash
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/context_contract.rb > ../fresh4-domain-oracles/agents_context_contract.json 2> ../fresh4-domain-oracles/context_contract.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/direct_messages_contract.rb > ../fresh4-domain-oracles/agents_direct_messages_contract.json 2> ../fresh4-domain-oracles/direct_messages_contract.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/lifecycle_contract.rb > ../fresh4-domain-oracles/agents_lifecycle_contract.json 2> ../fresh4-domain-oracles/lifecycle_contract.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/streaming_contract.rb > ../fresh4-domain-oracles/agents_streaming_contract.json 2> ../fresh4-domain-oracles/streaming_contract.log
PARITY_NAMESPACE=ws11api-fresh4 PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/stream_trailing_contract.rb > ../fresh4-domain-oracles/agents_stream_trailing_contract.json 2> ../fresh4-domain-oracles/stream_trailing_contract.log
```

Each regenerated file was compared directly as bytes with the committed fresh-clone vector. Raw comparison summaries:

```text
WS11-api fresh domain oracle: agents_context_contract.json; byte-identical committed vector
WS11-api fresh domain oracle: agents_direct_messages_contract.json; byte-identical committed vector
WS11-api fresh domain oracle: agents_lifecycle_contract.json; byte-identical committed vector
WS11-api fresh domain oracle: agents_streaming_contract.json; byte-identical committed vector
WS11-api fresh domain oracle: agents_stream_trailing_contract.json; byte-identical committed vector
WS11-api fresh domain oracles: 5 byte-identical files
```

## Open questions and integration handoff

No user clarification is required to continue the API work. Domain readiness and presenter/callback ownership are the remaining integration dependencies. Keep the WS15g seam explicit until its live resolver is available; do not implement GitHub access in WS11-api. Keep the current selected HTTP wire tests while completing the 17 REST and 23 MCP service successes and then expanding each source file's validators/error/status/header/callback cases. This report remains partial.
