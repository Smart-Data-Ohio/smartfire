# WS12 — PR #193 round-two query fixes

Branch: `rust/ws12-boards`. All four round-two review requests are addressed. Failing-first regression commit: `03e4cd534`; production fix: `5c9a8270940c1e99dc5468030a5e5ac00f566b55`; verified merged source: `d239c89ebbd467625e11008d2e5f1b1b879e756a`. The final report commit changes documentation only. This run stops after the requested review fixes and verification. Overall WS12 remains partial, with unblocked recorder/automation work remaining; this is **not an owner-blocked-only stop**.

## Changes and boundaries

- `controllers/presenters/work_threads.rs` loads the page's rooms, display names and owner availability once across all rows. `models/channel_thread/board.rs` exposes `ChannelThread::work_owners`, with membership and capability facts keyed by room and owner; the existing single-room adapter shares that implementation. `presenters/boards.rs` uses the same owner facts without a duplicate user preload.
- `models/agent_access.rs` and `models/agent.rs` expose the batch capability reader. It preserves the existing single-agent policy: active account, suspension, recognized capabilities, soft-deleted rooms, revoked-grant suppression of legacy access, global grants and room scope. It is a request-local read snapshot; mutation authorization continues to perform its existing fresh checks. `models/membership.rs` supplies the room batch reader.
- `models/direct_room.rs` batches unnamed direct-room members while preserving the ordinary association's SQLite `LOWER(name)` order. It does not switch to the differently ordered Ruby/preloaded-member path. Named rooms and viewer-only/empty-room fallbacks retain their existing behavior.
- `models/calendar_event.rs` exposes `work_link_candidates`; `presenters/board_posts.rs` fetches the event-picker rows in one query, retaining cancellation/time/link exclusion and `starts_at,id` ordering. Empty history actor lists need no user lookup.
- `controllers/channel_threads.rs` requests only the owner facts used by an ordinary work pane. It skips board reply/link counts, tags, duplicate lifecycle/room work and the duplicate owner preload. `presenters/github.rs` renders from the PR association already loaded for refresh intent, preserving the existing private-safe card adapter and refresh behavior.
- `models/work_thread_event.rs` and `models/channel_thread.rs` expose unscoped association batches. `presenters/activity.rs` replaces the flagged per-item WorkThreadEvent fact readers with preloaded events, threads, rooms and actors, **after** ActivityItem's accessibility query. Snapshot owner names, actor fallbacks, body truncation, source URLs, pagination and authorization are unchanged. Room names are shared within the request. `controllers/activity_items.rs` uses the expanded `Sources` adapter. Message-only inbox pages skip the empty actor preload, preserving #188's existing query regression.
- `controllers/human_work_tests/query_round_two.rs` adds five real HTTP/SQL regressions at 10 and 100 rows; `human_work_tests.rs` registers them. `models` tests in `tests/work_read_test.rs` check batch/single-agent policy equivalence after mutations, cross-room owner availability, and direct-room name parity. `reference-tools/work/query_round_two.rb` independently repeats the corresponding actual Rails requests and counts uncached SQL.

Cross-workstream touches are shared read APIs in WS8's room/event/membership models, WS11's agent policy, WS11-UI's activity presenter/controller, and WS15g's PR header adapter. No authorization writer, queue writer, activity/star dirty-column implementation or broadcast snapshot implementation was replaced. The frozen `rust/ws12b-board-writes` branch was not touched. No new feature work was started.

## Failing-first evidence

