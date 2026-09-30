# WS11 report — PARTIAL, coherent bot authorization slice

Branch `rust/ws11-agents`, worktree `.claude/worktrees/rust-ws11`, based on `bb6c5d78`. Reference pin `d7c7de92`. Commits pushed as slices: `bf9da994` (reply/key/scrub), `f8cf0aff` (credential/grant read policy), followed by the verification/report commit. No PR, merge, deployment, Rails-source change or parity-mask change.

**WS11 acceptance is not complete.** The existing Rust suite is green, but most agent features and their Rails tests are still unported. This branch is not a cutover claim.

## What changed, by file

- `rust/crates/campfire/src/controllers/messages/by_bots.rs`: create-only reply tokens, 422 on boards, root-only total count, system-note edit/delete guard, and read/post capabilities after membership checks. Existing root timeline pagination is reused.
- `rust/crates/campfire/src/controllers/messages/boosts/by_bots.rs`: forbid reply tokens and enforce react capability. Preserve Rails' inherited message/boost lookup order: an unknown boost destroy is 404 before the token callback; unknown message update/delete remains 403 because the fork re-registers its message callback after the reply guard.
- `rust/crates/db/src/models/message.rs`: additive `count_roots_in_room`; no callback-chain rewrite.
- `rust/crates/db/src/models/agent_access.rs`, `models.rs`: domain module for credential auth and capability reads. Auth checks persisted digest, revoked/expired state, agent suspension and current user activity. Conditional writes throttle credential last-use/IP/updated_at and agent last-seen to one minute. Every grant check reads current rows; a revoked grant still disables legacy fallback. All seven capability names are recognized, with only read/post/react in the zero-grant legacy fallback. This is a read-side seam, **not** the eight models' CRUD implementation.
- `rust/crates/campfire/src/concerns.rs`: valid credentials authenticate, then default endpoint policy rejects their tokens with 403; unknown/revoked/expired credentials are 401. Capability denials have Rails JSON, separate from the controller's membership 404. Bearer auth and bot keys retain the existing CSRF/auth precedence.
- `rust/crates/campfire/src/controllers/accounts/bots/keys.rs`: admin/sudo gate, atomic rotation plus audit, no-store/no-cache response, and show-once key rendering. Audit targets the Agent when present, otherwise the User, and never includes the key.
- `rust/crates/views/src/accounts.rs`, `templates/accounts/bots/keys/show.html`: copy of the Rails reveal-once key view using established helpers. Functional HTTP behavior is tested; exhaustive byte/pixel comparison of this new page is **not** claimed.
- `rust/crates/campfire/src/jobs/periodic.rs`, `jobs/tests.rs`: register the existing one-time scrub in the actual scheduler, remove its dead-code exemptions, and exercise it through the actual job loops without manually registering a test-only task. The WS8 task-name assertion remains scoped to WS8 tasks; no allowlist/mask is touched.
- `rust/parity/seeds/default.rb`: NULL every stale plaintext token; deterministic keys live in labels, backed by their SHA-256 digests. Bender keeps its fixture key. Newly seeded legacy bots receive deterministic digests rather than retired plaintext.
- `rust/crates/campfire/src/controllers/messages/tests.rs`, `controllers/presenters/accounts/tests.rs`: seven WS11 HTTP regressions plus the updated `manages_bots` assertions. Its previous fixture label was BOT_KEY, making the old post-reset assertion necessarily fail; the corrected seed labels now contain real test keys, and the test asserts that stored pages conceal them. Reset expectations match Rails' 200 reveal-once response instead of the upstream redirect.
- `rust/reference-tools/agents/bot_contract.rb`, `rust/vectors/agents_bot_contract.json`: actual pinned Rails request outcomes for reply-token/lookup ordering, system notes, root pages, credential digest/auth, and grants. No literal auth header is stored; the test header is assembled at runtime.
- `rust/reference-tools/agents/{check-reference.py,check-seeds.py,verify-mutations.py,record-cable.sh,summarize-tests.py}`: source/pin and seed checks, compiled discrimination probes, WS11-scoped recording, and raw Cargo-summary counting.
- `rust/crates/campfire/src/channels/tests/golden/reference.json`: fresh Rails recording, including new session cookies/rows. Both cable goldens were regenerated; `rust/crates/cable/tests/golden/reference.json` was byte-identical and has no tracked diff. The application golden's bot has a NULL plaintext token. No sorting, masks or tolerances were added.
- `rust/plans/ws11-report.md`: tracked mirror of the requested external report.

