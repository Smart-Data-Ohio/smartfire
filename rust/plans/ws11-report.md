# WS11 report — PARTIAL: main merge, Agent delivery and access foundation

Branch `rust/ws11-agents`; worktree `.claude/worktrees/rust-ws11`; original base `bb6c5d78`; reference Rails `d7c7de92`. This report replaces the earlier webhook-only status. Last implementation tip: `67ab17338729a2347148307ae8c2a3514a8dc7f1`; the verification/report commit follows it. No PR or deployment.

**WS11 acceptance is still incomplete.** The message delivery pipeline, retry/recovery policy, webhook payload builders and selected bot gaps are shipped. Grant/credential records and event selection/acknowledgment are shipped as domain foundations. The full Agent domain, REST, MCP and management pages remain unfinished. Green Rust tests do not imply full Rails parity.

## Pushed coherent slices

- `ffd7b42c1719a2405bcaa3d80784babe7bf34cd1`: merge commit for authorized `origin/main` at `21a7332f2d3c324f0862cdf448baf17a84395aa0` (WS19b). `af007e9a81fdec45a600f49e0330433886bef609` retains its CI seed gate while enabling `manages_bots`. Incoming cable close/drain behavior is retained. `manages_bots` has no ignore and passes in the final seeded suite.
- `caf989843aaed9853e214342dd04d024fc4352b2`: atomic message event ledger and durable delivery jobs, attempt claims, five-attempt backoff, recovery and real HTTP/concurrency tests.
- `2cd6f5906366407ba7be593541309c3584efbd23`: webhook payload dispatch for approval, slash, GitHub/Fizzy completion, work assignment/unassignment/handoff and message PR/Drive cases.
- `94c5b4df4501b5da069c62746e6ba57fe618f37f`: raw bot edits, shortcode normalization and selected non-string client-id coercion/replay.
- `4804f47e9e9450cb8c8c785c6505fc30fc2eef19`: typed grants/credentials, digest-only creation and atomic membership/user-status grant revocation.
- `67ab17338729a2347148307ae8c2a3514a8dc7f1`: fresh capability checks, readable event selection before pagination and idempotent acknowledgment that preserves webhook claims.

The previously accepted auth/reply-token/key-reset/scrub, legacy signed webhooks/fanout and posting-budget/idempotency foundations remain in place and run in the final suite.

## What changed, by file

