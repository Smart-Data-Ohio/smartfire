# WS8bm2 message features E — provider runtime checkpoint; wider cutover scope partial

Branch `rust/ws8bm2-message-features-e` starts at D `6211c93fcbf309c5f33d85ade63a6b8d67a00d3f`. D's branch was not changed. Initial main merge `7c13d365ff153a61b1f526aa2aab4b4ab075562d` included main `b573dd24c`; implementation is `c7e7ec364d162fdfbcf01714e37469ad670c6725`. Checkpoint merge `00eb5e98a6aa3bee353d410d7338c4a75ba8ec9c` included #204/main `500c3f698`. After #201 landed, final merge **`0cb5b5d7a04fee32a4cac959402aeeb36f5f3d34`** included main **`692f998ddbcb1d962cccf2c3911693955fafb70f`**. All main merges kept both sides without conflicts; locked metadata passed immediately after each. The final report commit changes documentation only; its pushed SHA is in the final reply.

This is a coherent PR checkpoint, **not cutover-ready and not owner-blocked-only**. No deployment or PR was created. Date/coercion work was not started; #196 remained OPEN at the final source checkpoint.

## Complete in E

- **Calendar callbacks:** real root and reply streams outside both current 40-message windows; ordinary/populated sibling cards, suppression, title edits, two saves in one transaction, Meet-link edits, cancellation, rollback and reference deletion before commit. All **196** distinct pinned Rails frame strings compare directly, without masks. Query-only replay also checks 160 of those frames.
- **Calendar batching:** Rails looks up references in `after_update_commit`. The previous Rust implementation captured references in the writer and sent a frame for a reference subsequently removed before commit. A typed `Event#broadcast_event_card_updates` identity now resolves references after commit. The existing record guard suppresses a callback followed by destroy; same-record saves coalesce while retaining registration order. Model code carries no HTML. The app loads messages, rooms and card facts in ordered batches of 1,000. Event associations use one JSON bind; organizer and venue lists are independently bounded. **999/1,000/1,001** references produce **3,000** ordered frames with exact Rails SHA256, first and last frames. No buffer, timeout or test-thread setting changed.
- **Calendar jobs:** **16** actual registered `Calendar::MeetLinkJob` executions use the production Google calendar consumer, encrypted owner account API and recorded Google client. PUT/PATCH methods, paths and JSON request values match Rails. Success publishes **20** exact root/reply frames; pending conference creation retries twice; forbidden responses complete; missing account, cancelled event, existing link, disconnected account and deleted event are ten guarded no-ops. This is owner-client injection, not a real Google network test.
- **Generic/LinkedIn network jobs:** **20** durable `LinkEmbed::FetchJob` executions use the actual owner fetcher and real TLS/HTTP against whitelist-only local fake hosts. Success plus guarded image HEAD, login/no usable metadata, HTTP 502, public redirect and private-image rejection cover both providers at 4/16 old root/reply references. All **200** frame strings, outbound method/host/path order and persisted metadata/error/positive-or-negative TTLs match Rails. No Cookie or Authorization header is sent. Unexpected DNS/HTTP endpoints fail; no external network is used.
- **Stale siblings:** each parent job claims and queues exactly one stale sibling of the same provider, deduplicated across references. Opposite-provider and suppressed-only siblings retain their old claim. A deterministic parent-completion barrier stops the runner before dispatching children, allowing exact durable queue-state comparison with Rails' test adapter. **Eight** outer/savepoint rollback probes preserve parent metadata, sibling claims and queue state and publish nothing. Child jobs remain pending for inspection; this checkpoint does not claim their subsequent network execution.

## Files and boundaries

