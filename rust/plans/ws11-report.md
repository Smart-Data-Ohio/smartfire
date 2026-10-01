# WS11 continuation — partial, 2026-10-01

Verified source: `e366dd5556f75610f7fe8fce9d02f92dadb0645c` on `rust/ws11-agents`. This continuation closes **33** of the requested 114 named comparisons: **297 passed, 81 deferred** across the original 378 names in 26 pinned Rails files. Runner probes and named Rust comparisons were executed; these counts do not claim that entire Ruby test files ran.

## Pushed slices and changes by file

| Commit | Result |
| --- | --- |
| `4a8c17e2` | `app.rs`, message presenter/sink, job registry and lifecycle merge resolutions. Merge commit, parents `25b94524` and main `65ad0d39` (#167/#171/#173/#170). Preserve both callback chains, agent streaming renderers, durable delivery, icon/cap owner APIs, and enabled `manages_bots`. Adopt held Fizzy listeners. |
| `bcf7bd06` | Presenter test support and `integrations/agent_repositories/live_tests.rs`: the boot-installed reader shares WS15g's real `Accounts` cache/network with the GitHub service. Compare 200/403/404/401/500 decisions, redaction and retry behavior. Exercise the registered approved GitHub job after a kill switch and the production GitHub message-reference hook during streaming. `tests/agent_security_lifecycle_cases_test.rs`: eight revocation and seven kill-switch names. |
| `633dbb0b` | `tests/agent_presence_slash_cases_test.rs`: six working-presence and six slash-registration names. `jobs/periodic.rs`: extract the conditional plaintext-token heal; its caller signature stays stable. `agent_repositories/bot_plaintext_cases.rs`: three retirement names including a reset between snapshot and heal. |
| `1be35884` | `tests/agent_work_payload_cases_test.rs`: all three WorkPayload reader names, including full thread shape, human/null owners, board identity, sorted tags, result timestamps, run URL and links. `callbacks.rs`: distinguish installed message/dependency paths from the remaining peer stubs. |
| `e366dd55` | `controllers/github/agent_tests.rs`: replace the incoming unpinned inbox-rollback assumption with a pinned HTTP/database/replay oracle. Keep its queue-rejection rollback and actual approved-action execution assertions. `github/approval_requests.rs`: correct the transaction documentation. |

Corresponding scripts and generated vectors are under `rust/reference-tools/agents/` and `rust/vectors/agents_*.json`. `case-ports.json` maps each closed source name to an executed Rust test. No REST/MCP or server-rendered agent pages were implemented here.

## Design and peer boundaries

WS15g's reader is live: normal GitHub callers and agent callers share the same linked-account cache, refresh policy, denial/disconnect behavior and injected network. Owner/account authority is revalidated after the external read. The former GitHub seam is no longer a production stub. Public/unknown/private redaction remains pinned.

Main's `Env.message_reference_syncs` installs GitHub reference reconciliation. Full stream finalization synchronizes the final URL once and enqueues one fetch; starts/appends stay quiet, repeated finalization does nothing, and quiet finalization creates no GitHub reference/fetch. The previously installed WS8 message payload helper, ordinary ActivityItem producer, Fizzy/link/Twitter reference paths, and GitHub/Fizzy hard-removal adapters remain installed. Main's GitHub deactivation hook runs alongside WS11's suspension and Fizzy deactivation.

WS13 huddle, WS14 Google/calendar/events and WS16 Slack dependency/reference phases remain flagged until their owners install adapters. WS12 still owns board/work mutations, assignment validation, handoffs and work inbox producers. WorkPayload comparisons use persisted fixtures matching Rails producer output; they close reader assertions, not those mutation callbacks. WS11-ui owns agent HTML/status directory broadcasts and human approval pages; WS11-api owns agent REST/authentication/MCP. Existing owner-service signatures were kept stable.

Rails oddity preserved: a workspace-wide `post_messages` grant and the model capability query remain true after membership removal. The room-scoped grant is revoked. Posting/delivery services retain their membership checks.

## Failing-first and discrimination evidence

The first fresh gate at `1be35884` found the incoming WS15g inbox assumption. A rejecting `AgentApproval` ActivityItem trigger produced one persisted approval, where the test's literal expected zero. Pinned Rails independently returns HTTP 500, retains one pending approval, creates no inbox/job, and returns HTTP 200 with that same ID on replay. This is an oracle-backed correction to an unpinned integration assertion; production fanout stays after commit. The durable-job rejection still rolls back the approval decision and ledger.

Executed commands:

```sh
python3 rust/reference-tools/agents/check-security-lifecycle-mutations.py
rust/reference-tools/agents/run-focused.sh -p campfire github_agent_http_races_fanout_rollback_expiry_and_real_approved_job
```

Raw compiled regression and before/after summaries:

```text
membership-revocation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 872 filtered out; finished in 0.20s
quiet-after-commit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 872 filtered out; finished in 0.23s
github-kill-authority: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1041 filtered out; finished in 0.73s
WS11 security lifecycle discrimination: 3 compiled regressions rejected; sources restored
plaintext-reset: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1045 filtered out; finished in 0.62s
WS11 plaintext discrimination: stale snapshot overwrote reset; compiled regression rejected; source restored
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1045 filtered out; finished in 1.38s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1045 filtered out; finished in 1.51s
```

The plaintext mutation was executed by the inline Python driver captured in `.scratch/plaintext-reset-mutation.log`: remove the plaintext equality from the production heal, run `ws11_plaintext_case_key_reset_beats_snapshot`, require a compiled assertion failure, then restore in `finally`. All four mutants were restored. Earlier incorrect test setup (an FTS column name, a missing owner on a validation input, and fake-network routing in the boot helper) was fixed to represent the Rails inputs; no Rails expected values were relaxed.

## Reproduced oracle and merge checks

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/verify-security-domain-cases.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
```

Metadata exited 0. Strict `tomllib` parsing of `rust/Cargo.toml` also exited 0. Raw summaries:

```text
WS11 workspace dependencies: 77 unique keys; 0 duplicate keys (strict TOML parse)
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 security/domain Rails oracles: 7 pinned lifecycle, repository, stream, presence/registration, plaintext, work-payload and HTTP after-commit contracts regenerated byte-identical
WS11 source case files: 25 pinned Git files matched; 0 checkout mismatches
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 8 pinned files; 81 named source cases; owners recorded per file
```

The reference is `d7c7de92` plus `_common.md`'s approved drift; no agent oracle input changed. Seven contracts regenerate byte-identically. No pixel comparisons were performed.

## Final fresh-clone gate

```sh
rust/reference-tools/agents/run-fresh.sh
```

This clones the pushed branch into `.scratch/fresh-ws11-github-reconciled`, starts with an empty target, copies the nine built pinned seeds and validates their bot digests, then executes:

```sh
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/agents/summarize-tests.py .scratch/fresh-workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-workspace.log
```

Both Cargo commands ran in the fresh clone with `CI=1`, `CARGO_BUILD_JOBS=2`, machine-wide rustc throttling, `CARGO_INCREMENTAL=0`, and line-table debug profiles. Mail/cable use 52200–52249; integration/GitHub use 52250–52298. Storage vectors execute the freshly built binary in pinned Rails through `pinned-media-runner.py`; other binaries run natively. No concurrency or timing threshold was weakened. `manages_bots` ran and passed. No seeded test skipped. The 11 ignores are inherited helpers/doctests.

The first fresh gate and its clippy result are preserved in `.scratch/fresh-first-*.log`:

```text
WS11 workspace totals: 2606 passed; 1 failed; 11 ignored; 48 result summaries
WS11 missing-seed skips: 0
WS11 fresh exits: test=101 clippy=0
```

Raw final fresh summaries:

```text
WS11 fresh source: e366dd5556f75610f7fe8fce9d02f92dadb0645c
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 fresh exits: test=0 clippy=0
WS11 workspace totals: 2607 passed; 0 failed; 11 ignored; 48 result summaries
WS11 missing-seed skips: 0
test result: ok. 1044 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 571.29s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 46.33s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 884 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 93.25s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.44s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.81s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.45s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.68s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.37s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.62s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.72s
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
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 59s
```

## Executed named comparisons, largest files first

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

## Precisely remaining — 81 names

- `test/models/agent_test.rb`: **2** — WS11-ui rendered broadcast cases; WS11 domain cases compared.
- `test/services/slash_commands/dispatcher_test.rb`: **35** — WS11 agent dispatch; WS8 built-in commands.
- `test/jobs/agent/delivery_job_test.rb`: **2** — WS11 domain.
- `test/models/channel_thread_agent_assignment_test.rb`: **29** — WS11 agent callbacks; WS12 mutation producers.
- `test/models/message_streaming_test.rb`: **4** — WS11 finalization; WS12 activity; WS14/15 external reference sync.
- `test/models/user/bot_test.rb`: **6** — WS11 bot domain/removal; WS11-api by-bot HTTP surface.
- `test/models/agent_budgets_test.rb`: **2** — WS11 domain.
- `test/models/agent_kill_switch_test.rb`: **1** — WS11 callbacks; WS12 owned-board mutation producer.

Every remaining name and its owner is listed in [deferred-domain-cases.json](/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11/rust/reference-tools/agents/deferred-domain-cases.json). The two delivery names are transaction-depth instrumentation and the WS12 self-assignment lineage scenario. The four streaming names require complete named start/append/trailing/all-side-effect comparisons; existing byte-identical cable/presenter contracts are additional evidence, not claimed closure. Bot names still include two deterministic generator-stub cases, three reply-token model names and actual queued webhook execution. Both budget names need WS12 inbox/handoff producers; the remaining kill-switch name needs a WS12 owned-board mutation.

No new policy questions. This is partial. Reports are mirrored at `rust/plans/ws11-report.md` and the requested `wave4/ws11-report.md`. The first and final fresh targets were removed after completion; logs, source clones and seeds remain. No WS11 test process was left running.
