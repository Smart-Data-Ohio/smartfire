# WS11 report — PARTIAL: polling and shared agent domain services

Branch `rust/ws11-agents`; worktree `.claude/worktrees/rust-ws11`; reference Rails `d7c7de92`. Implementation tip `08057a15e8f13fa3d50fc9800f026f13c52122dd`; the report commit follows it. Six implementation slices were committed and pushed in this continuation. No PR or deployment. This replaces the previous checkpoint report.

**Acceptance remains incomplete.** Polling assembly, approval decisions/services, Agent records/status/presence, registered slash dispatch, structured steps and the working-presence service are shipped. REST/MCP, lifecycle orchestration and cross-owner integrations remain unfinished. No new ignore or parity mask was added.

The accepted merge `ffd7b42c` includes main `21a7332f`; `af007e9a` enables `manages_bots` while retaining WS19b's seed gate and disconnect ordering. The fresh seeded suite still runs and passes it.

## Pushed slices and changed files

Explicit crate paths are relative to `rust/`. Abbreviated `models/` paths are under `rust/crates/db/src/`; app `integrations/` paths are under `rust/crates/campfire/src/`.

| Commit | Changes |
|---|---|
| `fbce72d0` | `crates/db/src/models/agent_event_polling.rs`, polling test/vector/oracle/probes: shared poll matrix, live access preload, payload drops and last-scanned cursor. |
| `859d904c` | `models/agent_approval.rs`, `agent_approvals.rs`, `agent_service.rs`, `activity_item.rs`, approval tests/vectors/oracles/probes and app `integrations/agent_jobs.rs`: typed records, validations, expiry, authorization, decision/inbox/event/action-job/webhook atomicity and shared create/list/show/cancel services. |
| `31ba51d7` | `models/agent.rs` and record tests/vector/oracle/probes: typed fields/changes, validators, status stamp and broadcast requests, descriptions/summaries, directory ordering, live grant/activity reads and working presence. |
| `5cd0de1c` | `models/agent_slash_command.rs`, `slash_commands.rs`, slash tests/vector/oracle/probes and app queue test: register/re-register/remove and invocation through the composer dispatcher, delivered ledger rows and atomic webhook jobs. |
| `be4ed22d` | `models/agent_step.rs` and step tests/vector/oracle/probes: typed create/update, live ownership/grants, errors, immutable parents, positions, count cap, parent-change requests and parent deletion. |
| `08057a15` | `models/agent_working_presence.rs` and service tests/vector/oracle/probes: shared set/clear and validation responses, preserving failed writes and rollback. |

Model/test registries are updated in `crates/db/src/models.rs` and `crates/db/src/tests.rs`. `models/agent_delivery.rs` adds delivered-event creation/enqueue helpers used by approvals and slash invocation. New tests live in `crates/db/src/tests/agent_{event_polling,approval,record,slash_command,step,working_presence}_test.rs`. Seven actual Rails oracles/vectors cover polling, approvals, approvals services, Agent records, slash, steps and working-presence services. `reference-tools/agents/verify-contracts.py` records 15 contracts; `check-reference.py` verifies 45 pinned source files. Six mutation scripts verify privacy, authorization, caps, expiry, writes and queue atomicity. `rust/plans/ws11-report.md` mirrors the requested external report.

Previously accepted bot auth/key reset/scrub, legacy signed/pinned/encrypted webhooks, message delivery/retry/recovery, bot posting budgets/idempotency and credential/grant/access foundations remain in place and run in the final suite.

## Behavior and stable domain seams