## Design and validation boundaries

Domain policy knows nothing about HTML. Controllers authorize and call it, then render. The key reset uses the existing WS2 digest generator and constant-time bot comparison; it does not duplicate security primitives. `bot_reply` signing/expiry/purpose are the existing Rails-derived WS1 verifier, exercised by the full suite and the new HTTP scope regressions.

The read-side module creates no Agent, Credential or Grant rows in production. The activity writes mirror Rails `update_all` (intentionally no validators/callbacks); the scrub mirrors the existing conditional heal (no timestamp touch). Audit creation uses the existing `AuditLog::record` validation/filtering. The key reset reuses `User::reset_bot_key`; full Rails User-save profile/settings/GitHub uniqueness validations and callbacks are not yet complete in that inherited core implementation and remain a **WS11 follow-up with WS2/core-model seams** before this endpoint is fully validated for unusual pre-existing rows. This report does not hide that limitation. Agent/credential/grant CRUD validators and lifecycle callbacks are deferred below because those writes are not implemented.

No new durable webhook/event jobs are added in this slice. Existing WS8 enqueue/broadcast logic is retained. Agent-backed bot create still calls the inherited message path: its budget, posting ledger and full idempotency/Markdown contract must be fixed next.

## Current commands and raw evidence

All commands below were executed in this worktree on the final implementation. Cargo uses Rust 1.98.1, four jobs, `rust/target`, on-disk `.scratch`, and only WS11 ports. Commands marked `cd rust` use that directory; others use the worktree root.

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/seed build
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/check-seeds.py
```

```text
seed: building crowd
seed: crowd -> parity/.seed/crowd (6.6M)
seed: building custom_styles
seed: custom_styles -> parity/.seed/custom_styles (6.8M)
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building imports
seed: imports -> parity/.seed/imports (6.1M)
seed: building live_rooms
seed: live_rooms -> parity/.seed/live_rooms (6.1M)
seed: building restricted
seed: restricted -> parity/.seed/restricted (6.1M)
seed: building smartfire
seed: smartfire -> parity/.seed/smartfire (6.1M)
seed: building unread
seed: unread -> parity/.seed/unread (6.1M)
WS11 reference sources: 11 matched; 0 mismatched (d7c7de92)
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --seed default rust/reference-tools/agents/bot_contract.rb > .scratch/bot-contract-verify.json 2> .scratch/bot-contract-verify.log
cmp .scratch/bot-contract-verify.json rust/vectors/agents_bot_contract.json
```

Both exit 0; cmp has no output. The JSON is the raw oracle: 5 reply endpoint outcomes, 2 missing-message outcomes, 2 system-note outcomes, 3 credential auth outcomes, 7 grant outcomes, root-only page properties and the credential SHA-256 digest. Secrets are test-only; no production records were read.

```sh
python3 rust/reference-tools/agents/verify-mutations.py > .scratch/mutations-final.log 2>&1
```

```text
reply-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.43s
reply-missing-message: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.34s
system-notes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.37s
root-count: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.38s
sudo-rotation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.87s
scrub-registration: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 5.19s
credential-auth: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.35s
capability-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.52s
credential-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.52s
credential-expiry: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.36s
credential-use-throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 311 filtered out; finished in 0.35s
WS11 discrimination: 11 compiled regressions detected; sources restored
```

These are assertion failures from compiled code, not compile errors. Initial tests also failed before the fixes: reply GET was 200 instead of 403, a bot could edit its own system note, counts included a thread reply, the scrub never ran, a valid agent token was 401 instead of 403, and a revoked-only grant still admitted reads. The key test initially redirected to bots without requiring sudo. The unknown-message expectation was first checked against Rails and corrected to 403 before acceptance; that mistaken 404 expectation is not claimed as failure-first evidence. The committed mutation now proves the correct Rails expectation discriminates. No deliberate mutation remains.

```sh
bash rust/reference-tools/agents/record-cable.sh > .scratch/cable-record-final.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 310 filtered out; finished in 128.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 9.26s
WS11 cable recording: both Rails goldens regenerated (d7c7de92)
```

Run from `rust/`:

```sh
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire --bin campfire ws11_ -- --nocapture > ../.scratch/ws11-tests-final.log 2>&1
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 mise exec rust@1.98.1 -- cargo test --locked -j 4 --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../.scratch/workspace-final.log 2>&1
```

Selected new HTTP tests:

```text
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 305 filtered out; finished in 0.89s
```

Full workspace, including the seeded binary and doctests (raw summary lines, in emitted order):

```text
test result: ok. 310 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 30.60s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.05s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 71.75s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.86s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.24s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.27s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.40s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.11s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.64s
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

