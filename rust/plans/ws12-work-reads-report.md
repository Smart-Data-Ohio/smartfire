# WS12 work JSON and opening-recorder read growth

The requested read-growth slice is complete on `rust/ws12-work-reads`. Verified source: `4f5edf9b9ef2bb2149292c2806e428fab256e41c`. This branch starts from `033ab0ca` and includes current main `b98904b88032edfc8e06e2b765380a1d944e9afa` through merge commits. `rust/ws12-board-automations` remains unchanged at `b7829b1e3daed923b298babf0b509d27212f678c` (#200). The final publication commit adds only this report.

## Implementation and scope

`ChannelThread::work_read_facts(conn, threads, viewer)` returns a typed `WorkReadFacts` page snapshot. Rooms, users, owner availability and agent grants, viewer room/thread membership, message/member counts and custom avatar-icon names are loaded in batches. Its permission facts preserve the model's administrator/room creator/thread creator and current owner rules. It is read-only data, not mutation authority; writes retain their live transactional checks. The work JSON index uses those facts once across the page. Ordinary thread reads and the index share one response serializer, preserving field order, nulls, avatar paths, lifecycle and permission fields. The existing HTML index stays on its reviewed batch path.

Opening creation reuses its already-authorized human roster and persisted opener through the crate-private `ActivityItem::record_authorized_board_opener(tx, &User, &Message)`. Active humans, creator exclusion, invisible memberships, everything-followers and explicit owners keep the original recipient rules. The recorder keeps grouping, idempotency, dirty-state writes and transaction/commit behavior. It broadcasts the item already loaded by the operation and reuses the recipient snapshot, removing duplicate item/user/message loads. Unread grouping broadcasts still occur only when the old read timestamp is cleared; source/time-only changes do not broadcast.

Two regressions measure actual SQLite reads at both requested sizes. The JSON regression additionally checks distinct rooms/human owners, distinct agent owners with explicit grants, and distinct custom icons. All eight complete JSON responses and all 208 opening inbox rows are pinned Rails facts, with no masks. Inbox rows compare IDs, user/source/event identities, read/handled state and both timestamps. No feature work or Rails source edits are included.

## Before and after reads

JSON pairs are 10 / 100 tracked rows. Opening pairs are 9 / 199 recipients (10 / 200 board members). JSON counts cover the real HTTP request after the review's HTML warm-up; opening counts trace SELECT/WITH statements during the entire source write. Before is unchanged `033ab0ca` production, not an estimate or a fixture error.

| Probe | Rust before | Rust after | Rails measured here |
|---|---:|---:|---:|
| Work JSON, shared room/owner | 95 / 905 | 12 / 12 | 38 / 308 |
| Work JSON, distinct human owners/rooms | 105 / 1,005 | 12 / 12 | 58 / 508 |
| Work JSON, distinct agent owners/rooms | 165 / 1,605 | 14 / 14 | 88 / 808 |
| Work JSON, distinct custom icons | 125 / 1,205 | 13 / 13 | 58 / 508 |
| Opening creation, total reads | 110 / 1,250 | 74 / 454 | 41 / 611 |
| Opening creation, user reads | 23 / 403 | 5 / 5 | Not separately measured |

JSON growth is zero. Opening growth is 380 reads for 190 additional recipients, against Rails' 570: two per extra recipient against three. Rust's fixed opening cost is still higher at nine recipients; this meets the requested growth bar and does not claim identical absolute counts.

## Failing-first proof

The original two-case regressions ran before any production edit. Both complete response/fact comparisons passed before their read-growth assertions failed.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked -p campfire query_read_growth -- --test-threads=2 --nocapture > .scratch/work-reads/logs/fail-first.log 2>&1
```

```text
WS12 opening recipients=9: 110 SELECTs; 23 user reads
WS12 opening recipients=199: 1250 SELECTs; 403 user reads
opening read growth must not exceed Rails' [41, 611]: [110, 1250]
WS12 work JSON rows=10: 95 reader SQL
WS12 work JSON rows=100: 905 reader SQL
JSON read growth must not exceed Rails' [38, 308]: [95, 905]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2418 filtered out; finished in 1.70s
```

The expanded vector was then proved against the exact same unchanged production files, temporarily replayed from `033ab0ca` and restored byte for byte in a `finally` block. The driver requires both runtime failures, all size measurements and passing full response/fact comparisons. Compile errors cannot satisfy it. The first optional icon-fixture attempt exposed Rails cache reuse after raw SQL setup; the producer now invokes `Icons.expire_custom_cache!` between fixtures, matching the invalidation normally provided by WorkspaceIcon writes. The corrected proof below fails only the two growth assertions; the earlier setup log is retained separately in scratch.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 python3 .scratch/work-reads/prove-main.py > .scratch/work-reads/logs/expanded-fail-first-driver.log 2>&1
```

```text
WS12 opening recipients=9: 110 SELECTs; 23 user reads
WS12 work JSON shared rows=10: 95 reader SQL
WS12 opening recipients=199: 1250 SELECTs; 403 user reads
opening read growth must not exceed Rails' [41, 611]: [110, 1250]
WS12 work JSON shared rows=100: 905 reader SQL
WS12 work JSON humans rows=10: 105 reader SQL
WS12 work JSON humans rows=100: 1005 reader SQL
WS12 work JSON agents rows=10: 165 reader SQL
WS12 work JSON agents rows=100: 1605 reader SQL
WS12 work JSON icons rows=10: 125 reader SQL
WS12 work JSON icons rows=100: 1205 reader SQL
agents JSON read growth must not exceed Rails' [88, 808]: [165, 1605]
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2418 filtered out; finished in 3.92s
```

```text
Both regressions reject unchanged 033ab0ca production at both sizes; all response/fact checks passed; restored every production file byte for byte.
```

The fixed source passes the same expanded regression, with every complete response and recorder-fact assertion enabled.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/work-reads-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/work-reads-merged/source/rust/Cargo.toml --locked -p campfire query_read_growth -- --test-threads=2 --nocapture > .scratch/work-reads/logs/provider-query.log 2>&1
```

```text
WS12 opening recipients=9: 74 SELECTs; 5 user reads
WS12 work JSON shared rows=10: 12 reader SQL
WS12 opening recipients=199: 454 SELECTs; 5 user reads
WS12 work JSON shared rows=100: 12 reader SQL
WS12 work JSON humans rows=10: 12 reader SQL
WS12 work JSON humans rows=100: 12 reader SQL
WS12 work JSON agents rows=10: 14 reader SQL
WS12 work JSON agents rows=100: 14 reader SQL
WS12 work JSON icons rows=10: 13 reader SQL
WS12 work JSON icons rows=100: 13 reader SQL
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2431 filtered out; finished in 3.86s
```

## Rails differential and source pin

Pinned Rails is `d7c7de92` plus the approved board drift. **For the approved board/work/activity files, origin/main's Rails is the reference.** The new producer validates seven committed source hashes (work controller, thread model, message payload helper, activity model/recorder and icon model/helper). All seven match origin/main; the approved drift files also match our reference image `ws12-reference:boards-b908ebc2`. The existing 90/29/96 response corpora were unchanged; no masks were added or widened.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/work/read_growth.rb > .scratch/work-reads/read-growth-final.json 2> .scratch/work-reads/logs/rails-read-growth-final.log
cmp rust/vectors/work_read_growth.json .scratch/work-reads/read-growth-final.json
```

```text
Rails work JSON shared size=10: 38 SELECTs; complete response; 0 masks
Rails work JSON shared size=100: 308 SELECTs; complete response; 0 masks
Rails work JSON humans size=10: 58 SELECTs; complete response; 0 masks
Rails work JSON humans size=100: 508 SELECTs; complete response; 0 masks
Rails work JSON agents size=10: 88 SELECTs; complete response; 0 masks
Rails work JSON agents size=100: 808 SELECTs; complete response; 0 masks
Rails work JSON icons size=10: 58 SELECTs; complete response; 0 masks
Rails work JSON icons size=100: 508 SELECTs; complete response; 0 masks
Rails board opening recipients=9: 41 SELECTs; 9 complete recorder facts
Rails board opening recipients=199: 611 SELECTs; 199 complete recorder facts
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/human_http.rb" > .scratch/work-reads/human_work_http.json 2> .scratch/work-reads/logs/human-oracle.log
cmp rust/vectors/human_work_http.json .scratch/work-reads/human_work_http.json
```

```text
Rails human work HTTP oracle: 90 complete responses; committed handoffs; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/work/link_model.rb" > .scratch/work-reads/work_link_model.json 2> .scratch/work-reads/logs/link-oracle.log
cmp rust/vectors/work_link_model.json .scratch/work-reads/work_link_model.json
```

```text
Rails work link model oracle: 29 validation/persistence cases; 0 masks
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference exec --seed default --time 2026-03-02T16:00:00Z --freeze -- bin/rails runner --skip-executor /work/reference-tools/boards/write_views.rb > .scratch/work-reads/boards_write.json 2> .scratch/work-reads/logs/board-oracle.log
cmp rust/vectors/boards_write.json .scratch/work-reads/boards_write.json
```

```text
Rails board write oracle: 96 complete HTTP responses; no masks
```

All four producer commands and all four complete-file comparisons exit 0. The fresh-clone workspace runs their Rust consumers again, including the reviewed inbox/WS12 consumer vectors and recorder/callback regressions.

## Fresh clone and verified seeds

```sh
git fetch origin
git clone --local --no-hardlinks --single-branch --branch rust/ws12-work-reads . .scratch/work-reads-merged/source
mkdir -p .scratch/work-reads-merged/source/rust/parity/.seed .scratch/work-reads-merged/.scratch
cp -a rust/parity/.seed/default rust/parity/.seed/first_run rust/parity/.seed/agents_ui .scratch/work-reads-merged/source/rust/parity/.seed/
```

```sh
git -C .scratch/work-reads-merged/source fetch origin rust/ws12-work-reads
git -C .scratch/work-reads-merged/source merge --ff-only FETCH_HEAD
```

Main first moved from `033ab0ca93a567bb7ddaf239ca29db9d1253a9ff` to `a0c06de985cb8d361cfd29e9a0e5964ec5fc534f` (#200). Merge `11d1b564cdd6dadfc7bc6e0e3a2fbadad3ba40e5` keeps both the reviewed nudge source/reader APIs and these read fixes; the recorder merged cleanly. Main later moved to `b98904b88032edfc8e06e2b765380a1d944e9afa` (#198). Merge `4f5edf9b9ef2bb2149292c2806e428fab256e41c` keeps its reviewed provider callback/preload behavior without conflicts. The earlier fresh clones passed 4,425 and 4,433 tests respectively before those subsequent main updates. The second fresh clone is fast-forwarded to `4f5edf9b9ef2bb2149292c2806e428fab256e41c` and all final gates below repeat on that source, with no uncommitted implementation files. All three restored seeds were independently reverified against Rails in this run. `CI=1` prevents missing seeds from silently skipping tests.

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" default > .scratch/work-reads/logs/default-seed.log 2>&1
```

```text
  "passed": 29,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" first_run > .scratch/work-reads/logs/first-run-seed.log 2>&1
```

```text
  "passed": 4,
  "failed": 0
```

```sh
env PARITY_NAMESPACE=ws12 PARITY_OWNER=ws12 PARITY_RUNTIME=docker PARITY_IMAGE=ws12-reference:boards-b908ebc2 rust/parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze "$PWD/rust/reference-tools/campfire/verify_parity_seed.rb" agents_ui > .scratch/work-reads/logs/agents-ui-seed.log 2>&1
```

```text
  "passed": 40,
  "failed": 0
```

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/work-reads-merged/.scratch" CI=1 mise exec rust@1.98.1 -- cargo metadata --manifest-path .scratch/work-reads-merged/source/rust/Cargo.toml --locked --format-version 1 > .scratch/work-reads/logs/provider-metadata.json
```

Locked metadata exits 0. No lockfile change was needed.

## Full workspace, strict clippy and production inputs

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/work-reads-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo test --manifest-path .scratch/work-reads-merged/source/rust/Cargo.toml --locked --workspace --no-fail-fast -- --test-threads=8 > .scratch/work-reads/logs/provider-workspace-test.log 2>&1
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.31s
test result: ok. 2426 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 391.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.32s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1278 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 161.59s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.97s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.47s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 24.19s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.57s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.62s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.55s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
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
```

```text
Workspace aggregate: 4441 passed; 0 failed; 16 ignored; 61 target summaries; exit 0
```

```text
test controllers::activity_items::tests::board_nudge::board_nudge_inbox_matches_rails_complete_responses_and_permissions ... ok
test controllers::human_work_tests::query_read_growth::opening_fanout_read_growth_meets_rails_and_preserves_recorder_facts ... ok
test controllers::activity_items::tests::ws11ui_inbox_http_matches_pinned_rails_bytes_and_permissions ... ok
test controllers::human_work_tests::query_read_growth::work_json_read_growth_meets_rails_and_preserves_complete_responses ... ok
test controllers::channel_threads::board_write_tests::board_writes_match_complete_rails_responses_without_masks ... ok
test controllers::message_features::ws12_consumer_tests::ws12_board_message_consumers_match_rails_through_the_real_inbox ... ok
test controllers::human_work_tests::human_work_http_matches_complete_rails_responses ... ok
test integrations::github::notifier::tests::github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails ... ok
test tests::activity_item_test::ws12_activity_stale_handle_writes_its_dirty_read_timestamp ... ok
test tests::activity_item_test::ws12_activity_stale_handle_does_not_restore_concurrently_cleared_read ... ok
test tests::activity_item_test::ws12_activity_stale_huddle_read_broadcast_uses_its_updated_snapshot ... ok
test tests::activity_item_test::ws12_activity_stale_huddle_unhandle_broadcast_uses_its_updated_snapshot ... ok
test tests::activity_item_test::ws12_activity_stale_read_preserves_concurrent_handling ... ok
test tests::activity_item_test::ws12_activity_stale_unhandle_does_not_restore_concurrently_cleared_read ... ok
test tests::activity_item_test::ws12_activity_stale_unread_only_clears_previously_read_column ... ok
test tests::activity_item_test::ws12_activity_stale_unread_only_clears_previously_handled_column ... ok
test tests::agent_approval_cases_test::ws11_approval_inbox_recipients_commit_separately_after_primary_like_rails ... ok
test tests::agent_approval_cases_test::ws11r_approval_inbox_failure_retains_primary_record_like_rails ... ok
test tests::agent_approval_test::ws11_approval_decision_expiry_cancellation_and_inbox_match_rails ... ok
test tests::work_thread_link_test::work_link_models_match_rails_validations_and_persistence ... ok
test agents_ui::inbox_page_bytes_match_pinned_rails ... ok
```

All pre-existing ignores remain unchanged; no new ignore or seed skip was introduced. Complete raw logs remain under `.scratch/work-reads/logs/`.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/work-reads-merged/.scratch" CI=1 CABLE_TEST_PORT_RANGE=53420-53449 MAIL_TEST_PORT_RANGE=53400-53419 GITHUB_TEST_PORT_RANGE=53450-53499 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/work-reads-merged/source/rust/Cargo.toml --locked --workspace --all-targets -- -D warnings > .scratch/work-reads/logs/provider-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.65s
```

Strict clippy exits 0 with zero warnings.

```sh
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/target" TMPDIR="$PWD/.scratch/work-reads-merged/.scratch" CI=1 LD_LIBRARY_PATH="$PWD/.scratch/rails-media/native-libs" PATH="$PWD/.scratch/rails-media/usr/bin:$PATH" .scratch/work-reads-merged/source/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo check --locked -p campfire > .scratch/work-reads/logs/provider-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
```

The release-input guard exits 0. It checks production compilation with only crates/manifests and the explicit asset inputs: no vectors, parity or reference tools are available. This is a production `cargo check`, not a linked release-profile build.

## Cleanup and delivery

```text
28G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/work-reads-clean/source/rust/target
40K	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws12/.scratch/work-reads-merged/source/rust/target
Removed 3 WS12 scratch target directories; no active WS12 compiler/test processes.
Remaining target directories under WS12 .scratch: 0
```

Machine-wide rustc throttling was left unchanged; Cargo jobs stayed at two, focused test threads at two and full workspace threads at eight. Listener ranges stayed within 53400–53499. No model-server process was touched. Seeds, native media inputs, the source clone and raw evidence remain; all owned scratch Cargo targets were removed.

Both requested read-growth follow-ups are complete. #200 was left alone and no new automation feature work was started. This stop follows the requested scope boundary, not an owner-blocked-only condition. The reviewed #200 checkpoint enters only through main's merge. Remaining SLA/settings/recurring-dispatcher feature work stays on the separate automation track; overall WS12 remains partial there. No stash, rebase or frozen-branch edit was used.

Final origin refresh retained `b98904b88032edfc8e06e2b765380a1d944e9afa`; `git rev-list --count HEAD..origin/main` printed `0`. Both main updates are included through merge commits.