The worktree began clean apart from the existing untracked `.scratch/`, at `9467e3983fd7bac73221cf7ce69a061b6e133532`. Regression commit `03e4cd534` adds only tests and the Rails producer; its production code is exactly `9467e398`. The first runnable regression suite failed all five tests on their query assertions, after measuring both sizes. There were no fixture/HTTP errors in that run.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire human_work_tests::query_round_two -- --test-threads=2 --nocapture > .scratch/pr193-r2/logs/query-red.log 2>&1
```

```text
WS12 R2 inbox JSON rows=10: 24 reader SQL
WS12 R2 index bots rows=10: 105 reader SQL
WS12 R2 inbox HTML rows=10: 52 reader SQL
WS12 R2 index bots rows=100: 915 reader SQL
test controllers::human_work_tests::query_round_two::index_batches_distinct_agent_owners_and_rooms ... FAILED
WS12 R2 inbox JSON rows=100: 204 reader SQL
WS12 R2 inbox HTML rows=100: 412 reader SQL
test controllers::human_work_tests::query_round_two::inbox_batches_work_events_and_associations_in_json_and_html ... FAILED
WS12 R2 index humans rows=10: 45 reader SQL
WS12 R2 pane fixed reads history=10: 40 reader SQL; 3 unused board facts; 4 PR association reads
WS12 R2 index humans rows=100: 315 reader SQL
test controllers::human_work_tests::query_round_two::index_batches_distinct_human_owners_and_rooms ... FAILED
WS12 R2 pane fixed reads history=100: 40 reader SQL; 3 unused board facts; 4 PR association reads
test controllers::human_work_tests::query_round_two::ordinary_pane_skips_unused_board_facts_and_duplicate_pr_association ... FAILED
WS12 R2 pane event options rows=10: 47 reader SQL
WS12 R2 pane event options rows=100: 137 reader SQL
test controllers::human_work_tests::query_round_two::pane_batches_upcoming_unlinked_events ... FAILED
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 2244 filtered out; finished in 3.29s
```

The regression command exits **101**, as expected against the baseline. The two index tests fail Rails-growth/flatness assertions; the pane choices and both inbox formats fail flatness assertions; the fixed-read test fails the unused-board-facts assertion. Both sizes are measured before each assertion.

## Final merged query measurements

Counts are actual reader SQL for the served HTTP request, with FrozenClock and stopped job runners. Sizes are **10 / 100**. Each fresh seeded app excludes setup/authenticity-token priming from the trace. The index probes assert all requested rows appear and inaccessible sentinel rows do not.

| Probe | Before `9467e398` | After merged `d239c89e` |
|---|---:|---:|
| HTML index, distinct human owners and rooms | 45 / 315 | 17 / 17 |
| HTML index, distinct bot owners and rooms | 105 / 915 | 19 / 19 |
| Pane, upcoming unlinked event choices | 47 / 137 | 27 / 27 |
| Work-event inbox JSON | 24 / 204 | 8 / 8 |
| Work-event inbox HTML | 52 / 412 | 16 / 16 |
| Ordinary pane, history plus private PR | 40 / 40 | 29 / 29 |
| Unused pane board counts/tags | 3 / 3 | 0 / 0 |
| Pane PR association/record reads | 4 / 4 | 2 / 2 |

All five new regressions pass. The original shared-room/owner index remains flat, **18/18 → 17/17**; ordinary repeated-actor history **38/38 → 28/28**; board repeated-actor history **46/46 → 45/45**. Those original baseline counts are Astra-confirmed in the read-only round-two summary; their final counts are rerun here.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-r2-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr193-r2-clean/source/rust/Cargo.toml --locked -p campfire human_work_tests::query -- --test-threads=2 --nocapture > .scratch/pr193-r2/logs/query-final.log 2>&1
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.21s
WS12 R2 inbox JSON rows=10: 8 reader SQL
WS12 R2 index bots rows=10: 19 reader SQL
WS12 R2 inbox HTML rows=10: 16 reader SQL
WS12 R2 inbox JSON rows=100: 8 reader SQL
WS12 R2 index bots rows=100: 19 reader SQL
WS12 R2 inbox HTML rows=100: 16 reader SQL
WS12 R2 index humans rows=10: 17 reader SQL
WS12 R2 pane fixed reads history=10: 29 reader SQL; 0 unused board facts; 2 PR association reads
WS12 R2 index humans rows=100: 17 reader SQL
WS12 R2 pane fixed reads history=100: 29 reader SQL; 0 unused board facts; 2 PR association reads
WS12 R2 pane event options rows=10: 27 reader SQL
WS12 ordinary history=10: 28 reader SQL
WS12 board history=10: 45 reader SQL
WS12 R2 pane event options rows=100: 27 reader SQL
WS12 ordinary history=100: 28 reader SQL
WS12 board history=100: 45 reader SQL
WS12 index rows=10: 17 reader SQL; 0 unused event-option queries
WS12 index rows=100: 17 reader SQL; 0 unused event-option queries
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2288 filtered out; finished in 4.11s
```

