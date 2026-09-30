# WS11-api Wave 4 report — PARTIAL

Date: 2026-09-30. Worker: GPT-6.1 Sol. Branch: `rust/ws11api-rest-mcp`.
Base: WS11 `35b4bff4`, including main `21a7332f`.
Verified implementation head: `9b7dc3472f8c3c4a016e5a2dc06f93306c64ab3b`.
The subsequent report commit changes documentation only. No PR or deployment was created.

## Delivered slice

Eight REST actions are wired through the real router: GET/PATCH `/agents/me`, GET `/agents/events`, POST `/agents/events/:id/ack`, POST/PATCH `/agents/steps[/:id]`, and POST/DELETE `/rooms/:room_id/agents/slash_commands[/:name]`. They call the existing WS11 services and use persisted credentials, grants, membership, and SQLite transactions. GET me preserves Rails' session behavior; the mutation and service endpoints enforce their Rails Bearer-only policies.

The stateless MCP transport supports the four pinned versions (`2026-07-28`, `2025-11-25`, `2025-06-18`, `2025-03-26`), modern header mirroring, strict Base64/UTF-8 names, Origin checks, framing, initialize/discover/list/ping/notifications, GET/DELETE 405, and all seven observed JSON-RPC error codes. Seven tool operations are implemented: `poll_events`, `ack_events`, `register_slash_command`, `unregister_slash_command`, `set_presence`, `add_step`, `update_step`.

This is **not full WS11-api acceptance or cutover readiness**. The tool list contains the exact Rails metadata for all 38 tools, but 31 operations still produce Internal error. Message-bearing event polling has an explicit unimplemented presenter seam. REST coverage beyond this slice, complete bot API parity, and full happy/error vectors for every MCP tool remain.

Pushed implementation commits, in order:

| Commit | Slice |
| --- | --- |
| `f7c18609` | Agent event, step and slash HTTP adapters |
| `ca06740e` | Stateless MCP transport and available tools |
| `6b0ebb02` | Profile updates, Ruby coercions and raw step validation |
| `9b7dc347` | Session contract test creates its output directory in clean checkouts |

All include the required GPT-6.1 Sol co-author trailer. Work stayed in the assigned worktree; this external deliverable is the explicit report-path exception. Scratch and Docker names use `ws11api` or `ws11api-clean`; no HTTP servers or ports were needed.

## Changes by file

Paths below are relative to `rust/`.

