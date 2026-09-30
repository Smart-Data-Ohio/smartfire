# WS11 agent domain continuation — partial

Implementation checkpoint: `aea968ed301b60cb65909a9eb17e18855a816f6b` (pushed). Reference: frozen Rails `d7c7de92`, image `triage-reference-d7c7de92`. No new main merge was requested this continuation; the earlier merge of main `4278cb1e` remains in the branch. The final report/verification-helper commit changes no Rust implementation.

Four coherent slices were committed and pushed:

| Commit | Changes by file |
|---|---|
| `73beb1fe` | DB `callbacks.rs`, `events.rs`, `database.rs`: typed installable peer callbacks, dispatched by production `campfire::Jobs::model_callbacks`. Message create/edit/finalize invoke activity and GitHub/Fizzy/Twitter/event/link-reference adapters; user/session removal invoke declared dependency adapters. `controllers/presenters/agent_payload.rs`, `presenters.rs`, `messages.rs`, `app.rs`: request-specific payload adapter with the stable `Presenter::agent_message_payload(&Message)` signature, `current_user_id`, and request base URL. Model and production-sink tests exercise invocation/order/rollback. |
| `d41b5399` | `db/models/message.rs`: claim finalization before looking up active agent state; preserve the claim/indexing and earlier effects after an ordinary callback exception; return the exception after commit. Durable queue persistence failure still rolls the whole originating write back. Added Rails failure oracle/vector and production queue rejection test. Hard-removal test confirms an uninstalled peer FK and rescued error preserve rows and discard broadcasts. |
| `fd0aa0d8` | 39 named Agent comparisons, five agent dispatcher comparisons, all 28 private-network guard comparisons. `agent_access.rs` adds serialized `reset_webhook_signing_secret`; ensure delegates to it when generation is required. Reset rejects an unprotected after-commit write. Guard inputs and vectors come from the Rails runtime and pinned test file. Mapping checker supports honest partial files; execution-based counters only count tests actually run. Repository reader's uninstalled branch is explicitly flagged WS15g. |
| `aea968ed` | 22 named delivery-domain comparisons: real message chain, deliverable/suppressed outcomes, rollback, deletion, rate limits, reply/DM selection and hop chains across rooms. Seven precise HTTP/race/lock/work-producer comparisons remain deferred. Added compiled revocation and hop-limit probes. |

The earlier accepted bot, encrypted webhook, signing/network guard, ledger/retry/recovery, agent services and cleanup implementations remain present. The fresh suite and 23 existing Rails runtime contracts below revalidate them.

## Production seams and exact remaining integration

- **WS8b-m presenter:** install `AppState.agent_message_payload` with an implementation of `MessagePayload::message(presenter, message, viewer, base_url)` that calls the peer's shared `controllers::messages::payload::message`. This branch contains the production call seam, not a copied peer payload implementation. An uninstalled adapter returns an explicit error. It requires Current.user and the request URL and bypasses the cached stock JSON. The two dead-code allowances are flagged solely for the unmerged WS8b-m/WS11-api callers; remove them when those callers land.
- **WS11-api:** `Presenter::agent_message_payload`, `current_user_id`, `cache_base_url`, and all existing domain service signatures stay available. REST/MCP, by-bot HTTP, authentication and throttle remain the peer's scope. No routes were implemented here.
- **WS11-ui:** all admin/directory/approvals/events HTML and human approvals controller stay with that peer. Agent model tests do not count the two rendered-broadcast cases as complete. Credential absent-HTTP-input conversion remains the API boundary noted in `case-ports.json`.
- **WS12/14/15 message adapters:** install `Jobs.model_callbacks` for MessageActivity and the five external reference phases. Model adapters receive the writer transaction and persisted record id. They must perform no network I/O in the writer. Message-reference sync runs between event and link phases, matching Rails' declaration order. Both ordinary create and finalization use these calls; source edits resync references. Work owner/handoff and approved-action producers still call the previously exposed domain functions when their branches merge. WS16 import suppression/fetch context is still its flagged integration responsibility.
- **WS13/14/15/16 user dependencies:** install the huddle prepend, Google account, Calendar meeting cache, Google identity, GitHub account, Fizzy account, Slack connection/imports, event calendar entries, and session huddle phases. Default `User::destroy` now calls the production dispatcher. `destroy_with_dependencies` keeps its previous signature for callers providing explicit adapters. The registry's uninstalled phases are flagged stubs; connected-account FK failures retain the entire model operation. Remote preparation belongs outside the writer. Therefore connected-user removal and full activity/reference finalization are **not declared globally complete** until those peers install their implementations.
- **WS15g live private-PR reader:** existing `RepositoryReader::readable(RepositoryRequest)` and `AppState.agent_repositories.resolve_threads` remain the seam. No GitHub access was implemented. The explicitly flagged uninstalled reader grants no private/unknown repository access. The installed-reader webhook path still resolves owner access outside the writer and rechecks ownership/disconnection before revealing fields; polling/context callers can use the same resolver.