- `rust/crates/db/src/models/agent_delivery.rs`: AgentEvent reads/create validations, hop mirror, posted/recipient ledger rows, rate/hop suppression, delivery outcome CAS, independent webhook attempt CAS, settle/retry state and stranded/exhausted recovery. `message.rs` installs its enqueue in WS8a's create transaction, including threaded and synchronous replies. `bot_webhook_fanout.rs` shares live hop/chain reads with legacy fanout.
- `rust/crates/campfire/src/integrations/agent_jobs.rs`, `integrations.rs`: actual `Agent::DeliveryJob` and `Agent::EventWebhookJob` registration; HTTP preparation outside the writer lock; pinned guarded transport and exact response/Retry-After classification. Only successful message POSTs create synchronous replies; reply-save errors are suppressed after the successful POST. Missing payloads/guard failures settle permanently; retryable responses and transport errors use ledger backoff. `integrations/jobs.rs` shares reply writing helpers; `integrations/webhook.rs` exposes response headers without changing the existing seven-second open/read policy.
- `rust/crates/campfire/src/jobs/periodic.rs`: 30-second stranded-webhook sweep; each candidate's durable enqueue is its own writer transaction and failure does not prevent later recovery. `jobs/tests.rs` explicitly checks this task while preserving WS8's scheduler inventory assertions. No durable queue schema/storage redesign.
- `rust/crates/db/src/models/agent_payloads.rs`: exact webhook JSON builders for every deliverable kind, shared read-only work fields, ordered links and nonempty Drive/PR fields. Owner/repository-keyed private access is an injected decision. Production currently supplies an empty access set because WS15g's live lookup has not landed at this base. Authorized private PR details therefore remain incomplete in production; the seam is explicitly tested, including denial for a different owner.
- `rust/crates/campfire/src/integrations/agent_payload_tests.rs`: 16 pinned Rails byte payloads, four unavailable-payload errors, private-owner isolation and an actual non-message HTTP 2xx body ignored without a reply.
- `rust/crates/db/src/models/agent_posting/client_ids.rs`, `agent_posting.rs`: separate stored-string versus lookup coercion, array IN lookup/flattening, blank/hash/error behavior and Ruby string/float inspection for the 26 recorded inputs. Both preflight and final writer checks use the same lookup, preserving replay before budgets/attachment validation. This is selected coercion coverage, not a claim about every arbitrary Ruby/JSON value.
- `rust/crates/db/src/models/message.rs`, `campfire/src/controllers/messages.rs`, `messages/by_bots.rs`: raw bot edits clear stale Markdown and use WS8's edit callbacks/stamp. `db/src/rich_text.rs`, `models/boost.rs`, `campfire/src/rich_text.rs`: shared boost-content resolution via the existing icon catalog; eight Rails normalization/status cases. Complete reaction HTML/broadcast presentation remains a WS8b-r/WS11 seam.
- `rust/crates/db/src/models/agent_credential.rs`, `agent_grant.rs`, `agent_access.rs`: typed records/create validations/revoke, reveal-once 32-byte hex secrets stored only as SHA-256 digest plus digest-derived display id, conditional one-minute use stamps, credential/Agent request identity seam, fresh room/workspace/anywhere capability checks and revoked-row denial of legacy fallback. Full Agent CRUD/administration is not included.
- `rust/crates/db/src/models/membership.rs`, `user.rs`: room grants revoke before membership deletion and all Agent grants revoke on a changed inactive user status, in the same transaction. Room destruction/owner/Agent suspension/hard-destroy callbacks remain incomplete.
- `rust/crates/db/src/models/agent_event_access.rs`: readable ordered pages (default 50/max 100) filter before limit without duplicate rows from overlapping grants. The selection intentionally matches Rails' raw scope; later payload drops must still advance the future poll cursor. Ack applies the actual Rails message-membership versus non-message-anywhere gate, excludes null/suppressed/ledger-only/other-Agent rows and preserves webhook status/attempts. Presenter payload assembly, dropped-page cursor and production REST/MCP callers remain absent.
- `rust/crates/db/src/tests/{agent_delivery_test,agent_access_model_test,agent_event_access_test}.rs`, `campfire/src/controllers/messages/tests.rs`: real writer concurrency, durable enqueue failure/rollback, grant callback rollback, live auth/expiry/stamps, bot HTTP vectors and access/ack matrix; model/module registries expose the new domain types.
- `rust/reference-tools/agents/{delivery_contract,event_payload_contract,bot_gaps_contract,grant_credential_contract,event_access_contract}.rb`, matching `rust/vectors/agents_*_contract.json`: actual pinned Rails jobs/models/services/controllers. Payload oracle stubs only the network and the explicit pending WS15g repository-decision seam. It does not supply a fake Agent delivery implementation.
- `rust/reference-tools/agents/{verify-delivery-mutations,verify-payload-mutations,verify-event-access-mutations}.py`: compiled regression discrimination, restoration in `finally`. `verify-contracts.py` re-records all eight contracts and compares deterministic fields. `check-reference.py` checks 36 pinned files; merged main's only permitted checkout drift is #159's rubyzip 3.0.2 → 3.7.0 lock entry. The oracle still uses the actual pinned lock/image. `rust/Cargo.toml`, `Cargo.lock`, `crates/campfire/Cargo.toml` add the locked `httpdate` dependency for Retry-After dates.
- `rust/plans/ws11-report.md`: tracked mirror of the requested external report.

## Design and correctness limits