| File | Change |
| --- | --- |
| `crates/campfire/src/concerns/agent_api.rs` | Bearer-only guard, selective CSRF rescue, no-store helper, SHA-256 credential minute buckets, exact overflow JSON/Retry-After, counter boundary/isolation test |
| `crates/campfire/src/concerns.rs` | Preserve `CurrentAgent { agent_id }` from WS11's existing `authenticate_identity` together with CurrentUser; expose the new concern |
| `crates/campfire/src/controllers/agents.rs` | Thin profile/event/step/slash JSON actions and shared transport operations, Ruby string/inspect conversions and duration input checks |
| `crates/campfire/src/controllers/agents/mcp.rs` | Stateless protocol, seven service-backed tool operations, shared REST/tool buckets and explicit partial fallback |
| `crates/campfire/src/controllers/agents/mcp_metadata.json` | Vendored descriptions, schemas, versions and throttle metadata from our Rails oracle; no build-time external source dependency |
| `crates/campfire/src/controllers.rs` | Handler dispatch and agent test modules |
| `crates/campfire/src/app.rs` | Explicit unparsed MCP routes, preserving the kit body bound and ordinary dispatch scope |
| `crates/campfire/Cargo.toml`, `Cargo.lock` | Use the existing workspace `url` dependency for Origin parsing |
| `crates/campfire/src/controllers/presenters.rs` | Named `agent_message_payload` seam for MessagePayloadHelper; returns an error until filled |
| `crates/campfire/src/controllers/presenters/test_support.rs` | Optional injected frozen clock; existing TestApp boot behavior retained |
| `crates/campfire/src/controllers/agent_http_tests.rs` | Eight mandatory seeded test functions, 33 asserted Rails REST vectors, DB persistence assertions and replay checks |
| `crates/campfire/src/controllers/agent_mcp_tests.rs` | Six mandatory seeded test functions, 53 asserted MCP vectors, coarse/shared throttles and upload bound |
| `crates/db/src/models/agent_step.rs` | Small input-error seams after authorization and before writes; existing typed APIs delegate with empty errors |
| `crates/db/src/slash_commands.rs` | Export existing error-sentence formatter; no formatting logic change |
| `crates/campfire/src/concerns/session_keys.rs` | Existing contract-output test creates its default output directory; clean checkout with an external Cargo target exposed this dependency |
| `reference-tools/agents/http_contract.rb`, `mcp_contract.rb` | Actual requests through pinned Rails in production over the default seed; clock frozen, fake auth header assembled at runtime |
| `reference-tools/agents/check-http-reference.py` | Hash 21 controller/concern/service/model/helper inputs against git pin, image and checkout |
| `reference-tools/agents/extract-mcp-metadata.py` | Reproducible vendored metadata extraction |
| `reference-tools/agents/check-http-mutations.py` | Deliberately break three guards, require assertion failures and restore files in finally blocks |
| `reference-tools/agents/summarize-http-tests.py` | Sum raw cargo summaries and distinguish real seed skips from the deliberate missing-seed unit notice |
| `vectors/agent_http.json`, `agent_mcp.json` | 33 REST and 84 MCP request/response pairs from Rails; all error codes and one empty-arguments pair per advertised tool |
| `plans/ws11api-wave4-report.md` | This report, also copied to the requested delegation path |

## Design and behavior

Authentication reuses WS11's persisted credential digest/revocation/expiry/active-agent policy. Grant checks stay in the domain and read current rows. Missing grant is 403 while slash nonmembership is 404; step authorization runs before raw input errors can be returned. Service callbacks, positions, broadcasts, rollback, and parent ownership remain in WS11's transaction-backed models. No second domain implementation was added.

Each REST action uses its Rails controller/action bucket; MCP tools use those same keys. The coarse MCP bucket is 600/min and runs before Origin validation. Each acknowledgment ID consumes its own tool charge, and batches above 100 are rejected before those per-ID charges. Credential digests, not plaintext, key the in-process kit counter. Minute boundaries, credential isolation, action isolation, and exact REST overflow Retry-After are tested. MCP per-tool overflow shape and coarse overflow ordering are tested, but not every tool's overflow path.

The MCP route reads unparsed bodies to reproduce Rails' two parse-error paths. Invalid `application/json` fails before authentication/no-store; invalid text/plain fails after callbacks. The route retains the kit/config upload maximum and rejects max+1 with 413. Modern-only header requirements, version disagreement, unsupported typed versions, malformed Base64 and invalid UTF-8 match the committed vectors. Origin uses host comparison, including case and port behavior; encoded hosts are denied, and Rails' accepted network-path Origin `//campfire.test` is reproduced.

Profiles expose Rails' compact owner and non-null fields, millisecond UTC timestamps, status/notes and live working presence. Only the three mutable fields are assigned; supplied provider and cap fields remain unchanged. Rails AR string assignment (`false` becomes `f`) and Ruby to_s/inspect behavior for presence hashes are distinguished. Invalid duration is validated before integer casting and never persists; field errors are merged with model errors after parent policy, preserving the tested response order. Exhaustive malformed/odd input coercions are not claimed.

No parity masks or allowlists were changed. Tests compare status, decoded JSON, selected headers (including absent values), and exact MCP content text. Byte-for-byte regeneration of the **vector files** is verified; this is not a claim that every HTTP JSON serialization or all 38 tool operations have byte parity.

