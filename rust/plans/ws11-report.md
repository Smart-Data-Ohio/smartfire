# WS11 report — PARTIAL: work events, context, DMs and streaming lifecycle

Branch `rust/ws11-agents`; worktree `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11`; frozen Rails `d7c7de92`. Tested implementation tip `876c940bb7ca85729cb009f0a82ddfee901062a2`; the report/probe-only commit follows it. This replaces the accepted `35b4bff4` checkpoint report. Five domain slices plus one scheduler correction were committed and pushed in this continuation. No PR, deployment, schema or migration change.

**Partial.** Work-event hooks/deletion snapshots, context, DM/shared posting, suspension/kill switch/quiet finalization, and streaming/trailing/overdue services are implemented. Live private-PR access, production presenter and peer callbacks, hard user removal and full Rails case closure remain. REST/MCP/authentication/by-bot HTTP belong to WS11-api; HTML/admin/directory/human approvals/events belong to WS11-ui. Those surfaces were not implemented here. The lead merges the branches.

The previously accepted merge `ffd7b42c` includes main `21a7332f`. `af007e9a` removes the WS11 ignore from `manages_bots`; the fresh-clone suite still runs and passes it with CI's seeded failure gate. No newer main/peer merge was made in this continuation, and no peer implementation was copied.

## Pushed slices and files

Paths below are relative to `rust/`; model paths begin `crates/db/src/models/` and app paths begin `crates/campfire/src/`.

| Commit | Files and resulting behavior |
|---|---|
| `96f5cbce` | `models/agent_work_events.rs`, `bot_webhook_fanout.rs`, `channel_thread.rs`; `tests/agent_work_events_test.rs`; app `integrations/agent_jobs.rs`: assignment/unassignment/handoff ledger hooks, shared hop/chain inference, redacted pre-delete snapshots and atomic deletion/event/webhook jobs. |
| `f153b1fa` | `models/agent_context.rs`, `tests/agent_context_test.rs`: live access before grants/mismatch, bounded root/thread conversation windows, author ordering and injectable presenter. |
| `0d194f3f` | `models/agent_direct_messages.rs`, `agent_posting.rs`, `agent_service.rs`, `tests/agent_direct_messages_test.rs`: target policy, DM reuse/create, audited membership/sidebar state, scoped posting recheck and shared savepoint posting/replay/budget/Drive validation. |
| `8f338ac6` | `models/agent_lifecycle.rs`, `message.rs`, `user.rs`, `tests/agent_lifecycle_test.rs`: owned-agent suspension, grant revocation, effective pending approval cancellation/inbox settlement, repeated kill audits and after-commit quiet stream finalization. |
| `63852938` | `models/agent_streaming.rs`, `agent_posting.rs`, `message.rs`, `tests/agent_streaming_test.rs`; app `integrations/agent_streaming.rs`, `integrations.rs`, `integrations/jobs.rs`, `jobs/periodic.rs`: start/append/replace/finalize, 250 ms coalescing, durable trailing worker and 30 s overdue sweep, available normal finalization callbacks. |
| `876c940b` | app `jobs/tests.rs`, `jobs/periodic.rs`, reference source checks/oracle roster: positively verifies all three WS11 tasks against Rails and places stream finalization after retention. The initial fresh-clone run found the stale WS8-only roster assertion; this correction resolved it. |

`crates/db/src/models.rs` and `tests.rs` register the new modules. Six actual Rails oracle/vector pairs were added: `work_events_contract`, `context_contract`, `direct_messages_contract`, `lifecycle_contract`, `streaming_contract`, and `stream_trailing_contract` under `reference-tools/agents/` and `vectors/agents_*_contract.json`. `check-reference.py` and `verify-contracts.py` now cover 54 pinned source files and 21 contracts. Five mutation scripts cover 28 compiled regressions. This report commit fixes rustfmt comma anchors in `verify-work-mutations.py`, adds `domain-case-inventory.py`, and mirrors this report to `rust/plans/ws11-report.md`.

Previously accepted polling/approval/record/slash/step/presence services, signing and AR encryption, private-address guard/pinned connection, durable five-attempt delivery/recovery, bot key healing, posting budgets/idempotency and domain limits remain in the tested branch.

## Domain behavior and stable signatures