- Polling filters before limit, resolves every deliverable kind and advances `next_since` past dropped rows. Approval resolution uses only metadata's own-agent approval id; live work differs from deleted unassignment snapshots. Actions preserve false/empty values, and missing message PR data is explicit null. Access is preloaded once per poll. `agent_event_polling::poll` takes a message presenter and `RepositoryAccess` collaborator.
- `AgentApproval::decide_authorized`, `decidable_by` and `approvable_by` expose the human decision policy. Active owners/admins may decide; approving `github.*`/`fizzy.*` is admin-only, while owners may deny. Fresh rows guard stale/concurrent decisions. Expiry/cancel settles inbox items; cancel creates no decision event. Optional action jobs precede webhook jobs; their real GitHub/Fizzy handler registrations await the peer merges and are not verified here. Decisions, inbox, ledger and durable enqueue share one transaction; real app queue rejection proves rollback.
- `agent_approvals::{create,list,show,cancel}` returns `ServiceResult`. Membership/not-found precedes grants; replay precedes prohibited external action names and budgets. Effective-status filters precede the 100-row limit. Requests are typed `ApprovalRequest`; raw expiry/JSON/attribute coercion remains to port.
- Public `Agent`, `NewAgent` and `AgentChanges` provide typed data/updates. A changed non-null suspension revokes grants atomically; this save callback is **not** full suspension/kill-switch orchestration. Status changes alone stamp the status time; note-only changes request badge/directory replacements. Presence follows Ruby strip/blank and five-minute TTL reads without rewrites. Last-seen uses a conditional update. Secrets remain outside the public record.
- `agent_slash_command::{register,unregister,invoke}` is installed in WS8's dispatcher with built-in priority. Room/name uniqueness, foreign-owner denial, description/flag updates, active Agent/member/post checks and the 20-per-minute invocation cap are covered. Competing writers accept exactly 20 calls. Poll-only agents enqueue no webhook; durable queue rejection rolls back the invocation.
- `agent_step::{create,update}`, `AgentStep`, `NewAgentStep` and `AgentStepChanges` expose shared steps. Foreign/missing parents return 404 before grants; updates recheck live ownership/membership/grants. Exactly one parent, Unicode lengths, statuses, nonnegative durations, 50 rows per parent and monotonic positions are enforced. No-op saves still request a parent replacement. Optional missing-parent model associations match Rails validation; services reject them. WS8's message/thread dependent deletes remove steps.
- `agent_working_presence::set` returns Rails success/null-clear/error payloads and preserves failed saves. Its callers provide authentication. Typed text input does not certify every raw JSON `to_s` case.

Domain modules contain no HTML. WS11-ui owns admin bot controllers/pages, directory, approvals/events HTML, human approvals, step partials and `/agents/me` HTML. No such controller/page was implemented here. Its render adapters must consume `agent::AgentStatusChange` (badge/directory targets) and `agent_step::StepParentChange` (message/thread ids). **These new broadcast kinds are not registered in this app branch.** Whole wire-frame HTML parity is not claimed.

## Exact remaining work, in requested order

1. **Delivery/polling/bot gaps:** consume WS15g's live owner/repository access, including refresh/cache/401 behavior. Its implementation exists on `rust/ws15g-github` but not this base; no main SHA was supplied for the pending dependency question. Production webhook preparation still supplies empty `RepositoryAccess`, so readable private details remain redacted. Consume WS8b-m's actual message JSON presenter when merged: the polling oracle injects a sentinel presenter to isolate assembly/cursors, and no production polling caller exists. Install work assignment/unassignment/handoff and GitHub/Fizzy completion producers through WS12/WS15 domains; approval/slash producers are installed. Imported-message skip remains the WS16 seam. Complete remaining bot validation/attachment/update/coercion and reaction render/broadcast cases, plus unverified transport exception/write-backpressure behavior.
2. **Lifecycle/services:** approval raw-field/expiry parsing with Rails precedence, per-credential WS4 counters/throttles, DMs, context, board/work/Fizzy/GitHub shared callers and budget notices, credential/grant update/destroy lifecycle, owner/user/room revocation, full suspension/kill-switch/audit, quiet/normal stream finalization, append/replace/coalescing/trailing job/overdue sweep, reference finalize, hard user removal and Agent dependent cleanup. Clearing presence during finalize awaits streaming. Schema/backfill stays with existing owners; ownerless workspace updates are supported.
3. **REST:** all Agent JSON endpoints, authenticated Agent/credential request state and exact 401/403/404/422/429, errors/envelopes/cursors/no-store/Retry-After contracts, calling shared services and peer seams. Existing bot coverage is retained. Human pages belong to WS11-ui.
4. **MCP:** the entire stateless Streamable HTTP server, four protocol versions, mirrored/base64 headers, Origin, errors, rate limits and 38 shared-domain tools; mismatch/version failure-first tests and tool vectors remain undone.
5. **Cases:** close every partial/deferred file below. WS11-ui owns transferred HTML/controller/system cases; WS15g/WS15e own action handlers, while WS11 retains Agent callers. The lead merges peer branches; no peer implementation was copied or cherry-picked.

