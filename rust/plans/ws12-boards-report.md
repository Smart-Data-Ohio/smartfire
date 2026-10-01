# WS12: #182 merge verification

Branch: `rust/ws12-boards`. Merge: `68207058b78beaff559731309ee7265ec614b4d0`, with parents `d8e26efb89ce3be97f4425df72967ff1dcff34f1` and origin/main `0681adc6894ba93d8e279f1590f9856165bdbfcd` (#182).

This run is limited to the requested merge and verification. No new feature work, PR, Rails changes, parity masks or new ignores. The accepted board-write/human-work/recorder slice, its design, failing-first proofs and prior receipts remain in the [report at d8e26efb](https://github.com/Smart-Data-Ohio/smartfire/blob/d8e26efb89ce3be97f4425df72967ff1dcff34f1/rust/plans/ws12-boards-report.md). Those historical commands/results are not claimed as rerun here.

## Merge resolution

- `controllers/channel_threads/writes.rs`: retained #182's entire reviewed channel creation function, formatting only. It preserves client-ID coercion/retry handling, staged signed/uploaded attachments, and attachment processing/analysis enqueue in the source transaction. Kept WS12 board creation, metadata/work/result/lifecycle/delete writes and their attempted validation snapshots.
- `controllers/channel_threads/page_tests.rs`: retained main's new work-content authorization/seam checks and WS12's successful board-post assertion. The ordinary-work `/content` seam remains explicit; completed board post and pane rendering stay active. The automatic merge initially blocked the board pane as well; the original 29-response read test failed on 501 versus Rails 200 before the guard was narrowed in `7b273cc30f960ebb7192bb904b41f70e2190f98a`. The PR-refresh queue test uses `TestApp::boot_frozen().without_job_runner()` so the worker cannot race queue assertions.
- `controllers/channel_threads.rs`: keeps main’s ordinary-work conversation seam while restoring the completed board `/content` endpoint; board list/post/pane behavior remains active.
- `crates/views/templates/channel_threads/board_post.html`: removed one wrapper newline after #182 added the newline to the shared pending-message partial. All nine first-run response mismatches consisted of exactly that single inserted newline; the existing literal-response test failed before this correction. The shared reviewed partial remains unchanged.
- Other #182 changes, including reviewed message fragments, cache dependencies, GitHub headers, and ordinary thread content/provider integration, were merged without removing either side's behavior. Existing reviewed #181 ActivityItem/UserStar dirty-column writes and operation-snapshot broadcasts remain; UserStar is byte-identical to main. ActivityItem's only WS12 changes remain the recorder module/export.
- Locked Cargo metadata succeeded after the merge. All final test/build checks below use a fresh local clone, created at the merge and fast-forwarded through spacing correction `5e52f573c5c925e4c5437e4060e2d6411571d7a5` to board-pane correction `7b273cc30f960ebb7192bb904b41f70e2190f98a`, with only the verified default and first_run parity seeds copied into it. `CI=1` disallows silent missing-seed skips. Build jobs remain 2 under the machine's rustc throttle, test threads 4, and listener ranges 53400–53499.

## Rails reference

Pinned Rails is `d7c7de92` plus the approved drift in `_common.md`. Rechecked the board files against origin/main: `app/models/board_automations/nudge_pusher.rb` uses #162's notification tag; `app/views/channel_threads/_board_post.html.erb` uses #164's body-class correction and #165's message template/current-room meta; `app/views/channel_threads/new.html.erb` uses #164's body-class correction. Those origin/main versions are in `ws12-reference:boards-b908ebc2`; other owned sources remain pinned. The Rails write oracle checks its source hash ledgers before generating the 82 literal responses.

## Scope and remaining work

WS12 remains **partial, not owner-blocked-only**. No owned declarations were closed or newly deferred in this merge; the per-declaration inventory remains [ws12-rails-cases.json](ws12-rails-cases.json) (114 ported, 3 existing peer tests, 371 deferred of 488 declarations). Remaining WS12 work is board tag assignment/auto-assignment and remaining board cases; human handoffs, links, work index/views and remaining audit cases; SLA/nudges/digests and atomic `BoardNudgeJob`; remaining recorder sources and inbox domain integration; agent work/board-post/handoff/presence services and their API/MCP integration. WS11 and #181 are on main, so agent work is unblocked. WS11-UI owns inbox HTTP/rendering, WS12 the APIs beneath it; the lead's next requested slice is those inbox APIs after this PR is frozen. This run stops after push as requested.

## Current verification receipts

All commands below were rerun in this run from the assigned worktree. Raw logs are retained in `.scratch/logs/`.

### Locked metadata

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml > .scratch/logs/ws182-metadata.json
```

Exit 0 after merge and again after the integration corrections; JSON saved in the cited file. No textual summary is emitted.

### Reference seeds and actual Rails response regeneration

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/logs/ws182-default-seed-check.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/logs/ws182-first-run-seed-check.log 2>&1
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/ws182-write-oracle.json 2>.scratch/logs/ws182-write-oracle.log
cmp .scratch/ws182-write-oracle.json rust/vectors/boards_write.json
```

All exit 0. The default and first_run raw seed summaries, respectively:

```text
  "passed": 29,
  "failed": 0
```

```text
  "passed": 4,
  "failed": 0
```

```text
Rails board write oracle: 82 complete HTTP responses; no masks
```

The regenerated JSON equals the committed vector byte for byte. No masks or output normalization.

### Focused final merge checks

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/ws182-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/ws182-clean/source/rust/Cargo.toml --locked -p campfire controllers::channel_threads:: -- --test-threads=4 > .scratch/logs/ws182-channel-threads.log 2>&1
```

```text
test controllers::channel_threads::board_read_tests::board_post_forms_pages_and_panes_match_complete_rails_http_responses ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 1883 filtered out; finished in 64.78s
```

Exit 0 at `7b273cc30f960ebb7192bb904b41f70e2190f98a`: all 30 tests ran; no ignores; 1883 app tests filtered here, then covered by the workspace command. The two literal-response tests compare all 82 write cases and 29 read cases. This also reruns the reviewed ordinary-thread tests in the conflicted files.

### Failures demonstrated before the integration corrections

The original literal write comparator failed at merge `68207058` (nine HTML cases, each exactly one inserted newline):

```text
complete Rails response mismatches: ["owner-result-limit-html", "manager-archive-html", "metadata-tags-error-html", "metadata-name-error-html", "result-error-after-metadata-html", "work-error-after-result-html", "invalid-owner-update-html", "remove-board-work-html", "update-invalid-tags-turbo"]
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1912 filtered out; finished in 181.41s
```

The isolated original read comparator then failed at `5e52f573`, before narrowing the guard:

```text
  left: 501
 right: 200
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1912 filtered out; finished in 30.72s
```

Both are actual failing assertions against the committed implementations, not compilation/setup errors. The existing tests pass on the final source above. Earlier broad attempts were interrupted after those defects surfaced; they are not passing workspace receipts. Their logs remain separate (`ws182-workspace-interrupted.log`, `ws182-workspace-before-pane-fix.log`). No new security contract or test was added in this merge; prior slice discrimination is linked at the top.

### Full workspace

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/ws182-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/ws182-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/logs/ws182-workspace-test.log 2>&1
```

```text
test result: ok. 1910 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 647.33s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.73s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1199 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 125.07s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.44s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.56s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.45s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.11s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.69s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

Exit 0 at committed `7b273cc30f960ebb7192bb904b41f70e2190f98a`. Sum of all 60 raw unit/integration/doctest summaries: **3825 passed, 0 failed, 12 existing ignored**. Includes the vendored html5ever workspace member. No missing-seed skips. Existing ignored declarations (none added):

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
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

### Strict clippy and production-only inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/ws182-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/ws182-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/logs/ws182-clippy.log 2>&1
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/ws182-clean/.scratch" CI=1 mise exec rust@1.98.1 -- .scratch/ws182-clean/source/rust/ci/with-release-inputs.sh cargo check --locked -p campfire --bin campfire > .scratch/logs/ws182-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.22s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.90s
```

Both exit 0 on final committed source, in that order. Strict clippy has zero warnings. The release guard checked the production binary target with only crates/manifests and the separate allowed asset inputs, without vectors, parity data or reference tools.

## Cleanup and handoff

All test/build/reference processes completed. No WS12 container or listener in 53400–53499 remains. Removed the measured 19G `.scratch/target` and the 1.7M diagnostic-only `.scratch/ws182-clean/source/rust/target`; both are verified absent, as is root `rust/target`. Raw logs and the before-fix HTML/text diagnostics are retained under this worktree’s `.scratch/logs/` and `.scratch/diagnostics/ws182/`. Reference images, media tools, seeds and source checkouts remain. The Python model server was not touched.

The final report commit changes documentation only; production source is the verified `7b273cc3` descendant of merge `68207058`. Latest checked origin/main remains `0681adc6894ba93d8e279f1590f9856165bdbfcd`, already included. Push and stop; inbox domain APIs are the next lead-requested slice after the PR is frozen. **WS12 is still partial with unblocked owned work, not owner-blocked-only.**