| Own changed paths under `rust/` | Purpose |
| --- | --- |
| `crates/db/src/models/calendar_event.rs` | Flagged WS14 consumer contract carrying event identity; post-commit lookup; JSON-bound association preload. |
| `crates/campfire/src/channels/{event_cards.rs,sink.rs}`, `channels.rs` | Register and render ordered event-card callbacks with the existing conversation stream helper. |
| `crates/campfire/src/controllers/presenters/events.rs` | Reuse the existing card DTOs, templates and renderer zone; bound organizer/venue facts and render a message batch without per-message reads. #179 viewer-zone behavior is retained. |
| `crates/campfire/src/integrations/message_batches.rs` | Add Event reference identity to the already bounded provider selector. |
| `crates/db/src/tests/calendar_event_test/calendar_api_test.rs` | Decode the typed intent to check existing logical stream/target metadata. The owner's pinned `calendar_api.json` is unchanged. App tests independently verify actual frames. |
| `crates/campfire/src/controllers/message_features/{older_calendar_tests,older_embed_job_tests}.rs`, `message_features.rs` | Five new runtime regression functions, real sockets, real queue consumers and physical SQL capture. |
| `crates/campfire/src/controllers/message_features/quote_integration_tests.rs`, `app.rs` | Test-only fixture import for owner attendance/calendar-entry rows and visibility of the existing queue-drain helper. |
| `reference-tools/messaging/older_calendar_callbacks.rb`, `older_calendar_jobs.rb`, `older_embed_jobs.rb`, matching `vectors/messaging/*.json` | Three independently generated pinned Rails fixtures: **3,416** distinct callback/job/boundary frames. Credentials have fixture names and authorization headers are composed at runtime. |
| `reference-tools/messaging/{verify_oracles,features-reference-check}.py` | Replay all 39 fixtures and verify 118 consumed reference files plus two rejected source-check corruptions. |

No production LinkEmbed fetcher/store change is included: those owner paths already behave correctly for this corpus. A temporary store mutation was used only as a negative control and restored before committing. The implementation commit changes none of the WS12-owned board/work files. #201's reviewed changes to those files arrive solely through the main merge. Real WS17 notifications, WS11 invocation/auth and WS13 huddle integrations remain wired. D's Cable lifetime fix is retained.

## Query cost and failing-first evidence

Physical reads mean executed SELECT/WITH statements, including writer-connection reads. Calendar callback capture includes the triggering model write, record guard and production app rendering. Generic/LinkedIn capture surrounds the real fetch consumer, including its writer and callback reads; queue polling/acknowledgement and test observation are outside that interval. Calendar job queue overhead was not separately counted; its card callback uses the measured batch path.

| Operation | References | Before Rust | Final Rust | Rails |
| --- | --- | --- | --- | --- |
| Calendar callback, ordinary/populated | 4 → 16 | 37 → 121 | 13 → 13 | 9 → 21 |
| Generic parent network fetch, each of five outcomes | 4 → 16 | Already implemented; served negative control below | 10 → 10 | 14 → 26 |
| LinkedIn parent network fetch, each of five outcomes | 4 → 16 | Already implemented; served negative control below | 10 → 10 | 14 → 26 |