The DB domain has no HTML dependency. Ledger changes and durable enqueues are atomic with message writes, while DNS/HTTP happen outside the writer lock. Polling acknowledgment and owed webhook delivery are independent. Retrying 408/429/5xx uses Retry-After (seconds/date, clamped to one hour) or attempt^4 + 2 seconds, and fails after the fifth claimed attempt. Permanent failures revert the speculative attempt increment as Rails does. Recovery preserves future retries, resumes older pending rows, marks abandoned exhausted rows after seven minutes and continues after an isolated queue failure.

16 byte payload samples cover every kind, null/compact behavior, live/deleted work snapshots, handoff, nonempty links/Drive, public/private PR and the injected owner-readable private case. This is builder parity, not a claim that the unimplemented approvals/work/slash/action domain callbacks create all those rows in production. Poll payloads differ from webhook payloads and still need their own complete builder/cursor implementation. Imported-message Agent skip state also awaits the WS16 seam.

Exact HTTP write/backpressure timing and Rust-versus-Ruby transport exception/error text remain unverified. Open/read timeouts, private-network denial, one resolution with a pinned connection, HMAC bytes and real status/retry behavior are tested. Private PR live access needs WS15g. Full User-save validators and reaction rendering remain cross-owner seams. No parity mask or test ignore was added; the WS11 ignore was removed.

## Current commands and raw evidence

Every command below was executed afresh in this continuation. Logs are this worktree's `.scratch/`; Rust is 1.98.1, `-j4`, own target/scratch and ports 52200–52299. All final green runs occurred after restoring deliberate mutations. The seeded app tests ran with `CI=1`, so missing seeds fail instead of silently skipping.

From the worktree root:

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/seed build >.scratch/seeds-delivery-final.log 2>&1
python3 rust/reference-tools/agents/check-reference.py >.scratch/reference-check-delivery-final.log 2>&1
python3 rust/reference-tools/agents/check-seeds.py >.scratch/seed-check-delivery-final.log 2>&1
python3 rust/reference-tools/agents/verify-contracts.py >.scratch/contracts-delivery-final.log 2>&1
```

Raw output:

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
WS11 reference sources: 36 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 delivery/payload/bot-gaps/grant-credential/event-access Rails oracles: 5 byte-identical contract files
```

The contract runner invokes our pinned reference runner on private default-seed copies for all eight Ruby scripts. Five new contracts and the original bot/posting contracts are byte-identical; only the webhook's deliberately random encrypted secret is compared by its deterministic invariants. The Rust tests also decrypt the committed actual Rails ciphertext sample.

Security/correctness failure-first snapshots from this continuation (historical pre-fix runs, not compile failures): non-message payload not dispatched; raw edit left stale Markdown; boost/client coercion mismatched; membership/user status did not revoke grants. The revocation run had two genuine failed assertions after fixture setup was corrected. Initial mention/body setup runs are excluded; the fresh delivery probes below use corrected fixtures. No setup or compiler failure is counted.

```text
agent-payload-red.log: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 337 filtered out; finished in 0.35s
bot-markdown-red.log: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 338 filtered out; finished in 0.65s
bot-gaps-red.log: test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 333 filtered out; finished in 1.12s
agent-access-model-red.log: test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 413 filtered out; finished in 0.17s
```

Fresh compiled regression probes (all source files restored, followed by green tests):

```sh
python3 rust/reference-tools/agents/verify-delivery-mutations.py >.scratch/delivery-mutations-final.log 2>&1
python3 rust/reference-tools/agents/verify-payload-mutations.py >.scratch/payload-mutations-final.log 2>&1
python3 rust/reference-tools/agents/verify-event-access-mutations.py >.scratch/event-access-mutations-final.log 2>&1
```

```text
ledger-hook: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.11s
queue-atomicity: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.73s
rate-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.14s
revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.10s
attempt-claim: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.37s
five-attempt-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.11s
future-retry: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.11s
recovery-isolation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.35s
WS11 delivery discrimination: 8 compiled regressions detected; sources restored
private-title: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.35s
owner-access: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.35s
deleted-snapshot: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.36s
compact-false: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 340 filtered out; finished in 0.35s
WS11 payload discrimination: 4 compiled regressions detected; sources restored
selection-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.11s
selection-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.10s
selection-membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.08s
ack-owner: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.10s
ack-membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.09s
ack-webhook-isolation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 420 filtered out; finished in 0.10s
WS11 event access discrimination: 6 compiled regressions detected; sources restored
```