Core removal ordering, dependency-delete versus destroy semantics, rescued rollback, quiet/normal finalization, once-only claim, trailing job staleness and the callback-error retention behavior are tested. The durable queue rejection exception to Rails' partial callback persistence follows the fixed queue-atomicity decision; it is tested against the real production sink.

## Fresh-clone verification

Created a new independent clone at `.scratch/fresh-ws11-callbacks`, with its own Cargo target and scratch, and built all nine seeds there. No seed/target/source was copied into this clone. Its initial dependency/test build preceded the delivery slice; the clone was then fetched/fast-forwarded to `aea968ed301b60cb65909a9eb17e18855a816f6b` and the complete suite below ran at that checkpoint. The only cleanup removed the previous verification clone's regenerable `.scratch/fresh-ws11-ws9/rust/target` after confirming no process used it; old source, seeds and logs remain.

Setup commands (original worktree; fetch/fast-forward in the new clone):

```sh
git clone --single-branch --branch rust/ws11-agents https://github.com/Smart-Data-Ohio/smartfire.git .scratch/fresh-ws11-callbacks
PARITY_NAMESPACE=ws11-fresh-callbacks PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 .scratch/fresh-ws11-callbacks/rust/parity/bin/seed build
```
```sh
git fetch origin rust/ws11-agents
git merge --ff-only FETCH_HEAD
```

The fresh clone ran locked cargo metadata and parsed Cargo.toml with Python tomllib (duplicate keys are rejected), and checked the annotation immediately before `manages_bots`:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml > /dev/null
python3 rust/reference-tools/agents/check-seeds.py
python3 rust/reference-tools/agents/check-case-ports.py
```
```text
WS11 workspace dependencies: 76 keys; 0 duplicate keys
WS11 manages_bots: enabled; 0 WS11 ignores
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 named ports: test/models/agent_credential_test.rb: 11 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_grant_test.rb: 12 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_backfill_test.rb: 1 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_test.rb: 39 mapped cases; 2 unmapped case names
WS11 named ports: test/services/slash_commands/dispatcher_test.rb: 5 mapped cases; 35 unmapped case names
WS11 named ports: test/jobs/agent/delivery_job_test.rb: 22 mapped cases; 7 unmapped case names
WS11 named ports: test/lib/restricted_http/private_network_guard_test.rb: 28 mapped cases; 0 unmapped case names
WS11 source case files: 7 pinned Git files matched; 0 checkout mismatches
```

Full suite and strict clippy commands (cwd fresh clone):

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 WS11_SECRET_EXPORT="$PWD/.scratch/ws11-rust-secrets.json" mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever -- --test-threads=4 --nocapture > ../callbacks-fresh-workspace.log 2>&1
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings > ../callbacks-fresh-clippy.log 2>&1
```

Raw result lines, in execution order (first is the seeded app; eighth is DB). `manages_bots` ran:

```text
test controllers::presenters::accounts::tests::manages_bots ... ok
test result: ok. 471 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 116.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.61s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 609 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 68.40s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.42s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.70s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.17s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.72s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.49s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.94s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.06s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 14s
```

Execution summary commands (original worktree):

```sh
python3 rust/reference-tools/agents/summarize-tests.py .scratch/callbacks-fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/callbacks-fresh-workspace.log
python3 rust/reference-tools/agents/write-case-status.py
```
```text
WS11 workspace totals: 1700 passed; 0 failed; 10 ignored; 46 result summaries
WS11 missing-seed skips: 0
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 22 passed; 0 failed; 7 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 118 passed; 0 failed; 260 deferred
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 22 pinned files; 260 named source cases; owners recorded per file
```