- `agent_work_events::{record_owner_change,record_handoff}` are additive shared hooks. Same-owner changes do nothing; old/new bot owners with Agent rows get unassignment/assignment events. Handoff excludes the receiver from the previous-owner notification. Human/new chains begin at hop zero; eligible recent foreign bot events carry the chain; hop three suppresses it. Missing live read permission/membership does not erase the recorded event, but prevents optional webhook enqueue. Event stamp and durable queue enqueue are atomic. `ChannelThread::destroy_by(tx, actor)` captures tags/links before deletion; existing `destroy(tx)` delegates with no actor. Deleted work payloads are always redacted, matching Rails' no-Agent snapshot path.
- `agent_context::build` returns `ServiceResult`; it accepts message/thread ids, an optional limit value, current time and a message presenter. Membership/not-found precedes read grant, which precedes conversation mismatch. Default 30/max 100, descending limited reads reversed for presentation, trigger cutoff, root/thread selection and ordered bot/human authors match the Rails oracle. Stream/system rows remain eligible. The sentinel presenter tests assembly, not production message JSON.
- `agent_direct_messages::{allowed,open_and_post}` returns `DirectMessageResult`. Missing/bot/inactive targets precede grants. Owner, workspace `dm_anyone`, or prior inbound ledger relationship permits the target. Any post grant may open a new room as Rails does; existing rooms recheck scope. New-room membership/sidebar/audit state commits even when later validation or budget denial is a service result. Outside-room reply ids return 404. `agent_posting::post_service` returns `PostResult`; existing `post` remains unchanged. Thread/reply/lock checks precede replay/budget; replay precedes Drive validation. Creator is forced to the Agent, Markdown clears raw body, and a savepoint rolls failed thread mutations back while surrounding DM/notices may commit. Attachment processing remains a caller seam.
- `agent_lifecycle::{suspend,suspend_owned,kill_switch}` uses fresh rows and audit context. Suspension is idempotent, revokes grants and finalizes open drafts after commit; rollback leaves them open. Kill first persists due expiries, cancels effective pending approvals without decision events/webhooks, settles inbox, clears presence and audits every invocation. One stream failure does not prevent the others. Existing User `deactivate`/`ban` signatures remain via additive audited variants, called at Rails' cleanup/status order. Quiet `Message::finalize_stream_quietly` claims once, updates count/presence/indicator/final frame, and skips normal index/unread/push/fanout effects, even for locked threads.
- `agent_streaming::{start,update,finalize,broadcast_update,trailing,overdue_ids}` preserves access/error order, own-message checks, locked-thread denial, append-over-replace precedence and idempotent finalize. Streaming updates leave edit timestamps alone. Coalescing uses six-digit UTC stamps and a registered durable trailing worker that waits outside the writer transaction, then rechecks the fresh stamp/stream. Duplicate/stale/finalized jobs cannot create another trailing job. The sweep backfills nil activity from creation, uses a strict ten-minute boundary, runs every 30 seconds and continues after per-row failure. Normal finalization calls available WS8 indexing/receive/push, WS11 ledger/legacy fanout/unread, message references and final presence/indicator/frame effects; absent peer callbacks are explicitly flagged.

Domain modules remain independent of HTML. Existing public services are retained; additions are usable by WS11-api/UI without changing accepted signatures. New domain broadcast requests still require UI render adapters. Cross-owner touches are the WS8 message/thread/User callback seams and scheduler registration; no peer HTTP/HTML or action handler code was implemented.

## Exact remaining work, in requested order

1. **Live delivery/presenter integration:** wire owner-keyed private-repository lookup/refresh/cache/401 behavior from WS15g after the lead's merge. Production webhook preparation still passes `RepositoryAccess::default()`, redacting private details. Install the full WS8 message presenter into polling/context and production callers; builders/cursors already exist. Install ownership/handoff mutation producers in WS12 and GitHub/Fizzy completed-action callbacks in their peer handlers; work deletion, approval and slash producers are installed. Preserve the WS16 import skip. Reconcile full transport exception/write-backpressure behavior and remaining payload cases.
2. **Remaining bot/domain services:** credential/grant update/destroy, direct backfill service, Agent dependent cleanup and hard bot/user removal remain. DMs/context/posting domain services are implemented; app/storage attachment analysis and thumbnails require callers. Existing daily budget notices and 20-per-minute message/slash limits remain; board/work/GitHub/Fizzy production budget callers and complete source-case comparisons are still open. Per-credential HTTP throttle/auth/raw request casts and by-bot message/reaction route gaps are assigned to WS11-api, not this worker.
3. **Lifecycle completion:** normal stream finalization still needs WS12 activity/inbox recording and the external reference callbacks from WS14/15. Exact failure persistence/exception taxonomy is not closed: Rails claims finalization before callbacks; the Rust writer currently keeps its normal callback mutations atomic. Hard user removal and all dependent Agent rows remain unimplemented. WS8 controller callers must use audited User variants and pass the deletion actor to `destroy_by`; default wrappers preserve signatures with nil audit actor. Status/step render adapters belong to WS11-ui.
4. **Rails case closure:** all partial/deferred domain files below still need their remaining named comparisons. Consolidated passing Rust checks and Rails oracle vectors do not imply every source case passed. API/MCP/authentication and HTML cases are transferred to the named workers; the lead merges/reconciles their flagged seams.

