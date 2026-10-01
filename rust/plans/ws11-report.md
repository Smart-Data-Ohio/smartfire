# WS11 — PR #176 review fixes, 2026-10-01

All four findings from the review of `356fc9e3` are fixed. The independently verified code is `6542ed379ef3c778ebcde2545badf6876e364e63` on `rust/ws11-agents`; the final report commit changes documentation only. Source fixes were pushed as `7d12a934` and `6542ed37`. The wider domain remains **partial: 297 named comparisons passed, exactly 81 deferred** out of 378 names in 26 pinned Rails files. Nine additional review/boundary regressions are not counted as closures of those deferred names.

The reviewer evidence under `/home/riels/.cache/rust-port/ws11r/review-356fc9e3/` was read and left unchanged. Our own probes execute Rails **d7c7de92**, call the real Rails callbacks/HTTP client, and inject SQLite failures. No Rails code, parity masks, timing thresholds or concurrency limits were weakened. No pixel work was performed.

## Fixes and files

| Area | Change |
| --- | --- |
| `campfire/src/integrations/agent_repositories.rs`, `agent_jobs.rs` | Walk work links in link-ID order and polling occurrences in event order. Use WS15g's real shared Accounts service/cache, including its transient retry policy; remove the second batch cache. Resolve only the work or message fields that the webhook serializes. Stop private reads after a disconnect. |
| `db/src/models/agent_payloads.rs`, `agent_event_polling.rs` | Bind live decisions to each link/thread and event occurrence. Seal the completed batch against the owner/account identity and encrypted credential/version state; validate on the serialization connection and recheck complete payloads. A changed snapshot rebuilds work/message/poll payloads with private details redacted while preserving public details. Repeated links/events cannot inherit an allowance from an earlier occurrence. |
| `db/src/models/channel_thread.rs`, `agent_work_events.rs`, `agent_delivery.rs` | Insert deletion ledger rows after the outer deletion commits, in their own write. Reserve the future event ID and persist any deliverable webhook job with deletion, preserving the required enqueue-rejection rollback. A failed after-commit ledger insert leaves deletion committed; its reserved job safely returns without HTTP if no event exists. Preserve the captured belongs_to Agent identity even if that row is deleted in the same outer transaction. Ordinary event creation still validates that an agent exists. |
| `db/src/models/message.rs`, `events.rs`, `campfire/src/jobs.rs`, `integrations.rs` | Run the actual GitHub, Fizzy, Twitter, event adapter phase, quote and link callbacks at their Rails positions. The former combined sink ran link/Fizzy/Twitter after quote synchronization; phase dispatch now places the real adapters correctly. Keep the existing combined/import entry points, with import fetch policy unchanged. |
| `db/src/models/agent_approval.rs` | Query the active human owner first, then active administrators, with the owner deduplicated. Existing after-commit per-recipient writes preserve the owner's item if the next insert fails. |
| Tests, `review_fixes_contract.rb`, vectors and verifiers | Commit the failing-first record, 21 ordered/mixed/repeated-repository vectors, repeated-thread polling, both deletion modes, captured-agent removal, actual adapter order and partial approval fanout. |

### Rails behavior that must be preserved

Rails serializes links incrementally. If an allowed private link precedes a later 401, its details remain in that response; private links at and after the failed read are redacted. Reversed ordering therefore has a different result. Our 21 vectors establish this directly rather than assuming every disconnect redacts all earlier fields. The reported `z`-then-`a` case redacts both, and public links remain visible at every position.

Rails also keeps a deletion ledger row with a captured Agent association after the same outer transaction deletes that Agent. The table has no Agent foreign key. The boundary audit caught an initial skip in `7d12a934`; the follow-up reproduces and preserves this oddity. It does not relax normal `AgentEvent.create` validation.

The durable queue decision requires atomic enqueues even where Rails uses after-commit Redis enqueue. Event-ID reservation preserves that requirement without moving the ledger insert into deletion's transaction. If the ledger callback fails, Rust may retain an internal reserved job; it is proven to be a harmless no-op. Queue-insert failure still rolls back thread deletion and leaves no ledger row.

The older single-repository probe captured HTTP paths before building WorkPayload. It therefore omitted a transient retry performed for the second private/unknown link. The probe now observes the complete Rails WorkPayload path; only the generated 500-path vector gains a request. Rust was corrected to defer caching to Accounts rather than caching a transient denial itself. Expected values were regenerated from Rails, not edited by hand.

