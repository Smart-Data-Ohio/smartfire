WS11 PR #176 publication-order fix — verified, wider domain partial

The new P2 from the re-review of `b795aa8e` is fixed. Verified code: `0402c96cc525a3d9a6498cbdbfcd11ffa3201f66` on `rust/ws11-agents`. The final report commit changes documentation only. `92a8eaa8` merges `origin/main` at `20dc8ea3` with a merge commit, including WS14e #174 (`ae43b34d`) and #178. Locked metadata succeeded after the merge and again in the fresh clone. `manages_bots` runs and passes.

The cursor regression was demonstrated against production `b795aa8e` before any production edits; `40a52fb4` commits those tests, probe and vector. `710d77d0` fixes publication and atomic enqueueing. `0402c96c` corrects broadcast parity discovered during merge verification. The reviewer evidence directory was read without modification. No stashes, widened timing thresholds, reduced test concurrency or pixel work were used. Every Cargo build used CARGO_BUILD_JOBS=2; every Cargo test run used four test threads.

| Files | Change |
|---|---|
| `db/src/models/agent_delivery.rs` | Remove the reservation helper and explicit-ID insertion path. All event IDs are allocated by SQLite at insertion. NewEvent is serializable for the durable captured-deletion envelope; existing domain signatures remain stable except the private captured-event helper. |
| `db/src/models/agent_work_events.rs`, `channel_thread.rs` | Persist a captured deletion webhook job in the deletion transaction. After commit, insert the ledger row and bind the existing job arguments to the new event ID in the same publication transaction. Recheck current webhook eligibility, retain the captured Agent association, and reuse an already-published chain on duplicate publication. |
| `campfire/src/integrations/agent_jobs.rs` | Accept both existing event-ID jobs and captured-deletion jobs under the same registered class. If a process stops after deletion commits, the durable runner publishes the missing event. A stale claimed envelope reloads its now-bound arguments so removing a published event later does not recreate it. The existing attempt/claim, signing, private-access and backoff paths remain shared. |
| `db/src/tests/agent_work_events_test.rs`, `campfire/src/integrations/agent_jobs/publication_tests.rs` | Same-parent-transaction cursor regression, separate-connection poll/commit/poll/publication regression, and a real SQLite snapshot of the committed deletion/queue before publication. Verify recovery, one event on duplicate replay, binding, and the stale-claim missing-event no-op. |
| `campfire/src/integrations/agent_repositories/review_tests.rs` | Keep pinned Rails deletion/failure assertions unchanged. Replace only the obsolete internal assertion that a failed callback retains a reserved event-ID job; assert a durable captured job has no premature event ID and cannot bypass a rejected ledger insertion. |
| `campfire/src/channels/sink.rs`, `sink/stream_tests.rs`, `channels/tests/events_test.rs` | Preserve main's event-card broadcasts and the existing streamed append/replace renderer. Honor APP_URL when configured, otherwise Rails' renderer host is example.org. Render the partial directly, as Rails broadcasts do, so same-timestamp stream updates bypass the collection cache. Event socket fixtures explicitly configure the route defaults their Rails probe sets. Existing expectations are unchanged. Add configured HTTPS/port frame byte comparisons. |
| `reference-tools/agents/deletion_publication_contract.rb`, `verify-deletion-publication.py`, publication vector and failing-first artifact | Independent pinned Rails observations, including actual separate connection commits and resumed polling. |
| `reference-tools/agents/verify-stream-frame-origins.py`, configured stream vector | Reproduce default and configured start/update/final broadcast bytes on the pin. |
| `reference-tools/agents/run-fresh.sh` | A distinct final clone path and clearing stale exit status before a gate. The driver hash was checked unchanged throughout the final run. |

Deletion retains Rails' after_destroy_commit boundary: a rejected ledger insert leaves the thread deleted and no ledger row. The durable enqueue exception remains atomic: rejecting `Agent::EventWebhookJob` insertion rolls back deletion. For a webhook-enabled deletion, a crash between parent commit and publication leaves a durable captured payload rather than a missing reserved identity. Without a webhook, the ledger stays an ordinary after-commit side effect, as in Rails. Publication and argument binding are one BEGIN IMMEDIATE write, so a cursor can only observe IDs whose rows already exist. The captured chain is used for replay identity; it is not a polling cursor. Jobs already stored in the ordinary event-ID format remain readable.

Failing first on b795aa8e, with production unchanged and only tests/probe/vector added:

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_publication_
```

Both tests observed a committed later event, an earlier deletion ID and an empty resumed page. Pinned Rails returns work_unassigned on the resumed page, with the deletion ID above the previously committed event. The concurrent test adds another real connection's commit and polls again before permitting deletion publication. Raw baseline summaries:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1052 filtered out; finished in 0.91s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 891 filtered out; finished in 0.11s
```

The full diagnostic objects are committed in `reference-tools/agents/review-176-publication-failing-first.txt`; both report deletion_id_after_* false and resumed_types []. The pinned oracle independently reports both ordering checks true and resumed_types [work_unassigned].