```sh
python3 rust/reference-tools/agents/summarize-tests.py .scratch/workspace-final.log
```

```text
WS11 workspace totals: 1321 passed; 0 failed; 9 ignored; 45 result summaries
WS11 missing-seed skips: 0
```

**Ran versus skipped:** the binary actually ran 310 tests, including `manages_bots` and all seven new WS11 cases; its 2 ignored cases are recorder and latency measurement. Workspace ran 1,321 tests and reported 9 explicit ignores, with zero missing-seed/reference skip notices. The two cable recorders were executed successfully above. Remaining explicit ignores: latency measurement, three DB fixture/scenario/export jobs, one mail export, and two illustrative kit doctests. These were not silently counted as run. Vendored html5ever is excluded as `rust/AGENTS.md` prescribes; no test or parity allowlist was loosened. This is Rust-suite verification, not a claim that all Rails agent tests have been ported.

Run from `rust/`:

```sh
TMPDIR="$PWD/../.scratch" CARGO_TARGET_DIR="$PWD/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --all-targets -- -D warnings > ../.scratch/clippy-final.log 2>&1
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.58s
```

Clippy exits 0. Locked metadata exits 0 with no output and no Cargo.lock change. `bash -n rust/reference-tools/agents/record-cable.sh` and `git diff --check` exit 0 with no output.

## Rails test file accounting

Partial means selected contracts only, not the entire file. Every other listed file is deferred to its named owner. There is no `test/services/agents/` test tree in this checkout; the relevant service contracts are exercised in model/controller/job files. Fixtures and test helpers are not counted as test files. The following inventory includes agent integration/system cases as well as the requested model/controller/job cases.