There is no new user approval question. The dependency question remains the lead's merge timing for presenter/work/action owners; no pending response is treated as permission to merge or copy their branches.

## Fresh-clone command evidence

All commands below actually ran in this continuation. Rust 1.98.1, locked dependencies, `-j4`, own targets/scratch, ports 52200–52299. CI turns missing seeds into failures. Implementation tested at `876c940bb7ca85729cb009f0a82ddfee901062a2`; the later report/probe-only changes do not alter Rust source. Logs stay in the original worktree's `.scratch/`.

From the original worktree, created an independent clone (no copied Cargo target/source/seeds):

```sh
git clone --single-branch --branch rust/ws11-agents https://github.com/Smart-Data-Ohio/smartfire.git .scratch/fresh-ws11
mkdir -p .scratch/fresh-ws11/.scratch
PARITY_NAMESPACE=ws11-fresh PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 .scratch/fresh-ws11/rust/parity/bin/seed build > .scratch/fresh-seed-build.log 2>&1
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

The clone was created during this continuation and fast-forwarded as coherent slices were pushed. After fixing its scheduler-roster failure, all workspace tests were rerun there. Final clone commands, with cwd `.scratch/fresh-ws11`:

```sh
git fetch origin rust/ws11-agents
git merge --ff-only FETCH_HEAD
git rev-parse HEAD > ../fresh-tested-sha.log
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 - <<'PYKEYS' > ../fresh-metadata.log
import tomllib
from pathlib import Path
manifest=tomllib.loads(Path('rust/Cargo.toml').read_text())
print(f"WS11 workspace dependency keys: {len(manifest['workspace']['dependencies'])} parsed; 0 duplicates")
PYKEYS
python3 rust/reference-tools/agents/check-seeds.py > ../fresh-seed-check.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../fresh-workspace.log 2>&1
python3 rust/reference-tools/agents/summarize-tests.py ../fresh-workspace.log > ../fresh-workspace-summary.log
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > ../fresh-clippy.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 --nocapture > ../fresh-ws11.log 2>&1
```

Metadata exited 0 with no output. TOML parsing rejects duplicate dependency keys. Raw summary lines:

```text
876c940bb7ca85729cb009f0a82ddfee901062a2
```
```text
WS11 workspace dependency keys: 76 parsed; 0 duplicates
```
```text
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```
```text
test result: ok. 345 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 38.87s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 46.17s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 462 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 54.04s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.45s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.92s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.94s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.06s
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
WS11 workspace totals: 1422 passed; 0 failed; 9 ignored; 46 result summaries
WS11 missing-seed skips: 0
```
```text
test controllers::presenters::accounts::tests::manages_bots ... ok
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.66s
```
```text
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 306 filtered out; finished in 10.58s
test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 402 filtered out; finished in 4.06s
```

The workspace has zero failures and zero missing-seed skips. Nine inherited ignores remain: app/cable reference recorders, app measurement, three DB fixture/differential exports, mail export and two kit doctests. WS11 focused checks have zero ignores. No ignore or parity mask was added.

From the original worktree, reran frozen Rails source checks and every contract after the final scheduler correction:

```sh
python3 rust/reference-tools/agents/check-reference.py > .scratch/reference-continuation-final.log 2>&1
python3 rust/reference-tools/agents/verify-contracts.py > .scratch/contracts-continuation-final.log 2>&1
```
```text
WS11 reference sources: 54 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
```
```text
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 18 byte-identical contract files; 21 contracts recorded in total
```

Twenty deterministic contracts match byte for byte; the webhook contract also checks randomized AR secret invariants. The stream-trailing oracle invokes the actual Rails job and observes broadcast ordering/state while substituting only transport recording; it does not certify HTML bytes.

## Failing security/correctness probes

Against the final implementation, each deliberate mutation had to compile and fail an assertion. Compiler/setup failures are rejected by the scripts. Originals are restored in `finally`; the final source comparison and focused green run followed all probes. These are regression discrimination evidence, not a claim that the entire Rails test file ran.

```sh
python3 rust/reference-tools/agents/verify-work-mutations.py > .scratch/work-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-context-mutations.py > .scratch/context-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-dm-mutations.py > .scratch/dm-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-lifecycle-mutations.py > .scratch/lifecycle-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-streaming-mutations.py > .scratch/streaming-mutations.log 2>&1
```
```text
read-grant: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 464 filtered out; finished in 0.09s
membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 464 filtered out; finished in 0.09s
hop-limit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 464 filtered out; finished in 0.08s
self-trigger: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 464 filtered out; finished in 0.20s
deleted-snapshot: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 464 filtered out; finished in 0.15s
work-queue: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.40s
WS11 work discrimination: 6 compiled regressions detected; sources restored
```
```text
membership: test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.16s
read-grant: test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.14s
limit-cap: test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.18s
conversation: test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.16s
trigger-cutoff: test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 463 filtered out; finished in 0.14s
WS11 context discrimination: 5 compiled regressions detected; sources restored
```
```text
target-rule: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.23s
existing-room-scope: test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.32s
inbound-types: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.26s
replay-before-budget: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.24s
thread-savepoint: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.30s
WS11 DM discrimination: 5 compiled regressions detected; sources restored
```
```text
suspension-revocation: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.34s
quiet-callback: test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.31s
expire-before-cancel: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.14s
finalize-claim: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.13s
owner-deactivation: test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 462 filtered out; finished in 0.16s
WS11 lifecycle discrimination: 5 compiled regressions detected; sources restored
```
```text
membership: test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.18s
post-grant: test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.29s
locked-thread: test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.34s
append-precedence: test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.36s
stamp-check: test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.30s
trailing-queue: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 346 filtered out; finished in 0.43s
overdue-boundary: test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 461 filtered out; finished in 0.32s
WS11 streaming discrimination: 7 compiled regressions detected; sources restored
```

The 28 detected regressions cover membership/read/post grants, work hop limit/self-trigger/deleted snapshots/atomic enqueue, context window cap/conversation/cutoff, DM target/existing-room scope/inbound relationship/replay-before-budget/savepoint, suspension/grant/quiet-callback/expiry/claim/owned-user hooks, and stream lock/append/stamp/trailing-queue/overdue boundaries. Earlier accepted probes were not rerun as part of these 28 and are not counted in this checkpoint.

Restored original-worktree sources, then ran:

```sh
git diff --exit-code -- rust/crates
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 --nocapture > .scratch/restored-ws11-final.log 2>&1
```

The source comparison exited 0 with no output. Raw restored run:

```text
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 306 filtered out; finished in 10.61s
test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 402 filtered out; finished in 3.26s
```

## Rails reads fresh Rust encryption

Staged a private copy of the fresh clone's default seed and its Rust test export, then used Rails' real model readers. Secrets stay in ignored scratch; no seed was modified.

```sh
python3 - <<'PYSTAGE' > .scratch/ar-reader-stage-final.log
from pathlib import Path
import shutil
source=Path('.scratch/fresh-ws11/rust/parity/.seed/default')
target=Path('.scratch/ws11-ar-reader-continuation')
target.mkdir(exist_ok=True)
for child in ['db','storage']:
    shutil.copytree(source/child,target/child,dirs_exist_ok=True)