## Failing-first evidence

On unchanged production `356fc9e3`, after adding only tests and the independent oracle:

| Finding | Rust before fix | Pinned Rails and corrected Rust |
| --- | --- | --- |
| Private disclosure | `z/a/m` input checks `a/m/z`; private a/m titles and branches escape after z's 401. Seven of the first 18 matrix cases disagree. | Requests and fields follow link order; the first failed private read redacts all subsequent private occurrences. Final coverage: 21 cases, every private failure position, public links in each position, reversed and repeated repositories. |
| Deletion | Rejected ledger insert leaves `thread_exists=true`. | `thread_exists=false`, `events=0`; demonstrated with and without a configured webhook. |
| Streaming | Invalid GitHub pull/0 leaves one quote reference after finalizing. | Finalized claim survives, zero quotes; GitHub validation runs before quote synchronization. |
| Approval | With higher-ID owner and second insertion rejected, administrator 127326141 gets the sole item. | Owner 712064548 gets the sole item; pending approval persists. |

Exact original commands (run before production edits):

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever ws11_review_
rust/reference-tools/agents/run-focused.sh -p campfire_db ws11_review_
```


Raw evidence, including the additional failing boundary check on `7d12a934`:

```text
Production baseline: 356fc9e3f913517a079ebd50fd45f93980c333bf
New tests and independently generated Rails oracle added before production edits.
Pinned image: triage-reference-d7c7de92

.scratch/review-failing-first.log
REVIEW stream reference error: streaming=false quote_references=1
    integrations::agent_repositories::review_tests::ws11_review_repository_batch_order_disconnect_and_public_fields_match_rails
    integrations::agent_repositories::review_tests::ws11_review_stream_github_failure_precedes_quote_sync
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1046 filtered out; finished in 11.26s

.scratch/review-db-failing-first.log
REVIEW deletion-event failure: thread_exists=true
REVIEW approval-recipient failure: owner=712064548 notified_users=[127326141]
    tests::agent_approval_cases_test::ws11_review_approval_partial_fanout_reaches_owner_first
    tests::agent_work_events_test::ws11_review_deleted_ledger_failure_keeps_deletion
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 888 filtered out; finished in 0.15s

Repository regression: Rails z/a/m disconnect-at-first redacted all private fields. Baseline posted Private a/Private m and their private head/base branches; paths were /repos/a/private,/repos/m/private,/repos/z/private. The regression ran all 18 initial matrix cases and reported seven order-dependent mismatches.

Additional boundary audit, baseline 7d12a9346a0415f4a2648f6345da5a7fc3cbecc7:
WS11 captured-agent deletion event: {"error":null,"thread_exists":false,"agent_exists":false,"events":0}
Pinned Rails preserves events=1 when the same outer transaction deletes the Agent after destroying its thread.
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 890 filtered out; finished in 0.11s
```

Final focused command and raw summaries:

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_review_
```

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1046 filtered out; finished in 15.15s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 888 filtered out; finished in 0.14s
```

An existing repository-access helper check still expected the obsolete batch deduplication count after removing ownership. That assertion now verifies two occurrence checks and zero further calls after removal; the real Accounts HTTP cache remains pinned. The dedicated existing-adapter run passed:

```sh
rust/reference-tools/agents/run-focused.sh -p campfire ws11_repository_
```

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1049 filtered out; finished in 1.99s
```

## Reader audit and owner seams

- WorkPayload and board payloads share the guarded work-link serializer; neither has a separate private-title cache.
- Thread/message PR metadata uses thread-occurrence decisions and the same snapshot validation.
- Polling has event-scoped decisions. The pinned repeated-thread case allows `a`, disconnects on `z`, then redacts the repeated `a` without losing cursor advancement. The full returned batch is revalidated before serialization.
- Webhooks use the production Accounts adapter and payload-specific resolver before entering the writer; the serialized payload and secret write hold the writer transaction. No database lease crosses external GitHub reads.
- ContextBuilder adds no independent private-PR payload cache; its messages use WS8's presenter. Existing WS15g single-PR readers resolve each call through Accounts; no additional accumulated agent allowance set was found.
- Uninstalled/test states continue to deny private access. Explicit manually supplied sets remain a trusted compatibility input, not a production live resolver.