## Fresh command evidence

All commands below ran in this continuation. Final green runs followed restoration of deliberate mutations. Rust 1.98.1, locked dependencies, `-j4`, own target/scratch, ports 52200–52299. Commands run at the worktree root unless stated otherwise; CI makes missing seeds fail.

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/seed build > .scratch/seeds-domain-final.log 2>&1
python3 rust/reference-tools/agents/check-reference.py > .scratch/reference-domain-final.log 2>&1
python3 rust/reference-tools/agents/check-seeds.py > .scratch/seed-check-domain-final.log 2>&1
python3 rust/reference-tools/agents/verify-contracts.py > .scratch/contracts-domain-final.log 2>&1
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
```
```text
WS11 reference sources: 45 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
```
```text
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```
```text
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 12 byte-identical contract files; 15 contracts recorded in total
```

Fourteen deterministic files match committed bytes: 12 domain contracts plus bot/posting. The webhook contract regenerates its random encrypted secret and verifies deterministic invariants.

Failure-first evidence: these 34 compiled probes deliberately remove tested privacy/authorization/expiry/limit/write/queue behavior. Setup/compiler failures are excluded. Each probe restores sources in `finally`; fresh green checks follow.

```sh
python3 rust/reference-tools/agents/verify-polling-mutations.py > .scratch/polling-mutations-final.log 2>&1
```
```text
cursor-dropped-page: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
inactive-agent: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.24s
revoked-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
foreign-approval: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.28s
approval-metadata-only: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.08s
assigned-deleted-thread: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.10s
WS11 polling discrimination: 6 compiled regressions detected; sources restored
```
```sh
python3 rust/reference-tools/agents/verify-approval-mutations.py > .scratch/approval-mutations-final.log 2>&1
```
```text
external-admin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
inactive-agent: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
settled-guard: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
effective-expiry: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
payload-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.10s
decision-webhook: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.37s
service-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.08s
service-replay: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.08s
service-budget: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.08s
WS11 approval discrimination: 9 compiled regressions detected; sources restored
```
```sh
python3 rust/reference-tools/agents/verify-agent-record-mutations.py > .scratch/agent-record-mutations-final.log 2>&1
```
```text
suspension-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.07s
presence-expiry: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.22s
status-stamp: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.23s
presence-char-limit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.08s
live-agent: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.10s
WS11 agent record discrimination: 5 compiled regressions detected; sources restored
```
```sh
python3 rust/reference-tools/agents/verify-slash-mutations.py > .scratch/slash-mutations-final.log 2>&1
```
```text
registration-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.10s
unregister-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
invocation-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.09s
flood-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.12s
builtin-collision: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.13s
invocation-queue: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 342 filtered out; finished in 0.40s
WS11 slash discrimination: 6 compiled regressions detected; sources restored
```
```sh
python3 rust/reference-tools/agents/verify-step-mutations.py > .scratch/step-mutations-final.log 2>&1
```
```text
message-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.18s
thread-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.15s
parent-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.18s
create-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.15s
update-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.14s
parent-broadcast: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.14s
WS11 step discrimination: 6 compiled regressions detected; sources restored
```
```sh
python3 rust/reference-tools/agents/verify-presence-mutations.py > .scratch/presence-mutations-final.log 2>&1
```
```text
presence-write: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.19s
presence-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 449 filtered out; finished in 0.19s
WS11 presence discrimination: 2 compiled regressions detected; sources restored
```
```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets-domain-final.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 --nocapture > .scratch/ws11-domain-final.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets-domain-final.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4 --nocapture > .scratch/workspace-domain-final.log 2>&1
python3 rust/reference-tools/agents/summarize-tests.py .scratch/workspace-domain-final.log > .scratch/workspace-summary-domain-final.log
```
Focused raw summaries:

```text
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 306 filtered out; finished in 10.17s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 402 filtered out; finished in 1.57s
```

Full workspace raw summaries, in emitted order:

```text
test result: ok. 341 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 42.29s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.27s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 447 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 43.59s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.86s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.56s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.67s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.86s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
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
WS11 workspace totals: 1403 passed; 0 failed; 9 ignored; 46 result summaries
WS11 missing-seed skips: 0
```
```text
test controllers::presenters::accounts::tests::manages_bots ... ok
```

Nine ignores are inherited: app/cable reference recorders, app job measurement, three DB fixture/differential exports, mail export and two kit doctests. No missing-seed skip or WS11 ignore occurred.

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > .scratch/clippy-domain-final.log 2>&1
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.87s
```