shutil.copyfile('.scratch/fresh-ws11/.scratch/ws11-rust-secrets.json',target/'db/ws11-rust-secrets.json')
print('WS11 AR reader: staged a private fresh-clone default seed and fresh Rust export')
PYSTAGE
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/ws11-ar-reader-continuation" -e WS11_RUST_SECRETS_PATH=/rails/storage/db/ws11-rust-secrets.json rust/reference-tools/agents/read_rust_webhook_secrets.rb > .scratch/ar-reader-final.log 2>&1
```
```text
WS11 AR reader: staged a private fresh-clone default seed and fresh Rust export
```
```text
WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated
```

## File-level passing checks and deferred domain cases

The fresh focused run groups passed tests by Rust module/file. These disjoint groups total 104; none were ignored. Existing HTTP groups are retained evidence from accepted slices, not newly implemented API work.

| Rust group | Passed |
|---|---:|
| `controllers::messages::tests` | 17 |
| `controllers::presenters::accounts::tests` | 1 |
| `integrations::agent_jobs` | 2 |
| `integrations::agent_jobs::payload_tests` | 2 |
| `integrations::agent_jobs::tests` | 5 |
| `integrations::agent_streaming::tests` | 3 |
| `integrations::jobs::tests` | 3 |
| `integrations::webhook::tests` | 8 |
| `tests::agent_access_model_test` | 4 |
| `tests::agent_approval_test` | 8 |
| `tests::agent_context_test` | 2 |
| `tests::agent_delivery_test` | 4 |
| `tests::agent_direct_messages_test` | 3 |
| `tests::agent_event_access_test` | 4 |
| `tests::agent_event_polling_test` | 1 |
| `tests::agent_lifecycle_test` | 3 |
| `tests::agent_posting_test` | 5 |
| `tests::agent_record_test` | 6 |
| `tests::agent_slash_command_test` | 6 |
| `tests::agent_step_test` | 6 |
| `tests::agent_streaming_test` | 4 |
| `tests::agent_work_events_test` | 3 |
| `tests::agent_working_presence_test` | 2 |
| `tests::bot_webhook_fanout_test` | 2 |

The reproducible inventory reads `test ... do` declarations from OUR Rails pin, not from current main. It does not execute or claim passing Rails source cases:

```sh
python3 rust/reference-tools/agents/domain-case-inventory.py > .scratch/domain-inventory-final.log
```
```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
```

The 26 files below contain 378 source cases. The Rust-check column refers to consolidated passing groups above; **counts overlap across Rails files and cannot be summed or interpreted as a 1:1 case port**. All files still need remaining named comparisons before cutover acceptance. Zero means no dedicated file port here.

| Rails domain file | Source cases | Relevant Rust checks passed | Remaining / owner |
|---|---:|---|---|
| `test/jobs/agent/delivery_concurrency_test.rb` | 1 | delivery 4 + app jobs 7 | Concurrent cap and queue/race checks pass; remaining exact Rails interleavings and file closure: WS11. |
| `test/jobs/agent/delivery_job_test.rb` | 29 | delivery 4 + app jobs 7 | Message claims/hops/rate/suppression plus work producers covered in consolidated checks; complete lifecycle matrix and WS16 import skip remain. |
| `test/jobs/agent/event_webhook_job_test.rb` | 17 | app jobs 7 + payload 2 | Durable five-attempt handling/payloads covered; exact transport exceptions/write-backpressure and live private-PR access remain WS11/WS15g. |
| `test/lib/restricted_http/private_network_guard_test.rb` | 28 | webhook 8 | 62 numeric/3 DNS vectors and pinned transport covered; additional transport/file closure WS11; shared unfurl/push guard discrepancy WS5. |
| `test/models/agent/delivery_recovery_test.rb` | 13 | delivery 4 + app jobs 7 | Grace, future retry stamps, exhausted rows, CAS and enqueue-failure continuation covered; remaining file/lifecycle comparisons WS11. |
| `test/models/agent_approval_test.rb` | 20 | approvals 8 + app queue | Domain validations/expiry/cancel/deciders/inbox/decision enqueue covered; remaining exact external-id/ownerless/opt-out case comparisons WS11. Human page/controller WS11-ui. |
| `test/models/agent_backfill_test.rb` | 1 | 0 dedicated | Ownerless workspace records supported; direct backfill service/test still deferred WS11; no migration added. |
| `test/models/agent_budgets_test.rb` | 11 | posting 5 + DM 3 | Usage/notices/DST/idempotence/replay covered; board/work/external production callers and remaining per-file comparisons WS11. Budget HTTP/admin surfaces split workers. |
| `test/models/agent_credential_test.rb` | 11 | access 4 (2 selected) | Create/digest/auth/expiry/revocation/use stamps covered; update/destroy/Agent-dependent cleanup remain WS11; HTTP authentication WS11-api. |
| `test/models/agent_event_test.rb` | 10 | access 4 + polling 1 + delivery 4 | Ledger/access/ack/polling builders covered; Agent-dependent cleanup and full presenter/caller wiring remain WS11. |
| `test/models/agent_grant_test.rb` | 12 | access 4 + record 6 | Typed create/capabilities/duplicate/regrant/revoke covered; update/destroy and final hard-user cleanup remain WS11; controller WS11-ui. |
| `test/models/agent_kill_switch_test.rb` | 8 | lifecycle 3 + streaming 4 | Suspend/revoke/quiet streams/rollback/cancel/inbox/repeated audit covered; approved external-action handler integration WS15 peers and remaining exact file comparisons WS11. |
| `test/models/agent_revocation_test.rb` | 8 | access 4 + record 6 + lifecycle 3 | Membership/room/inactive-user/suspension and owned-user deactivation/ban covered; hard user destruction still WS11; audited controller callers WS8. |
| `test/models/agent_slash_command_test.rb` | 6 | slash 6 | Six model behaviors consolidated; remaining raw request casts/REST dispatch cases WS11-api; exact file closure WS11. |
| `test/models/agent_step_test.rb` | 10 | steps 6 | Model ownership/validation/position/cap/dependent-delete behaviors consolidated; remaining case comparisons WS11, REST/MCP WS11-api, partials WS11-ui. |
| `test/models/agent_test.rb` | 41 | record 6 + access 4 | Typed records/status/grants/presence/last-seen covered; Agent-dependent destroy and remaining race/file closure WS11; status HTML adapters WS11-ui. |
| `test/models/agent_working_presence_test.rb` | 6 | presence 2 + lifecycle 3 + streaming 4 | Set/clear/TTL and quiet/normal finalization integration covered; exact case comparisons WS11, raw endpoint casts WS11-api. |
| `test/models/agents/work_payload_test.rb` | 3 | payload 2 + polling 1 + work events 3 | Live/deleted matrices and deleted redacted snapshots covered; live owner-keyed repository lookup WS15g integration and actual presenter remain WS11. |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | work events 3 + app queue | Assignment/unassignment/handoff ledger hooks and deletion snapshots covered; install ownership/handoff mutation callers through WS12 and close 29 exact cases. |
| `test/models/message/bot_webhook_fanout_test.rb` | 2 | fanout 2 + delivery 4 + work events 3 | Legacy/Agent message chain retained, normal finalization now calls it, shared work hop inference covered; WS16 import skip and full file closure remain. |
| `test/models/message_streaming_test.rb` | 23 | streaming 4 + lifecycle 3 + app streams 3 | Start/update/quiet and available normal callbacks/trailing/sweep covered; activity/external reference callbacks and Rails failure-persistence taxonomy remain WS11 with WS12/14/15. |
| `test/models/user/bot_test.rb` | 15 | message controller 17 + lifecycle 3 | Accepted token/reset and posting behavior retained; removal/Agent-dependent lifecycle and remaining typed update/file comparisons WS11. Bot HTTP/reaction gaps WS11-api. |
| `test/models/webhook_agent_key_test.rb` | 9 | payload 2 + access 4 | Agent additive key/legacy reply URL/encrypted secret and injected PR/Drive matrices covered; live private lookup and full presenter/file closure remain WS11/WS15g/WS8. |
| `test/models/webhook_test.rb` | 22 | webhook 8 + app jobs 7 + payload 2 | Signed/pinned/encrypted transport, retry/sync reply handling covered; exact full transport exception/write behavior and lifecycle file closure WS11. |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | inherited heal + jobs 3 | Healing/idempotence/scheduler registration retained; direct reset/heal race and complete source-case comparison still WS11. |
| `test/services/slash_commands/dispatcher_test.rb` | 40 | slash 6 + app queue | Agent invocation/flood/grant checks covered; non-Agent built-ins WS8; complete dispatcher source-case closure deferred. |

No `test/services/agents/` tree exists at the pin. Context/DM services use actual Rails service oracles derived from the transferred context (13 cases) and DM (18 cases) HTTP files; their endpoint tests still belong to WS11-api. All other Agent REST/MCP/auth concerns and message/boost by-bot endpoint cases are WS11-api; admin/directory/approval/event pages and human approval controller cases are WS11-ui. GitHub/Fizzy handler model/job cases remain with WS15g/WS15e, with this worker's Agent delivery integration still flagged. Streaming browser/system acceptance awaits these shared dependencies and render/HTTP workers.

Final `git diff --check` exits 0 with no output. This is a coherent pushed partial checkpoint; full parity/cutover acceptance is not claimed.
