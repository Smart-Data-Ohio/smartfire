# WS11 domain parity report — partial

Implementation through `9abda135`; the final report-only commit follows. Rails oracle: `d7c7de92`. Branch: `rust/ws11-agents`. All changes were made in the assigned worktree. REST/MCP and agent-authentication HTTP surfaces remain WS11-api's; server pages and the human approval controller remain WS11-ui's.

## Review fix and failing-first evidence

An archived checkout of exact `bee97633`, with only the reviewer regression injected, reproduces the P2. Rejecting AgentApproval activity inserts returns an error and leaves **zero approvals**. Pinned Rails returns ActiveRecord::StatementInvalid and leaves **one**. Automatic fanout now starts after the primary approval commits, in separate recipient writes. Durable jobs remain in the primary writer transaction.

The callback audit also found that Rails commits each recipient separately: if a later insert fails, earlier inbox items remain. My initial whole-fanout transaction lost the earlier item. A second differential produced Rails `approvals=1, inbox_items=1`; its regression failed at `b3084cd0` with zero items before `d71b6d46` fixed it. The explicit `fan_out_inbox_items(tx)` service signature remains stable.

Audit: Agent's status badge is already an after-update-commit event; suspension's quiet message finalization now has its own committed write per message. Approval's automatic inbox fanout is after-create-commit. AgentCredential, AgentGrant, AgentEvent, AgentStep and AgentSlashCommand have validation/create hooks, not comparable synchronous after-commit fanout. AgentBudgetNotice explicitly calls its recorder rather than declaring an after-commit callback. Approval decision/action jobs retain the queue-atomicity exception in decisions.md.

## Changes by file and integration boundary

| Files under `rust/` | Result |
|---|---|
| `crates/db/src/models/agent_approval.rs`, `agent_lifecycle.rs`, approval tests and two inbox failure Rails probes/vectors | Primary approval survives inbox errors; successful earlier recipients survive a later error; suspension finalizes streams after commit. Both trigger regressions pass. |
| `crates/db/src/models/user.rs`, `user/icon.rs`, `data/icon-names.json` | Rails icon normalization, changed-field validation, create/update/clear setters and avatar cache timestamp invalidation. Unknown icons yield RecordInvalid (`icon_name: is not a known icon`) for the UI owner's 422. Existing create_bot signature delegates unchanged to create_bot_with_attributes. |
| `crates/db/src/models/agent.rs`, `agent/cap_input.rs`, `tests/agent_ui_owner_test.rs`, `plans/ws11-owner-apis.md` | Three raw before-type-cast cap fields preserve submitted JSON values, AR casts and validation messages; complete prospective validation is read-only. Signing-secret getter reads/decrypts without ensuring, generating, rotating or touching the row. |
| `crates/campfire/src/integrations/agent_jobs/webhook_cases.rs`, `integrations/jobs.rs`, DB bot cases | All 22 WebhookTest names compared using real HTTP, signing/guards and persisted reply/attachment writes. Legacy job shares the same production implementation with an injectable network; only resolver/dialer collaborators are replaced in tests. |
| `crates/campfire/src/integrations/agent_jobs/recovery_cases.rs`, presenter test support | All 13 DeliveryRecoveryTest names exercise the production periodic sweep, persisted durable jobs, real HTTP 429, Retry-After, stale snapshots, exhausted attempts, fresh rows and a rejected enqueue. |
| `crates/db/src/models/agent_posting.rs`, `tests/agent_budget_cases_test.rs` | Add read-only persisted budget usage, sharing counts with the existing cap preflight; nine named Rails comparisons. Inbox access filtering and WorkHandoffs.create are still WS12 boundaries. |
| `crates/db/src/models/agent_delivery.rs`, `agent_event_access.rs`, event and step case tests | All ten event and ten step names compared. Ledger scopes and low-level model acknowledgment are separate from polling's existing live-access authorization gate. Existing service signatures remain stable. |
| `crates/db/src/models/message.rs`, `events.rs`, `crates/campfire/src/integrations.rs`, `integrations/jobs.rs`, presenter and message controller merge resolutions | Preserve both sides of WS15e's production reference/account callbacks and WS11's callback chain. Actual link-reference/fetch-job writes occur at full finalization and roll back on durable enqueue failure. No GitHub access implementation. |
| `crates/campfire/src/channels/sink.rs` | Install generic MessageReplace rendering for final/trailing frames through the real cable publisher and session-bound markup guard. The previously silent quiet-finalization frame now reaches a subscribed client within the original one-second assertion. |
| `crates/campfire/src/integrations/action_claims/tests.rs`, `controllers/fizzy_message_cards/tests.rs` | Reconcile incoming expectations with WS11's three periodic tasks and the Rails test's actual legacy bot. The fixture's dedicated port is configurable; WS11 uses 52299. |
| `reference-tools/agents/*`, new vectors | Pinned Rails runtime oracles, compiled mutation checks and explicit named mappings. Named pass counting covers all 26 source files, including zero-pass deferred files. |