No timing test flaked in this run. No concurrency or timing threshold was changed. The ten inherited ignores are app 2, cable 1, DB 4, mail 1 and kit doctests 2; none was added by these slices.

## Runtime oracles and fail-first evidence

Commands run again on the final implementation (original worktree):

```sh
python3 rust/reference-tools/agents/verify-contracts.py
python3 rust/reference-tools/agents/verify-callback-contracts.py
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/check-peer-callback-mutations.py
python3 rust/reference-tools/agents/check-named-case-mutations.py
python3 rust/reference-tools/agents/check-delivery-case-mutations.py
git diff --exit-code -- rust/crates
```
```text
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 20 byte-identical contract files; 23 contracts recorded in total
WS11 callback Rails oracles: 2 byte-identical vectors; 28 guard case names and address inputs matched the pin
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 peer mutation: bypassed transactional adapter; 4 tests failed; original restored
WS11 named mutation: unknown-capability; 1 test failed; original restored
WS11 named mutation: nat64-local-use; 1 test failed; original restored
WS11 delivery case mutation: revocation; 1 test failed; original restored
WS11 delivery case mutation: hop-limit; 1 test failed; original restored
```

The mutations compile and produce actual test failures, then restore the exact source. The callback probe bypasses the transactional dispatcher (four failures); the named probes allow an unknown capability or remove the local-use NAT64 guard (one failure each); delivery probes invert revocation or relax the hop cap (one failure each). Subsequent normal tests and the fresh suite verify restored code. Mutation scripts were run before committing their slices and rerun on the final implementation; `git diff --exit-code -- rust/crates` after the fresh suite exits zero without output.

The new Rails finalization failure vector raised from the real model's activity callback. Before the fix, the Rust repeat-finalize test failed because rollback allowed the failing callback to run again:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 546 filtered out; finished in 0.34s
```

The fixed test passes in the fresh full suite. The vector confirms streaming=false, one indexed row, zero posted rows, unchanged working presence, and repeat=false. This is ordinary callback failure; a rejected durable enqueue still rolls all those writes back.

Rails also read two newly Rust-written encrypted model columns, from a private copy of the fresh clone's default seed and its fresh export. No secret values are printed or committed:

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/ws11-ar-reader-callbacks" -e WS11_RUST_SECRETS_PATH=/rails/storage/db/ws11-rust-secrets.json rust/reference-tools/agents/read_rust_webhook_secrets.rb
```
```text
WS11 AR interoperability: Rails read 2 Rust-written model columns; US-ASCII preserved; 0 secrets regenerated
```


The final mutation rerun was followed by a normal focused run in the original worktree:

```sh
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml -p campfire -p campfire_db ws11 -- --test-threads=4 > .scratch/callbacks-restored-focused.log 2>&1
```
```text
test result: ok. 75 passed; 0 failed; 0 ignored; 0 measured; 398 filtered out; finished in 12.24s
test result: ok. 164 passed; 0 failed; 0 ignored; 0 measured; 449 filtered out; finished in 12.17s
```

## Named Rails comparisons, largest source files first

Counts below are **Rust executions of named comparisons against the pinned Rails source**, not a claim that Rails' whole Minitest suite ran in the production image. The 94 new named comparisons plus 24 previously mapped comparisons all passed in the fresh suite. Unmapped names remain deferred even where a consolidated vector/test already exercises part of them. Exact mapping is `reference-tools/agents/case-ports.json`; exact 260 remaining names and owners are committed in `reference-tools/agents/deferred-domain-cases.json`.