The two calendar byte/read regressions failed against the stacked pre-fix production source `7c13d365`: an extra empty frame targeted the removed reference, and the read guard detected 37 → 121. The corrected baseline command ran 17 selected tests: 13 passed, 4 failed. Two failures were those real calendar defects; the other two were new harness mistakes (registering unrelated push jobs without a push worker, and parsing Rails' ` UTC` fixture timestamp with the wrong utility). Those harness errors were corrected without production changes and are not claimed as app defects.

Commands used through `.scratch/ws8bm2-e/ci-env.sh` (configured throttle unchanged, `CARGO_BUILD_JOBS=2`, normal owned native target):

```sh
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire --bin campfire -j2 older_ -- --test-threads=4 --nocapture
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire --bin campfire -j2 older_calendar_meet_jobs -- --test-threads=4 --nocapture
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked -p campfire --bin campfire -j2 older_generic_and_linkedin_network_jobs -- --test-threads=4 --nocapture
```

Logs: `baseline-runtime-corrected.log`, `calendar-and-jobs-after.log`, `calendar-job-served-mutation.log`, `embed-job-served-mutation.log`, all under the assigned `.scratch/ws8bm2-e/`.

The served negative controls compiled production mutations, not changed expectations: Event callback `maintain_scroll=false` made the Meet job's actual WebSocket frame fail; replacing the store's same-provider predicate with `true` queued `[4,5]` instead of Rails' `[4]`. Both failed 0/1, then their original sources were restored. These are discrimination evidence for already implemented job paths, not claims that the unmodified owners had those defects. Raw closing lines are included below. The final fresh-clone suite runs with all mutations restored.

## Everything still incomplete for WS8b-m2 cutover

This list distinguishes missing implementation from missing proof. No broad parity claim extends beyond the pinned corpora.

1. **Known implementation gap — standalone pin-list batching (WS8b-m2):** `controllers/rooms/pins.rs::list` still finds each message, pinner, author and body inside the per-pin loop; Rails' list preloads them. This inherited gap was carried in earlier reports as missing constant-query proof and is now named explicitly from source inspection. It needs a failing-first 4/16 Rails query differential and a batch loader. No pin-list read numbers were measured in E. Standalone poll-card scaling also still lacks its previously listed dedicated physical-query proof; root/search preload proofs do not substitute for it.
2. **Partial implementation/matrices — exceptional date and parameter coercion (WS8b-m2, held for #196):** remaining Ruby `Date._parse`/`Time.zone.parse` grammar beyond the existing pinned corpora; exceptional multiparameter and Array/Hash lookup/coercion for scheduling/reminders/slash time input; structured pager/link/file parameters; unsampled DST gap/fold combinations and exact exceptional messages. Existing compact-width, signed/expanded-year, offset/fraction, HTTP and DST corpora remain replayed. Reuse WS11-UI's shared parser once available; no second parser was built here.
3. **Owner API gap — typed `AgentBudgetNotice` reader (WS11-API):** the named read-only SQL fact-reader remains in `presenters/activity.rs`; main still does not export its typed owner model. Writes/fan-out stay with the owner. `BoardSlaNudge`, physical push sending, agent invocation/auth and huddle launch are integrated, so their former stand-in flags are retired.
4. **Implementation present, additional old-window proof unblocked (WS8b-m2 with WS14g/WS15e):** Calendar `InboundSyncJob`/`SyncEntryJob` old-reference permutations beyond the Meet/card-producing job matrix; actual dispatch of the queued stale siblings after the parent finishes; deleted-during-fetch and broader failing-queue/job permutations. E proves the parent, claims, pending queue and outer/savepoint rollback state, not those later executions. Owner APIs already exist; these are unblocked verification work, not owner-API blockers. Two inherited WS14g agent/Drive HTTP polling checks also remain ignored with a pre-merge WS11-API annotation (`integrations/agent_jobs/drive_attachment_cases.rs`); the current `/agents/events` route and Drive-capable payload are present. Their owner needs to refresh/re-enable that proof. They are not counted as a missing API or a newly completed E behavior.
5. **External ownership, retired inherited gap:** WS12 #201 has merged and is included at `692f998d`; the old `/work.json` read-growth/bind-limit flag is retired. WS8b-m2 made no direct changes to `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, `board_posts.rs` or `work_threads.rs`. Broader board/work parity belongs to WS12.
6. **System phase:** the 45 browser behaviors already passed on both apps at accepted checkpoints. They are not unported; they were not rerun in E. New provider scope is proved through real HTTP/TLS, queue and WebSocket differentials. Browser/pixel work was not added. The final cutover system phase remains separate, with no pixel checks claimed.

## Retained Rails file coverage inventory

These are named Rails behaviors covered by grouped Rust tests/vectors, not a claim that the Rails controller suites were executed as suites in E. The full workspace suite reruns the Rust ports. Ordinary named controller coverage is **140/140**; the former single agent case is now integrated.

| File under `test/controllers/` | Named behaviors covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16/16 |
| `messages/pins_controller_test.rb` | 6/6 |
| `rooms/pins_controller_test.rb` | 3/3 |
| `saved_items_controller_test.rb` | 12/12 |
| `scheduled_messages_controller_test.rb` | 19/19 |
| `searches_controller_test.rb` | 36/36 |
| `rooms/slash_commands_controller_test.rb` | 10/10 |
| `autocompletable/icons_controller_test.rb` | 6/6 |
| `autocompletable/slash_commands_controller_test.rb` | 7/7 |
| `autocompletable/users_controller_test.rb` | 5/5 |
| `rooms/message_links_controller_test.rb` | 12/12 |
| `rooms/files_controller_test.rb` | 8/8 |

| Browser file under `test/system/` | Prior accepted Rails/Rust behavior results |
| --- | --- |
| `polls_test.rb` | 4/4 each |
| `pins_saved_test.rb` | 7/7 each |
| `slash_commands_test.rb` | 26/26 each |
| `search_files_test.rb` | 4/4 each |
| `scheduled_messages_test.rb` | 4/4 each |
| Total | 45/45 each; not rerun in E |

## Fresh-clone gates and raw receipts

Final verification is from `.scratch/ws8bm2-e/fresh-final`, a new `git clone --no-hardlinks --branch rust/ws8bm2-message-features-e . .scratch/ws8bm2-e/fresh-final` at **0cb5b5d7**. Default, first-run and agent-UI seeds were rebuilt inside that clone. Only registry cache and execution/resource configuration were reused; no untracked seed or source fixture was copied. The 13 workspace packages were cleaned from the normal native target before this final compilation, with a guarded nonempty package list. The earlier pre-#201 clone was stopped while compiling, before any test ran, and is not counted as a workspace result.

Commands below ran from that clone's repository root; stdout/stderr logs are in its parent `.scratch/ws8bm2-e`. All exited 0. `--workspace` includes html5ever, with no exclusions. Four test threads and two Cargo build workers retain the unchanged shared rustc throttle. No timeout, retry, ignore or assertion was weakened. Cargo's color escape bytes are removed from the following raw summary text for readability.

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-e/ci-env.sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/seed build default first_run agents_ui > ../final-seeds.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings > ../final-clippy.log 2>&1
WS8BM2_ORACLE_SCRATCH=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-e/final-oracles python3 rust/reference-tools/messaging/verify_oracles.py > ../final-oracles.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture > ../final-workspace.log 2>&1
python3 rust/reference-tools/messaging/features-reference-check.py --self-test > ../final-reference-source.log 2>&1
CARGO_TARGET_DIR=/native-target bash rust/ci/cargo.sh metadata --locked --format-version 1 > ../final-fresh-metadata.json 2> ../final-fresh-metadata.stderr
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > ../final-release.log 2>&1
```

Exact execution/resource bridge (not test data):

```bash
export WS8BM2_ROOT=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2
export CI=1 CARGO_BUILD_JOBS=2
export RUST_CI_IMAGE=campfire-toolchain-ci-rust-speedups RUST_CI_CONTAINER_PREFIX=ws8bm2
export RUNNER_TEMP="$WS8BM2_ROOT/.scratch/ws8bm2-e/ci-temp"
docker() {
  if [[ "$1" == run ]]; then
    shift
    local args=() arg
    for arg in "$@"; do
      [[ "$arg" == CARGO_BUILD_JOBS=4 ]] && arg=CARGO_BUILD_JOBS=2
      args+=("$arg")
    done
    command docker run --cpus 4 \
      --volume "$WS8BM2_ROOT/rust/target:/native-target" \
      --volume "$WS8BM2_ROOT/.scratch/main-merge/rustc-throttle.sh:/rustc-throttle:ro" \
      --volume /tmp/rust-port-rustc-slots:/rustc-slots \
      --volume /home/riels/.cache/rust-port/rustc-slots:/slot-count:ro \
      --env RUSTC_WRAPPER=/rustc-throttle \
      --env CABLE_TEST_PORT_RANGE=52500-52549 \
      --env INTEGRATION_TEST_PORT_RANGE=52500-52549 \
      --env MAIL_TEST_PORT_RANGE=52550-52599 "${args[@]}"
  else
    command docker "$@"
  fi
}
export -f docker
```

Raw seed summaries:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

Raw full-workspace summaries, in emitted order:

```text
test result: ok. 2570 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 1070.57s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 40.80s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1283 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 216.15s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.51s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.41s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.64s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.78s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.39s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.93s
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

The following line is the separately emitted sum of those Cargo summaries, not an extra Cargo result:

```text
WS8bm2 workspace totals: 4590 passed; 0 failed; 16 ignored; 61 raw Cargo summaries
```

Raw strict-clippy and release-input build closing lines (respectively):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 18s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 21s
```

Locked metadata was parsed successfully; full JSON is retained in `final-fresh-metadata.json`. Emitted metadata/source/ownership summaries:

```text
WS8bm2 cargo metadata --locked: 13 workspace packages; valid format-version 1; success
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 own-slice ownership guard: 20 changed paths; 0 WS12-owned paths; 0 date-parser paths; D unchanged at 6211c93f
```

Raw final independent Rails replay summaries (the full log retains all 39 receipts):

```text
WS8bm2 oracle replay: older_provider_callbacks.json byte-identical
WS8bm2 oracle replay: older_owner_callbacks.json byte-identical
WS8bm2 older-calendar Rails: 4 groups; 196 exact frames; 4 silent rollbacks; 3000 ordered boundary frames
WS8bm2 older-calendar Rails reads: false:4:9, false:16:21, true:4:9, true:16:21
WS8bm2 oracle replay: older_calendar_callbacks.json byte-identical
WS8bm2 older-calendar jobs Rails: 16 real jobs; 20 exact frames; 2 pending retries; 10 guarded no-ops; no external network
WS8bm2 oracle replay: older_calendar_jobs.json byte-identical
WS8bm2 older-embed jobs Rails: 20 real network jobs; 200 exact frames; 20 deduplicated same-provider sibling jobs; 8 silent outer/savepoint rollbacks
WS8bm2 older-embed job Rails reads: generic:4:14/14/14/14/14, generic:16:26/26/26/26/26, linkedin:4:14/14/14/14/14, linkedin:16:26/26/26/26/26
WS8bm2 oracle replay: older_embed_jobs.json byte-identical
WS8bm2 oracle replay: 39/39 independently replayed fixtures byte-identical
```

The separate `fresh-oracles.log` and `final-oracles.log` each report 39/39, providing two independent new-fixture regenerations. Original generation/re-generation logs and their output trees are also retained; no repinning occurred.

Raw final E runtime/read diagnostics from the full suite:

```text
WS8bm2 older-calendar Rust reads populated=false size=4: 13; Rails=9
WS8bm2 older-calendar Rust reads populated=false size=4: 13; Rails=9
WS8bm2 older-calendar Rust reads populated=false size=16: 13; Rails=21
WS8bm2 older-calendar Rust reads populated=false size=16: 13; Rails=21
WS8bm2 older-calendar Rust reads populated=true size=4: 13; Rails=9
WS8bm2 older-calendar Rust reads populated=true size=4: 13; Rails=9
WS8bm2 older-calendar Rust reads populated=true size=16: 13; Rails=21
WS8bm2 older-calendar Rust reads populated=true size=16: 13; Rails=21
WS8bm2 older-calendar Rust boundary: 999 ordered frames; exact Rails SHA256
WS8bm2 older-calendar Rust: 4 groups; 160 exact Rails frames; 4 silent rollbacks; post-commit reference snapshots
WS8bm2 older-calendar Rust: 4 groups; 196 exact Rails frames; 4 silent rollbacks; post-commit reference snapshots
WS8bm2 older-embed job Rust generic success size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust generic login size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-calendar Rust boundary: 1000 ordered frames; exact Rails SHA256
WS8bm2 older-embed job Rust generic http_error size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust generic redirect size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust generic private_image size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-calendar Rust boundary: 1001 ordered frames; exact Rails SHA256
WS8bm2 older-embed job Rust generic success size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust generic login size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust generic http_error size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust generic redirect size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust generic private_image size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust linkedin success size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin login size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin http_error size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin redirect size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin private_image size=4: 10 consumer reads; Rails=14; 4 exact frames
WS8bm2 older-embed job Rust linkedin success size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust linkedin login size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust linkedin http_error size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust linkedin redirect size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed job Rust linkedin private_image size=16: 10 consumer reads; Rails=26; 16 exact frames
WS8bm2 older-embed jobs Rust: 20 real network jobs; 200 exact Rails frames; 20 deduplicated same-provider sibling jobs; 8 silent outer/savepoint rollbacks; flat consumer reads; no external network
WS8bm2 older-calendar jobs Rust: 16 real registered jobs; 20 exact Rails frames; 2 pending retries; 10 guarded no-ops; exact owner API requests
```

Raw failing-first/negative-control closing lines from this slice, followed by the restored selected runtime result:

`baseline-runtime-corrected.log`:

```text
test result: FAILED. 13 passed; 4 failed; 0 ignored; 0 measured; 2554 filtered out; finished in 9.82s
```

`calendar-job-served-mutation.log`:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2570 filtered out; finished in 0.46s
```

`embed-job-served-mutation.log`:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2570 filtered out; finished in 1.23s
```

`calendar-and-jobs-after.log`:

```text
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 2554 filtered out; finished in 16.50s
```


All 16 inherited ignored tests are recorded here; E adds no ignore. The single missing-default-seed diagnostic comes from the deliberate empty-directory guard test, not the actual freshly rebuilt seed. Browser/gateway/live-ACME and external Rails export harnesses remain outside this run:

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
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


## Cleanup and final state

Both verification clones were inspected before deleting four generated JSON exports by exact path (32,806 bytes), then removing the empty target directory. There were no scratch compiled artifacts. The normal owned `rust/target` native cache remains; no additional compiled scratch target remains. Release-input temporary trees were removed by the guard. Logs, clones, independent seeds and oracle output trees remain as evidence. The final clone has no tracked changes; its only status entry is generated `.scratch/` output. No owned test/reference container is running. The configured throttle, other workers and Python model server were untouched; no stash was used.

```text
WS8bm2 scratch target cleanup: 4 generated exports (32806 bytes) removed by exact path; both clone targets absent; 0 extra compiled targets
```
