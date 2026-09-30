# WS11 domain continuation — partial, 2026-09-30

Branch `rust/ws11-agents`; Rails pin `d7c7de92`. Tested Rust implementation: `18c9219cc2d43430ac1fb485b85a6df2161317bc`. The final report/inventory commit changes documentation and reference inventory tools only. This replaces the previous checkpoint report; its accepted implementation remains on the branch.

## Pushed slices and changed files

| Commit | Files and behavior |
|---|---|
| `be932a18` | Merge commit with parents `ced256c9` and main `4278cb1e`. Retains enabled `manages_bots`; reconciles WS9's real sudo concern and User device cleanup with the existing WS11 suspension hook. App `controllers/concerns.rs`, `controllers/concerns/bot_keys.rs`, `controllers/presenters/accounts/tests.rs`, DB `models/user.rs`, and merged module/dependency declarations. App `jobs/tests.rs` waits for the earlier retention job as well as the quote job before asserting the queue is drained. No ignore or mask added. |
| `d60effce` | App `integrations/agent_repositories.rs`, `integrations/agent_jobs.rs`, `integrations.rs`, `app.rs`: additive owner-account repository reader seam, installed in production webhook payload preparation. Two reader tests and a real webhook POST test. `reference-tools/agents/verify-repository-mutations.py` proves three privacy/wiring regressions are detected. The live WS15g implementation is still uninstalled. |
| `f5ac1c18` | DB `models/agent_credential.rs`, `models/agent_grant.rs`, `models/agent.rs`, `models.rs`, `tests.rs`, `tests/agent_cleanup_test.rs`: validated patch types and destroy operations, Agent declared dependents, model savepoint, and explicit one-shot backfill. `reference-tools/agents/cleanup_contract.rb`, `verify-cleanup-mutations.py`, `verify-contracts.py`, `check-reference.py`; `vectors/agents_cleanup_contract.json`. |
| `03a58327` | DB `models/user.rs`, new `models/user/removal.rs`, `tests.rs`, `tests/agent_user_removal_test.rs`: core hard removal, dependency phases, and rescued-error rollback. `reference-tools/agents/user_removal_contract.rb`, `verify-removal-mutations.py`, oracle/source rosters; `vectors/agents_user_removal_contract.json`. Peer-owned dependencies remain flagged. |
| `b2bd96ff` | DB `models/agent_grant.rs`: additive active/revoked/workspace predicates and read scopes. `tests.rs`, `tests/agent_credential_cases_test.rs`, `tests/agent_grant_cases_test.rs`: 23 individually named Rails case ports. `reference-tools/agents/case-ports.json`, `check-case-ports.py`, `verify-credential-case-mutations.py`: maps these plus the existing backfill check to 24 pinned case names and detects four compiled regressions. |
| `18c9219c` | App `integrations/agent_repositories.rs`: lexical scope for a test MutexGuard before later awaits. The first fresh-clone clippy run identified this; corrected without a lint suppression, then reran the full fresh-clone suite and strict clippy. |

All Rust paths above are below `rust/crates/`; reference tools and vectors are below `rust/`. These WS11 slices add no Rails implementation, migration, schema, transferred API/MCP surface or HTML work. The main merge imports WS9 changes; no in-flight peer branch implementation was copied. Domain signatures previously accepted by WS11-ui/API remain; additions are additive. The final inventory files are `reference-tools/agents/write-case-status.py` and `deferred-domain-cases.json`.

## Design and parity findings