From `rust/`:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Exit 0, no output. Strict TOML parsing rejects duplicate workspace dependency keys:

```sh
python3 - <<'PYKEYS'
import tomllib
from pathlib import Path
manifest=tomllib.loads(Path('rust/Cargo.toml').read_text())
print(f"WS11 workspace dependency keys: {len(manifest['workspace']['dependencies'])} parsed; 0 duplicates")
PYKEYS
```
```text
WS11 workspace dependency keys: 76 parsed; 0 duplicates
```

The fresh Rust export was staged on a private default-seed copy; the shared seed was not modified:

```sh
python3 - <<'PYSTAGE'
from pathlib import Path
import shutil
source=Path('rust/parity/.seed/default')
target=Path('.scratch/ws11-ar-reader-domain-final')
target.mkdir(exist_ok=True)
for child in ['db','storage']:
    shutil.copytree(source/child,target/child,dirs_exist_ok=True)
shutil.copyfile('.scratch/ws11-rust-secrets-domain-final.json',target/'db/ws11-rust-secrets.json')
print('WS11 AR reader: staged a private default-seed copy and the fresh Rust export')
PYSTAGE
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/ws11-ar-reader-domain-final" -e WS11_RUST_SECRETS_PATH=/rails/storage/db/ws11-rust-secrets.json rust/reference-tools/agents/read_rust_webhook_secrets.rb > .scratch/webhook-ar-reader-domain-final.log 2>&1
```
```text
WS11 AR reader: staged a private default-seed copy and the fresh Rust export
```
```text
WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated
```

`git diff --check` exits 0 with no output. No schema or migration change.

## File-level pass counts and deferrals

Fresh focused Rust test groups; no ignored tests counted:

| Rust group | Passed |
|---|---:|
| `controllers::messages::tests` | 17 |
| `controllers::presenters::accounts::tests` | 1 |
| `integrations::agent_jobs` | 2 |
| `integrations::agent_jobs::payload_tests` | 2 |
| `integrations::agent_jobs::tests` | 4 |
| `integrations::jobs::tests` | 3 |
| `integrations::webhook::tests` | 8 |
| `tests::agent_access_model_test` | 4 |
| `tests::agent_approval_test` | 8 |
| `tests::agent_delivery_test` | 4 |
| `tests::agent_event_access_test` | 4 |
| `tests::agent_event_polling_test` | 1 |
| `tests::agent_posting_test` | 5 |
| `tests::agent_record_test` | 6 |
| `tests::agent_slash_command_test` | 6 |
| `tests::agent_step_test` | 6 |
| `tests::agent_working_presence_test` | 2 |
| `tests::bot_webhook_fanout_test` | 2 |

All previous 83 Rails files are retained, plus the slash dispatcher: 84 files, 1181 source cases at the pin. Source counts are inventory, **not a claim that these Rails cases ran or passed**. The Rust column lists relevant consolidated checks from the table above; counts overlap across Rails files and cannot be summed. Zero means no dedicated accepted Rust file port here. Every partial file needs remaining case closure; builder/domain checks do not certify a route or HTML.

