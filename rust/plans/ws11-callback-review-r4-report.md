WS11 PR #176 callback review fixes — verified code, wider domain partial

Final verified source: `d25678fd0b162fde9f3fb94961b92a2c7265913d` on `rust/ws11-agents`. `4027d35698959987dbb991afc53a19312a6a142b` is the merge commit with origin/main `434d1c14` (#175 rooms HTTP). This report supersedes the earlier claim that a durable deletion intent should reconstruct an unpublished event after a process stop.

The reviewer evidence under `/home/riels/.cache/rust-port/ws11r/r3/review/` was read without modification. `66f469cb` commits independent pinned Rails probes, vectors and regressions while production was still `151cba7c`. `eabfca60` fixes the callbacks, crash behavior and reference-phase encryption cost. `43784133` prepares an isolated fresh gate. `71f992c0` completes main's presenter merge. `583a482c` preserves notifications for already-committed jobs after a callback error and adds its failing-first regression and pinned Calendar failure probe. `d25678fd` fixes the pinned media runner's external-cache mount. Builds use CARGO_BUILD_JOBS=2 and tests use four threads. No stash, timing-threshold increase, reduced test concurrency or pixel work was used.

| Files | Change |
|---|---|
| `crates/db/src/database.rs` | Keep a record's first transaction-registration slot, group its after-commit hooks there, and discard new slots/hooks on savepoint rollback. Return an unhandled callback error after waking already-committed durable jobs, dropping all subsequent model callbacks/broadcasts while keeping the primary commit. Existing rescued broadcasts retain their rescue behavior. Main's scoped-writer and queue-observation APIs remain. |
| `crates/db/src/models/channel_thread.rs`, `agent_work_events.rs` | Register the thread on create/save/destroy and publish deletion/indicator callbacks from that first slot. Ledger insertion remains after commit, with SQLite assigning its polling ID at insertion. Durable enqueue stays atomic with deletion; publication and event-ID binding share a separate transaction. |
| `crates/campfire/src/integrations/agent_jobs.rs` | Reload a claimed captured-deletion envelope. A bound envelope uses the normal event job; an unpublished envelope finishes without creating an event. No ledger resurrection after a process stop or an earlier callback error. Ordinary retries, signing and backoff remain unchanged. |
| `crates/campfire/src/jobs.rs` | Reuse boot's AR encryption object for both reference entry points. Remove per-phase key derivation from the serialized message writer. |
| `crates/db/src/tests/agent_work_events_test.rs`, `campfire/src/integrations/agent_jobs/publication_tests.rs` | Regressions for create/update registration order, reverse-destroy control, savepoint rollback, stop-on-first-error and restart before publication. Keep the earlier concurrent-cursor and rejected-enqueue regressions. Replace the old Rust-only crash-recovery expectation with the independently observed Rails result. |
| `reference-tools/agents/deletion_callback_contract.rb`, `deletion_crash_contract.rb`, `verify-deletion-callbacks.py`, new vectors | Real pinned Rails methods and transactions, one-event pagination, rejected ledger trigger, actual exit(73), and a second process on the same database. Vectors are direct stdout, compared byte for byte. |
| `reference-tools/agents/repeat-concurrent-posts.py`, review evidence artifacts | Run the real HTTP regression fifty times with four test processes, libtest threads=4 and its unchanged Tokio workers=8. Preserve baseline assertions and the active writer's PBKDF2 stack. |
| Merge: `campfire/src/channels/sink.rs`, `controllers/messages*`, `controllers/presenters*`, `jobs*`, `rich_text.rs`, `db/src/models/user*` | Keep both main's message-feature/directory/event renderers and WS11's uncached streamed append/replace path, route defaults and agent peers. Main's production payload helper is retained. Search-preload presenters carry the agent adapter and current user. Public owner-facing agent services remain stable. The obsolete private grant helper is removed; real owner suspension and connected-account hooks remain. `manages_bots` runs without an ignore. |

Failing first on production `151cba7c3503b3665660090592bf066da399568d`, before production edits:

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_r4_
```

| Regression | Baseline Rust | Pinned Rails d7c7de92 / fixed Rust |
|---|---|---|
| Create A then B; destroy B then A | polls B,A | polls A,B |
| Update A first; destroy B then A | polls B,A edited | polls A edited,B |
| Reject B's first ledger insert, webhooks disabled | both threads deleted; A published | both threads deleted; no event published |
| Restart between deletion commit and publication, webhook eligible | deleted thread; one recovered/polled event | deleted thread; zero ledger/polled events |

The reverse-destroy control passes on baseline. The committed `review-176-r4-failing-first.txt` includes each failed observation and both raw summaries. Additional savepoint cases discard a separately created/destroyed C on rollback; they do not claim general Active Record object-instance identity parity.

Crash choice: match Rails. The atomic durable intent survives the deletion transaction, but it cannot reconstruct an after-commit side effect that never ran. The worker consumes an unpublished intent as a terminal no-op. This preserves decisions.md's atomic-enqueue exception and its separate after-commit boundary; rejected enqueue still rolls back deletion. The Rails probe really exits before the callback and reads the database from a new process. Rust reopens a snapshot taken after commit but before publication and exercises the installed durable delivery path. Both observe no event after restart. Existing event-ID jobs remain readable.

Concurrency root cause: the writer was actively deriving AR encryption keys with PBKDF2-SHA256 (65,536 iterations) in `Jobs::sync_message_reference_phase` on each message phase. Baseline gdb shows PBKDF2 -> ArEncryption::new -> phase hook -> Message::create -> run_write; the CI test exceeded its existing 60-second timeout. Local baseline posts took approximately 42–47 seconds. After reusing boot's encryption object, fifty real HTTP tests pass both before and after merging main at CI's four-process concurrency. The timeout and test are unchanged. The committed `review-176-r4-concurrent-posts.txt` contains the CI failure and writer stack. No lock-order change or connection-pool workaround was required.

Rails CI isolation: run 36834186614 at 151cba7c timed out at stage_test.rb:297 waiting for the browser's huddle:join event after clicking Join stage. The preceding Rails CI run 36831308310 at 0402c96c is green. `git diff 0402c96c..151cba7c --name-only -- ':!rust'` is empty: Rails code, test and workflow inputs are identical. The LiveKit shard runs Rails, not the Rust binary. This confirms the failure is independent of these Rust callback changes; its exact browser timing cause was not reproduced locally, and no Rails test or timeout was edited.

Final commands ran in the remote clone `.scratch/fresh-ws11-review-176-r4-final`, fast-forwarded only for the runner fix. `git diff 583a482c d25678fd -- rust/crates` is empty. Its compiler cache reused the first clone's absolute target path with CARGO_INCREMENTAL=0; only one extra physical compiler target was kept. Exactly 81 of 378 original named comparisons remain deferred; these review regressions are additional tests, not closures of those names.

The first complete fresh gate at 71f992c0 had 3169 passed, one failed and 11 ignored; clippy passed. Its only failure was main's `pr174_invitation_failure_preserves_committed_recipients`: a recorded Calendar job notification was lost when the new stop-on-error rule returned. The independently pinned failure probe confirms Rails stops later Calendar callbacks and enqueues zero Calendar jobs, for all three recipient positions with and without a requested Meet link. Rust's approved durable-enqueue exception has already committed these jobs before the hook runs. The fix wakes those committed jobs on error while discarding remaining model hooks, broadcasts and disconnects. It neither continues ledger callbacks nor reconstructs missing deletion events. The existing assertion is unchanged; its misleading comment now identifies the durable-job exception. A new regression failed first on merged 71f992c0 (no job notifications instead of [1,2]) and then verifies two committed typed jobs, a native purge notification, suppression of a non-surviving record job, and no later hook or broadcast. First-gate logs are archived under `.scratch/r4-first-*`, not counted as the passing final gate.

The requested main snapshot 434d1c14 is merged. The remote moved to 72fc8b05 during verification; that later snapshot is not part of this sealed gate.

The next gate at 583a482c had no failed assertions, but storage vectors did not start: Docker lacked a mount for the executable in the external compiler cache (OCI exit 127; Cargo exit 101). Archived logs: `.scratch/r4-runner-failed-*`. The runner now mounts that executable directory read-only and uses a process-specific container name. The unchanged pinned storage target passed, then the entire workspace, clippy and fifty concurrent HTTP tests were rerun on d25678fd. The final gate below is fully green. No executing driver was edited.

Failing-first raw summaries on unchanged production 151cba7c (exit 101):

```sh
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_r4_
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1075 filtered out; finished in 0.74s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 0.00s
test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 926 filtered out; finished in 0.28s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 52 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 119 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 32 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 44 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 78 filtered out; finished in 0.00s
```

Additional wakeup regression, failing first on merged 71f992c0 (exit 101):

```sh
rust/reference-tools/agents/run-focused.sh -p campfire_db --features test-support --lib ws11_r4_callback_failure_stops_hooks_but_wakes_committed_jobs
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 932 filtered out; finished in 0.18s
```

Pinned Rails oracles regenerated byte-identical this session:

```sh
python3 rust/reference-tools/agents/verify-deletion-callbacks.py
python3 rust/reference-tools/agents/verify-event-callback-failure.py
python3 rust/reference-tools/agents/verify-deletion-publication.py
python3 rust/reference-tools/agents/verify-stream-frame-origins.py
python3 rust/reference-tools/agents/verify-review-fixes.py
python3 rust/reference-tools/agents/check-reference.py
```

```text
WS11 r4 Rails callbacks: 8 registration-order/savepoint cases and stop-on-error regenerated byte-identical (d7c7de92)
WS11 r4 Rails crash: exit=73; restarted process sees deleted thread and zero events (d7c7de92)
WS11 merge Calendar Rails probe: 6 rejected-recipient/Meet cases regenerated byte-identical; 2 model sources matched d7c7de92; 0 later Calendar callbacks enqueued jobs
WS11 deletion publication Rails probes: 2 real commit/poll interleavings regenerated byte-identical (d7c7de92)
WS11 stream frame origins: default and configured start/update/final frames regenerated byte-identical (d7c7de92)
WS11 review Rails probes: 21 repository batches, repeated-thread polling, deletion with/without webhook and captured-agent removal, real reference order and partial fanout regenerated byte-identical (d7c7de92)
WS11 reference sources: 62 pinned files matched; 0 image mismatches (d7c7de92)
WS11 checkout drift: Gemfile.lock rubyzip 3.0.2 -> 3.7.0 from merged main; oracle stays pinned
```

Focused nonzero summaries, four test threads:

```sh
rust/reference-tools/agents/run-focused.sh -p campfire_db --features test-support --lib ws11_r4_
rust/reference-tools/agents/run-focused.sh -p campfire_db --features test-support --lib calendar_event_test
rust/reference-tools/agents/run-focused.sh --workspace --exclude html5ever --no-fail-fast ws11_stream
rust/reference-tools/agents/run-focused.sh -p campfire --bin campfire channels::tests::events_test
```

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 927 filtered out; finished in 1.15s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 901 filtered out; finished in 5.98s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1535 filtered out; finished in 3.05s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 914 filtered out; finished in 1.07s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1544 filtered out; finished in 2.58s
```

Locked metadata succeeded after the merge and in the final clone. Strict TOML parsing found no duplicate workspace dependency keys. Latest normal binary build with only Docker source inputs:

```sh
cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > .scratch/r4-final-metadata.json
python3 -c 'import tomllib; from pathlib import Path; n=len(tomllib.loads(Path("rust/Cargo.toml").read_text())["workspace"]["dependencies"]); print(f"WS11 workspace dependency keys: {n} unique; duplicate keys=0 (strict TOML parse)")'
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_TARGET_DIR="$PWD/rust/target" rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins
```

```text
WS11 workspace dependency keys: 77 unique; duplicate keys=0 (strict TOML parse)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.35s
```

Final commands ran from `.scratch/fresh-ws11-review-176-r4-final`, the remote clone fast-forwarded from 583a482c to d25678fd (runner only). Environment: CI=1; CARGO_BUILD_JOBS=2; CARGO_INCREMENTAL=0; dev/test debug=line-tables-only; TMPDIR=this clone's .scratch; CARGO_TARGET_DIR=the first clone's rust/target; CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=this clone's pinned-media-runner.py. Cable/mail ports 52200–52249; integration/GitHub/WS15e ports 52250–52298.

```sh
cargo test --locked --manifest-path rust/Cargo.toml -p campfire_storage --test vectors -- --test-threads=4 --nocapture
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/agents/repeat-concurrent-posts.py
```

```text
WS11 fresh source: d25678fd0b162fde9f3fb94961b92a2c7265913d
WS11 seeds: 9 built; 0 plaintext tokens; every labeled bot key matches its digest
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.95s
WS11 fresh exits: test=0 clippy=0
WS11 workspace totals: 3171 passed; 0 failed; 11 ignored; 58 result summaries
WS11 missing-seed skips: 0
Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.43s
WS11 concurrent HTTP posts: 50/50 passed; test-process concurrency=4; libtest threads=4; Tokio workers=8; timeout unchanged at 60s
```

Every raw final workspace summary (ignored tests are not counted as run):

```text
test result: ok. 1544 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 399.98s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.41s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 929 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 58.72s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.45s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.83s
test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
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

Explicit ignored tests, unchanged; manages_bots runs and passes:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

Original named comparison accounting, largest files first; review regressions do not close unmapped names:

```sh
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/domain-case-inventory.py
python3 rust/reference-tools/agents/write-case-status.py
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/fresh-workspace.log
```

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 deferred case inventory: 8 pinned files; 81 named source cases; owners recorded per file
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

Precisely remaining: 81 original named comparisons across eight files, listed with owners in `reference-tools/agents/deferred-domain-cases.json`: dispatcher 35 (WS8); channel-thread assignment 29 (WS11/WS12); bot 6 (WS11/WS11-api); streaming 4 (WS11/WS12/reference peers); agent rendered status 2 (WS11-ui); delivery 2 (WS11); budgets 2 (WS11/WS12); kill switch 1 (WS11/WS12). Pages stay with WS11-ui and REST/MCP/by-bot HTTP with WS11-api. Owner APIs are stable. Wider domain work remains partial at those 81 names; the assigned callback/crash/concurrency review fixes are complete.

The cursor audit remains unchanged: deletion IDs are allocated only in the separate ledger insertion transaction; record grouping changes publication order without reserving IDs. Captured intents contain UUID chains, not polling identities. No missing deletion event is reconstructed. Agent SQL reservation searches find only comments or test setup; ordinary agent events, ActivityItem and AuditLog streams allocate in their insertion writes. Committed-job wakeups do not publish ledger rows.

Cleanup (normal worktree rust/target retained; every scratch target deleted):

```text
WS11 cleanup: removed .scratch/fresh-ws11-review-176-r4/rust/target (compiler cache)
WS11 cleanup: removed .scratch/fresh-ws11-review-176-r4-final/rust/target (40K generated JSON outputs, preserved as evidence)
WS11 cleanup: 0 scratch target directories; 0 own test/compiler processes; logs and clone sources retained
```

Full logs and clone sources remain in this worktree's `.scratch/`. The final report commit changes documentation only. No reviewer evidence directory was modified.