## Merge and fresh-clone verification

`5cbac4e0` is a true merge of the locally available origin/main `b66199b7` (WS15e #166 and asset fix #168). Locked metadata passed after the merge and in the fresh clone. Strict TOML parsing finds 76 distinct workspace dependency keys and zero duplicates. `manages_bots` remains enabled and ran successfully in the fresh suite.

The first fresh run caught three app integration failures: the old periodic list, an agent-backed fixture used for a legacy webhook assertion, and a missing generic MessageReplace renderer. All were fixed; the quiet timeout was an absent renderer, not a timing threshold problem. It also caught one needless-question-mark clippy finding, now removed.

The host media guard correctly rejected libvips 8.18.6 / ffmpeg n9.0.2 versus pinned 8.16.1 / 7.1.5. The very same fresh-clone vector binary passes all eight tests and all 19 media byte comparisons in `triage-reference-d7c7de92`. The final cargo run uses the committed pinned-media runner for that binary and runs every other test natively. CI stays enabled, seeds remain mandatory, test concurrency stays four, and no timing thresholds, goldens, masks or WS11 ignores were added or relaxed. The machine-wide rustc queue and two build jobs were retained.

The fresh clone independently built all nine seeds from the pinned reference, starting with an empty target directory. Implementation updates were pulled from the pushed branch; no untracked source, target output or seeds were copied into it. Its extra target is removed after verification; source, seeds and raw logs remain.


## Exact verification commands and raw summaries

Commands below were executed this session. Unless stated otherwise, cwd is the assigned worktree. Cargo uses Rust 1.98.1, two build jobs, line-table debug information and the machine-wide compiler queue. Tests use four threads (below the requested maximum of eight). Raw logs are retained under this worktree's `.scratch/`.

Failing-first against an archive of **exact bee97633**, with only the reviewer's appended regression in `crates/db/src/tests.rs`, and an initially empty extra target:

```sh
CI=1 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only TMPDIR="$PWD/.scratch" CARGO_TARGET_DIR="$PWD/.scratch/approval-bee97633/rust/target" mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/approval-bee97633/rust/Cargo.toml -p campfire_db ws11r_approval_inbox_failure_retains_primary_record_like_rails -- --test-threads=4 --nocapture
```

```text
WS11R approval inbox failure: persisted approvals = 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 613 filtered out; finished in 0.19s
```
Pinned Rails probe (same trigger and valid seeded request):

```sh
PARITY_NAMESPACE=ws11 PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/reference runner --seed default rust/reference-tools/agents/approval_inbox_failure_contract.rb
```

```text
WS11R Rails approval inbox failure: error=ActiveRecord::StatementInvalid; persisted approvals=1
```

The later-recipient regression was first run against b3084cd0 before applying the second fix. It asserts the oracle's retained earlier inbox item:

```sh
CI=1 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only TMPDIR="$PWD/.scratch" mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db ws11_approval_inbox_recipients_commit_separately_after_primary_like_rails -- --test-threads=4 --nocapture
```

```text
WS11 approval later-recipient failure: persisted inbox items = 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 688 filtered out; finished in 0.35s
```
Rails result from `approval_partial_fanout_contract.rb`:

```text
{
  "error": "ActiveRecord::StatementInvalid",
  "approvals": 1,
  "inbox_items": 1
}
```
After both fixes, all approval checks were rerun:

```sh
CI=1 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only TMPDIR="$PWD/.scratch" mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire_db approval -- --test-threads=4 --nocapture
```

```text
WS11 approval later-recipient failure: persisted inbox items = 1
WS11R approval inbox failure: persisted approvals = 1
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 660 filtered out; finished in 1.28s
```

All of these regressions also ran in the final fresh workspace gate. Source/image checks and deterministic differential regeneration:

```sh
python3 rust/reference-tools/agents/check-reference.py
python3 rust/reference-tools/agents/verify-contracts.py
python3 rust/reference-tools/agents/verify-callback-contracts.py
python3 rust/reference-tools/agents/verify-ui-owner-inputs.py
python3 rust/reference-tools/agents/verify-ledger-contracts.py
```

```text
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
WS11 bot/posting Rails oracles: 2 byte-identical contract files
WS11 webhook Rails oracle: 62 numeric hosts; 3 DNS cases; 4 signatures; 3 payloads matched; randomized AR secret regenerated
WS11 domain Rails oracles: 20 byte-identical contract files; 23 contracts recorded in total
WS11 callback Rails oracles: 5 byte-identical vectors; 28 guard case names and address inputs matched the pin
WS11 UI owner Rails oracle: 81 cap; 21 icon; 4 read-only secret inputs; regenerated bytes match
WS11 ledger Rails oracles: 2 regenerated byte-identical budget/event model vectors
```

Compiled failing mutations (each command restores source in `finally`; restored source is what the final fresh gate tests):

```sh
CI=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only python3 rust/reference-tools/agents/check-ui-owner-mutations.py
CI=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only python3 rust/reference-tools/agents/check-installed-webhook-mutations.py
CI=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only python3 rust/reference-tools/agents/check-recovery-mutations.py
CI=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only python3 rust/reference-tools/agents/check-ledger-mutations.py
```

```text
WS11 UI owner mutation: icon; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 658 filtered out; finished in 0.22s
WS11 UI owner mutation: cap; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 658 filtered out; finished in 0.36s
WS11 UI owner mutation: secret; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 658 filtered out; finished in 0.71s
WS11 UI owner mutations: 3 compiled mutations caught; source restored
WS11 installed webhook mutation: reply-parent; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 788 filtered out; finished in 0.95s
WS11 installed webhook mutation: private-guard; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 788 filtered out; finished in 0.65s
WS11 installed webhook mutation: winner-secret; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 659 filtered out; finished in 0.37s
WS11 installed webhook mutation: peer-refs; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 788 filtered out; finished in 0.36s
WS11 installed webhook mutations: 4 compiled mutations caught; source restored
WS11 recovery mutation: fresh-grace; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 801 filtered out; finished in 0.71s
WS11 recovery mutation: retry-snapshot; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 801 filtered out; finished in 0.89s
WS11 recovery mutation: stale-attempt; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 801 filtered out; finished in 0.66s
WS11 recovery mutations: 3 compiled mutations caught; source restored
WS11 ledger mutation: count-openers; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 688 filtered out; finished in 0.18s
WS11 ledger mutation: repeat-notice; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 688 filtered out; finished in 0.75s
WS11 ledger mutation: message-scope; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 688 filtered out; finished in 0.16s
WS11 ledger mutation: foreign-step; exit=101; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 688 filtered out; finished in 0.18s
WS11 ledger mutations: 4 compiled mutations caught; source restored
```

Fresh clone: cloned the remote branch directly, starting before the later slices, then pulled each committed implementation update with `git merge --ff-only origin/rust/ws11-agents`. At the final gate its HEAD was `9abda1359daf87c8b3c76141f6c71d7b07aaf356`; the final report-only commit does not change tested Rust code.

```sh
git clone --single-branch --branch rust/ws11-agents https://github.com/Smart-Data-Ohio/smartfire.git .scratch/fresh-ws11-review-fix
```

```text
Cloning into '.scratch/fresh-ws11-review-fix'...
```

In that clone all nine seeds were freshly built before the tests (`fresh-review-seeds.log`). The build command was executed again after the final gate to verify the report's exact invocation; no test depended on a copied seed:

```sh
PARITY_NAMESPACE=ws11-review-fresh PARITY_OWNER=ws11 PARITY_IMAGE=triage-reference-d7c7de92 rust/parity/bin/seed build
python3 rust/reference-tools/agents/check-seeds.py
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
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
```

Locked metadata and duplicate dependency-key check were rerun in the fresh clone:

```sh
CI=1 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 >/dev/null
python3 - <<'PY'
import tomllib
from pathlib import Path
keys=tomllib.loads(Path('rust/Cargo.toml').read_text())['workspace']['dependencies']
print(f'WS11 locked workspace metadata: {len(keys)} dependency keys; 0 duplicate keys')
PY
```

```text
WS11 locked workspace metadata: 76 dependency keys; 0 duplicate keys
```

Final whole-workspace test and strict clippy commands, cwd `.scratch/fresh-ws11-review-fix`. Only the vendored html5ever package is excluded; all application workspace packages and targets are included. The media runner executes the actual storage test binary in the pinned container, without changing test arguments or source. Other binaries run natively.

```sh
CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/rust/target" CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/agents/pinned-media-runner.py" CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only TMPDIR="$PWD/.scratch" WS15E_FIZZY_MESSAGE_CASE_PORT=52299 CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52298 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture > ../fresh-review-final-workspace.log 2>&1
CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/rust/target" CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only TMPDIR="$PWD/.scratch" WS15E_FIZZY_MESSAGE_CASE_PORT=52299 CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249 INTEGRATION_TEST_PORT_RANGE=52250-52298 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > ../fresh-review-final-clippy.log 2>&1
```

```text
     Running unittests src/main.rs (rust/target/debug/deps/campfire-cbd4512f01b33109)
test controllers::presenters::accounts::tests::manages_bots ... ok
test result: ok. 800 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 238.42s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_assets-e47daa34d8102ba2)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/reference.rs (rust/target/debug/deps/reference-0dc3427cd8dc9d47)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_cable-9ff61dc90c1c019f)
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/disconnect.rs (rust/target/debug/deps/disconnect-4d1ad3a580f87a62)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 48.44s
     Running tests/golden.rs (rust/target/debug/deps/golden-0065f3c4e4c3792a)
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
     Running tests/protocol.rs (rust/target/debug/deps/protocol-5c631da233379e66)
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.14s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_db-a9858b6465df6182)
WS11 approval later-recipient failure: persisted inbox items = 1
WS11R approval inbox failure: persisted approvals = 1
test result: ok. 685 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 73.72s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_jobs-52ab149be7847fbe)
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.87s
     Running tests/crash.rs (rust/target/debug/deps/crash-96006cc38a2401be)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_kit-90d963fb5689332a)
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
     Running tests/front.rs (rust/target/debug/deps/front-0ae64cdc69d61226)
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
     Running tests/http.rs (rust/target/debug/deps/http-313d3b491d19c06e)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/params_vectors.rs (rust/target/debug/deps/params_vectors-0be98416e9df7e3d)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/rails_vectors.rs (rust/target/debug/deps/rails_vectors-17bcc1f3febf2f77)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_mail-34d0ea721fe2f624)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
     Running tests/config.rs (rust/target/debug/deps/config-40b8ce3197b38431)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/goldens.rs (rust/target/debug/deps/goldens-1cdd561c0b3aed6d)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
     Running tests/inbound.rs (rust/target/debug/deps/inbound-970da2fe8fae5892)
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.51s
     Running tests/security.rs (rust/target/debug/deps/security-36ab7a2b20c256da)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests/smtp.rs (rust/target/debug/deps/smtp-c77142012d69115d)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_richtext-771939bcb3dac794)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/corpus.rs (rust/target/debug/deps/corpus-9c0e3a9f43932478)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.33s
     Running tests/fork_regressions.rs (rust/target/debug/deps/fork_regressions-00d0ee43252ae4a9)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/hardening.rs (rust/target/debug/deps/hardening-654da2ae0da02c13)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.62s
     Running tests/markdown_corpus.rs (rust/target/debug/deps/markdown_corpus-d3f94aeef42811a0)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.41s
     Running tests/markdown_security.rs (rust/target/debug/deps/markdown_security-13dd18945a9db335)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
     Running tests/reference_tests.rs (rust/target/debug/deps/reference_tests-30e6688af57bedaa)
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests/sgid_json_corpus.rs (rust/target/debug/deps/sgid_json_corpus-91d1a83762be5f2a)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_routes-81f95cd571ee6de0)
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_storage-c45743a9431d8b27)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
     Running tests/vectors.rs (rust/target/debug/deps/vectors-849131d7e92963fc)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.53s
     Running unittests src/lib.rs (rust/target/debug/deps/campfire_views-7d8f7ae80fc14012)
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
     Running tests/core.rs (rust/target/debug/deps/core-dccc9fa22fd67b41)
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
     Running unittests src/lib.rs (rust/target/debug/deps/rails_compat-85ecbbb147467672)
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.71s
   Doc-tests campfire_assets
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_cable
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_db
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_jobs
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_kit
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_mail
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_richtext
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_routes
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_storage
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests campfire_views
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
   Doc-tests rails_compat
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.39s
```

Both command exit statuses were zero. Totals and missing-seed checks were derived from that actual final log:

```sh
python3 rust/reference-tools/agents/summarize-tests.py .scratch/fresh-review-final-workspace.log
```

```text
WS11 workspace totals: 2130 passed; 0 failed; 11 ignored; 46 result summaries
WS11 missing-seed skips: 0
```

The eleven unchanged ignores are explicit: two app reference/measurement tests, one cable reference recorder, four DB differential/export/rollback harnesses, one kit ACME fixture, one mail export harness, and two kit doctest examples. No WS11 test is ignored. The seeded app reports 800 passed, zero failed; `manages_bots` ran. Missing-seed skips are zero. No unresolved timing flake remains from this session; the finalization failure was repaired at the missing renderer, without extending its one-second bound.

## Named Rails comparisons, largest source files first

These are actual Rust executions of named Rails-case ports, validated against pinned source names. They are not a claim that the 378-case Ruby suite ran. Additional consolidated Rust tests and Rails runner contracts do not close deferred names. Source mapping and exact deferred inventory were regenerated, then all pass counts were extracted from the fresh test log:

```sh
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/write-case-status.py
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-review-final-workspace.log
```

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 17 pinned files; 132 named source cases; owners recorded per file

WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 22 passed; 0 failed; 7 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 0 passed; 0 failed; 29 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 19 passed; 0 failed; 4 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 19 passed; 0 failed; 1 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 9 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 9 passed; 0 failed; 2 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 0 passed; 0 failed; 9 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 0 passed; 0 failed; 8 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 0 passed; 0 failed; 8 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 0 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 0 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 0 passed; 0 failed; 3 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 0 passed; 0 failed; 3 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 0 passed; 0 failed; 2 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 0 passed; 0 failed; 1 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 246 passed; 0 failed; 132 deferred
```

## Precisely remaining and where to resume

Status is **partial**, with 246 named comparisons passed and 132 still deferred, down from the 260 at the review baseline. The exact 132 case names and per-file owners are committed in [`reference-tools/agents/deferred-domain-cases.json`](../reference-tools/agents/deferred-domain-cases.json); `case-ports.json` records the actual mapped Rust function for each compared name. Resume with the remaining largest **owned** files, preserving the mapped/unmapped distinction.

| Pinned Rails file | Remaining | Owner/boundary |
|---|---:|---|
| `test/models/agent_test.rb` | 2 | WS11-ui rendered broadcast cases; WS11 domain cases compared |
| `test/services/slash_commands/dispatcher_test.rb` | 35 | WS11 agent dispatch; WS8 built-in commands |
| `test/jobs/agent/delivery_job_test.rb` | 7 | WS11 domain |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | WS11 agent callbacks; WS12 mutation producers |
| `test/models/message_streaming_test.rb` | 4 | WS11 finalization; WS12 activity; WS14/15 external reference sync |
| `test/models/agent_approval_test.rb` | 1 | WS11 domain; WS11-ui human approval controller/pages |
| `test/models/user/bot_test.rb` | 6 | WS11 bot domain/removal; WS11-api by-bot HTTP surface |
| `test/models/agent_budgets_test.rb` | 2 | WS11 domain |
| `test/models/webhook_agent_key_test.rb` | 9 | WS11; WS8 presenter; WS15g live private repository access |
| `test/models/agent_kill_switch_test.rb` | 8 | WS11; WS15g/WS15e approved-action execution callbacks |
| `test/models/agent_revocation_test.rb` | 8 | WS11 domain |
| `test/models/agent_slash_command_test.rb` | 6 | WS11 domain |
| `test/models/agent_working_presence_test.rb` | 6 | WS11 domain |
| `test/models/agents/work_payload_test.rb` | 3 | WS11; WS8 message presenter; WS15g private repository reader |
| `test/services/bots/clear_plaintext_tokens_test.rb` | 3 | WS11 domain |
| `test/models/message/bot_webhook_fanout_test.rb` | 2 | WS11; WS16 import suppression integration |
| `test/jobs/agent/delivery_concurrency_test.rb` | 1 | WS11 domain |

Production seams that remain open:

1. WS8b-m's request-specific `MessagePayloadHelper` adapter: imported main b66199b7 contains no implementation of that helper. `controllers/presenters/agent_payload.rs::MessagePayload` and `Presenter::agent_message_payload` retain stable signatures and fail explicitly when uninstalled; the test adapter is not presented as production. WS11-api needs the actual shared helper at reconciliation. Legacy webhook stock payloads and the installed message replacement renderer are distinct, already tested paths.
2. WS15g's live private-PR reader: `integrations/agent_repositories.rs::RepositoryReader` remains an explicit FLAGGED STUB that denies private access until the linked-account reader is installed. WS11 contains no GitHub network implementation.
3. WS12 thread/activity mutation producers and WS14/WS15g remaining reference/removal adapters. WS15e link/Fizzy/X reference sync and connected-account removal are installed in this merge; that does not close every peer path. Approval's complete preference-neighbour case still needs the ordinary mention activity producer. The kill-switch and revocation named files, remaining delivery races, slash/presence files and token scrub names still need complete mapped case comparisons.
4. REST/MCP, `messages/by_bots`, `boosts/by_bots`, authorization/authentication/throttle HTTP concerns remain WS11-api's. Server pages and the human approvals controller remain WS11-ui's. UI owner APIs are completed and documented in [`ws11-owner-apis.md`](ws11-owner-apis.md); HTTP 422 response rendering and redisplay belong to that owner.

No claim of complete hard-user-removal/finalization parity or production cutover readiness is made while those peer paths and named cases remain. Existing removal/finalization contracts were freshly re-recorded and their consolidated tests pass. No privilege expansion was introduced to fill a missing peer. No open design decision was changed.

The extra fresh-clone target was removed after the final gates; the primary target, fresh source/seed checkout and all evidence logs remain. No WS11 test/build process or container is left running. The final report-only commit also updates deferred boundary wording for the now-installed stream renderer; it changes no Rust implementation or vector.