## Cross-workstream seams and owners

1. **WS8b-m / WS8b-m2:** `Presenter::agent_message_payload(&self, message: &Message) -> campfire_db::Result<serde_json::Value>`. Fill with request-specific MessagePayloadHelper JSON. It currently errors; message-bearing event polling is partial. It intentionally does not reuse cached Jbuilder JSON because reply/thread permissions are per user.
2. **WS11:** `agent_step::create_with_input_errors(tx: &mut Tx<'_>, agent_id: i64, attributes: NewAgentStep, input_errors: Errors) -> Result<ServiceResult>` and `agent_step::update_with_input_errors(tx: &mut Tx<'_>, agent_id: i64, id: i64, changes: AgentStepChanges, input_errors: Errors) -> Result<ServiceResult>`. `AgentStep::update_with_input_errors(&mut self, tx: &Tx<'_>, changes: AgentStepChanges, input_errors: Errors) -> Result<()>` is the model seam. Existing `create`, `update` and model `update` preserve signatures and delegate with empty errors. Only these small flagged additions touch WS11 domain files.
3. **WS11 / WS15g:** polling currently supplies empty `RepositoryAccess`, as the existing WS11 delivery adapter does. Live private-PR access decisions still need the resolver; restricted payloads are omitted/denied rather than granted. Domain access correctness is not marked finished by this report.
4. **WS11 and relevant domain workers:** approvals, contexts, DMs, messaging, streaming, work/handoff/result, Fizzy and GitHub API adapters still need shared service calls or minimal seams. The missing-service rule does not excuse HTTP work: WS11-api still owns those auth/grant/validation/error paths.
5. **WS4 test support:** the existing session-key output test failed in a clean checkout using an external Cargo target because `rust/target/` did not exist. Added only `create_dir_all(out.parent().unwrap())` before writing. The fresh suite was rerun successfully; no failure was dismissed as inherited.

## Tests ported and deferred

These are selected Rails scenarios reproduced through new vectors and real-router Rust tests, **not entire Rails file ports**:

| Rails source | Covered here | Still deferred / owner |
| --- | --- | --- |
| `test/controllers/agents/events_controller_test.rb` | Bare/enveloped polling, next cursor after dropped event, acknowledgments/replay/missing ID | Message-bearing/private-PR event matrix: WS11-api with presenter and access seams above |
| `test/controllers/agents/steps_controller_test.rb` | Missing parent/step, create payload, raw duration validation, no persistence, authorization ordering | Full message/thread/ownership/update/callback HTTP matrix: WS11-api (domain remains WS11) |
| `test/controllers/agents/slash_commands_controller_test.rb` | Nonmember 404, grant 403, invalid registration, registration/replay, missing delete | Remaining happy deletion and complete coercion/ownership HTTP matrix: WS11-api |
| `test/controllers/agents_controller_test.rb` | Profile get/update, invalid status, clear presence, ignored immutable fields, boolean note | All session, cap, status-note boundary, usage-stamp HTTP cases: WS11-api |
| `test/controllers/concerns/agent_authentication_test.rb` | Unknown/revoked/expired credential, agent-on-human denial | All session/malformed auth/bot-key/suspension HTTP cases: WS11-api; domain credential matrix remains WS11 |
| `test/controllers/agents/api_throttle_test.rb` | Poll120+1, minute boundary, credential/action isolation, MCP shared/coarse bucket ordering | All other REST/tool rates, human session exclusion and exact MCP overflow headers for every tool: WS11-api |
| `test/controllers/agents/mcp_controller_test.rb` | Four versions, discovery/list/ping/notifications, headers/Origin, framing, every error code, seven tools' selected cases | Full 71-test equivalent, every tool happy/error contract, grants, budgets and race paths: WS11-api |

Explicit remaining controller/test ownership:

- **WS11-api:** `approvals_controller_test`, `budgets_endpoints_test`, `contexts_controller_test`, `dms_controller_test`, `messages_controller_test`, `pins_controller_test`, `polls_controller_test`, `posts_controller_test`, `streaming_messages_controller_test`, `work_controller_test`, `work_handoff_test`, `mcp_fizzy_test`, `mcp_handoff_test`, `mcp_slash_polls_test`, `mcp_streaming_test`; under agents/fizzy, `boards_controller_test`, `cards_controller_test`, `card_actions_controller_test`; under agents/github, `pull_request_actions_controller_test`. HTTP grant/revocation/budget behavior in top-level `agent_capability_test`, `agent_revocation_endpoints_test`, `agent_owner_deactivation_test` remains WS11-api with domain coordination. `messages/by_bots_controller_test` and `messages/boosts/by_bots_controller_test` still need the complete bot API audit, including reply-token create-only fail-first evidence. This slice does not change inherited bot controllers or claim those full file ports.
- **WS11:** agent model/services/jobs and delivery behavior. Under agents, `approval_delivery_test`, `drive_attachments_delivery_test`, `github_action_delivery_test`, `slash_command_delivery_test`, `work_delivery_test`; agents/fizzy `action_delivery_test`. This branch runs the WS11 tests already present but does not claim new delivery, SSRF or budget-overflow fail-first work.
- **WS11-ui:** `directory_controller_test`, admin/directory/approval/event HTML pages, `agent_approvals_controller_test`, `agent_approvals_controller_fizzy_test`, accounts/bots page/credential/grant/secret management tests and agent system-page tests. Domain transitions remain WS11, integrations WS15g/WS15e.
- **WS12 / WS15g / WS15e:** board/work domain, GitHub action domain/live access, Fizzy domain respectively; the associated agent-facing HTTP adapters remain WS11-api.

### Fail-first evidence

Saved historical logs show failures before the relevant implementation/fix. The commands against historical broken source are not represented as current verification commands. Raw summaries:

- `http-before.log`: Missing REST dispatch; grant/revocation/overflow scenarios failed; existing human denial already passed.

```text
test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 343 filtered out; finished in 0.89s
```

- `mcp-before.log`: Transport absent; protocol/header/Origin/shared throttle scenarios failed.

```text
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 349 filtered out; finished in 1.06s
```

- `mcp-bound-before.log`: Encoded Origin and upload bound regressions failed.

```text
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 349 filtered out; finished in 1.15s
```

- `profile-before.log`: GET me returned unported 501.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 355 filtered out; finished in 0.73s
```

- `presence-before.log`: Hash presence used JSON rather than Ruby inspect.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 355 filtered out; finished in 0.66s
```

- `duration-before.log`: Fractional duration lost raw validation error.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 356 filtered out; finished in 0.67s
```

- `origin-relative-before.log`: Rails accepted network-path Origin while Rust denied it.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 1.03s
```

A current reproducible mutation check additionally disables the inherited human-endpoint guard, disables modern MCP header checks, and removes duration input errors. All three fail by assertion (not compile failure or absent fixture). The script restores every file even on failure. Original focused tests were rerun after restoration. The reply-token create-only, SSRF and broader budget fail-first requirements are still partial as assigned above.

## Re-run verification commands and raw output

All Cargo commands use Rust 1.98.1, `-j 4`, no release build, and private targets/TMPDIR. Below commands were run in this session. No seeded API test returns early when the seed is missing; setup uses expect, and CI=1 is set.

From the assigned worktree's `rust/`:

```bash
python3 reference-tools/agents/check-http-reference.py
python3 reference-tools/agents/extract-mcp-metadata.py
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 > ../.scratch/metadata-locked.json
```

```text
WS11-api reference sources: 21 pinned files matched; 0 image or checkout mismatches (d7c7de92)
WS11-api MCP metadata: 38 Rails tools; 4 protocol versions
```