- `RepositoryReader::readable(RepositoryRequest)` receives the Agent owner's linked account id/user id and normalized repository pair. Thread PRs and work links are deduplicated. Database and reader-lock leases end before external awaits. A successful answer is accepted only after a fresh owner/account/disconnect check. Missing reader, owner or linked account denies private/unknown PR details. The real webhook path now passes this resolved access into `agent_payloads::build`; a socket capture proves an injected permitted private title reaches the payload. This is an adapter seam, not completed live GitHub refresh/cache/401 parity.
- `CredentialChanges` and `GrantChanges` validate changed records with own-id uniqueness exclusion, preserve no-op update stamps and support revoked-grant edits exactly as the Rails cleanup contract records. `Agent::destroy` uses a savepoint and removes its declared credentials, grants, slash commands, events and approvals, including approval inbox settlement. It deliberately preserves undeclared steps, budget notices and handoffs; FK failures remain errors. `backfill_existing_bots` reproduces the pinned INSERT SELECT: one ownerless workspace Agent per bot, and a second invocation raises a unique constraint. It is not installed in startup or a new migration.
- Rails `User::Bot` uses `dependent: :delete` for the Agent and webhook. Hard User removal therefore revokes grants, deletes the Agent row, and preserves credential/grant rows. Calling `Agent::destroy` there would change Rails behavior. Core removal follows the declaration order through messages/threads, boosts, votes, pins, saved/scheduled/search rows, categories, sessions and devices/leases/bans. Surviving pins run their callbacks once; owned-message pins are already removed by the message cascade. Work owner FKs nullify as Rails does. A nested savepoint restores rows, grants, jobs and broadcasts when a dependency/FK error is rescued by the outer writer.
- `User::destroy_with_dependencies` exposes `BeforeDestroy` (WS13 huddle revocation) and `Connections` (WS14/15/16 dependent models) phases. Adapters must prepare network work before entering the writer. Default `destroy` leaves peer rows intact and fails/rolls back if their FKs block deletion. This is core hard-removal parity, not complete production User destruction.
- Rails test files are omitted from the runtime image. Runtime model/service files are compared with image bytes and the Git pin; case files are separately compared with the Git pin. The authorized main merge's sole allowed checkout drift in this roster is `Gemfile.lock` rubyzip 3.0.2 → 3.7.0; all oracles still run in the frozen image.

## Exact remaining work, in requested order

1. **Live private-PR access and production integrations:** WS15g must install its real owner-keyed linked-account reader (decryption, refresh, ten-minute grants/denials, disconnect/401 and exception policy). Boot currently has no reader, so production private details remain redacted. Install the full WS8 message presenter in polling/context and production callers; current builder/cursor and sentinel tests do not establish full production JSON parity. Install WS12 ownership/handoff mutation producers and WS15g/WS15e completed-action callbacks; existing work deletion, approval and slash producers are installed. Preserve WS16 import suppression. Close transport exception/write-backpressure and the remaining payload cases.
2. **Credential/grant/backfill cleanup:** the requested core operations and direct backfill are implemented and tested. Remaining comparisons are explicitly named in the deferred inventory. Board/work/external-action budget callers, attachment processing and other peer production callers still need integration. Optional/absent HTTP credential conversion and HTTP authentication/throttle belong to WS11-api.
3. **Hard removal and complete finalization:** install actual huddle and Google/Calendar/GitHub/Fizzy/Slack/import/event dependent callbacks at the removal phases; default removal cannot silently delete those rows. Reconcile peer User lifecycle/controller callers after the lead's merge. Normal stream finalization still lacks WS12 activity/inbox and WS14/15 external reference callbacks. Failure persistence remains unclosed: Rails claims finalization before subsequent callbacks, while Rust currently keeps normal callback mutations in its writer transaction. Available normal/quiet/trailing/sweep behavior remains implemented; complete finalization parity is not claimed.
4. **Remaining Rails comparisons:** 24 top-level named case ports are mapped below; 354 other named source cases across 23 files remain for explicit case closure. Consolidated tests and real Rails contracts are additional evidence, not proof every assertion in each source file passed. `deferred-domain-cases.json` lists every remaining case name and owner. API/MCP/auth/by-bot HTTP cases are WS11-api; admin/directory/approval/event HTML and the human approval controller are WS11-ui. No Rails test files themselves are claimed executed.

**Open integration question:** reconcile Rails claim-before-callback-error persistence with atomic durable enqueue when the peer finalization callbacks land. This is a correctness gap to close, not a proposed change to the fixed queue decision.

## Fresh-clone verification

An independent clone was created during this continuation at `.scratch/fresh-ws11-ws9`, initially at the merge slice and fast-forwarded as slices were pushed. Cargo target and all nine seeds were built there; no source, seed or target was copied from the original worktree. Initial setup commands (cwd original worktree):

