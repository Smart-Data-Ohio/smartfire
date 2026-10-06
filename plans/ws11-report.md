WS11 merge of main #179/#180 — verified merge; wider domain partial

Verified merge/source: `f2c30b8c2d296f3b76c7cc30fbb17a1c8cb3ee98` on `rust/ws11-agents`. Parents: `3a2cf0d8e3a096170216e1aec60bb56392947746` and main `2b05160758307effa035b7d2581618014c46b755` (#179 viewer-zone timestamps/cache keys and #180 PR-refresh deflake). The final report commit changes documentation only. No behavior changed beyond bringing in and reconciling the merge. The two CodeQL findings were left unchanged as instructed. No stash, test weakening, timing-threshold increase, pixel work or Python model-server operation was used.

Conflict reconciliation:

| File | Both sides preserved |
|---|---|
| `crates/campfire/src/controllers/messages.rs` | Keep WS11's required authenticated user and `presenter.current_user_id` for the agent payload reader. Reuse that authenticated ID for main's `use_viewer_zone`. Keep main's viewer-zone creation response and actor-zone creation broadcast rendering intact. |
| `crates/campfire/src/controllers/presenters.rs` | Keep `current_user_id`, the agent payload state, agent module and editable-body owner API. Add main's `render_zone` and preserve it alongside the agent state in both the ordinary and search-preload constructors. All main rendering/cache changes survive. |

`message_cache.rs` is byte-identical to main: expand the Rails timestamp components in `self.render_zone`, keep plain record versions in UTC, and add no zone-name suffix. Alias zones therefore share keys; ordinary fragments retain Rails' observed cross-zone cache sharing. Main's actor-zone shared event broadcasts and UTC detached-job fallback remain unchanged. All 22 incoming non-conflict files match main byte for byte, including #180's held PR-refresh queue assertion. The agent database/domain, delivery, private repository readers and reviewed indicator fix did not change in this merge.

Raw merge checks:

```text
WS11 merge cargo metadata --locked: exit=0; 77 workspace dependency keys; 0 duplicates
WS11 merge preservation: 22/22 non-conflict incoming files byte-identical to main; 2 union conflict resolutions
WS11 viewer-zone preservation: main timestamp-key expansion and all 3 Rails corpora byte-identical; recorded source hashes match d7c7de92
```

The three incoming Rails corpora and recorded source hashes were checked without rewriting them. The full fresh suite runs their seven HTTP, complete-card-container, cache-key/cache-reuse and actual shared-socket comparisons:

```text
test controllers::rooms::native_integration_tests::zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient ... ok
test controllers::rooms::native_integration_tests::zone_audit_github_message_cards_match_rails_with_warm_zones ... ok
test controllers::rooms::native_integration_tests::zone_audit_message_creation_matches_rails_and_reload ... ok
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_honolulu_share_github_fragments_like_rails ... ok
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_india_share_ordinary_fragments_like_rails ... ok
test controllers::rooms::native_integration_tests::zone_cache_timestamp_components_match_rails_across_dst_and_fractional_zones ... ok
test controllers::rooms::native_integration_tests::zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths ... ok
```

Verification ran from a fresh remote clone at `.scratch/ws11-main-179-verification`, exactly the merge SHA above. All ten pinned seeds were copied and checked. Pinned libvips/ffmpeg libraries were used from the start, and the storage corpus uses the committed pinned-media runner. Cargo jobs=2, libtest threads=4 and the configured machine-wide rustc throttle were retained. Only one extra scratch target existed; it was removed after every process completed. The documented vendored html5ever exclusion remains the workspace-test/clippy convention.

Setup and effective commands executed for the fresh clone (paths/env below refer to that clone):

```sh
PARITY_IMAGE=triage-reference-d7c7de92 WS8BR2_MEDIA_DIR="$PWD/.scratch/rails-media-ws11-179" bash rust/reference-tools/users/media_runtime.sh
export CI=1 TMPDIR="$PWD/.scratch" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_PROFILE_TEST_DEBUG=line-tables-only
export CARGO_TARGET_DIR="$PWD/rust/target"
export PATH="$PWD/.scratch/rails-media-ws11-179/usr/bin:$PATH"
export LD_LIBRARY_PATH="$PWD/.scratch/rails-media-ws11-179/native-libs"
export CABLE_TEST_PORT_RANGE=52200-52249 MAIL_TEST_PORT_RANGE=52200-52249
export INTEGRATION_TEST_PORT_RANGE=52250-52298 WS15E_TEST_PORT_RANGE=52250-52298 GITHUB_TEST_PORT_RANGE=52250-52298
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/rust/reference-tools/agents/pinned-media-runner.py"
python3 rust/reference-tools/agents/check-seeds.py
cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=4 --nocapture
cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
# In the clone's rust/ directory:
cargo metadata --locked --format-version 1 >/dev/null
```

Locked metadata also ran immediately after resolving the merge, and before the fresh suite. TOML parsing checks for duplicate workspace keys. Raw fresh-clone summaries, including every test-result line:

```text
WS11 seeds: 10 built; 0 plaintext tokens; every labeled bot key matches its digest
WS11 cargo metadata --locked: exit=0; 77 workspace dependency keys; 0 duplicates
WS11 workspace totals: 3637 passed; 0 failed; 12 ignored; 58 result summaries
WS11 missing-seed skips: 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 53s
WS11 fresh seeds-final: exit=0
WS11 fresh workspace-pinned: exit=0
WS11 fresh clippy-pinned: exit=0
```

The exact requested release-input build ran from the worker worktree:

```sh
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

Environment: RUST_CI_IMAGE=campfire-toolchain-ci-rust-speedups, CI=1, RUNNER_TEMP=.scratch/ws11-179-ci-temp, RUST_CI_CONTAINER_PREFIX=ws11-179, CARGO_HOME=/usr/local/cargo and CARGO_TARGET_DIR=/src/rust/target. The scratch launcher reserved the CI wrapper's existing four compiler workers in the configured machine-wide slot pool, without editing the throttle or adding job flags. Raw output (ANSI color escapes removed):

```text
WS11 CI build: reserved 4 configured machine-wide rustc slots; throttle configuration unchanged
WS11 release-inputs CI build: exit=0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 43.31s
```

Twenty repetitions ran the unchanged exact test app::tests::concurrent_message_posts_all_complete from the same fresh source. The scratch harness differs from the committed 50-run script only in repetition count and explicit source-root argument. It built with locked Cargo, then ran four test processes at once, libtest threads=4, Tokio workers=8 and the unchanged 60-second timeout. With the fresh target/media/port environment above, the command from this worktree was:

```sh
python3 .scratch/ws11-179-repeat-20.py .scratch/ws11-main-179-verification
```

Raw repetition summaries:

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.47s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.24s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.33s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.21s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.23s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.24s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.31s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.23s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.36s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.30s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.58s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.29s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.28s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.23s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.38s
WS11 concurrent HTTP posts: 20/20 passed; test-process concurrency=4; libtest threads=4; Tokio workers=8; timeout unchanged at 60s
```

The complete workspace has zero failures and zero missing-seed skips. manages_bots is unignored and passes. The reviewed agent indicator, stream, event-socket, privacy, publication-order, stop-on-error and queue-atomicity checks all run in that workspace. Existing explicit ignores were retained; none was added here. This merge-only slice adds no new failing-first claim. The prior indicator fix and its failing-first Rails/socket evidence are preserved unchanged in plans/ws11-deletion-indicator-r5-report.md; previous callback/merge evidence remains historical in plans/ws11-callback-review-r4-report.md and plans/ws11-main-b908-report.md.

Named comparison accounting was rerun from the worker worktree:

```sh
python3 rust/reference-tools/agents/domain-case-inventory.py
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/named-case-pass-counts.py .scratch/ws11-179-workspace-pinned.log
python3 rust/reference-tools/agents/summarize-tests.py .scratch/ws11-179-workspace-pinned.log
```

Raw grouped counts:

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named ports: test/models/agent_credential_test.rb: 11 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_grant_test.rb: 12 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_backfill_test.rb: 1 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_test.rb: 39 mapped cases; 2 unmapped case names
WS11 named ports: test/services/slash_commands/dispatcher_test.rb: 5 mapped cases; 35 unmapped case names
WS11 named ports: test/jobs/agent/delivery_job_test.rb: 27 mapped cases; 2 unmapped case names
WS11 named ports: test/lib/restricted_http/private_network_guard_test.rb: 28 mapped cases; 0 unmapped case names
WS11 named ports: test/models/message_streaming_test.rb: 19 mapped cases; 4 unmapped case names
WS11 named ports: test/models/agent_approval_test.rb: 20 mapped cases; 0 unmapped case names
WS11 named ports: test/jobs/agent/event_webhook_job_test.rb: 17 mapped cases; 0 unmapped case names
WS11 named ports: test/models/user/bot_test.rb: 9 mapped cases; 6 unmapped case names
WS11 named ports: test/models/webhook_test.rb: 22 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent/delivery_recovery_test.rb: 13 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_budgets_test.rb: 9 mapped cases; 2 unmapped case names
WS11 named ports: test/models/agent_event_test.rb: 10 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_step_test.rb: 10 mapped cases; 0 unmapped case names
WS11 named ports: test/jobs/agent/delivery_concurrency_test.rb: 1 mapped cases; 0 unmapped case names
WS11 named ports: test/models/message/bot_webhook_fanout_test.rb: 2 mapped cases; 0 unmapped case names
WS11 named ports: test/models/webhook_agent_key_test.rb: 9 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_revocation_test.rb: 8 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_kill_switch_test.rb: 7 mapped cases; 1 unmapped case names
WS11 named ports: test/models/agent_working_presence_test.rb: 6 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agent_slash_command_test.rb: 6 mapped cases; 0 unmapped case names
WS11 named ports: test/services/bots/clear_plaintext_tokens_test.rb: 3 mapped cases; 0 unmapped case names
WS11 named ports: test/models/agents/work_payload_test.rb: 3 mapped cases; 0 unmapped case names
WS11 source case files: 25 pinned Git files matched; 0 checkout mismatches
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
WS11 deferred ownership: 81 exact named comparisons across 8 files; every deferred case has an owner
```

Precisely remaining: **81 named comparisons**, unchanged across eight files. Every exact deferred name and owner is retained in reference-tools/agents/deferred-domain-cases.json. WS11-ui owns the pages; WS11-api owns REST/MCP. Their surfaces were not implemented or changed here.

| Pinned Rails file | Remaining named comparisons | Owner |
|---|---:|---|
| `test/services/slash_commands/dispatcher_test.rb` | 35 | WS11 agent dispatch; WS8 built-in commands |
| `test/models/channel_thread_agent_assignment_test.rb` | 29 | WS11 agent callbacks; WS12 mutation producers |
| `test/models/user/bot_test.rb` | 6 | WS11 bot domain/removal; WS11-api by-bot HTTP surface |
| `test/models/message_streaming_test.rb` | 4 | WS11 finalization; WS12 activity; WS14/15 external reference sync |
| `test/models/agent_test.rb` | 2 | WS11-ui rendered broadcast cases; WS11 domain cases compared |
| `test/jobs/agent/delivery_job_test.rb` | 2 | WS11 domain |
| `test/models/agent_budgets_test.rb` | 2 | WS11 domain |
| `test/models/agent_kill_switch_test.rb` | 1 | WS11 callbacks; WS12 owned-board mutation producer |


Raw cleanup summary:

```text
WS11 cleanup: removed fresh-clone rust/target; 0 scratch target directories remain; ordinary rust/target cache retained
```

Logs and fresh source remain in this worktree's .scratch, with generated test evidence at .scratch/ws11-179-generated-test-evidence. The ordinary rust/target cache is retained. The report mirror in rust/plans/ws11-report.md and the requested wave4/ws11-report.md are byte-identical.