Locked metadata exited 0. The oracle image is `ws11api-reference:d7c7de92` (tagged from the available pinned reference); the hash check verifies the 21 inputs used here, rather than assuming a tag establishes provenance.

A separate local clone under `.scratch/clean` has no source edits. It was cloned without hardlinks, fast-forwarded to the verified implementation head, and given separate `.scratch/clean-target` and `.scratch/clean-tmp` directories. Dependency build output was reflink-copied as a cache; the new source paths rebuilt the workspace. Both seed databases were freshly rebuilt from the pinned reference, not borrowed from the original worktree.

From the assigned worktree root:

```bash
PARITY_NAMESPACE=ws11api-clean PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/clean/rust/parity/bin/seed build default first_run
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

From `.scratch/clean/rust/`:

```bash
PARITY_NAMESPACE=ws11api-clean PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/http_contract.rb > /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-http.json
PARITY_NAMESPACE=ws11api-clean PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference runner --seed default reference-tools/agents/mcp_contract.rb > /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-mcp.json 2> /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-mcp-oracle.log
```

Both exited 0. Compare outputs from the original `rust/` directory:

```bash
python3 - <<'CHECK'
from pathlib import Path
import json
import re
for kind in ['http', 'mcp']:
    actual = Path(f'../.scratch/clean-{kind}.json').read_bytes()
    expected = Path(f'vectors/agent_{kind}.json').read_bytes()
    assert actual == expected, kind
    print(f"WS11-api clean {kind.upper()} oracle: {len(json.loads(actual)['cases'])} request/response pairs; byte-identical committed vectors")
vectors = json.loads(Path('vectors/agent_mcp.json').read_text())
names = set(re.findall(r'"([^"]+)"', Path('crates/campfire/src/controllers/agent_mcp_tests.rs').read_text()))
covered = sum(c['name'] in names for c in vectors['cases'])
print(f"WS11-api MCP coverage: {covered} asserted vectors; {len(vectors['cases']) - covered} explicitly deferred vectors")
CHECK
```

```text
WS11-api clean HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api clean MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api MCP coverage: 53 asserted vectors; 31 explicitly deferred vectors
```

All 33 REST cases are asserted. The 31 unasserted MCP pairs are explicitly deferred operation contracts, not silently skipped Rust test functions. Metadata alone is not operation parity.

Mutation and restoration verification, from original `rust/`:

```bash
python3 reference-tools/agents/check-http-mutations.py > ../.scratch/mutations.log 2>&1
TMPDIR="$PWD/../.scratch/tmp" CI=1 mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire agent_ -- --nocapture > ../.scratch/agent-after.log 2>&1
```

```text
WS11-api mutation human_token: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.66s
WS11-api mutation mirror_headers: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.83s
WS11-api mutation duration_input: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 357 filtered out; finished in 0.88s
WS11-api mutations: 3 broken guards rejected; 3 source files restored
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 330 filtered out; finished in 8.13s
```

This filter ran 28 tests, including 15 added here and inherited WS11 checks. It did not run all 358 app tests; the full fresh run below did.

Fresh seeded workspace and clippy, from `.scratch/clean/rust/`:

```bash
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-tmp CI=1 mise exec rust@1.98.1 -- cargo test --locked -j 4 --workspace --exclude html5ever -- --nocapture > /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-tests.log 2>&1
CARGO_TARGET_DIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-target TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-tmp mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/clean-clippy.log 2>&1
```

Both exited 0. All raw Cargo result summaries, in output order:

```text
test result: ok. 356 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 23.66s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.63s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
test result: ok. 447 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 56.18s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.74s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.46s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.32s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.59s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s
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

From original `rust/`:

```bash
python3 reference-tools/agents/summarize-http-tests.py ../.scratch/clean-tests.log
```