Final focused and entire workspace checks:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets-final.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 --nocapture >.scratch/ws11-delivery-final.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets-final.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4 --nocapture >.scratch/workspace-delivery-final.log 2>&1
python3 rust/reference-tools/agents/summarize-tests.py .scratch/workspace-delivery-final.log
```

Focused raw summaries:

```text
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 306 filtered out; finished in 11.64s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 402 filtered out; finished in 0.87s
```

Full workspace raw summaries, in emitted order:

```text
test result: ok. 339 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 41.71s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.22s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 418 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 56.95s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.43s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.55s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.30s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.78s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.83s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.05s
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
WS11 workspace totals: 1372 passed; 0 failed; 9 ignored; 46 result summaries
WS11 missing-seed skips: 0
```

The nine ignored results are inherited reference-recording/export/measurement tests and doctests. Both ignored app tests are the existing live-reference recorder and job measurement; `manages_bots` runs. No missing-seed test was skipped. Raw app line:

```text
test controllers::presenters::accounts::tests::manages_bots ... ok
```

Rust-to-Rails encryption proof: a fresh private copy of default seed DB/storage was staged at `.scratch/ws11-ar-reader-delivery-final`, with this focused run's actual export copied into `db/ws11-rust-secrets.json`; no shared seed was modified. Then:

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/ws11-ar-reader-delivery-final" -e WS11_RUST_SECRETS_PATH=/rails/storage/db/ws11-rust-secrets.json rust/reference-tools/agents/read_rust_webhook_secrets.rb >.scratch/webhook-ar-reader-delivery-final.log 2>&1
```

```text
WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated
```

Strict clippy from the worktree root:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings >.scratch/clippy-delivery-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.89s
```

Locked metadata from `rust/`:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null
```

Exit 0, no output. Workspace dependency duplicate check from root (strict TOML parsing rejects duplicate keys):

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

`git diff --check` exited 0, no output. The lockfile is valid and the `httpdate` addition is intentional.

## Rails test file accounting

