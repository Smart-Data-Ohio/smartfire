# WS11 — partial domain parity continuation

Implementation: `80d72a1bc77ba8178b0dc610e20bd6794f06435a`, pushed to `rust/ws11-agents`. This continues the accepted `bee97633` baseline. **Partial:** production peer adapters remain uninstalled and 197 named comparisons remain deferred. No main merge, Rails/schema change, allowlist change, threshold change, REST/MCP change or server-page implementation was made.

## Pushed slices and changes

- `f7a861b4`: 18 named `MessageStreamingTest` comparisons. `crates/db/src/models/message.rs` now rejects the persisted finalized-to-streaming transition before content/activity updates, including attachment and body save paths. `models/agent_streaming.rs` uses the partial activity index, then sorts selected IDs to preserve ascending finalization order. `tests/agent_streaming_cases_test.rs` checks 14 domain cases; `campfire/src/integrations/agent_streaming/case_tests.rs` checks four cases through the production periodic sweep. The actual Rails resume validation is recorded in `vectors/agents_stream_resume_contract.json`.
- `8cd8f30b`: 19 named `AgentApprovalTest` comparisons in `db/src/tests/agent_approval_cases_test.rs`: validation, defaults, uniqueness, expiry, stale decisions, inbox recipients/handling and enqueue visibility through an outer transaction. The preference-neighbour case also requires a real ordinary-message mention inbox item and remains unmapped until WS12's producer is installed.
- `6a5cdaee`: all 17 named `Agent::EventWebhookJobTest` comparisons in `campfire/src/integrations/agent_jobs/case_tests.rs`. These call the unchanged production posting path with controlled DNS and TCP routing, real HTTP sockets/parsing, ledger claims and durable queue writes. Includes the real seven-second connection timeout, retry deadlines, five-attempt exhaustion, permanent errors, attachment error responses, deletion/configuration changes, and acknowledgment before delivery.
- `80d72a1b`: nine named `User::BotTest` digest/authentication and plaintext-retirement comparisons. Eight live in `db/src/tests/agent_bot_cases_test.rs`; the ninth calls the production periodic scrub from `agent_jobs/case_tests.rs`.

`reference-tools/agents/case-ports.json` maps exact pinned Rails names to executed Rust tests. The inventory/pass-count scripts now combine one Rails file's cases across multiple Rust files without losing or double-counting groups. `deferred-domain-cases.json` records every remaining name and its owner. The new mutation runners restore original bytes in `finally` blocks. Production service signatures used by WS11-ui/WS11-api were retained.

## Peer installation and remaining work

Remote main was checked after fetching: it reached `eaba80d5`, and `controllers/messages/payload.rs` is still absent there. The helper is present on `origin/rust/ws8bm-messages-http` at `3b639782`. A clarification about selectively importing that unmerged peer helper was sent; no answer arrived during this work. The common brief reserves in-flight peer areas. Main remains unmerged as requested, and the request-specific `MessagePayload` adapter remains explicitly uninstalled.

The transactional callback registry still requires the real activity, external-reference, huddle, Google/calendar, GitHub/Fizzy and Slack adapters from their owning peers. Full ordinary-message/removal/finalization integration cannot be declared complete before installation. WS15g's live `RepositoryReader` remains a flagged fail-closed stub; GitHub access was not implemented here. WS11-ui owns all server pages and WS11-api owns REST/MCP/authentication surfaces.

There are 181 executed named comparisons and 197 remaining names across the 26 pinned source files. Larger source files appear first below. These are Rails comparison counts, separate from Cargo's executed Rust test counts. Existing consolidated contracts do not close unmapped cases. The exact 197 names, grouped by source file and owner, are committed in [deferred-domain-cases.json](/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11/rust/reference-tools/agents/deferred-domain-cases.json).