```text
WS11-api cargo totals: 1418 passed; 0 failed; 9 ignored; 46 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

The app summary is 356 passed and 2 ignored. WS11's `manages_bots` runs and passes. The nine pre-existing explicit ignores are two reference-recording tests (app channels/cable), the push-latency measurement, three external Ruby DB comparison/export tests, the mail Rails-export test, and two kit documentation examples. No ignores were added. The only `skipping locally` line is emitted by the unit test that deliberately checks missing-seed behavior; all real seeded tests ran. Vendored html5ever is excluded per rust/AGENTS.md.

Raw clippy summary:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.37s
```

### Direct Rails controller suites: blocked, zero tests executed

Re-run from original `rust/`:

```bash
PARITY_NAMESPACE=ws11api PARITY_IMAGE=ws11api-reference:d7c7de92 parity/bin/reference exec --seed default -e RAILS_ENV=test -e PARALLEL_WORKERS=1 -- bin/rails test test/controllers/agents/events_controller_test.rb test/controllers/agents/steps_controller_test.rb test/controllers/agents/slash_commands_controller_test.rb test/controllers/agents/mcp_controller_test.rb > ../.scratch/rails-tests.log 2>&1
```

Exit 1. The production parity image includes fixture data, but not these controller test sources. Raw first error:

```text
/usr/local/lib/ruby/3.4.0/bundled_gems.rb:82:in 'Kernel.require': cannot load such file -- /rails/test/controllers/agents/events_controller_test.rb (LoadError)
```

No Rails test count is claimed for this attempt. The independent production Rails requests above did execute; they are the source of the committed golden vectors. Complete direct Rails suite execution remains verification work for WS11-api.

### First fresh-checkout failure and correction

The initial external-target clean run reached `concerns::session_keys::tests::writes_the_keys_as_rails_does` and failed writing its output file because the default target parent did not exist. Its raw app summary was:

```text
test result: FAILED. 355 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out; finished in 22.79s
```

Commit `9b7dc347` makes that existing test create its output parent. The complete fresh workspace test/clippy results above are after this fix. Historical failed log retained in `.scratch/clean-tests-first.log`.

## Precisely what remains and resume point

Resume WS11-api from this pushed branch, retaining the verified slice and vectors. The next coherent slice should wire approvals REST to WS11's existing approval service and add complete authorization/grant/not-found/validation/throttle/budget HTTP and MCP request/response cases. Then continue contexts/DMs, agent messages, board posts/work/result/handoff, pins/polls/streaming, and the integration adapters. Missing domain services need small named seams while their HTTP denial/error paths are implemented now.

The 31 remaining MCP operations are:

```text
list_rooms, read_messages, post_message, react,
list_board_posts, create_board_post, update_board_post,
set_result, list_work, update_work, request_approval, get_approval, get_context, open_dm,
list_fizzy_boards, get_fizzy_board, search_fizzy_cards, get_fizzy_card,
create_fizzy_card, comment_on_fizzy_card, move_fizzy_card, close_fizzy_card, reopen_fizzy_card,
pin_message, unpin_message, create_poll, get_poll, handoff_work,
start_stream, append_stream, finalize_stream
```

For each, replace the current Internal error fallback with the shared service (or an exact small domain seam), port its validation/grant/budget/throttle response, and assert happy and failure vectors. GitHub PR action REST endpoints also remain even though no dedicated PR tool appears in this 38-tool list.

Fill the message payload and live repository-access seams, add message-bearing event vectors, audit every by_bots/boost response including root-only listing/board restrictions/create-only reply tokens, and complete raw-input/session/CSRF response coverage across the new endpoints. All final 401/403/404/422/429 bodies and Retry-After variants still need an endpoint-wide audit. Byte-level HTTP output parity beyond the selected text payloads is not proven here.

No product decision is being reopened. Open coordination questions are limited to the presenter helper landing point (WS8b-m or WS8b-m2), the live repository-access resolver API (WS11/WS15g), and exact service signatures for still-missing HTTP operations. These are remaining implementation seams, not requests for user approval.