Partial means selected contracts only, not an entire Rails file or complete end-to-end parity. Every deferred file remains with the named owner. No `test/services/agents/` test tree exists at this pin; its service contracts live in the listed model/controller/job tests. Fixtures/helpers are not counted. This retains the full earlier inventory and updates this continuation's coverage.

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
| `test/controllers/agents/events_controller_test.rb` | Partial underlying domain selection/ack only: own-row access, filtering before limits and ack policy from Rails vectors. REST endpoint, JSON poll payloads, dropped-page cursor/header/envelope and ledger HTML: WS11. |
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
| `test/controllers/messages/boosts/by_bots_controller_test.rb` | Partial: reply-token/react gates plus eight exact Rails normalization/status cases, including emoji/brand aliases, unknown shortcode and blank/NBSP. Full reaction HTML/broadcast rendering and remaining validation cases: WS11 with WS8b-r. |
| `test/controllers/messages/by_bots_controller_test.rb` | Partial: auth/permissions, reply tokens, notes, root counts, budgets/notices, replay before validation/budget, legacy raw bodies, raw edit Markdown clearing through WS8 edit callbacks, 26 non-string/array/hash/float/escape id cases and atomic ledger enqueue failure. Remaining full validation JSON, attachment/update combinations, arbitrary coercion edge cases and complete lifecycle: WS11. |
| `test/integration/agent_boards_test.rb` | Deferred: WS11. |
| `test/jobs/agent/delivery_concurrency_test.rb` | Partial: concurrent message rate cap, durable enqueue rollback, outcome and attempt CAS, duplicate queued HTTP job sends one POST. Full Rails file/end-to-end interleavings: WS11. |
| `test/jobs/agent/delivery_job_test.rb` | Partial: message availability/access/grant/hop/rate claims, suppressed rows, acknowledgment still owes a POST, actual registration and durable queue. Remaining complete lifecycle matrix: WS11. |
| `test/jobs/agent/event_webhook_job_test.rb` | Partial: all payload kind builders, real HTTP claims, 2xx/permanent/retryable statuses, eight Retry-After cases, five-attempt exhaustion, locked sync-reply error suppression, private-address denial and non-message response ignoring. Remaining complete Rails file and exact transport error/write-backpressure taxonomy: WS11; live private-repository decision: WS15g seam. |
| `test/jobs/fizzy/perform_agent_action_job_test.rb` | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/jobs/github/perform_agent_action_job_test.rb` | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/lib/restricted_http/private_network_guard_test.rb` | Partial: 62 numeric and 3 DNS vectors, actual HTTP transport, private-address denial, one resolution and pinned connection. Shared unfurl/push guard pin discrepancy belongs to WS5; additional transport edge cases WS11. |
| `test/models/agent/delivery_recovery_test.rb` | Partial: two-minute grace, future Retry-After preservation, seven-minute exhausted recovery, snapshot attempt CAS and continuation after one durable enqueue failure. Full Rails file/lifecycle: WS11. |
| `test/models/agent_approval_test.rb` | Deferred: WS11. |
| `test/models/agent_backfill_test.rb` | Deferred: WS11. |
| `test/models/agent_budgets_test.rb` | Partial: message/board/external usage readers, local days including both DST folds, exact budget JSON, notices/inbox idempotence, inactive-owner/admin fallback and transaction rollback. Board/external production callers, admin edits and remaining cases WS11. |
| `test/models/agent_credential_test.rb` | Partial: typed create validations/error hashes, digest uniqueness, generated 64-hex reveal-once secret/digest display id, expiry/revocation/auth and conditional one-minute usage stamps. Generic updates/destroy and admin credential endpoints: WS11. |
| `test/models/agent_event_test.rb` | Partial: create/type/outcome/hop model, message ledger, ordered readable SQL pages, suppression exclusion, scoped/workspace/legacy/revoked access, ack idempotence/isolation and pagination clamp. Full polling presenter/cursor and Agent cascade destroy: WS11. |
| `test/models/agent_grant_test.rb` | Partial: typed create validation/error hashes, seven capabilities, active duplicate/regrant, optional-room predicate (zero versus missing nonzero), workspace-only dm_anyone, idempotent revoke and bulk revocation. Generic updates/admin grants and all lifecycle hooks: WS11. |
| `test/models/agent_kill_switch_test.rb` | Deferred: WS11. |
| `test/models/agent_revocation_test.rb` | Partial: membership removal and inactive-user status revoke grants in the triggering transaction, rollback preserved. Room destruction, Agent suspension, owner deactivation, hard removal and finalization remain WS11. |
| `test/models/agent_slash_command_test.rb` | Deferred: WS11. |
| `test/models/agent_step_test.rb` | Deferred: WS11. |
| `test/models/agent_test.rb` | Partial: active/grant/last-seen reads and row-locked encrypted webhook-secret generation. Agent CRUD validators/callbacks, status/presence, admin budget updates and directory still WS11. |
| `test/models/agent_working_presence_test.rb` | Deferred: WS11. |
| `test/models/agents/work_payload_test.rb` | Partial: exact webhook work fields/timestamps/tags/links, public/private PR fields, live and deleted snapshot/handoff samples. Poll payloads and full Rails file: WS11 with WS12 and live WS15g owner-access seam. |
| `test/models/channel_thread_agent_assignment_test.rb` | Deferred: WS11 with WS12 work-thread seam. |
| `test/models/fizzy/agent_card_action_test.rb` | Deferred: WS15e domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/github/agent_pull_request_action_test.rb` | Deferred: WS15g domain; WS11 owns its agent-facing caller/delivery integration. |
| `test/models/message/bot_webhook_fanout_test.rb` | Partial: shared legacy fanout, recipient ordering, note/thread/stream skips and hop/chain inference; Agent ledger now installed in Message create, including replies. Imported-message skip and full lifecycle matrix: WS11 with WS16. |
| `test/models/message_streaming_test.rb` | Deferred: WS11. |
| `test/models/user/bot_test.rb` | Partial: inherited digest/reset checks plus HTTP reply scope; remaining lifecycle/update cases WS11. |
| `test/models/webhook_agent_key_test.rb` | Partial: no exposed bot key in Agent payloads, legacy reply URL, exact message PR/Drive samples and owner-keyed repository-access decision injection. Live private-repository lookup still needs WS15g; full Rails file: WS11. |
| `test/models/webhook_test.rb` | Partial: signed/pinned/guarded transport and encryption retained; Agent jobs now handle response classification/backoff, sync-reply error suppression and every payload kind. Complete transport taxonomy/write behavior and lifecycle: WS11. |
| `test/services/bots/clear_plaintext_tokens_test.rb` | Partial: inherited healing/idempotence plus actual scheduler registration; direct heal/reset-race case still WS11. |
| `test/system/agent_approvals_test.rb` | Deferred: WS11. |
| `test/system/agent_streaming_test.rb` | Deferred: WS11. |
| `test/system/agent_work_assignment_test.rb` | Deferred: WS11. |
| `test/system/agents_test.rb` | Deferred: WS11. |


## Cross-workstream seams and exact restart point

1. **Finish delivery integration/bot gaps:** install approval/work/handoff/slash/GitHub/Fizzy event-producing callbacks through their domains; WS15g live owner repository access (production payload caller currently denies private detail by default); WS16 imported-message skip; exact transport/write/error behavior; full posting validation JSON, every attachment/update/coercion edge and reaction presentation/broadcasts. WS3 queue is reused without a storage redesign. WS8a message chain is installed; WS8b-r still owns its rendering domain.
2. **Finish the Agent domain:** full Agent CRUD/save validation, grant/credential update/destroy/admin lifecycle; full EventPolling payload/cache/cursor behavior (especially pages whose selected payloads all drop); approvals including admin-only GitHub/Fizzy decisions; steps; slash registration/invocation through WS8 dispatcher; DMs/context; board/external budget production callers and edits/notices; WS4 per-credential counters; working presence TTL/status/broadcasts; suspension/kill switch, quiet streaming/trailing broadcasts, hard user removal, room destruction and reference/work finalization/recovery. Typed auth identity exists but still needs Agent/credential request-state installation in the future endpoints.
3. **REST:** every Agent endpoint and exact 401/403/404/422/429, headers/Retry-After and thin shared domain calls. Current selection/ack are foundations, not installed endpoints. WS12 board/work seams and WS15g/WS15e action seams must be called, not duplicated.
4. **MCP:** entire stateless Streamable HTTP server, four versions, mirrored/base64 headers, Origin/error codes, per-credential rate limits and all 38 shared-domain tools; every tool/error Rails vector. MCP security failure-first work remains undone.
5. **HTML:** admin bot/Agent creation/edit/credentials/grants/secrets/connections/kill switch, directory, approvals and ledger pages; exhaustive byte comparison and presence/directory broadcasts beyond WS7. Existing key-reset coverage and passing `manages_bots` do not certify these pages. WS9 sudo UI/full User-save validators and WS6/WS7 render/cable seams remain.
6. **Reference/network note:** general unfurl/push guard remains WS5's inherited Surfguard-policy seam; this branch's webhook policy follows the actual pinned `910be917fd0ab782c5ea939698d44f26694a501d`. Merged #159 rubyzip lock drift is permitted only as the exact one-entry checkout difference; all 36 image sources remain pinned.
7. **Acceptance:** complete each deferred/partial Rails file with its named owner and rerun scoped failure-first/differential/full seeded checks. This report is a coherent pushed partial checkpoint, not a cutover claim.