Independent pinned Rails requests are reproducible with the committed producer:

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/query_round_two.rb" > .scratch/pr193-r2/rails-query-counts.json 2> .scratch/pr193-r2/logs/rails-queries.log
```

```text
Rails R2 human_rooms size=10: 35 SQL; 4 cached
Rails R2 human_rooms size=100: 213 SQL; 4 cached
Rails R2 agent_rooms size=10: 63 SQL; 4 cached
Rails R2 agent_rooms size=100: 513 SQL; 4 cached
Rails R2 pane_options size=10: 22 SQL; 3 cached
Rails R2 pane_options size=100: 22 SQL; 3 cached
Rails R2 inbox_json size=10: 9 SQL; 18 cached
Rails R2 inbox_json size=100: 9 SQL; 198 cached
Rails R2 inbox_html size=10: 10 SQL; 27 cached
Rails R2 inbox_html size=100: 10 SQL; 297 cached
```

The small cold Rails human index includes two extra reads (35 here versus the review's warm 33); the event probe is already warm here (22/22 versus the review's 24/22). Large counts and bot/inbox counts match the review. Rust's slope is zero for each requested probe, below every Rails slope/budget. No query counts are estimated or simulated.

## Response-byte oracles and reference provenance

All four Rails producers were rerun, and their entire generated JSON files compared to the committed corpora with `cmp`, exit 0. Main's merge changes none of these four corpora. The final fresh-clone workspace runs the consuming Rust tests again. The 90 human HTTP responses, 29 link model cases and 96 board responses are compared without masks. The 86 #188 inbox cases retain #188's existing CSRF-only form/meta-token normalization; no normalization or masks were added in this run. Their status/header, source, access and mutation checks pass.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/pr193-r2/human_work_http.json 2> .scratch/pr193-r2/logs/human-oracle.log
cmp rust/vectors/human_work_http.json .scratch/pr193-r2/human_work_http.json
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/pr193-r2/work_link_model.json 2> .scratch/pr193-r2/logs/link-oracle.log
cmp rust/vectors/work_link_model.json .scratch/pr193-r2/work_link_model.json
```

```text
Rails work link model oracle: 29 validation/persistence cases; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/pr193-r2/boards_write.json 2> .scratch/pr193-r2/logs/board-oracle.log
cmp rust/vectors/boards_write.json .scratch/pr193-r2/boards_write.json
```

```text
Rails board write oracle: 96 complete HTTP responses; no masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/views/agents_ui/inbox_http.rb" > .scratch/pr193-r2/inbox-http.json 2> .scratch/pr193-r2/logs/inbox-oracle.log
cmp rust/vectors/inbox-http.json .scratch/pr193-r2/inbox-http.json
```

```text
inbox Rails HTTP oracle: 86 responses; 11 source types; reference d7c7de92
```

```text
test controllers::activity_items::tests::ws11ui_inbox_http_matches_pinned_rails_bytes_and_permissions ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
```

Reference is Rails `d7c7de92` plus the approved board drift. For board/work/activity reference files affected by approved drift, **origin/main's Rails is the reference**, as instructed. The reference image's nudge pusher, board-post/new templates and application layout are byte-identical to the versions on merged main; the work producer's existing source-hash ledger also passed. No Rails reference files were edited.

```sh
sha256sum app/models/board_automations/nudge_pusher.rb app/views/channel_threads/_board_post.html.erb app/views/channel_threads/new.html.erb app/views/layouts/application.html.erb
docker run --rm --entrypoint sha256sum ws12-reference:boards-b908ebc2 /rails/app/models/board_automations/nudge_pusher.rb /rails/app/views/channel_threads/_board_post.html.erb /rails/app/views/channel_threads/new.html.erb /rails/app/views/layouts/application.html.erb
```

```text
04c5ad74ad14463e99a987c8fadb52a02f4e26b4f83240797ff54deb00ff51ee  app/models/board_automations/nudge_pusher.rb
cdc80236fbb1ced4e70019d961e0795dae8e3a29352b8543ea8053b60aa4d4a2  app/views/channel_threads/_board_post.html.erb
a13234df714707b0d46477a924c6231866fc92b0f46f205d83f6e6ebd881190a  app/views/channel_threads/new.html.erb
53008a50df6bf20f53013c146ef0584690cdd1061b5b748d5bf2035f3fd854c9  app/views/layouts/application.html.erb
04c5ad74ad14463e99a987c8fadb52a02f4e26b4f83240797ff54deb00ff51ee  /rails/app/models/board_automations/nudge_pusher.rb
cdc80236fbb1ced4e70019d961e0795dae8e3a29352b8543ea8053b60aa4d4a2  /rails/app/views/channel_threads/_board_post.html.erb
a13234df714707b0d46477a924c6231866fc92b0f46f205d83f6e6ebd881190a  /rails/app/views/channel_threads/new.html.erb
53008a50df6bf20f53013c146ef0584690cdd1061b5b748d5bf2035f3fd854c9  /rails/app/views/layouts/application.html.erb
Approved Rails board drift: 4/4 source hashes match origin/main
```

