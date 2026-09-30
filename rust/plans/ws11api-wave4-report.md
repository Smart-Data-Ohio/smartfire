# WS11-api Wave 4 report — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws11api-rest-mcp`.
Assigned worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api`.
Base: WS11 `35b4bff4`, including main `21a7332f`. **Main was not merged.**
Fresh-clone Rust suite/clippy head: `c5a04a25d6546880898eba9836b7129af9eaf421`.
Fresh-clone oracle verification head: `49e12305a4c7702b3ad202585ad4fc1903aff05a` (Python verification only; identical Rust code and fixtures).
The following report commit changes documentation only. Both this tracked report and the requested `wave4/ws11api-report.md` contain the same report. No PR or deployment was created.

## Delivered and partial boundaries

The continuation adds approvals (four REST actions and two MCP tools) backed by WS11's existing transaction services. It adds the remaining 23 JSON REST adapters and explicit dispatch for the remaining 29 MCP tools, with shared REST/MCP authorization and selected validation/error paths around a flagged domain seam. All 35 JSON REST actions have Rails invalid-credential and authenticated-human-session vectors. All 29 throttled MCP tools have exact overflow response and Retry-After vectors; another 22 REST overflow cases extend the original rate coverage.

The bot slice compares 40 production Rails request/response pairs, including root-only listing, raw create/update, board rejection, creator/system-note policy, Drive fields, reply/forward/stream fields, ordinary thread summaries, user icons, boosts, credential rejection, grant rejection, daily budgets, and reply-token create-only scope. Persisted message/boost counts and body/Drive state are asserted. The existing upstream bot test now expects Smartfire's nine-field root payload; the fresh suite caught its stale six-field assertion.

**This is partial acceptance.** Nine MCP tools have selected working success paths (the original seven plus `request_approval` and `get_approval`). The other 29 have explicit transport/input/policy handling, but authorized success still reaches `agent_api_pending::execute`: REST raises an internal error and MCP returns `-32603`. No empty list or successful mutation is fabricated. Every validation/error variant is not complete. Full bot work-thread payloads and attachment/callback differentials remain. The source Rails files passing does not mean their entire scenarios have been ported to Rust.

There are **422 committed Rails pairs, 420 asserted distinct pairs**: 33 base REST, 82 of 84 base MCP, 265 surface (179 REST + 86 MCP), and 40 bot. The two unasserted pairs are the deliberately deferred `list_rooms` and `list_work` success responses. The 20 added Rust test functions in the continuation group the new surface and bot cases. The repeated focused bot budget/nonmembership tests do not add distinct vectors.

## Pushed slices

| Commit | Change |
| --- | --- |
| `f7c18609` | Event, step and slash REST adapters |
| `ca06740e` | Stateless MCP transport and original seven tools |
| `6b0ebb02` | Profiles, Ruby coercions and raw step-duration validation |
| `9b7dc347` | Existing session-key test creates its output parent in a clean checkout |
| `23b75654` | Approvals REST and shared MCP operations |
| `7abfed94` | Conversation/work policy and selected errors around WS11 seams |
| `c4d56ef0` | Integration policies, local action validators and explicit dispatch for all tools |
| `37079d4b` | Bot message/boost payloads and persisted-state Rails vectors |
| `23c04180` | Reproducible fail-first mutations for 11 security/input guards |
| `7bc52c14` | Endpoint-wide credential/session and overflow matrix |
| `c5a04a25` | Correct stale upstream bot assertion; verifier injection tests |
| `49e12305` | Prove the pinned source checker rejects image and checkout drift |

Each slice was pushed and includes `Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>`. The previous documentation commit `429a962e` is superseded by this report.

## Changes by file

Paths are relative to `rust/`; Rails sources were not edited.

| File(s) | Change |
| --- | --- |
| `crates/campfire/src/controllers/agents.rs` | Profile/event/step/slash JSON actions and shared operations; Ruby string conversions and raw duration checks; exports approval/pending/integration modules |
| `crates/campfire/src/controllers/agents/approvals.rs` | List/show/create/cancel; nested/top-level input precedence, raw expiry parsing, shared MCP create operation; WS11 lifecycle service calls |
| `crates/campfire/src/controllers/agents/pending.rs` | Seventeen REST actions; shared policy preflights for membership/reachability, grants, DM ledger eligibility, ownership, locked/nonstreaming messages, board type, budgets, poll limits, required fields and cursors |
| `crates/campfire/src/controllers/agents/integrations.rs` | Five Fizzy REST actions and GitHub action REST; same MCP Fizzy operations; linked-account usability, encrypted token reads, invalid-account marking, capability/account/ID checks, replay-before-budget-before-validation, selected action validators |
| `crates/campfire/src/controllers/agents/mcp.rs` | Four-version stateless protocol, Origin/header checks, shared throttles, seven error codes, all 38 explicit tool branches; dispatch to existing services or shared pending/integration operations |
| `crates/campfire/src/controllers/agents/mcp_metadata.json` | Byte-identical descriptions/schemas/versions/throttle metadata extracted from pinned Rails |
| `crates/campfire/src/concerns/agent_api.rs`, `concerns.rs` | Bearer policy, selective CSRF rescue, current identity, no-store, digest-based credential/action minute buckets; exact HTTP overflow and MCP Retry-After propagation |
| `crates/campfire/src/controllers.rs`, `app.rs` | Real-router dispatch and test modules; unparsed MCP route retains the kit upload limit |
| `crates/campfire/src/controllers/messages/by_bots.rs` | Index/update use request-specific MessagePayloadHelper JSON; existing WS11 auth/posting policies retained |
| `crates/campfire/src/controllers/messages/boosts/by_bots.rs` | Fresh user/icon response; RecordInvalid becomes Rails 422 errors response |
| `crates/campfire/src/controllers/messages.rs`, `presenters.rs` | Set requesting-user context and dispatch to root payload helper; remove obsolete cached MessageJson renderer |
| `crates/campfire/src/controllers/presenters/agent_payload.rs` | Root payload, timestamps/client ID, Markdown presentation, Drive IDs, creator/room icons, permalinks, reply/forward/stream fields; ordinary bot-viewed thread JSON; explicit work/human-thread seam |
| `crates/campfire/src/rich_text.rs` | Expose the existing shared icon loader within the crate; no duplicate catalog |
| `crates/campfire/src/controllers/messages/tests.rs` | Existing bot payload keys now match the pinned fork; clarify request-specific JSON cache behavior |
| `crates/campfire/src/controllers/agent_http_tests.rs`, `agent_mcp_tests.rs` | Original REST/transport/service/security vectors; assert 29 more tools' empty-argument errors, retaining two explicit list-success deferrals |
| `crates/campfire/src/controllers/agent_surface_tests.rs` | Ten seeded tests over 265 exact status/body/header vectors, grouped by service and matrix |
| `crates/campfire/src/controllers/bot_http_tests.rs` | Ten seeded test functions, 40 distinct Rails vectors, persisted-state checks and six separate reply-token actions |
| `crates/campfire/src/controllers/presenters/test_support.rs`, `concerns/session_keys.rs` | Frozen test clock; existing output-writing test creates its parent directory |
| `crates/db/src/models/agent_api_pending.rs`, `models.rs` | Minimal explicit no-success domain seam shared by REST and MCP; no writes |
| `crates/db/src/models/agent_approvals.rs` | Small input-error entry point after service policy/replay/budget and before validation/write; existing API delegates with no extra input error |
| `crates/db/src/models/agent_step.rs`, `slash_commands.rs` | Small raw-input validation seams and reuse of existing error sentence formatter |
| `reference-tools/agents/http_contract.rb`, `mcp_contract.rb` | Original production Rails captures; runtime-built fake authorization headers |
| `reference-tools/agents/surface_contract.rb`, `bot_http_contract.rb` | Production Rails service/matrix/bot captures with matching deterministic DB setup, raw responses and persisted state |
| `reference-tools/agents/check-http-reference.py`, `extract-mcp-metadata.py` | Check 48 pinned source inputs and regenerate metadata |
| `reference-tools/agents/run-controller-reference.py` | Run pinned controller test sources in isolated Docker containers; per-file logs and counts; two concurrent files |
| `reference-tools/agents/check-http-mutations.py`, `check-surface-mutations.py` | Broken implementation guard/preflight checks; require assertion failures and restore sources |
| `reference-tools/agents/verify-http-vectors.py`, `test-http-vector-verifier.py` | Exact regenerated vector comparison and coverage accounting; injected byte/status/body/header/missing-dispatch/image/checkout differences rejected; combined match-arm alternatives recognized |
| `reference-tools/agents/summarize-http-tests.py` | Aggregate raw workspace counts and distinguish actual seed skips from the deliberate missing-seed unit test |
| `vectors/agent_http.json`, `agent_mcp.json`, `agent_surface.json`, `agent_bot_http.json` | Actual pinned Rails request/response fixtures; no hand-edited responses |
| `crates/campfire/Cargo.toml`, `Cargo.lock` | Existing workspace URL dependency used by Origin parsing (original transport slice) |
| `plans/ws11api-wave4-report.md` | This partial report |

## Design notes and cross-workstream touches

REST and MCP enter the same operation/service path after their transport-specific callback, authentication and throttle order. Existing WS11 service APIs remain in use for events, acknowledgments, steps, slash registration, presence and approvals. Missing domains have a named shared seam, not transport-specific fake behavior. The pending adapter contains the shared policy/preflight code until WS11 supplies its service; it does not implement posting, work transitions or integration clients.

The inherited credential authentication queries current digest/revocation/expiry/active-user rows. Membership checks and current grants preserve the selected 404-before-403 behavior. Controller order is reproduced individually: some room controllers locate the member room before the Bearer check; the context REST grant check is earlier than its service validation; Fizzy uses its own forbidden message and keeps its 404 JSON body. Empty 404 bodies are preserved where Rails calls `head`. Authenticated human-session cases include a real signed Rails cookie and verified two-factor timestamp, and no CSRF token, rather than treating absence of a credential as a session.

Per-credential minute buckets are keyed by digest and Rails controller/action. MCP shares REST tool buckets and has a separate 600/min endpoint bucket, charged before Origin. Ack batches charge each event. Counter minute boundaries, credential/action isolation, limits, no-store and Retry-After are asserted. Daily overflow uses existing WS11 budget services and preserves structured cap/limit/retry_after. The matrix validates 70 REST authentication cases and 29 MCP overflow cases; it does not claim every service validation branch is finished.

MCP uses an unparsed route with the kit body bound. JSON parse failures before callbacks and text/plain failures after callbacks match the selected oracle cases. Modern Base64/UTF-8 headers, version disagreement, unsupported typed versions, notifications, methods, Origin case/port behavior and Rails network-path Origin are covered. Metadata/discovery is complete; operation success is separately partial.

Fizzy/GitHub token strings are decrypted through the real AR encryption implementation using fake encrypted fixture values. Unreadable linked accounts are marked unusable. GitHub identity precedence accepts the owner's App account then the agent's PAT; an owner PAT is ignored, and an expired App account is still locally usable as Rails does. Refresh, remote client errors and successful action creation/execution belong to WS15g/e and remain seamed. The new integration fixtures never call a remote service.

Message JSON is assembled for the requesting user/host and is not cached as shared Jbuilder JSON. Ordinary bot-viewed thread membership/permissions use existing ChannelThread APIs. Work/human thread payloads raise a named seam rather than guessing the WS12 fields. The root helper also improves message-bearing event rendering, but event vectors in this branch still lack message-bearing/private-PR coverage. Existing forged-host isolation and all inherited message tests pass in the fresh run; a new two-user work-thread payload differential remains deferred.

Explicit seams:

- **WS11:** `agent_api_pending::execute(tx: &mut Tx, agent_id: i64, operation: &str, fields: Value) -> Result<ServiceResult>` for missing service success/domain behavior. Fields retain omission/null distinctions. Before implementing approved external actions, extend this generic seam to carry the authenticated credential ID (the current pending seam does not carry it); the working approval path already passes `CurrentAgent.credential_id`. Do not infer credential identity from parameters.
- **WS11:** `agent_approvals::create_with_input_error` and the original step input-error entry points are the only flagged extensions to existing WS11 domain service files. They preserve existing typed entry points and policy-before-input-validation order.
- **WS8b-m / WS8b-m2 and WS12:** request-specific complete MessagePayloadHelper/work-thread payloads; `Presenter::bot_thread_payload` currently handles ordinary bot-viewed threads only. Coordinate helper ownership instead of merging duplicate presenters.
- **WS11 / WS15g:** event polling still passes default empty `RepositoryAccess`; live private-PR decisions must be supplied by the domain resolver. Restricted fields are not granted by this seam.
- **WS12:** board/work/result/handoff mutations, owner eligibility, full field coercions, callbacks/history and atomic events.
- **WS15g / WS15e:** GitHub/Fizzy read/action clients, approval creation, refresh, success payloads and remote failure mapping.

No parity masks, allowlists, ignores or missing-seed escape hatches were added. Status and decoded JSON are compared exactly, selected response headers include Retry-After, and MCP content text is exact. Byte-identical regenerated fixture files do not prove byte-identical serialization for every HTTP body or an entire Rails file port.

## Rails controller cases, grouped by file

All 38 listed source files ran in Rails at `d7c7de92` with zero failures/errors/skips. These source counts are independent of the Rust scenario counts. The table below records which scenarios are represented in the Rust vectors and what remains; full Ruby-file-equivalent ports are still partial.

| Rails file/group (under `test/controllers/`) | Selected Rust coverage | Remaining ownership |
| --- | --- | --- |
| `agents_controller_test` | Profile get/update/clear/invalid/ignored fields; auth matrix | WS11-api: remaining profile/session/status-note boundary cases |
| `concerns/agent_authentication_test` | Unknown/revoked/expired credentials, agent on human endpoint, bot reply scope; auth matrix | WS11-api with WS11 domain: all malformed auth/suspension/credential transitions |
| `agents/api_throttle_test` | Credential/action/minute isolation, shared MCP/REST buckets, coarse order, all throttled MCP tools and selected REST overflow | WS11-api: remaining callback/order variants and exhaustive per-action audit |
| `agents/events_controller_test` | Bare/enveloped polling, cursor filtering, ack/replay/not-found and grants | WS11-api: message/private-PR event payloads; WS11/WS15g live access |
| `agents/steps_controller_test`, `agents/slash_commands_controller_test` | Create/register/replay, missing parent/record, raw duration, grants/nonmembership, overflow | WS11-api: all update/delete/coercion/parent types; WS11 callbacks |
| `agents/approvals_controller_test` | List/show/expire/cancel/create/nested fields/replay/prefix denial/expiry error/grants/budget/rate | WS11-api: full expiry coercions, status filters and remaining HTTP cases; WS11 lifecycle/races |
| `agents/budgets_endpoints_test` | Approval/message/bot/external-action overflows with body and retry header | WS11-api: board and all endpoint budget variants; WS11 notices/atomic concurrency |
| `agents/contexts_controller_test`, `agents/dms_controller_test` | Required conversation, missing/mismatched source, read grant, target/bot/DM eligibility denials | WS11-api: full input and authorized responses; WS11 context/DM services |
| `agents/messages_controller_test`, `agents/streaming_messages_controller_test` | Member/grant/budget denials, missing/nonstreaming message, required tool arguments and throttles | WS11-api: full validation/errors and success; WS11 posting/streaming/trailing broadcast |
| `agents/pins_controller_test`, `agents/polls_controller_test` | Missing/reachable/grant paths, question/option count/shape and throttle | WS11-api: full poll closes_at/raw coercions, lifecycle and successes; WS11 domain |
| `agents/posts_controller_test` | Board type/grants/create budget preflight and MCP required arguments | WS11-api: status/owner/tag/title/run_url validation and success; WS12/WS11 domain |
| `agents/work_controller_test`, `agents/work_handoff_test` | Owned/member/read/manage hiding, missing markdown, required args and handoff throttles | WS11-api: work_status/note/tags/run_url/result length, receiver eligibility/package validation and success; WS11/WS12 domain |
| `agents/fizzy/boards_controller_test`, `agents/fizzy/cards_controller_test` | Workspace grant, owner/account/ID/query/card-number failures and throttle | WS11-api: all client response/error contracts; WS15e client |
| `agents/fizzy/card_actions_controller_test` | Local kind/field/length/account errors, budget and replay; shared MCP operations | WS11-api: remaining raw-field coercions/full success; WS15e action domain |
| `agents/github/pull_request_actions_controller_test` | Membership/mapped-PR/grant/account/identity, local action errors, budget/replay/rate | WS11-api: full client/action cases; WS15g client/domain/live access |
| `agents/mcp_controller_test` | Four versions, methods, transport errors, security headers/Origin, metadata, nine selected successful tools and remaining explicit error dispatch | WS11-api: complete tool success/error matrix, session/raw-input variants |
| `agents/mcp_fizzy_test` | Nine explicit Fizzy branches, grant/account/ID/query errors and overflow | WS11-api + WS15e: successful reads/actions and remote errors |
| `agents/mcp_handoff_test`, `agents/mcp_slash_polls_test`, `agents/mcp_streaming_test` | Required args, membership/grants/not-found/selected validation and throttle | WS11-api: full validators/successes; WS11/WS12 domains |
| `messages/by_bots_controller_test`, `messages/boosts/by_bots_controller_test` | Forty real requests, persisted state, six reply-token actions, board/root/grants/system-note/boost/payload cases; inherited runtime attachment tests pass | WS11-api: complete work-thread/attachment/error/callback differentials; mock-only Boost save-false Rails case is not reproduced through a mock |
| `agent_capability_test`, `agent_revocation_endpoints_test`, `agent_owner_deactivation_test` | Selected API credential/grant paths plus inherited WS11 domain tests | WS11-api: all top-level lifecycle HTTP scenarios; WS11 revocation/owner transitions |
| `agents/approval_delivery_test`, `agents/drive_attachments_delivery_test`, `agents/github_action_delivery_test`, `agents/slash_command_delivery_test`, `agents/work_delivery_test`, `agents/fizzy/action_delivery_test` | Existing WS11 tests run in the seeded Rust suite; no new full file port claimed | WS11 with WS15g/e/WS12: domain/jobs/delivery/SSRF/atomic enqueue |
| `agents/directory_controller_test` | Rails source suite executed; no UI port added here | WS11-ui: directory/HTML/admin/account tests and system pages |

Raw per-file Rails summaries (full counts; each command exited 0):

```text
test/controllers/agents/approval_delivery_test.rb: 5 runs, 26 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/approvals_controller_test.rb: 24 runs, 85 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/api_throttle_test.rb: 13 runs, 1088 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/budgets_endpoints_test.rb: 8 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/directory_controller_test.rb: 6 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/contexts_controller_test.rb: 13 runs, 50 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/dms_controller_test.rb: 18 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/drive_attachments_delivery_test.rb: 3 runs, 6 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/action_delivery_test.rb: 3 runs, 14 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/events_controller_test.rb: 43 runs, 206 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/boards_controller_test.rb: 13 runs, 28 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/cards_controller_test.rb: 8 runs, 19 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/fizzy/card_actions_controller_test.rb: 11 runs, 43 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github/pull_request_actions_controller_test.rb: 21 runs, 82 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github_action_delivery_test.rb: 9 runs, 51 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_fizzy_test.rb: 28 runs, 333 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_controller_test.rb: 71 runs, 660 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_handoff_test.rb: 8 runs, 95 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_slash_polls_test.rb: 14 runs, 60 assertions, 0 failures, 0 errors, 0 skips; exit 0
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
test/controllers/concerns/agent_authentication_test.rb: 13 runs, 22 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_handoff_test.rb: 13 runs, 106 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