| Rails test file | Coverage / remaining owner |
|---|---|
| `test/channels/agents_channel_test.rb` | Inherited WS7 authorization/replay coverage retained; directory/status broadcast behavior still WS11. |
| `test/controllers/accounts/bots/credentials_controller_test.rb` | Deferred: WS11. |
| `test/controllers/accounts/bots/github_connections_controller_test.rb` | Deferred: WS11. |
| `test/controllers/accounts/bots/grants_controller_test.rb` | Deferred: WS11. |
| `test/controllers/accounts/bots/keys_controller_test.rb` | Partial: sudo gate, reveal-once reset, digest invalidation, audit/no-store; dedicated member-reset case and exhaustive HTML comparison still WS11. |
| `test/controllers/accounts/bots/webhook_secrets_controller_test.rb` | Deferred: WS11. |
| `test/controllers/accounts/bots_budgets_test.rb` | Deferred: WS11. |
| `test/controllers/accounts/bots_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agent_approvals_controller_fizzy_test.rb` | Deferred: WS11. |
| `test/controllers/agent_approvals_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agent_capability_test.rb` | Partial bot-key membership 404, grant JSON 403, immediate revocation, room/workspace scopes; remaining API/capabilities WS11. |
| `test/controllers/agent_owner_deactivation_test.rb` | Deferred: WS11. |
| `test/controllers/agent_revocation_endpoints_test.rb` | Deferred: WS11. |
| `test/controllers/agents/api_throttle_test.rb` | Deferred: WS11. |
| `test/controllers/agents/approval_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/approvals_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/budgets_endpoints_test.rb` | Deferred: WS11. |
| `test/controllers/agents/contexts_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/directory_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/dms_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/drive_attachments_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/events_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/fizzy/action_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/fizzy/boards_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/fizzy/card_actions_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/fizzy/cards_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/github/pull_request_actions_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/github_action_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/mcp_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/mcp_fizzy_test.rb` | Deferred: WS11. |
| `test/controllers/agents/mcp_handoff_test.rb` | Deferred: WS11. |
| `test/controllers/agents/mcp_slash_polls_test.rb` | Deferred: WS11. |
| `test/controllers/agents/mcp_streaming_test.rb` | Deferred: WS11. |
| `test/controllers/agents/messages_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/pins_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/polls_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/posts_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/slash_command_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/slash_commands_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/steps_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/streaming_messages_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/work_controller_test.rb` | Deferred: WS11. |
| `test/controllers/agents/work_delivery_test.rb` | Deferred: WS11. |
| `test/controllers/agents/work_handoff_test.rb` | Deferred: WS11. |
| `test/controllers/agents_controller_test.rb` | Deferred: WS11. |
| `test/controllers/audit_log/agents_audit_test.rb` | Deferred: WS11. |
| `test/controllers/concerns/agent_authentication_test.rb` | Partial: valid token denied on human endpoints, revoked/expired credential and use throttle; agent me API and remaining identity cases still WS11. |
| `test/controllers/messages/boosts/by_bots_controller_test.rb` | Partial: reply-token denial and react gate; content resolution/422 error JSON/reaction rendering still WS11 with WS8b-r seam. |
| `test/controllers/messages/by_bots_controller_test.rb` | Partial: reply denials, system-note guard and root count; agent Posting/budgets/idempotency/legacy fanout and explicit Markdown clearing still WS11. |
| `test/integration/agent_boards_test.rb` | Deferred: WS11. |
| `test/jobs/agent/delivery_concurrency_test.rb` | Deferred: WS11. |
| `test/jobs/agent/delivery_job_test.rb` | Deferred: WS11. |
| `test/jobs/agent/event_webhook_job_test.rb` | Deferred: WS11. |
| `test/jobs/fizzy/perform_agent_action_job_test.rb` | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/jobs/github/perform_agent_action_job_test.rb` | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/lib/restricted_http/private_network_guard_test.rb` | Deferred: WS11. |
| `test/models/agent/delivery_recovery_test.rb` | Deferred: WS11. |
| `test/models/agent_approval_test.rb` | Deferred: WS11. |
| `test/models/agent_backfill_test.rb` | Deferred: WS11. |
| `test/models/agent_budgets_test.rb` | Deferred: WS11. |
| `test/models/agent_credential_test.rb` | Partial read/auth/use contract only; create validation, generated secret, revoke admin API and remaining cases WS11. |
| `test/models/agent_event_test.rb` | Deferred: WS11. |
| `test/models/agent_grant_test.rb` | Partial read checks only; grant validation, creation, revocation lifecycle and callback cases WS11. |
| `test/models/agent_kill_switch_test.rb` | Deferred: WS11. |
| `test/models/agent_revocation_test.rb` | Deferred: WS11. |
| `test/models/agent_slash_command_test.rb` | Deferred: WS11. |
| `test/models/agent_step_test.rb` | Deferred: WS11. |
| `test/models/agent_test.rb` | Partial active/grant/last-seen read policy only; creation/update/status/budget/presence/webhook and directory tests WS11. |
| `test/models/agent_working_presence_test.rb` | Deferred: WS11. |
| `test/models/agents/work_payload_test.rb` | Deferred: WS11 with WS12 work-thread seam. |
| `test/models/channel_thread_agent_assignment_test.rb` | Deferred: WS11 with WS12 work-thread seam. |
| `test/models/fizzy/agent_card_action_test.rb` | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/github/agent_pull_request_action_test.rb` | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/message/bot_webhook_fanout_test.rb` | Deferred: WS11. |
| `test/models/message_streaming_test.rb` | Deferred: WS11. |
| `test/models/user/bot_test.rb` | Partial: inherited digest/reset checks plus HTTP reply scope; remaining lifecycle/update cases WS11. |
| `test/models/webhook_agent_key_test.rb` | Deferred: WS11. |
| `test/models/webhook_test.rb` | Deferred: WS11. |
| `test/services/bots/clear_plaintext_tokens_test.rb` | Partial: inherited healing/idempotence plus actual scheduler registration; direct heal/reset-race case still WS11. |
| `test/system/agent_approvals_test.rb` | Deferred: WS11. |
| `test/system/agent_streaming_test.rb` | Deferred: WS11. |
| `test/system/agent_work_assignment_test.rb` | Deferred: WS11. |
| `test/system/agents_test.rb` | Deferred: WS11. |

