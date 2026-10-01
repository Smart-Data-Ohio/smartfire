# WS11 delivery and adapter parity report — partial

Continuation after `3d72b4e2`, on `rust/ws11-agents`. Implementation and fresh-clone gate source: `ecc652b7ba4e97e2c05396d33682280454025c98`; the report-only commit follows. Oracle: our Rails at `d7c7de92`. Prior approval-after-commit fixes and UI owner APIs remain included; their detailed historical evidence is in this report's `3d72b4e2` version. This continuation closes **18 more named comparisons**, reducing 132 deferred names to **114**. Additional adapter/render comparisons are evidence, not automatic closure of the four complete streaming source cases.

## Pushed slices and changes

| Commit | Files under `rust/` and behavior |
|---|---|
| `90a831a7` | Merge main `ea630861` with a merge commit. `models/message.rs` installs the main ordinary `ActivityItem::record_message` producer during message creation and full stream finalization. Keep `manages_bots` enabled. |
| `8b76fd0a` | `integrations/agent_jobs/delivery_path_cases.rs`: actual HTTP delivery, duplicate execution, lost delivery/suppression CAS, agent/legacy hop limit, and four concurrent producers of 28 mentions. `tests/agent_delivery_cases_test.rs`: both legacy fanout names. Reconcile three push AppState constructors with the stable owner services. |
| `ada86d8f` | `agent_jobs/webhook_key_cases.rs`: all nine WebhookAgentKey names through real HTTP. Ten Rails transcripts cover context, ownership, public PR, ordinary thread/room, approvals, work, completion and legacy. Compare serialized payloads, body HMAC and timestamp/signature headers; agents omit reply tokens, legacy tokens authenticate for the room. |
| `92056558`, `1f26a300` | `integrations/agent_repositories.rs`: production boot installs a thin `RepositoryReader` adapter to the existing WS15g Accounts domain. Preserve owner-linked account selection, post-await authority revalidation and conservative custom/uninstalled states. Tests use an owned local TLS endpoint for `api.github.com`, never the real service. A staged payload activation overlapped this commit; `1f26a300` immediately restored the inactive factory until its complete slice. |
| `4bc45631` | `controllers/messages/payload.rs`, `presenters/agent_payload.rs`, `presenters.rs`, `rich_text.rs`, `app.rs`: install the shared WS8 payload reader at boot. Imported reader helpers from WS8b-m `6bb4ea416d46775a4109d1fbd661659a2b87117f`, without its HTTP surface. Preserve Current.user, base URL and existing service signatures. Eight semantic JSON comparisons include byte-identical rendered HTML. Delegate plain text to Message so forwarded Markdown retains its note. |
| `7991aca1` | `channels/sink.rs` and real authenticated WebSocket tests: render ordinary Message append as well as MessageReplace; render callback partials directly to avoid stale same-timestamp drafts/final indicators. `agent_posting.rs`: use the actual `user_<id>_unreads` channel. `jobs/peer_callbacks.rs`: install available GitHub/Fizzy has-one removal adapters in declaration order with rollback. Approval preference-neighbor case now uses the installed ordinary mention producer rather than manual activity writes. |
| `ee21457a` | Merge main `76e54ad5` (#169 timing helpers, #171 durable attachment analysis). Resolve controller/jobs conflicts preserving agent replay/budget preflight before attachment resolution and main's durable analysis enqueue. Keep the model callback registry in all builds and the upstream ad-hoc handlers only in tests. Retain main's bounded test helpers; no WS11 threshold/concurrency relaxations. |
| `ecc652b7` | Strict clippy found an unused test-server wrapper and WS17 test-module placement after the merge. Remove the unused wrapper and move the test module below production items. No behavior, test policy or suppressions changed; both fresh gates are rerun on this source. |

## Differential and failing-first evidence

All new Ruby probes run inside the pinned Rails image. Vector values were regenerated, never hand-edited. The raw JSON is compared structurally where transport semantics permit; every Rails-rendered HTML field/frame is compared as an exact string. No pixel comparison was performed.

* **Delivery policy mutations:** compiled rate limit 20→28 and lost-CAS success `==1`→`>=0` both fail against the real installed app path. The restored source passes the same checks and the final whole-workspace gate.
* **Repository adapter:** the production-boot test first failed before installation. Actual Rails and Rust HTTP responses 200/403/404/401/500 agree on decisions, GET paths, grant/denial caching, disconnect and retry behavior. A compiled reader bypass returning true exposes private details and fails. Rails retains the private link while redacting title/branches on denial; Rust retains that exact behavior.
* **Shared presenter:** the adapter comparison first failed with the reader uninstalled. After installation, the pinned forward-note comparison exposed a real plain-text omission, fixed in Rust. Other cases cover rich text, Markdown, deleted replies, thread/bot/human permissions, request base URL and attachment metadata.
* **Hard removal:** the real production job first kept the user and linked accounts because adapters were missing. After installation it matches Rails success and trigger-rejected Fizzy deletion: success deletes all three rows; failure rolls back all three.
* **Streaming:** before the append renderer, a subscribed authenticated client received no append within the existing watchdog. After that fix the same frozen timestamp exposed stale `Starting` content versus Rails `Latest draft`; direct callback partials fix it. The final comparison receives exact append, update, full-final and unread frames, and verifies repeat finalization is quiet. These are deterministic missing-renderer/stale-content failures, not a reason to widen timing thresholds.

Raw summaries retained in `.scratch/` (failing-first runs are against the pre-fix implementations in this continuation):

```text
# delivery-path-mutations.log
WS11 delivery path mutation: rate; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 934 filtered out; finished in 15.66s
WS11 delivery path mutation: claim; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 934 filtered out; finished in 1.53s
WS11 delivery path mutations: 2 compiled mutations caught; source restored
# live-reader-failing-first.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 944 filtered out; finished in 0.63s
# live-reader-mutation.log
WS11 live reader privacy bypass: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 946 filtered out; finished in 0.34s
WS11 live reader mutation: 1 compiled owner-access bypass caught; source restored
# live-adapter-failing-first.log
test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 944 filtered out; finished in 1.61s
# peer-removal-failing-first.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 947 filtered out; finished in 1.18s
# stream-peer-failing-first.log
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 948 filtered out; finished in 5.43s
# stream-peer-restored.log, intermediate stale HTML reproduction
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 948 filtered out; finished in 0.67s
# delivery-path-cases.log
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 929 filtered out; finished in 7.76s
# fanout-named-cases.log
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 856 filtered out; finished in 0.35s
# webhook-key-complete.log
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 936 filtered out; finished in 1.87s
# live-reader-positive.log
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 945 filtered out; finished in 2.11s
# shared-payload-restored.log
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 942 filtered out; finished in 1.31s
# peer-removal-restored.log
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 948 filtered out; finished in 1.05s
# stream-peer-complete.log
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 948 filtered out; finished in 2.12s
# approval-neighbour-complete.log
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 858 filtered out; finished in 0.16s
```

An earlier streaming-test link was killed by the machine (`collect2: fatal error: ld terminated with signal 9 [Killed]`, `stream-peer-linker-killed.log`). No tests ran in that attempt; the unchanged build was retried. All executed failures above were resolved in Rust. Fresh-gate outcomes are recorded separately below.

## Final oracle regeneration and exact commands

Cwd: assigned worktree. These checks were rerun after the final main merge. Main's rubyzip lock drift is reported; all these agent-area oracles stay at the pin. The known Rails behavior of retaining redacted private-PR links is preserved, without redesign.

```sh
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/verify-contracts.py
python3 rust/reference-tools/agents/verify-ledger-contracts.py
python3 rust/reference-tools/agents/verify-path-contracts.py
python3 rust/reference-tools/agents/verify-peer-contracts.py
python3 rust/reference-tools/agents/verify-callback-contracts.py
python3 rust/reference-tools/agents/verify-ui-owner-inputs.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
(cd rust && cargo metadata --locked --format-version 1 >/dev/null)
```

```text
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 20 byte-identical contract files; 23 contracts recorded in total
WS11 ledger Rails oracles: 2 regenerated byte-identical budget/event model vectors
WS11 delivery path Rails oracle: 10 webhook, 5 repository and 8 presenter transcripts regenerated byte-identical
WS11 peer Rails oracles: 2 destroy outcomes, 1 approval/mention neighbour and 3 rendered cable phases regenerated byte-identical
WS11 callback Rails oracles: 5 byte-identical vectors; 28 guard case names and address inputs matched the pin
WS11 UI owner Rails oracle: 81 cap; 21 icon; 4 read-only secret inputs; regenerated bytes match
WS11 source case files: 19 pinned Git files matched; 0 checkout mismatches
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 13 pinned files; 114 named source cases; owners recorded per file
WS11 workspace dependencies: 77 unique keys; 0 duplicate keys
```

Strict TOML parsing after both main merges finds zero duplicate workspace dependency keys. Locked cargo metadata passes after the merge and in the fresh clone. The inherited PDF fixture's xref trailing spaces were preserved as fixture bytes.

## Fresh-clone gate

The committed runner was executed from the assigned worktree:

```sh
rust/reference-tools/agents/run-fresh.sh
```

It cloned the pushed branch into `.scratch/fresh-ws11-delivery-adapters` with an empty target. The nine already-built pinned seeds were copied, then validated again; no untracked source or target output was copied. The source SHA matched the remote branch at clone time. The runner sets CI, two build jobs, four test threads, the machine-wide rustc queue, private TMPDIR, line-table debug info and WS11-owned ports 52200–52299. The first fresh run passed all 2,499 tests but clippy rejected two merge cleanup issues. After `ecc652b7`, `.scratch/rerun-fresh-gate.sh` fast-forwarded this independently cloned checkout from the pushed remote and reran both complete gates (clippy first). No outside target/source artifacts were introduced. Its substantive commands, from the fresh clone, are:

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1
python3 rust/reference-tools/agents/check-seeds.py
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

Storage-vector media execution uses `pinned-media-runner.py`: the same freshly compiled binary runs in the pinned Rails image with no network; every other test runs natively. All 19 listed media byte comparisons pass. The 11 inherited ignores are nine reference/export/external-service/measurement helpers and two DB doctests; none is newly ignored by WS11. `manages_bots` ran and passed. No missing-seed skips, test failures or timing flakes occurred in this fresh gate.

Final summary commands, from the assigned worktree:

```sh
python3 rust/reference-tools/agents/summarize-tests.py .scratch/fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-workspace.log
```

The initial gate's honest exit summary, retained in `fresh-exits-before-clippy-cleanup.log`:

```text
WS11 fresh exits: test=0 clippy=101
error: function `trickling_server` is never used
error: items after a test module
```

Raw final fresh summaries:

```text
WS11 fresh source: ecc652b7ba4e97e2c05396d33682280454025c98
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 fresh exits: test=0 clippy=0
WS11 workspace totals: 2499 passed; 0 failed; 11 ignored; 48 result summaries
WS11 missing-seed skips: 0
test result: ok. 965 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 311.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.77s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 45.06s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.83s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 855 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 129.28s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.45s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.65s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.98s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.58s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.78s
```

## Executed named comparisons by pinned Rails file

Largest source files first. Counts below require the corresponding Rust test names to have actually passed in the fresh log. Rails runner probes were executed; these counts do not claim that the full Ruby test files were run.

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
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 0 passed; 0 failed; 8 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 0 passed; 0 failed; 8 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 0 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 0 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 0 passed; 0 failed; 3 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 0 passed; 0 failed; 3 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 264 passed; 0 failed; 114 deferred
```

## Precisely remaining — 114 named comparisons

The exhaustive names and owner boundaries are in `rust/reference-tools/agents/deferred-domain-cases.json`. The following counts are source-case deferrals; broader consolidated tests or vectors do not close these names.

| Rails file | Deferred | Owner / missing full named comparison |
|---|---:|---|
| `test/services/slash_commands/dispatcher_test.rb` | 35 | WS11 agent dispatch; WS8 built-in commands |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | WS11 agent callbacks; WS12 mutation producers |
| `test/models/agent_kill_switch_test.rb` | 8 | WS11; WS15g/WS15e approved-action execution callbacks |
| `test/models/agent_revocation_test.rb` | 8 | WS11 domain |
| `test/models/agent_slash_command_test.rb` | 6 | WS11 domain |
| `test/models/agent_working_presence_test.rb` | 6 | WS11 domain |
| `test/models/user/bot_test.rb` | 6 | WS11 bot domain/removal; WS11-api by-bot HTTP surface |
| `test/models/message_streaming_test.rb` | 4 | WS11 finalization; WS12 activity; WS14/15 external reference sync |
| `test/models/agents/work_payload_test.rb` | 3 | WS11; WS8 message presenter; WS15g private repository reader |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | WS11 domain |
| `test/jobs/agent/delivery_job_test.rb` | 2 | WS11 domain |
| `test/models/agent_budgets_test.rb` | 2 | WS11 domain |
| `test/models/agent_test.rb` | 2 | WS11-ui rendered broadcast cases; WS11 domain cases compared |

Remaining domain comparisons cover: explicit rate-check/insert locking and self-assignment hop delivery; complete stream start/finalize/append/trailing side effects; kill switch and revocation; slash registration; working presence; token reset/reply/queued delivery; plaintext cleanup; activity access/handoffs and the complete work payload/assignment mutation producers. Continue with delivery names, then these lifecycle/source-case comparisons. Built-in slash cases remain WS8, work mutation producers remain WS12, and two rendered agent badge/directory names remain WS11-ui.

Production repository and shared message-payload adapters are installed. Remaining peer flags: `MessageGithubReferences` (WS15g), `MessageEventReferences` and calendar/Google dependencies (WS14), `UserHuddles`/`SessionHuddles` (WS13), Slack connection/import dependencies (WS16), and the WS12 work/assignment producers needed for complete case closure. Available GitHub/Fizzy user-account deletion, main ordinary message activity, and WS15e link/Fizzy/Twitter reference synchronization are installed. Unavailable removal dependencies leave rows intact and let FK failures roll back; complete cross-domain hard removal/finalization is still partial.

WS11-ui still owns server pages/human approval controller. WS11-api still owns agent REST/MCP, by-bot HTTP endpoints, throttle and authorization/authentication concerns. Their public domain signatures remain stable. The thin WS15g adapter delegates to existing Accounts code; no GitHub HTTP, refresh or cache policy was implemented here. The shared WS8 reader import may overlap WS8b-m at lead merge; it is intentionally limited to the helper and Markdown reader.

No open permission request. No Rails expectation or source was changed to improve behavior. No pixel work, stash, added ignore, threshold widening or test-concurrency reduction. The fresh scratch target is deleted after verification; logs, vectors, seeds and source remain available.