```sh
git clone --single-branch --branch rust/ws11-agents https://github.com/Smart-Data-Ohio/smartfire.git .scratch/fresh-ws11-ws9
mkdir -p .scratch/fresh-ws11-ws9/.scratch
PARITY_NAMESPACE=ws11-fresh-ws9 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 .scratch/fresh-ws11-ws9/rust/parity/bin/seed build > .scratch/ws9-fresh-seed-build.log 2>&1
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
Final clone commands (cwd `.scratch/fresh-ws11-ws9`; Rust 1.98.1, locked dependencies, own target/scratch, `-j4`, ports 52200–52299):

```sh
git fetch origin rust/ws11-agents
git merge --ff-only FETCH_HEAD
git rev-parse HEAD > ../ws9-fresh-tested-sha.log
cd rust
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
cd ..
python3 - <<'PYKEYS' > ../ws9-fresh-metadata.log
from pathlib import Path
import tomllib
keys=tomllib.loads(Path('rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'WS11 workspace dependency keys: {len(keys)} parsed; 0 duplicates')
p=Path('rust/crates/campfire/src/controllers/presenters/accounts/tests.rs').read_text()
head=p[:p.index('fn manages_bots')]; annotation=head[head.rfind('#[test]'):]
assert '#[ignore' not in annotation
print('WS11 main merge: manages_bots remains enabled')
PYKEYS
python3 rust/reference-tools/agents/check-seeds.py > ../ws9-fresh-seeds.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../ws9-fresh-final-workspace.log 2>&1
python3 rust/reference-tools/agents/summarize-tests.py ../ws9-fresh-final-workspace.log > ../ws9-fresh-final-summary.log
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > ../ws9-fresh-final-clippy.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 --nocapture > ../ws9-fresh-final-ws11.log 2>&1
```

Metadata exits 0 with no stdout; strict TOML parsing rejects duplicate keys. Raw summaries:

```text
WS11 workspace dependency keys: 76 parsed; 0 duplicates
WS11 main merge: manages_bots remains enabled
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```
```text
test controllers::presenters::accounts::tests::manages_bots ... ok
test result: ok. 440 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 112.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.27s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 537 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 58.41s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.44s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.44s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.35s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.79s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.49s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.07s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.75s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
WS11 workspace totals: 1597 passed; 0 failed; 10 ignored; 46 result summaries
WS11 missing-seed skips: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.63s
```
Ten inherited ignores remain (app/cable reference/measurement cases, four DB fixture/export/rollback cases, mail export and two kit doctests); no WS11 ignore was introduced. Missing seeds fail under CI, and the skip counter is zero. `html5ever` is the excluded upstream crate, as in the shared workspace command. Focused WS11 raw results:

```text
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 398 filtered out; finished in 11.52s
test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 449 filtered out; finished in 3.99s
```
## Current Rails oracle and mutation evidence

Every command in this section was rerun in this continuation from the original worktree. The four probe scripts require compiled assertion failures and restore source bytes in `finally`; a compile/setup failure is rejected as evidence. Only the 16 regressions in these logs are counted here; prior accepted probes are not included. They do not claim additional source case closure.

```sh
python3 rust/reference-tools/agents/check-reference.py > .scratch/reference-sources-final.log 2>&1
python3 rust/reference-tools/agents/verify-contracts.py > .scratch/contracts-final.log 2>&1
python3 rust/reference-tools/agents/verify-repository-mutations.py > .scratch/repository-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-cleanup-mutations.py > .scratch/cleanup-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-removal-mutations.py > .scratch/removal-mutations.log 2>&1
python3 rust/reference-tools/agents/verify-credential-case-mutations.py > .scratch/credential-case-mutations.log 2>&1
git diff --exit-code -- rust/crates
```
```text
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
```
```text
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 20 byte-identical contract files; 23 contracts recorded in total
```
```text
owner-identity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 441 filtered out; finished in 0.56s
disconnect: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 441 filtered out; finished in 1.58s
production-access: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 441 filtered out; finished in 2.11s
WS11 repository discrimination: 3 compiled regressions detected; sources restored
```
```text
grant-duplicate: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.32s
credential-validation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.21s
agent-grants: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.23s
approval-inbox: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.34s
backfill-kind: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.28s
WS11 cleanup discrimination: 5 compiled regressions detected; sources restored
```
```text
grant-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.27s
agent-cascade: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.34s
pin-callback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.33s
savepoint-rollback: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.66s
WS11 hard-removal discrimination: 4 compiled regressions detected; sources restored
```
```text
revoked-auth: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.75s
expired-auth: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 1.80s
use-throttle: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.14s
grant-active-scope: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 540 filtered out; finished in 0.35s
WS11 credential/grant case discrimination: 4 compiled regressions detected; sources restored
```
The restored source comparison exits 0 with no output. The repository probes detect bot-vs-owner identity, disconnect bypass and replacing production resolved access with empty access. Cleanup probes detect duplicate/credential validation, missing grant destruction/inbox settlement and wrong backfill kind. Removal probes detect missed revocation, wrong Agent cascade, skipped pin callbacks and loss of rescued-error savepoint rollback. Credential/grant probes detect revoked/expired authentication, usage-throttle bypass and inverted active scopes. Fresh-clone full/focused runs above then verify the unmutated branch.

Rails reads fresh Rust encryption, using a private copy of the clone's default seed and its test export (no values are printed; no seed changed):

```sh
python3 - <<'PYSTAGE' > .scratch/ar-reader-stage-final.log
from pathlib import Path
import shutil
source=Path('.scratch/fresh-ws11-ws9/rust/parity/.seed/default')
target=Path('.scratch/ws11-ar-reader-ws9')
target.mkdir(exist_ok=True)
for child in ['db','storage']:
    shutil.copytree(source/child,target/child,dirs_exist_ok=True)
shutil.copyfile('.scratch/fresh-ws11-ws9/.scratch/ws11-rust-secrets.json',target/'db/ws11-rust-secrets.json')
print('WS11 AR reader: staged a private fresh-clone default seed and fresh Rust export')
PYSTAGE
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/ws11-ar-reader-ws9" -e WS11_RUST_SECRETS_PATH=/rails/storage/db/ws11-rust-secrets.json rust/reference-tools/agents/read_rust_webhook_secrets.rb > .scratch/ar-reader-final.log 2>&1
```
```text
WS11 AR reader: staged a private fresh-clone default seed and fresh Rust export
WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated
```
## File-level passing counts and deferred comparisons

Disjoint Rust module/file groups from the fresh focused run total 136 passing tests with zero ignores. Existing HTTP groups are retained verification of accepted slices, not new API ownership work.

| Rust module/file group | Passed |
|---|---:|
| `controllers::messages::tests` | 17 |
| `controllers::presenters::accounts::tests` | 1 |
| `integrations::agent_jobs` | 2 |
| `integrations::agent_jobs::payload_tests` | 2 |
| `integrations::agent_jobs::tests` | 6 |
| `integrations::agent_repositories::tests` | 2 |
| `integrations::agent_streaming::tests` | 3 |
| `integrations::jobs::tests` | 3 |
| `integrations::webhook::tests` | 8 |
| `tests::agent_access_model_test` | 4 |
| `tests::agent_approval_test` | 8 |
| `tests::agent_cleanup_test` | 3 |
| `tests::agent_context_test` | 2 |
| `tests::agent_credential_cases_test` | 11 |
| `tests::agent_delivery_test` | 4 |
| `tests::agent_direct_messages_test` | 3 |
| `tests::agent_event_access_test` | 4 |
| `tests::agent_event_polling_test` | 1 |
| `tests::agent_grant_cases_test` | 12 |
| `tests::agent_lifecycle_test` | 3 |
| `tests::agent_posting_test` | 5 |
| `tests::agent_record_test` | 6 |
| `tests::agent_slash_command_test` | 6 |
| `tests::agent_step_test` | 6 |
| `tests::agent_streaming_test` | 4 |
| `tests::agent_user_removal_test` | 3 |
| `tests::agent_work_events_test` | 3 |
| `tests::agent_working_presence_test` | 2 |
| `tests::bot_webhook_fanout_test` | 2 |

Pinned case inventory and exact name mappings:

```sh
python3 rust/reference-tools/agents/check-case-ports.py > .scratch/case-ports.log
python3 rust/reference-tools/agents/domain-case-inventory.py > .scratch/domain-inventory-final.log
python3 rust/reference-tools/agents/write-case-status.py > .scratch/deferred-cases.log
```
```text
WS11 named ports: test/models/agent_credential_test.rb: 11 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_grant_test.rb: 12 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_backfill_test.rb: 1 mapped cases; 0 unmapped case names
WS11 source case files: 3 pinned Git files matched; 0 checkout mismatches
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 23 pinned files; 354 named source cases; owners recorded per file
```
The 11 credential and 12 grant ports passed individually; the backfill port is one of three passing cleanup checks. Case-name closure does not claim Rails' nil-input subassertion at the HTTP boundary: Rust domain auth takes `&str`; WS11-api owns absent-input conversion. Exact mapped names and Rust functions are in `case-ports.json`. The following counts are named ports per Rails file, distinct from overlapping consolidated check groups above.

| Pinned Rails file | Source cases | Named Rust case ports passing | Named comparisons remaining / owner |
|---|---:|---:|---|
| `test/jobs/agent/delivery_concurrency_test.rb` | 1 | 0 | 1; WS11 domain |
| `test/jobs/agent/delivery_job_test.rb` | 29 | 0 | 29; WS11 domain |
| `test/jobs/agent/event_webhook_job_test.rb` | 17 | 0 | 17; WS11 delivery; WS15g live repository reader |
| `test/lib/restricted_http/private_network_guard_test.rb` | 28 | 0 | 28; WS11 domain |
| `test/models/agent/delivery_recovery_test.rb` | 13 | 0 | 13; WS11 domain |
| `test/models/agent_approval_test.rb` | 20 | 0 | 20; WS11 domain; WS11-ui human approval controller/pages |
| `test/models/agent_backfill_test.rb` | 1 | 1 | 0; Named ports closed |
| `test/models/agent_budgets_test.rb` | 11 | 0 | 11; WS11 domain |
| `test/models/agent_credential_test.rb` | 11 | 11 | 0; Named ports closed; credential nil input remains an API boundary check |
| `test/models/agent_event_test.rb` | 10 | 0 | 10; WS11 domain |
| `test/models/agent_grant_test.rb` | 12 | 12 | 0; Named ports closed |
| `test/models/agent_kill_switch_test.rb` | 8 | 0 | 8; WS11; WS15g/WS15e approved-action execution callbacks |
| `test/models/agent_revocation_test.rb` | 8 | 0 | 8; WS11 domain |
| `test/models/agent_slash_command_test.rb` | 6 | 0 | 6; WS11 domain |
| `test/models/agent_step_test.rb` | 10 | 0 | 10; WS11 domain |
| `test/models/agent_test.rb` | 41 | 0 | 41; WS11 domain |
| `test/models/agent_working_presence_test.rb` | 6 | 0 | 6; WS11 domain |
| `test/models/agents/work_payload_test.rb` | 3 | 0 | 3; WS11; WS8 message presenter; WS15g private repository reader |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | 0 | 29; WS11 agent callbacks; WS12 mutation producers |
| `test/models/message/bot_webhook_fanout_test.rb` | 2 | 0 | 2; WS11; WS16 import suppression integration |
| `test/models/message_streaming_test.rb` | 23 | 0 | 23; WS11 finalization; WS12 activity; WS14/15 external reference sync |
| `test/models/user/bot_test.rb` | 15 | 0 | 15; WS11 bot domain/removal; WS11-api by-bot HTTP surface |
| `test/models/webhook_agent_key_test.rb` | 9 | 0 | 9; WS11; WS8 presenter; WS15g live private repository access |
| `test/models/webhook_test.rb` | 22 | 0 | 22; WS11 domain |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | 0 | 3; WS11 domain |
| `test/services/slash_commands/dispatcher_test.rb` | 40 | 0 | 40; WS11 agent dispatch; WS8 built-in commands |

No `test/services/agents/` tree exists at this pin. Context/DM service contracts are exercised directly through the frozen Rails runner; transferred REST tests remain WS11-api's responsibility. All other agent REST/MCP/authentication/throttle and message/boost by-bot endpoint cases are WS11-api. Admin/directory/approval/event HTML and human approval controller cases are WS11-ui. GitHub/Fizzy action implementation cases remain WS15g/WS15e. The shared dispatcher includes WS8's non-agent commands, so its 40 deferred named ports are not all new WS11 implementation requirements.

Final `git diff --check` exits 0 with no output. This is a coherent pushed partial checkpoint; full domain or cutover parity is not claimed. Resume at the live provider/presenter and peer callback installations, then finalization failure semantics and the individually named deferred cases.