## Cross-workstream touches and open questions

- WS2: additive message root-count reader and a domain auth policy module; full Agent/Grant/Credential CRUD is not disguised as complete. No schema changes.
- WS3: register its existing scrub. No durable queue implementation changes.
- WS7/WS19: regenerate both original cable goldens using WS11 ports/names; keep replay policy unchanged. All parity seeds are present.
- WS6: new key template calls existing view helpers. Broader bot/agent HTML and byte/pixel verification remain WS11.
- WS9: consume the existing `require_sudo_mode` cookie-session gate. Main still lacks the sudo controllers at this base; the real confirmation/replay UI is WS9's seam. Tests install the same Rails-compatible encrypted verified session state, not a mocked gate.
- WS8b-m/WS8b-r: reuse message writes and broadcasts. Still need a shared explicit `markdown_source: nil` assignment path for bot attachment-only updates and reaction resolution/422/broadcast semantics. Do not claim those inherited paths match the fork yet.
- WS12: boards/work-thread agent API and deleted-work/finalization integrations remain pending seams.
- WS15g/WS15e: no client/action code is duplicated. Agent-facing callers and ledger/webhook results remain WS11.
- Open contract detail: the pinned Rails bot-key endpoints do not opt into `allow_agent_access`, so a valid Bearer-only call to them is 403. This branch follows the oracle/source even though some inventory prose describes agent-token callers. Any change to that behavior belongs after parity or to a lead-approved Rails fix.

## Precise restart point and remaining work

1. **Finish brief slice 1 first (WS11):** route Agent-backed bot create through `Agents::Posting`, daily budgets/notices/429 and Retry-After, client-message idempotency, explicit Markdown-source clearing on update/attachment paths, and boosts' content resolution/422 JSON/full reaction broadcasts. Complete core User-save validator integration for bot writes and the dedicated reset role/error cases. Do not begin cutover based on this report.
2. **Webhooks (WS11):** full payload matrix, signed reply paths, timestamp/HMAC, lazily generated encrypted secrets/both-direction vectors, pinned resolved IP/private-range/rebinding guard, 7-second timeouts, legacy transient retries, agent five-attempt backoff, threaded synchronous replies and `BotWebhookFanout`/hop guard. The inherited internal-host acceptance test remains; SSRF enforcement is **not implemented** by this slice.
3. **Eight-table domain (WS11):** Agent, Credential and Grant creation/validation/lifecycle APIs; ledger/poll cursor/acks; approvals and admin-only external decisions; steps; slash registration/invocation; DMs/context; budgets; per-credential minute counters; presence TTL/status broadcasts; suspension/kill switch; streaming/trailing jobs/recovery; hard-user removal/finalization; thread-work and reference finalization hooks with WS12/WS8.
4. **REST API (WS11):** every agent route and exact JSON 401/403/404/422/429/Retry-After, board/work callers into WS12, GitHub/Fizzy thin callers into WS15g/e. Store Current.agent/credential identity when building that layer; this minimal auth seam currently returns the authenticated user.
5. **MCP (WS11):** stateless Streamable HTTP, four versions, Origin/mirrored headers/base64/error codes, 38 tools sharing services with REST, rate limits and every tool/error request-response vector. None of the MCP endpoint/tool implementation is in this slice.
6. **Admin/agent HTML (WS11):** bot create/Agent creation/show-once key/sudo/audit, credentials/grants/webhook secrets/connections, directory/approvals/events, kill switch, full byte/pixel comparison (including the new key template).
7. **Remaining security/acceptance tests (WS11):** SSRF/private/DNS rebinding, rate/budget overflow, MCP header/version errors, full lifecycle concurrency/durable enqueue rollback and exhaustive Rails file parity. The exact deferred file inventory above remains the handoff.