| Pinned Rails file | Source cases | Named comparisons passed | Named comparisons deferred |
|---|---:|---:|---:|
| `test/models/agent_test.rb` | 41 | 39 | 2 |
| `test/services/slash_commands/dispatcher_test.rb` | 40 | 5 | 35 |
| `test/jobs/agent/delivery_job_test.rb` | 29 | 22 | 7 |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | 0 | 29 |
| `test/lib/restricted_http/private_network_guard_test.rb` | 28 | 28 | 0 |
| `test/models/message_streaming_test.rb` | 23 | 0 | 23 |
| `test/models/webhook_test.rb` | 22 | 0 | 22 |
| `test/models/agent_approval_test.rb` | 20 | 0 | 20 |
| `test/jobs/agent/event_webhook_job_test.rb` | 17 | 0 | 17 |
| `test/models/user/bot_test.rb` | 15 | 0 | 15 |
| `test/models/agent/delivery_recovery_test.rb` | 13 | 0 | 13 |
| `test/models/agent_grant_test.rb` | 12 | 12 | 0 |
| `test/models/agent_budgets_test.rb` | 11 | 0 | 11 |
| `test/models/agent_credential_test.rb` | 11 | 11 | 0 |
| `test/models/agent_event_test.rb` | 10 | 0 | 10 |
| `test/models/agent_step_test.rb` | 10 | 0 | 10 |
| `test/models/webhook_agent_key_test.rb` | 9 | 0 | 9 |
| `test/models/agent_kill_switch_test.rb` | 8 | 0 | 8 |
| `test/models/agent_revocation_test.rb` | 8 | 0 | 8 |
| `test/models/agent_slash_command_test.rb` | 6 | 0 | 6 |
| `test/models/agent_working_presence_test.rb` | 6 | 0 | 6 |
| `test/models/agents/work_payload_test.rb` | 3 | 0 | 3 |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | 0 | 3 |
| `test/models/message/bot_webhook_fanout_test.rb` | 2 | 0 | 2 |
| `test/jobs/agent/delivery_concurrency_test.rb` | 1 | 0 | 1 |
| `test/models/agent_backfill_test.rb` | 1 | 1 | 0 |

The Agent reset comparison checks the actual serialized writer transaction rather than a Ruby `with_lock` method spy. The credential absent-HTTP-input subassertion remains WS11-api. These boundaries are explicitly retained in the mapping; no HTML rendering is claimed by domain descriptors.

## Remaining, by file and owner

- `test/services/slash_commands/dispatcher_test.rb`: 35; WS11 agent dispatch; WS8 built-in commands.
- `test/models/channel_thread_agent_assignment_test.rb`: 29; WS11 agent callbacks; WS12 mutation producers.
- `test/models/message_streaming_test.rb`: 23; WS11 finalization; WS12 activity; WS14/15 external reference sync.
- `test/models/webhook_test.rb`: 22; WS11 domain.
- `test/models/agent_approval_test.rb`: 20; WS11 domain; WS11-ui human approval controller/pages.
- `test/jobs/agent/event_webhook_job_test.rb`: 17; WS11 delivery; WS15g live repository reader.
- `test/models/user/bot_test.rb`: 15; WS11 bot domain/removal; WS11-api by-bot HTTP surface.
- `test/models/agent/delivery_recovery_test.rb`: 13; WS11 domain.
- `test/models/agent_budgets_test.rb`: 11; WS11 domain.
- `test/models/agent_event_test.rb`: 10; WS11 domain.
- `test/models/agent_step_test.rb`: 10; WS11 domain.
- `test/models/webhook_agent_key_test.rb`: 9; WS11; WS8 presenter; WS15g live private repository access.
- `test/models/agent_kill_switch_test.rb`: 8; WS11; WS15g/WS15e approved-action execution callbacks.
- `test/models/agent_revocation_test.rb`: 8; WS11 domain.
- `test/jobs/agent/delivery_job_test.rb`: 7; WS11 domain.
- `test/models/agent_slash_command_test.rb`: 6; WS11 domain.
- `test/models/agent_working_presence_test.rb`: 6; WS11 domain.
- `test/models/agents/work_payload_test.rb`: 3; WS11; WS8 message presenter; WS15g private repository reader.
- `test/services/bots/clear_plaintext_tokens_test.rb`: 3; WS11 domain.
- `test/models/agent_test.rb`: 2; WS11-ui rendered broadcast cases; WS11 domain cases compared.
- `test/models/message/bot_webhook_fanout_test.rb`: 2; WS11; WS16 import suppression integration.
- `test/jobs/agent/delivery_concurrency_test.rb`: 1; WS11 domain.

All 260 exact names appear in `reference-tools/agents/deferred-domain-cases.json`. Resume with WS12's 29-case assignment producer integration, then the remaining delivery probes and 23 streaming cases before smaller files, while the WS8 built-in dispatcher and WS11-ui rendered cases remain their owners' work. Live private repository reads remain WS15g, not unfinished GitHub implementation work assigned to this worker. No schema/migration, Rails, HTML, REST or MCP implementation was changed here.