| Rails file | Source cases | Rust checks passed | Remaining / owner |
|---|---:|---|---|
| `test/channels/agents_channel_test.rb` | 2 | inherited WS7 | WS7 auth/replay retained; new status render adapters deferred with WS11-ui. |
| `test/controllers/accounts/bots/credentials_controller_test.rb` | 8 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots/github_connections_controller_test.rb` | 10 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots/grants_controller_test.rb` | 21 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots/keys_controller_test.rb` | 2 | 1 inherited | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots/webhook_secrets_controller_test.rb` | 7 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots_budgets_test.rb` | 8 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/accounts/bots_controller_test.rb` | 26 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/agent_approvals_controller_fizzy_test.rb` | 5 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/agent_approvals_controller_test.rb` | 17 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/agent_capability_test.rb` | 15 | 1 selected | Partial bot-key membership 404, grant JSON 403, immediate revocation, room/workspace scopes; remaining API/capabilities WS11. |
| `test/controllers/agent_owner_deactivation_test.rb` | 7 | 0 | Deferred: WS11. |
| `test/controllers/agent_revocation_endpoints_test.rb` | 4 | 0 | Deferred: WS11. |
| `test/controllers/agents/api_throttle_test.rb` | 13 | 0 | Deferred: WS11. |
| `test/controllers/agents/approval_delivery_test.rb` | 5 | 0 | HTTP cases deferred WS11; decision/queue/inbox core covered. |
| `test/controllers/agents/approvals_controller_test.rb` | 24 | 0 | HTTP cases deferred WS11; shared core covered by 2 service checks. |
| `test/controllers/agents/budgets_endpoints_test.rb` | 8 | 0 | Deferred: WS11. |
| `test/controllers/agents/contexts_controller_test.rb` | 13 | 0 | Deferred: WS11. |
| `test/controllers/agents/directory_controller_test.rb` | 6 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/controllers/agents/dms_controller_test.rb` | 18 | 0 | Deferred: WS11. |
| `test/controllers/agents/drive_attachments_delivery_test.rb` | 3 | 0 | Deferred: WS11. |
| `test/controllers/agents/events_controller_test.rb` | 43 | 0 | HTTP contracts deferred WS11 (access/polling core checks pass); HTML ledger WS11-ui. Actual message presenter/private-owner access pending. |
| `test/controllers/agents/fizzy/action_delivery_test.rb` | 3 | 0 | Deferred: WS11. |
| `test/controllers/agents/fizzy/boards_controller_test.rb` | 13 | 0 | Deferred: WS11. |
| `test/controllers/agents/fizzy/card_actions_controller_test.rb` | 11 | 0 | Deferred: WS11. |
| `test/controllers/agents/fizzy/cards_controller_test.rb` | 8 | 0 | Deferred: WS11. |
| `test/controllers/agents/github/pull_request_actions_controller_test.rb` | 21 | 0 | Deferred: WS11. |
| `test/controllers/agents/github_action_delivery_test.rb` | 9 | 0 | Deferred: WS11. |
| `test/controllers/agents/mcp_controller_test.rb` | 71 | 0 | Deferred: WS11. |
| `test/controllers/agents/mcp_fizzy_test.rb` | 28 | 0 | Deferred: WS11. |
| `test/controllers/agents/mcp_handoff_test.rb` | 8 | 0 | Deferred: WS11. |
| `test/controllers/agents/mcp_slash_polls_test.rb` | 14 | 0 | Deferred: WS11. |
| `test/controllers/agents/mcp_streaming_test.rb` | 11 | 0 | Deferred: WS11. |
| `test/controllers/agents/messages_controller_test.rb` | 15 | 0 | Deferred: WS11. |
| `test/controllers/agents/pins_controller_test.rb` | 10 | 0 | Deferred: WS11. |
| `test/controllers/agents/polls_controller_test.rb` | 12 | 0 | Deferred: WS11. |
| `test/controllers/agents/posts_controller_test.rb` | 24 | 0 | Deferred: WS11. |
| `test/controllers/agents/slash_command_delivery_test.rb` | 2 | 0 | HTTP cases deferred WS11; dispatcher/event/queue core covered. |
| `test/controllers/agents/slash_commands_controller_test.rb` | 15 | 0 | HTTP cases deferred WS11; registration core covered. |
| `test/controllers/agents/steps_controller_test.rb` | 12 | 0 | HTTP cases deferred WS11; 6 domain checks pass. |
| `test/controllers/agents/streaming_messages_controller_test.rb` | 17 | 0 | Deferred: WS11. |
| `test/controllers/agents/work_controller_test.rb` | 36 | 0 | Deferred: WS11. |
| `test/controllers/agents/work_delivery_test.rb` | 9 | 0 | Deferred: WS11. |
| `test/controllers/agents/work_handoff_test.rb` | 13 | 0 | Deferred: WS11. |
| `test/controllers/agents_controller_test.rb` | 10 | 0 | JSON me/raw-field/status/presence endpoint cases WS11; HTML branch WS11-ui. |
| `test/controllers/audit_log/agents_audit_test.rb` | 17 | 0 | Deferred: WS11. |
| `test/controllers/concerns/agent_authentication_test.rb` | 13 | 1 selected | Partial: valid token denied on human endpoints, revoked/expired credential and use throttle; agent me API and remaining identity cases still WS11. |
| `test/controllers/messages/boosts/by_bots_controller_test.rb` | 18 | 1 selected | Partial: reply-token/react gates plus eight exact Rails normalization/status cases, including emoji/brand aliases, unknown shortcode and blank/NBSP. Full reaction HTML/broadcast rendering and remaining validation cases: WS11 with WS8b-r. |
| `test/controllers/messages/by_bots_controller_test.rb` | 40 | 17 selected | Partial: auth/permissions, reply tokens, notes, root counts, budgets/notices, replay before validation/budget, legacy raw bodies, raw edit Markdown clearing through WS8 edit callbacks, 26 non-string/array/hash/float/escape id cases and atomic ledger enqueue failure. Remaining full validation JSON, attachment/update combinations, arbitrary coercion edge cases and complete lifecycle: WS11. |
| `test/integration/agent_boards_test.rb` | 1 | 0 | Deferred: WS11. |
| `test/jobs/agent/delivery_concurrency_test.rb` | 1 | DB 1 + app 1 | Partial: concurrent message rate cap, durable enqueue rollback, outcome and attempt CAS, duplicate queued HTTP job sends one POST. Full Rails file/end-to-end interleavings: WS11. |
| `test/jobs/agent/delivery_job_test.rb` | 29 | DB 4 + app delivery group | Partial: message availability/access/grant/hop/rate claims, suppressed rows, acknowledgment still owes a POST, actual registration and durable queue. Remaining complete lifecycle matrix: WS11. |
| `test/jobs/agent/event_webhook_job_test.rb` | 17 | app delivery/payload groups | Partial: all payload kind builders, real HTTP claims, 2xx/permanent/retryable statuses, eight Retry-After cases, five-attempt exhaustion, locked sync-reply error suppression, private-address denial and non-message response ignoring. Remaining complete Rails file and exact transport error/write-backpressure taxonomy: WS11; live private-repository decision: WS15g seam. |
| `test/jobs/fizzy/perform_agent_action_job_test.rb` | 14 | 0 | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/jobs/github/perform_agent_action_job_test.rb` | 36 | 0 | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/lib/restricted_http/private_network_guard_test.rb` | 28 | 8 webhook group | Partial: 62 numeric and 3 DNS vectors, actual HTTP transport, private-address denial, one resolution and pinned connection. Shared unfurl/push guard pin discrepancy belongs to WS5; additional transport edge cases WS11. |
| `test/models/agent/delivery_recovery_test.rb` | 13 | DB 1 + app 1 | Partial: two-minute grace, future Retry-After preservation, seven-minute exhausted recovery, snapshot attempt CAS and continuation after one durable enqueue failure. Full Rails file/lifecycle: WS11. |
| `test/models/agent_approval_test.rb` | 20 | 6 model + 2 service + 1 app | Partial consolidation: validators/defaults/expiry/cancel/deciders/admin external policy/inbox/event/queue covered. Remaining exact case comparisons include cross-agent external-id and ownerless-decider/neighbour opt-out assertions; no human controller claimed. |
| `test/models/agent_backfill_test.rb` | 1 | 0 | Deferred: WS11. |
| `test/models/agent_budgets_test.rb` | 11 | 5 | Partial: message/board/external usage readers, local days including both DST folds, exact budget JSON, notices/inbox idempotence, inactive-owner/admin fallback and transaction rollback. Board/external production callers, admin edits and remaining cases WS11. |
| `test/models/agent_credential_test.rb` | 11 | 2 selected | Partial: typed create validations/error hashes, digest uniqueness, generated 64-hex reveal-once secret/digest display id, expiry/revocation/auth and conditional one-minute usage stamps. Generic updates/destroy and admin credential endpoints: WS11. |
| `test/models/agent_event_test.rb` | 10 | 4 access + 1 polling + delivery group | Typed ledger/read/ack/polling foundation covered; Agent cleanup and actual caller/presenter remain WS11. |
| `test/models/agent_grant_test.rb` | 12 | 1 validation + access group | Partial: typed create validation/error hashes, seven capabilities, active duplicate/regrant, optional-room predicate (zero versus missing nonzero), workspace-only dm_anyone, idempotent revoke and bulk revocation. Generic updates/admin grants and all lifecycle hooks: WS11. |
| `test/models/agent_kill_switch_test.rb` | 8 | 0 | Deferred: WS11. |
| `test/models/agent_revocation_test.rb` | 8 | 2 revocation + 2 record | Membership/inactive-user/save-suspension callback covered; room/hard destroy and owner/suspend orchestration remain WS11. |
| `test/models/agent_slash_command_test.rb` | 6 | 2 model/registration; 6 slash group | Six model behaviors covered in consolidated checks; raw casts and complete REST/delivery cases remain WS11. |
| `test/models/agent_step_test.rb` | 10 | 6 | Ten model behaviors covered in 6 checks; raw casts and full REST/MCP/render delivery remain WS11/WS11-ui. |
| `test/models/agent_test.rb` | 41 | 6 record + access/secret groups | Typed record/status/grants/summaries/presence/directory/last-seen covered. Dependent destroy, full signing/last-seen race case closure and rendered status frames remain WS11/WS11-ui. |
| `test/models/agent_working_presence_test.rb` | 6 | 1 record + 2 service | Six model/service behaviors covered; finalization integration and transport coercion remain WS11. |
| `test/models/agents/work_payload_test.rb` | 3 | 1 payload + 1 polling | Webhook/poll live/deleted matrices covered; live WS15g owner access and work producer lifecycle remain WS11/WS12. |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | 0 | Deferred: WS11 with WS12 work-thread seam. |
| `test/models/fizzy/agent_card_action_test.rb` | 9 | 0 | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/github/agent_pull_request_action_test.rb` | 11 | 0 | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/message/bot_webhook_fanout_test.rb` | 2 | 2 | Partial: shared legacy fanout, recipient ordering, note/thread/stream skips and hop/chain inference; Agent ledger now installed in Message create, including replies. Imported-message skip and full lifecycle matrix: WS11 with WS16. |
| `test/models/message_streaming_test.rb` | 23 | 0 | Deferred: WS11. |
| `test/models/user/bot_test.rb` | 15 | inherited digest + bot HTTP group | Partial: inherited digest/reset checks plus HTTP reply scope; remaining lifecycle/update cases WS11. |
| `test/models/webhook_agent_key_test.rb` | 9 | 1 payload + 1 secret | Partial: no exposed bot key in Agent payloads, legacy reply URL, exact message PR/Drive samples and owner-keyed repository-access decision injection. Live private-repository lookup still needs WS15g; full Rails file: WS11. |
| `test/models/webhook_test.rb` | 22 | 8 guard + jobs/payload groups | Partial: signed/pinned/guarded transport and encryption retained; Agent jobs now handle response classification/backoff, sync-reply error suppression and every payload kind. Complete transport taxonomy/write behavior and lifecycle: WS11. |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | inherited heal/periodic | Partial: inherited healing/idempotence plus actual scheduler registration; direct heal/reset-race case still WS11. |
| `test/system/agent_approvals_test.rb` | 1 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/system/agent_streaming_test.rb` | 4 | 0 | Deferred: WS11. |
| `test/system/agent_work_assignment_test.rb` | 1 | 0 | Deferred: WS11. |
| `test/system/agents_test.rb` | 1 | 0 | Transferred to WS11-ui: all remaining controller/HTML/system cases; no new page port here. |
| `test/services/slash_commands/dispatcher_test.rb` | 40 | 4 invocation + 1 app; built-ins WS8 | Agent dispatch, flood cap and unavailable/revoked checks covered; non-agent built-ins WS8; full case closure pending. |

No `test/services/agents/` tree exists at this pin. Shared-service contracts live in the listed models/controllers/jobs and actual Rails service oracles. Deferred API files still need every endpoint-specific case even where the core has passing checks. This is a coherent pushed partial checkpoint, not cutover acceptance.