**WS11-api integration seam:** use `resolve_work`, `resolve_messages`, or `resolve_events` for the matching surface; polling supplies its filtered readable page IDs. `resolve_threads` remains implemented for source compatibility. `RepositoryAccess` is now a guarded wrapper; Default, set operations, array/HashSet From and FromIterator remain available. Lead reconciliation should route REST/MCP callers to the payload-specific/event-scoped seam rather than assembling unguarded grants. REST/auth/MCP and rendered pages were not implemented here.

WS14 event-reference execution and WS13/WS14/WS16 dependency adapters remain flagged with their owners. The stream-order test uses SQL triggers on all five installed reference domains plus a registered event-phase recorder; it does not claim WS14's event domain is implemented.

## Reproduced verification

Executed on this continuation:

```sh
python3 rust/reference-tools/agents/verify-review-fixes.py
python3 rust/reference-tools/agents/verify-security-domain-cases.py
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
rust/reference-tools/agents/run-fresh.sh
python3 rust/reference-tools/agents/summarize-tests.py .scratch/fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-workspace.log
```

```text
WS11 review Rails probes: 21 repository batches, repeated-thread polling, deletion with/without webhook and captured-agent removal, real reference order and partial fanout regenerated byte-identical (d7c7de92)
WS11 security/domain Rails oracles: 7 pinned lifecycle, repository, stream, presence/registration, plaintext, work-payload and HTTP after-commit contracts regenerated byte-identical
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 8 pinned files; 81 named source cases; owners recorded per file
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 workspace dependencies: 77 unique keys; 0 duplicates (strict TOML parse)
```

`origin/main` was fetched and remains `65ad0d39`, already an ancestor of this branch; there was no new merge or conflict. Fresh locked metadata succeeded. The first fresh run of `7d12a934` was intentionally stopped for the additional captured-agent fix, and its target was removed; it is not reported as a completed passing suite.

The final independent remote clone `.scratch/fresh-ws11-review-176-ledger` checked out `6542ed379ef3c778ebcde2545badf6876e364e63` and validated all nine labeled seeds. It ran:

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

CARGO_BUILD_JOBS=2, machine compiler throttle unchanged. The media runner used the pinned Rails media runtime. `manages_bots` ran and passed. Raw fresh summaries (ignored tests are not counted as executed):

```text
WS11 fresh source: 6542ed379ef3c778ebcde2545badf6876e364e63
WS11 workspace totals: 2616 passed; 0 failed; 11 ignored; 48 result summaries
WS11 missing-seed skips: 0
test result: ok. 1050 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 517.97s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.82s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 887 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 91.91s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.40s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.40s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.65s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.80s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 45s
WS11 fresh exits: test=0 clippy=0
```

Both fresh scratch target directories were deleted after their runs. Logs and source/seed snapshots remain; no WS11 test/compiler process is left running.

## Named Rails comparisons and precisely remaining

Largest files first, from the completed fresh run:

```text
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 27 passed; 0 failed; 2 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 0 passed; 0 failed; 29 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 19 passed; 0 failed; 4 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 9 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 9 passed; 0 failed; 2 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 9 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 7 passed; 0 failed; 1 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 297 passed; 0 failed; 81 deferred
```

Exactly **81** original named comparisons remain, listed individually in `rust/reference-tools/agents/deferred-domain-cases.json`:

- `test/models/agent_test.rb`: 2; WS11-ui rendered broadcast cases; WS11 domain cases compared.
- `test/services/slash_commands/dispatcher_test.rb`: 35; WS11 agent dispatch; WS8 built-in commands.
- `test/jobs/agent/delivery_job_test.rb`: 2; WS11 domain.
- `test/models/channel_thread_agent_assignment_test.rb`: 29; WS11 agent callbacks; WS12 mutation producers.
- `test/models/message_streaming_test.rb`: 4; WS11 finalization; WS12 activity; WS14/15 external reference sync.
- `test/models/user/bot_test.rb`: 6; WS11 bot domain/removal; WS11-api by-bot HTTP surface.
- `test/models/agent_budgets_test.rb`: 2; WS11 domain.
- `test/models/agent_kill_switch_test.rb`: 1; WS11 callbacks; WS12 owned-board mutation producer.

No review finding remains open in this branch. Domain completion, the listed peer adapters and separate UI/API owner integration remain partial; UI and API implementation stays with the assigned owners.