## Re-run commands and raw verification

These commands ran in this continuation. Rust 1.98.1, `-j 4`, private target/TMPDIR, no release profile, only assigned ports. The reference image tag is checked against source hashes, not trusted by its name. Source controller tests are bind-mounted from the pinned checkout because the production image lacks their files.

From the original worktree's `rust/`:

```bash
python3 reference-tools/agents/run-controller-reference.py > ../.scratch/rails-controller-summaries.log 2>&1
python3 reference-tools/agents/run-controller-reference.py test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb > ../.scratch/rails-security-controller-summaries.log 2>&1
```

Both exited 0. Aggregate calculated from the raw lines above: **38 files, 602 runs, 4138 assertions, 0 failures, 0 errors, 0 skips**. This runs source Rails tests and does not inflate the Rust coverage count.

### Fail-first and discriminating checks

From the original `rust/`:

```bash
python3 reference-tools/agents/check-http-mutations.py > ../.scratch/security-mutations-final.log 2>&1
TMPDIR="$PWD/../.scratch/tmp" CI=1 mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire agent_ -- --nocapture > ../.scratch/security-restored-final.log 2>&1
python3 reference-tools/agents/check-surface-mutations.py > ../.scratch/surface-mutations-final2.log 2>&1
```

The scripts require a nonzero test exit, a FAILED summary and assertion failure; compiler failure or absent seed cannot count as fail-first. Every mutation is restored in `finally`. Raw output:

```text
WS11-api mutation human_token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 2.90s
WS11-api mutation mirror_headers: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 0.97s
WS11-api mutation revoked_credential: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.25s
WS11-api mutation expired_credential: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 2.20s
WS11-api mutation missing_grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.33s
WS11-api mutation nonmember_hidden: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.09s
WS11-api mutation reply_create_only: test result: FAILED. 1 passed; 5 failed; 0 ignored; 0 measured; 372 filtered out; finished in 1.06s
WS11-api mutation rate_overflow: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.20s
WS11-api mutation budget_overflow: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 2.06s
WS11-api mutation mcp_version: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.72s
WS11-api mutation duration_input: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 377 filtered out; finished in 1.40s
WS11-api mutations: 11 broken guards rejected; all source files restored
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 340 filtered out; finished in 55.18s
WS11-api preflight mutation: test result: FAILED. 6 passed; 4 failed; 0 ignored; 0 measured; 368 filtered out; finished in 46.78s
WS11-api preflight mutation: four new groups rejected bypass; source restored
WS11-api integration mutation: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 376 filtered out; finished in 0.65s
WS11-api integration mutation: both groups rejected bypass; source restored
```

The security mutations individually disable human-endpoint denial, modern header checks, revoked credential checks, expiry, missing grants, hidden nonmembership, reply-create-only, minute overflow, daily budget overflow, unsupported versions and raw duration validation. Reply-create-only shows create still passes while all five non-create actions fail. The surface mutations reject bypasses in all four conversation/work test groups and both integration groups. These are executable regression checks, not mocked services.