| Pinned Rails file | Passing named comparisons | Deferred |
| --- | ---: | ---: |
| `test/models/agent_test.rb` | 39 | 2 |
| `test/services/slash_commands/dispatcher_test.rb` | 5 | 35 |
| `test/jobs/agent/delivery_job_test.rb` | 22 | 7 |
| `test/models/channel_thread_agent_assignment_test.rb` | 0 | 29 |
| `test/lib/restricted_http/private_network_guard_test.rb` | 28 | 0 |
| `test/models/message_streaming_test.rb` | 18 | 5 |
| `test/models/webhook_test.rb` | 0 | 22 |
| `test/models/agent_approval_test.rb` | 19 | 1 |
| `test/jobs/agent/event_webhook_job_test.rb` | 17 | 0 |
| `test/models/user/bot_test.rb` | 9 | 6 |
| `test/models/agent/delivery_recovery_test.rb` | 0 | 13 |
| `test/models/agent_grant_test.rb` | 12 | 0 |
| `test/models/agent_budgets_test.rb` | 0 | 11 |
| `test/models/agent_credential_test.rb` | 11 | 0 |
| `test/models/agent_event_test.rb` | 0 | 10 |
| `test/models/agent_step_test.rb` | 0 | 10 |
| `test/models/webhook_agent_key_test.rb` | 0 | 9 |
| `test/models/agent_kill_switch_test.rb` | 0 | 8 |
| `test/models/agent_revocation_test.rb` | 0 | 8 |
| `test/models/agent_slash_command_test.rb` | 0 | 6 |
| `test/models/agent_working_presence_test.rb` | 0 | 6 |
| `test/models/agents/work_payload_test.rb` | 0 | 3 |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 0 | 3 |
| `test/models/message/bot_webhook_fanout_test.rb` | 0 | 2 |
| `test/jobs/agent/delivery_concurrency_test.rb` | 0 | 1 |
| `test/models/agent_backfill_test.rb` | 1 | 0 |

## Discriminating failures

The first streaming run had this raw result:

```text
test result: FAILED. 11 passed; 3 failed; 0 ignored; 0 measured; 613 filtered out; finished in 1.05s
```

The finalized-resume test accepted an illicit content update; the plan test showed `SCAN messages`. Both defects were fixed. The third failure was a test setup error: suspension and a new stream had been put in the same transaction, so the suspension's after-commit sweep finalized that stream first. Setup now commits suspension before creating the stream, matching the Rails case. The first app quiet-sweep test also counted unrelated background jobs; its final version uses a message-specific rejecting queue trigger so immediately drained jobs cannot hide a forbidden side effect.

Each following command was executed with the committed mutation runner and restored the implementation. Raw summary lines:

```sh
python3 rust/reference-tools/agents/check-stream-case-mutations.py
python3 rust/reference-tools/agents/check-approval-case-mutations.py
python3 rust/reference-tools/agents/check-event-webhook-case-mutations.py
python3 rust/reference-tools/agents/check-bot-case-mutations.py
```

```text
WS11 streaming mutation: resume; 1 test failed; original restored
WS11 streaming mutation: throttle; 1 test failed; original restored
WS11 streaming mutation: sweep-plan; 1 test failed; original restored
WS11 approval mutation: expired or decided request accepted; 1 test failed; original restored
WS11 event webhook mutation: private-url; 1 test failed; original restored
WS11 event webhook mutation: permanent-status; 1 test failed; original restored
WS11 event webhook mutation: attempt-limit; 1 test failed; original restored
WS11 bot mutation: wrong digest accepted; 1 test failed; original restored
stream-case-mutation-resume.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 645 filtered out; finished in 0.08s
stream-case-mutation-throttle.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 645 filtered out; finished in 0.09s
stream-case-mutation-sweep-plan.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 645 filtered out; finished in 0.70s
approval-case-mutation.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 645 filtered out; finished in 0.34s
event-webhook-case-mutation-private-url.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 493 filtered out; finished in 0.43s
event-webhook-case-mutation-permanent-status.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 493 filtered out; finished in 0.53s
event-webhook-case-mutation-attempt-limit.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 493 filtered out; finished in 0.45s
bot-case-mutation.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 653 filtered out; finished in 0.34s
```

## Current verification

All commands below were executed for this report. Scratch logs are in the assigned worktree; the fresh clone uses its own target directory and independently built seed data. The debug-information setting reduces disk use and does not alter concurrency or timing assertions.