Commands rerun this session, with raw summary lines:

```sh
python3 rust/reference-tools/agents/verify-deletion-publication.py
python3 rust/reference-tools/agents/verify-review-fixes.py
python3 rust/reference-tools/agents/verify-stream-frame-origins.py
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/check-seeds.py
```

```text
WS11 deletion publication Rails probes: 2 real commit/poll interleavings regenerated byte-identical (d7c7de92)
WS11 review Rails probes: 21 repository batches, repeated-thread polling, deletion with/without webhook and captured-agent removal, real reference order and partial fanout regenerated byte-identical (d7c7de92)
WS11 stream frame origins: default and configured start/update/final frames regenerated byte-identical (d7c7de92)
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```

Focused verification ran the three publication/recovery tests, all nine earlier review regressions, seven work/queue tests, the two stream-origin frame tests and the two event socket tests. Commands and raw nonzero summaries:

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_publication_
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_review_
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_work_
rust/reference-tools/agents/run-focused.sh -p campfire ws11_stream_peer_
rust/reference-tools/agents/run-focused.sh -p campfire channels::tests::events_test
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1072 filtered out; finished in 1.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 925 filtered out; finished in 0.14s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1068 filtered out; finished in 15.70s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 923 filtered out; finished in 0.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1073 filtered out; finished in 0.73s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 920 filtered out; finished in 0.49s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1073 filtered out; finished in 2.71s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1073 filtered out; finished in 7.35s
```

The stream tests initially exposed the merge's renderer-host substitution; correcting the host then exposed stale collection-cache output (Starting instead of Latest draft). Rust was corrected and the original default vectors were regenerated unchanged from Rails. The added configured vector is direct pinned Rails output. The first fresh run of 710d77d0 exposed that merge mismatch. Its log capture was then invalidated by editing the executing driver; that run was discarded, not counted as a passing gate. Its clone sources and invalid logs are retained under `.scratch/`, its target was removed, and the final immutable-driver gate below used a new remote clone. Preflight clippy's large-enum warning was fixed by boxing the captured envelope, without adding a lint allowance.

The cursor audit examined agent_event_access/agent_event_polling and every AgentEvent writer, including message deliveries, slash/approval/work callbacks, posting and suppression rows, and the GitHub/Fizzy action ledgers. Normal model writes and GitHub completion SQL allocate IDs in their insertion transactions. Fizzy's claim row already exists before its HTTP work; completion updates that existing row. No other reserve-early, insert-late AgentEvent writer was found.

For other streams, ActivityItem.refresh_unread allocates at insert, including approval fanout's separate after-commit recipient writes. AuditLog writes allocate at insert. Message timeline and search cursors use existing (created_at,id) rows; refresh reads existing created_at/updated_at values. Calendar event reads use event rows and dates, not a reserved append-ledger high-water mark. Durable queue IDs are allocated at job insertion and workers claim ready rows by status/run_at/id, without permanently advancing a high-water cursor. The production sequence search found no remaining model/adapter reservation helper; the remaining explicit sqlite_sequence adjustments are schema preparation or test fixture setup. Permission filtering and updates to existing rows retain Rails' semantics and were not changed. REST/MCP and pages remain with their separate owners; no additional uninstalled reader is claimed verified.

Final gate: `rust/reference-tools/agents/run-fresh.sh` cloned the pushed branch into `.scratch/fresh-ws11-review-176-publication-final`, checked its SHA against origin, copied and validated nine labeled pinned seeds, and ran:

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/agents/summarize-tests.py .scratch/fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-workspace.log
```

Raw gate totals and clippy line:

```text
WS11 fresh source: 0402c96cc525a3d9a6498cbdbfcd11ffa3201f66
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 workspace totals: 2680 passed; 0 failed; 11 ignored; 50 result summaries
WS11 missing-seed skips: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 40s
WS11 fresh exits: test=0 clippy=0
WS11 workspace dependencies: 77 unique keys; 0 duplicates (strict TOML parse)
```

All raw cargo result summaries from that final clone (ignored tests are explicitly excluded from passed counts):

```text
test result: ok. 1073 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 885.96s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.81s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 922 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 78.08s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.70s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.91s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.38s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.81s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.52s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.50s
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

The independent named comparison accounting used check-case-ports, domain-case-inventory and write-case-status, all rerun this session. These are additional regressions, not closures of original deferred names. Exactly 81 of 378 original named comparisons remain deferred in eight pinned files; each name and owner remains in `reference-tools/agents/deferred-domain-cases.json`. Raw per-file executed counts, largest files first:

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

The assigned re-review fix and audit are complete. The wider WS11 domain is partial at those 81 names; WS11-ui owns pages and WS11-api owns REST/MCP/by-bot surfaces. Public owner-facing service signatures were kept stable. Fresh and baseline scratch target directories are deleted; logs, probes and clone sources remain as evidence. No own test/compiler process remains. The full verification logs are in this worktree's `.scratch/`; no review directory was modified.