Historical bot payload fail-first evidence, before the payload helper fix (retained logs; not current test totals):

```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 366 filtered out; finished in 3.67s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 375 filtered out; finished in 10.38s
```

Those assertions failed for the absent Smartfire payload/icon fields and ordinary thread summary. The final fresh run includes the restored/fixed bot checks. New atomic enqueue failure cases for every added REST service and **SSRF/private-address/DNS-rebinding fail-first evidence** remain WS11 work, coordinated with WS11-api's full HTTP paths; inherited webhook security/runtime tests pass in the fresh suite but are not presented as new fail-first proof.

### Fresh clone, rebuilt seeds, regenerated vectors

The verification clone is `.scratch/fresh2`, created from this branch with:

```bash
git clone --no-hardlinks --single-branch --branch rust/ws11api-rest-mcp . .scratch/fresh2
```

It was fast-forwarded to `c5a04a25` before the final Rust run and to `49e12305` before the final oracle-gate run. Rust code and fixtures are unchanged between those two commits. Its source tree was clean. It uses private `.scratch/clean-target` as its own warm Cargo cache and separate `.scratch/fresh2-tmp`; no other worker's cache is shared. All crate sources rebuild from the clone paths. Seeds were rebuilt inside this clone, not copied from the working checkout.

From the assigned root:

```bash
PARITY_NAMESPACE=ws11api-fresh2 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh2/rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

From `.scratch/fresh2/rust/`:

```bash
PARITY_NAMESPACE=ws11api-fresh2 PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/http_contract.rb > ../../fresh2-oracles/agent_http.json
PARITY_NAMESPACE=ws11api-fresh2 PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/mcp_contract.rb > ../../fresh2-oracles/agent_mcp.json 2> ../../fresh2-mcp-oracle.log
PARITY_NAMESPACE=ws11api-fresh2 PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/surface_contract.rb > ../../fresh2-oracles/agent_surface.json 2> ../../fresh2-surface-oracle.log
PARITY_NAMESPACE=ws11api-fresh2 PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/bot_http_contract.rb > ../../fresh2-oracles/agent_bot_http.json 2> ../../fresh2-bot-oracle.log
python3 reference-tools/agents/test-http-vector-verifier.py > ../../fresh2-verifier-tests.log 2>&1
python3 reference-tools/agents/verify-http-vectors.py ../../fresh2-oracles > ../../fresh2-oracle-summary-final.log
python3 reference-tools/agents/check-http-reference.py > ../../fresh2-reference-final.log
python3 reference-tools/agents/extract-mcp-metadata.py > ../../fresh2-metadata-summary-final.log
```

All exited 0. The verifier tests inject actual changed status, JSON body, header, bytes and a missing alternative dispatch branch into the functions used by the checker; all seven planted differences (including image and checkout drift) are rejected.

```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.012s