## Merge and fresh-clone verification

`origin/main` moved from `573762b5` to `056ab49acffd52007334356ada0bdba3774c0fe2` (#191). Merge commit `d239c89ebbd467625e11008d2e5f1b1b879e756a` keeps both sides without conflicts; reviewed main message/cache/room behavior is retained. A later fetch after the full workspace and clippy found main unchanged. No stash or rebase was used.

```sh
git fetch origin main
git merge --no-ff origin/main -m 'Merge main into PR193 round two query fixes' -m 'Co-Authored-By: GPT-6.1 Sol <noreply@openai.com>'
git clone --local --no-hardlinks --single-branch --branch rust/ws12-boards . .scratch/pr193-r2-clean/source
mkdir -p .scratch/pr193-r2-clean/source/rust/parity/.seed .scratch/pr193-r2-clean/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run rust/parity/.seed/agents_ui .scratch/pr193-r2-clean/source/rust/parity/.seed/
```

All three restored seeds were independently verified against Rails in this run, before testing with `CI=1` (missing seeds fail, rather than silently skipping).

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/pr193-r2/logs/default-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/pr193-r2/logs/first-run-seed.log 2>&1
```

```text
  "passed": 4,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/pr193-r2/logs/agents-ui-seed.log 2>&1
```

```text
  "passed": 40,
  "failed": 0
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-r2-clean/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/pr193-r2-clean/source/rust/Cargo.toml --locked --format-version 1 > .scratch/pr193-r2/logs/metadata.json
```

```text
Locked workspace metadata: exit 0
```

### Full workspace, including vendored html5ever and doctests

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-r2-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/pr193-r2-clean/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=4 > .scratch/pr193-r2/logs/workspace-test.log 2>&1
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6m 26s
test result: ok. 2290 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 741.34s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.43s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1273 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 189.64s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.51s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.82s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.27s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.91s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.33s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.70s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Workspace aggregate: 4300 passed; 0 failed; 14 ignored; 61 target summaries; exit 0
```

The 14 existing ignores are listed verbatim below; none was added or broadened by this change. All eight `work_read_test` cases, including the three new policy/name checks, ran and passed in this full gate.

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ignored, pending WS11-API PR: /agents/events shape at 60d97bd is not on main
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
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-r2-clean/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/pr193-r2-clean/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/pr193-r2/logs/clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 36s
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/pr193-r2-clean/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/pr193-r2-clean/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/pr193-r2/logs/release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 05s
```

Both exit 0; strict clippy has zero warnings. The release-input guard supplies only crates/manifests plus its explicit separate asset inputs, with no vectors, reference tools or parity files available to production code. This verifies a production `cargo check`, rather than a linked release-profile build.

## Cleanup and remaining work

CARGO_BUILD_JOBS stayed at 2, with the machine-wide rustc wrapper unchanged. The full suite used four test threads; focused suites used two. Listeners use the assigned 53400–53499 test ranges. The python model server was untouched.

Cleanup measured `.scratch/target` at 26G and the fresh clone's generated diagnostic target at 40K, checked for live WS12 compiler/test processes, then removed those two regenerable directories. Seeds, native media inputs, source checkout and raw logs are retained.

```text
Removed 2 WS12 scratch target directories; no active WS12 compiler/test processes.
Remaining target directories under WS12 .scratch: 0
```

The four review requests are complete. Overall WS12 remains **partial**: the remaining recorder/inbox sources (including the BoardSlaNudge facts/recorder adapter) and SLA/nudge/digest automation services, FrozenClock coverage and atomic `BoardNudgeJob { nudge_id }` source writes remain unblocked owned work. This is **not an owner-blocked-only stop**. No additional feature work was started in this review-fix run.