Fresh clone and all nine independent seeds (the clone was fast-forwarded to the final implementation before compiling):

```sh
git clone --single-branch --branch rust/ws11-agents https://github.com/Smart-Data-Ohio/smartfire.git .scratch/fresh-ws11-named
PARITY_NAMESPACE=ws11-fresh-named PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 .scratch/fresh-ws11-named/rust/parity/bin/seed build
```

Inside that fresh clone:

```sh
mkdir -p .scratch
git pull --ff-only
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml >/dev/null
python3 rust/reference-tools/agents/check-seeds.py
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52299 mise exec rust@1.98.1 -- cargo test --locked -j4 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
CI=1 TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only mise exec rust@1.98.1 -- cargo clippy --locked -j4 --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
```

Raw seed, app, database, full-workspace and clippy summaries:

```text
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
test result: ok. 493 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 275.43s
test result: ok. 650 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 148.87s
WS11 workspace totals: 1763 passed; 0 failed; 10 ignored; 46 result summaries
WS11 missing-seed skips: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 33s
```

The 10 existing ignores were not counted as executed tests: app 2, cable reference recorder 1, database external-oracle/export cases 4, SMTP reference recorder 1 and kit doctests 2. No WS11 test was ignored. The missing-seed support unit test intentionally prints one local skip notice; actual seeded app tests ran under CI with all seeds present. No timing test failed in this run.

At the assigned worktree root, the source inventory, runtime oracles and actual-execution count commands were:

```sh
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/verify-contracts.py
python3 rust/reference-tools/agents/verify-callback-contracts.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
python3 rust/reference-tools/agents/summarize-tests.py .scratch/named-fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/named-fresh-workspace.log
```

Raw source, oracle and case summaries:

```text
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 20 byte-identical contract files; 23 contracts recorded in total
WS11 callback Rails oracles: 3 byte-identical vectors; 28 guard case names and address inputs matched the pin
WS11 named ports: test/models/agent_credential_test.rb: 11 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_grant_test.rb: 12 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_backfill_test.rb: 1 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_test.rb: 39 mapped cases; 2 unmapped case names
WS11 named ports: test/services/slash_commands/dispatcher_test.rb: 5 mapped cases; 35 unmapped case names
WS11 named ports: test/jobs/agent/delivery_job_test.rb: 22 mapped cases; 7 unmapped case names
WS11 named ports: test/lib/restricted_http/private_network_guard_test.rb: 28 mapped cases; 0 unmapped case names
WS11 named ports: test/models/message_streaming_test.rb: 18 mapped cases; 5 unmapped case names
WS11 named ports: test/models/agent_approval_test.rb: 19 mapped cases; 1 unmapped case names
WS11 named ports: test/jobs/agent/event_webhook_job_test.rb: 17 mapped cases; 0 unmapped case names
WS11 named ports: test/models/user/bot_test.rb: 9 mapped cases; 6 unmapped case names
WS11 source case files: 11 pinned Git files matched; 0 checkout mismatches
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 21 pinned files; 197 named source cases; owners recorded per file
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 22 passed; 0 failed; 7 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 18 passed; 0 failed; 5 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 19 passed; 0 failed; 1 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 9 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 181 passed; 0 failed; 197 deferred
WS11 locked workspace metadata: 76 dependency keys; 0 duplicate keys
WS11 manages_bots: enabled; ran and passed in the fresh clone
```

Raw logs: `.scratch/named-fresh-workspace.log`, `.scratch/named-fresh-clippy.log`, `.scratch/named-fresh-seed.log`, `.scratch/named-fresh-pass-counts.log`, and the mutation logs named above. Normal implementations were restored and all production changes were tested from the independently built clone. This final report commit changes documentation only.

Resume with the peer adapter installation once the helper's source/import ownership is confirmed, then the exact deferred names in source-size order. In the maintained inventory, the largest remaining WS11 comparison file is `test/models/webhook_test.rb` (22); the larger unclosed assignment/dispatcher groups include WS12/WS8 producer cases and remain assigned to those peers. The independent review findings for `bee97633` have not yet been relayed in this turn.