OK
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 265 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 40 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 82 asserted vectors; 2 explicitly deferred list success vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 48 pinned files matched; 0 image or checkout mismatches (d7c7de92)
WS11-api MCP metadata: 38 Rails tools; 4 protocol versions
```

The first verifier run had already matched all four regenerated files, then incorrectly reported the first name in Rust's `register | unregister` match arm missing. `c5a04a25` corrects that branch recognizer and adds injection tests. It does not change the exact-byte comparison or relax a response assertion.

### Fresh seeded workspace and clippy

From `.scratch/fresh2/rust/`:

```bash
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/fresh2-tmp CI=1 CABLE_TEST_PORT_RANGE=52900-52919 MAIL_TEST_PORT_RANGE=52920-52949 INTEGRATION_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j 4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../../fresh2-tests-final.log 2>&1
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/fresh2-tmp mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > ../../fresh2-clippy-final.log 2>&1
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > ../../fresh2-metadata-final.json
python3 reference-tools/agents/summarize-http-tests.py ../../fresh2-tests-final.log > ../../fresh2-test-summary-final.log
```

Each exited 0. All raw Cargo summaries, in output order:

```text
test result: ok. 376 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 87.09s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 44.85s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 447 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 132.53s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.32s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 7.65s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.83s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.35s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
WS11-api cargo totals: 1438 passed; 0 failed; 9 ignored; 46 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.15s
```

The app has 376 passing tests and two pre-existing explicit ignores; workspace has 1438 passing tests and nine explicit ignores. WS11's `manages_bots` runs and passes. Ignored tests are two reference-recording tests, one push-latency measurement, three external Ruby DB comparisons/exports, one mail Rails export, and two kit documentation examples. No ignores were added. The only missing-seed notice is the unit test intentionally checking that local behavior; **zero real seeded tests skipped**. Vendored html5ever is excluded as instructed by `rust/AGENTS.md`.

The first fresh run exposed the old bot JSON key assertion after the new fork-correct helper landed:

```text
test result: FAILED. 375 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 108.20s
```

It expected the upstream six-field payload, while all new Rails response vectors passed. `c5a04a25` corrects that exact expected key list to Smartfire's nine fields. No failure was dismissed as inherited, no assertion was removed, and the entire workspace reran successfully above.

## Precisely what remains / resume point

Resume from this branch without merging main until the lead says WS11 has landed. The next service slice should complete conversation/message domain calls and their full raw-input/error vectors, followed by board/work/handoff, then integration clients. Replace the generic seam with WS11's final typed service APIs, including authenticated credential context where needed; retain the production Rails boundary tests.

1. **Twenty-three REST authorized-success paths remain seamed:** context; DMs; posting; pin/unpin; poll create/show; stream start/append/finalize; posts list/create; work list/show/update/result/handoff; Fizzy board list/show/card search/show/actions; GitHub PR actions. Complete their missing 401/403/404/422/429 combinations and validation ordering; the current credential/session/rate matrix is complete only for its stated cases.
2. **Twenty-nine MCP authorized-success paths remain:** `list_rooms`, `read_messages`, `post_message`, `react`, `get_context`, `open_dm`; `pin_message`, `unpin_message`, `create_poll`, `get_poll`, `start_stream`, `append_stream`, `finalize_stream`; `list_board_posts`, `create_board_post`, `update_board_post`, `set_result`, `list_work`, `update_work`, `handoff_work`; `list_fizzy_boards`, `get_fizzy_board`, `search_fizzy_cards`, `get_fizzy_card`, `create_fizzy_card`, `comment_on_fizzy_card`, `move_fizzy_card`, `close_fizzy_card`, `reopen_fizzy_card`. Add each success vector and every remaining error branch through the same REST service. The two committed list-success vectors still need Rust assertions after their services exist.
3. **HTTP preflight validation remains partial even for seam-backed services.** Finish raw shapes and nested-message input precedence; post/poll/thread restrictions and `closes_at`; board title/status/owner/tags/run_url filters; work status/note/tags/run_url/result length; receiver eligibility and handoff package normalization/limits. Complete Rails integer/date/expiry/coercion behavior for approvals and the remaining existing tools. These are WS11-api obligations coordinated with WS11/WS12, not excused by a missing domain.
4. **Bot full parity remains partial.** Complete work-thread summaries/context, rich attachment HTML/status/header vectors and callback/job failure paths, request-specific two-user permission tests, and complete file-equivalent coverage. Current broad inherited attachment runtime tests and 40 new exact Rails vectors pass. The Rails mocked Boost-save failure is recorded as deferred; no natural Boost validator exists to induce it in a real request.
5. **Event/live access remains partial.** Add message-bearing events and all private-PR access/error cases through the live WS11/WS15g resolver. Complete the shared presenter with WS8b-m/m2 and WS12.
6. **Remaining security/domain verification:** new full-HTTP atomic enqueue rollback and races for added mutations, SSRF to loopback/private ranges and DNS-rebinding fail-first evidence, full revocation/owner-deactivation lifecycle HTTP equivalents and the source files' remaining success/error scenarios. Domain/jobs remain WS11; HTTP evidence remains WS11-api. Integration remote error cases remain WS15g/e. HTML/admin/system-page cases remain WS11-ui.

No product decision or approval is pending. The open coordination questions are final WS11 service signatures (especially credential context), live repository-access resolver integration, and the common presenter landing point. This report does not claim full WS11-api acceptance, full bot parity, 38 functional tools, or production readiness.
