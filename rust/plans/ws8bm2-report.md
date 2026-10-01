# WS8bm2 merge and message behavior continuation

**PARTIAL.** Main snapshot `59ad94de3d0c53dfc15cae95901453f6506a7e83` is merged with both histories retained, through merge commit `40f616d5dce15b23c483b7608777288c4a829bbf`. No stash, rebase or force push was used. Subsequent pushed slices are `d1b2265fe368aff29ffb09ec1e0ae3aa9a44d894` (owner callbacks and agent invocation), `a4f6506b696b2213ac7d4fd3004be23a97777020` (autocomplete coercion), and `8b5c40fe7ec7ecaed0c00cf7173398aa18ad985c` (browser behavior driver). The final report commit changes documentation only.

All 140 inventoried controller behaviors are represented. The 24 existing Rails oracle fixtures plus three new ones independently replay byte identically (27 total). The fresh-clone workspace suite at `a4f6506b` passed 3,685 tests, zero failures and 12 existing ignores. The second full run at pushed source `8b5c40fe` also passed 3,685 tests, zero failures and 12 existing ignores; its raw summaries are below. Strict workspace/all-target clippy, locked metadata and the exact release-input build passed. No timing bound, existing test concurrency, source mask or golden allowlist was relaxed.

Browser coverage is partial: 28 cases are implemented. The final clean-seed sequence passed 28/28 on Rails and 27/28 on Rust. Rust's poll-by-Enter picker case hit its unchanged two-second selector deadline after passing in an earlier complete 20/20 slash run. It was recorded and not retried. Seventeen additional browser cases and broader coercion/date/runtime proof remain; **this is not an owner-blocked-only branch**.

## Changes by file and owner integration

- **Merge resolution:** kept main's ordinary room shell, thread rendering, room/user policies, presenter/cache API and viewer-zone behavior, together with the message feature controllers, callbacks, tests and vectors. Production template/vector constants remain crate-contained; the release-input build succeeds without repository files outside `crates/`. Cargo metadata was checked locked after the merge. Both merge parents are retained.
- `campfire/src/controllers/presenters.rs` and `presenters/{message_cache.rs,room_native.rs}`, `controllers/messages.rs`, `views/src/messages.rs` and `views/src/messages/composer.rs`: retained main's `use_viewer_zone`, `render_zone`, request-zone partial rendering and zone-bearing fragment keys from #179. Composer concrete STI identity is still supplied, including the five differential room types. Nullable quote digest semantics are retained in the updated cache-key API. Human edits still replace all eight message-owned targets in Rails order; main's read/write provider callback sinks are installed.
- `campfire/src/controllers/{rooms.rs,channel_threads.rs}` and the room/thread presenter adapters: use the real Markdown composer, pending template, schedule child, poll builder, pins and thread panels, Files link, room-list slot and provider PR header. The WS8b-m integration contract is consumed by the main shell. Ordinary room and thread content supply Drive configuration/metadata/share flow facts. Browser Files navigation now proves that mounted room-shell link and actual upload/Drive rows. Network Google/Drive endpoints are a separate owner boundary below.
- `campfire/src/concerns.rs`, `db/src/slash_commands.rs`, `db/src/models/agent_slash_command.rs`: use main's actual agent credential authentication and command dispatcher; the old WS11 authentication/invocation stand-ins are gone. The last named controller behavior now executes the real HTTP command, persists its event metadata and compares Rails' ephemeral response. `message_features/slash_tests.rs`, `reference-tools/messaging/agent_command.rb` and `vectors/messaging/agent_command.json` supply this differential.
- `campfire/src/jobs.rs`, `jobs/{notifications.rs,reminders.rs}`, `db/src/models/saved_item.rs`: production reminders use WS17's actual saved-reminder job handler, notification policy and `WebPushPool.queue`. The earlier tagged-send production seam is gone. `reminders.rs::deliver_with` remains a **cfg(test) transport adapter** for deterministic job/payload/error regressions, not the production sender. Its missing-record discard and membership behavior are retained. Saved status updates still write only dirty columns, preserving concurrent dispatch claims and Rails' stale loaded-instance JSON.
- `campfire/src/{config.rs,main.rs,huddle_readiness.rs}`: huddle readiness delegates to WS13's real `huddle::Config::from_lookup(...).configured()` contract. Real internal gateway, moderation and stage APIs are kept from main. Public huddle join/show and configured header/launch mounting are still absent from this main snapshot; no replacement endpoint was invented here.
- `campfire/src/controllers/quote_integration_tests.rs`: durable-queue oracle tests use `TestApp::without_job_runner()` where queue records are the assertion. The actual provider-refresh job test explicitly retains its job runner. A blanket disable was caught and corrected; no deadline was changed.
- `message_features/provider_tests.rs`, `reference-tools/messaging/provider_callbacks.rb`, `vectors/messaging/provider_callbacks.json`: three actual Rails HTTP edits change public/private cached GitHub and generic-link URLs, replace references, then remove them. Rust matches 24 complete socket frames and each reference set. Three scoped public/private card endpoint reads match their owned body bytes/statuses, including the wrong-message 404. The original five populated edits/40-frame presentation differential remains intact. This proves the sampled cached callbacks/endpoints, not every uncached provider-network permutation. Main's broader provider owner tests also run in the full workspace suite.
- `campfire/src/controllers/autocompletable.rs`, `message_features/coercion_tests.rs`, `reference-tools/messaging/user_coercions.rb`, `vectors/messaging/user_coercions.json`: 41 actual Rails URL cases cover 13 query, 13 page and 15 room shapes. Container queries retain Ruby stringification; structured page parameters return Rails' exact 500 JSON rather than silently becoming page 1. Room arrays retain reachable-room lookup semantics (valid arrays raise 500 at `.users`, unknown arrays 404). The lookup remains viewer-scoped. The assertions compare selected users, markdown names, pager/link headers and exact error bodies; no universal user-response byte parity is claimed.
- `reference-tools/messaging/verify_oracles.py`: adds the three independently replayed fixtures, without editing earlier expected bytes.
- `parity/behavior/{runtime.mjs,scheduled.mjs,slash.mjs,search-files.mjs,messages.mjs,README.md}` and `parity/bin/behavior`: pinned browser behavior runner, shared origin proxy and seed sessions, no screenshots/pixel checks, external network disabled. New contexts isolate browser state; public HTTP creates supply fixture rows through each app's callbacks. New-message assertions exclude old DOM IDs or scope persisted `data-message-id`. Coverage includes scheduling; 20 slash interactions; operator chips; same-room quote jumps; lazy cross-room private/member cards using three users; the mounted Files link, a real multipart upload, a Drive row, filename/type filters. The source's 15-second cable setup barrier, two-second selectors and explicit ten-second assertions are retained. The delayed-fetch two-second interleaving is unchanged.

Paths starting with a crate name above are beneath `rust/crates/`. No application assets, Rails source, dependency versions or browser timing constants were changed by these continuation slices. Merge-only Rails changes from main were retained. No deployment or PR was opened.

## Failing-first and diagnostic evidence

**Current coercion slice:** with only its tests/vector added, all three new regressions failed against `d1b2265f`; the autocomplete controller was still byte-identical to that commit. Array query returned all active users instead of the Ruby-string match; array page returned 200 instead of 500; a valid room array returned 404 instead of 500. After the production change, all three pass. Raw summaries:

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1830 filtered out; finished in 1.14s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1830 filtered out; finished in 1.72s
```

**Callback discrimination:** removing the installed GitHub reference sync from the app (a temporary mutation, then fully restored before positive runs) makes the changed-URL callback test fail at the intended stale-card socket assertion. The committed regression then passes with the real installed owner callback. Raw summaries and observations:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1829 filtered out; finished in 1.26s
WS8bm2 STI composer HTTP: 5/5 room types preserve Rails reply-control ids and labels
WS8bm2 broader dates: 202/202 Rails coercion/compact-width cases match
WS8bm2 review dates: 88/88 Rails compact/offset/DST cases match
WS8bm2 date HTTP: 88 saved timestamps, 88 scheduled timestamps and 88 before/due dispatch pairs match Rails
WS8bm2 provider callbacks: 3 HTTP edits, 24/24 socket frames and reference sets, 3/3 scoped card endpoints match Rails
WS8bm2 provider edits: 5 HTTP edits, 40/40 real socket replacement frames byte-identical to Rails
test result: ok. 163 passed; 0 failed; 0 ignored; 0 measured; 1670 filtered out; finished in 51.76s
```

**Earlier Astra P2 fixes remain intact:** five regressions had failed first at the exact reviewed `bcbac9f85b182329fd54aa332a07fec609ab6255` before the prior corrections. This turn reruns their fixed tests, not that historical failing archive. Dirty-column saves preserve the concurrent dispatch/reschedule; compact calendars retain clocks/offsets across the sampled DST zones; scheduled sends use the shared `broadcasts::unread_rooms_stream_name` consumed by `UnreadRoomsChannel`. Their deterministic race uses channels plus `queued_writes()`, not sleeps. No historical baseline command is presented as a new run.

**Initial host-media/setup diagnostic:** the first native fresh-clone run failed four tests. Two test-adapter mistakes (missing-record reminder discard and disabling the real quote refresh runner) were corrected in the merge. The other two were strict media-byte checks: host libvips 8.18.6/ffmpeg n9.0.2 differ from the vectors' pinned libvips 8.16.1/ffmpeg 7.1.5. The entire workspace, including both media checks, then passed in `campfire-toolchain-ci-rust-speedups`. No version check, golden or ignore was weakened. Raw failed summaries and the succeeding merge app summary:

```text
test result: FAILED. 1822 passed; 3 failed; 3 ignored; 0 measured; 0 filtered out; finished in 891.12s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.98s
test result: ok. 1825 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 751.39s
```

**Browser authoring diagnostics:** the initial driver lacked the source cable setup barrier; an event-picker timeout issued no autocomplete request. The barrier was restored, then the 20-case slash runs passed on both apps. Duplicate Rails scheduled field IDs required scoped textarea lookup. A room-jump assertion used the database ID as a DOM ID; both apps actually use `client_message_id`, so it was corrected to `data-message-id`. A missing import after extracting the shared driver was fixed. A Playwright response-body read hung; the driver now observes the browser's cloned fetch response under its existing two-second bound. Repeating file setup on the same live database creates duplicate upload rows, so the final sequence started from fresh app seed copies. These harness defects were corrected without production changes or timing widening. In that final sequence, **Rust poll-by-Enter timed out at 2,000 ms** after previously passing. It remains flagged; there is no retry or forced pass. Logs retain the earlier positive runs and the final failure.

## Fresh-clone verification and exact commands

The clone was created with `git clone --no-hardlinks --no-checkout . .scratch/main-merge/fresh`, detached to the merged branch and later advanced to each pushed source. Both seeds were built independently inside that clone, not copied from this worktree. Its tracked source is clean; `.scratch/` contains generated verification output only. The final Rust production changes are `a4f6506b`; the browser slice `8b5c40fe` changes only Node/shell/docs. The final full workspace run records `8b5c40fe`. Generated seed/log/cache files are not hidden test fixture inputs. The 12 existing ignores are three manual app capture/gateway/latency checks, one cable capture, four external Rails database export/rollback/differential checks, one mail export, one live ACME TLS integration and two kit doctests; none was added or widened here. The one extra target directory is removed after verification.

The CI runner uses the pinned media/toolchain image. `.scratch/main-merge/ci-env.sh` supplies only execution/resource configuration (not test data): image `campfire-toolchain-ci-rust-speedups`, `CI=1`, `CARGO_BUILD_JOBS=2`, container prefix `ws8bm2`, four CPUs, this worker's port ranges, and a read-only bridge to the **existing unchanged** machine rustc slot count and `/tmp/rust-port-rustc-slots` flock directory. Commands retain `-j4` and `--test-threads=4`. The normal root target is mounted as `/native-target` only to avoid a second extra target. The final release build also sets `CARGO_TARGET_DIR=/native-target`; its command arguments remain exactly the requested guard invocation. Its exact environment/bridge script follows so the checks do not require an undocumented local setting:

```bash
export WS8BM2_ROOT=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2
export CI=1 CARGO_BUILD_JOBS=2
export RUST_CI_IMAGE=campfire-toolchain-ci-rust-speedups RUST_CI_CONTAINER_PREFIX=ws8bm2
export RUNNER_TEMP="$WS8BM2_ROOT/.scratch/main-merge/fresh/.scratch/ws8bm2/ci-temp"
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

`.scratch/main-merge/rustc-throttle.sh` (same shared pool; slot count read only):

```bash
#!/bin/bash
rustc="$1"; shift
case " $* " in *" --crate-name "*) ;; *) exec "$rustc" "$@" ;; esac
slots=$(cat /slot-count)
while :; do
  for ((i=0; i<slots; i++)); do
    exec {fd}>>"/rustc-slots/$i"
    flock -n "$fd" && exec "$rustc" "$@"
    exec {fd}>&-
  done
  sleep 0.3
done

```

Seed command, run from `.scratch/main-merge/fresh`:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92   rust/parity/bin/seed build default first_run > .scratch/ws8bm2/seed.log 2>&1
```
Raw builder lines (the standalone seed-validation 29/4 counts were not rerun this turn and are not claimed):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Full suite, run from `.scratch/main-merge/fresh/rust`:

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/main-merge/ci-env.sh
bash ./ci/cargo.sh test --locked --workspace --no-fail-fast -j4 -- --test-threads=4   > ../.scratch/ws8bm2/pushed-workspace-test.log 2>&1
```
Exit 0; **3,685 passed, 0 failed, 12 existing ignores** (sum of Cargo summaries). All 60 raw summary lines:

```text
test result: ok. 1830 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 855.50s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.02s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1144 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 208.36s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.40s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.59s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 26.61s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.07s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.54s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.77s
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

Feature subset, also from the fresh clone's `rust/` (after the last production change):

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/main-merge/ci-env.sh
bash ./ci/cargo.sh test --locked -j4 -p campfire controllers::message_features   -- --test-threads=4 --nocapture > ../.scratch/ws8bm2/final-features-test.log 2>&1
```
```text
test result: ok. 163 passed; 0 failed; 0 ignored; 0 measured; 1670 filtered out; finished in 51.76s
```

Strict clippy, worktree root (all targets, including html5ever; no exclusions):

```bash
source .scratch/main-merge/ci-env.sh
CARGO_HOME=/src/.scratch/main-merge/fresh/rust/.cargo-home CARGO_TARGET_DIR=/native-target   bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j4 -- -D warnings   > .scratch/main-merge/pushed-clippy.log 2>&1
```
Exit 0; raw native summary, ANSI color removed only:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.25s
```

Locked metadata, fresh clone `rust/`:

```bash
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 mise exec rust@1.98.1 --   cargo metadata --locked --format-version 1 > ../.scratch/ws8bm2/pushed-metadata.json
```
Exit 0; Cargo emits JSON and no summary line. No lockfile change.

Release-input guard, fresh clone root:

```bash
source /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/main-merge/ci-env.sh
CARGO_TARGET_DIR=/native-target bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins > .scratch/ws8bm2/pushed-release-inputs.log 2>&1
```
Exit 0; raw native summary, ANSI color removed only:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 33s
```

All original 24 plus three new Rails oracles and the consumed-source check, fresh clone `rust/`:

```bash
python3 reference-tools/messaging/verify_oracles.py > ../.scratch/ws8bm2/pushed-oracles.log 2>&1
python3 reference-tools/messaging/features-reference-check.py > ../.scratch/ws8bm2/pushed-reference-check.log 2>&1
```
Exit 0; raw summary lines:

```text
WS8bm2 oracle replay: 27/27 independently replayed fixtures byte-identical
WS8bm2 reference source check: 106 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Final browser sequence, worktree root, after resetting both isolated app instances to the independently built default seed:

```bash
for ws8bm2_app in Rails Rust; do
  if [[ "$ws8bm2_app" == Rails ]]; then ws8bm2_port=52500; else ws8bm2_port=52501; fi
  for ws8bm2_suite in scheduled slash search-files; do
    PARITY_NAMESPACE=ws8bm2 bash rust/parity/bin/behavior "$ws8bm2_suite"       --target "http://127.0.0.1:$ws8bm2_port" --name "$ws8bm2_app"       --labels .seed/default/labels.json       > ".scratch/main-merge/browser-clean-${ws8bm2_suite}-${ws8bm2_app,,}.log" 2>&1
    ws8bm2_status=$?
    echo "$ws8bm2_app $ws8bm2_suite exit=$ws8bm2_status"
  done
done
```
App startup, canonical runtime and cleanup are documented in `parity/behavior/README.md`. Rails uses the pinned reference image; Rust uses its actual native binary built in the canonical toolchain image (not a candidate Docker deployment). No pixel/media-byte browser parity is claimed. Raw driver summaries; the Rust slash command exited 1, the other five exited 0:

```text
WS8bm2 browser scheduled (Rails): 4/4 passed; 0 failed
WS8bm2 browser slash (Rails): 20/20 passed; 0 failed
WS8bm2 browser search-files (Rails): 4/4 passed; 0 failed
WS8bm2 browser scheduled (Rust): 4/4 passed; 0 failed
WS8bm2 browser slash (Rust): 19/20 passed; 1 failed
WS8bm2 browser search-files (Rust): 4/4 passed; 0 failed

```

## Rails controller and browser inventories

These are named behavior ports, not claims that Rails' test files ran against Rust. Rails executes independently to produce the checked fixtures; Rust HTTP/socket/model assertions consume those results.

| Rails controller file beneath `test/controllers/` | Covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16 / 16 |
| `messages/pins_controller_test.rb` | 6 / 6 |
| `rooms/pins_controller_test.rb` | 3 / 3 |
| `saved_items_controller_test.rb` | 12 / 12 |
| `scheduled_messages_controller_test.rb` | 19 / 19 |
| `searches_controller_test.rb` | 36 / 36 |
| `rooms/slash_commands_controller_test.rb` | 10 / 10 |
| `autocompletable/icons_controller_test.rb` | 6 / 6 |
| `autocompletable/slash_commands_controller_test.rb` | 7 / 7 |
| `autocompletable/users_controller_test.rb` | 5 / 5 |
| `rooms/message_links_controller_test.rb` | 12 / 12 |
| `rooms/files_controller_test.rb` | 8 / 8 |
| **Total named behavior ports** | **140 / 140** |

| Rails system file beneath `test/system/` | Implemented / total | Final Rails | Final Rust |
| --- | --- | --- | --- |
| `polls_test.rb` | 0 / 4 | not run | not run |
| `pins_saved_test.rb` | 0 / 7 | not run | not run |
| `slash_commands_test.rb` | 20 / 26 | 20 pass | 19 pass, 1 timing failure |
| `search_files_test.rb` | 4 / 4 | 4 pass | 4 pass |
| `scheduled_messages_test.rb` | 4 / 4 | 4 pass | 4 pass |
| **Total** | **28 / 45** | **28 pass** | **27 pass, 1 timing failure** |

The six unimplemented slash cases are registered-agent listing; argument insertion without execution; immediate no-argument execution; registration after page load; ephemeral execution plus stored arguments; persisted custom status. They require additional valid runtime fixture/row-observation setup, not a fake production endpoint. Browser poll cases (4) and pins/saves cases (7), including DST fall-back reminder selection, remain. Existing controller/date tests do not count as these browser proofs.

## Precise remaining work and ownership

1. **Owned browser work:** implement the remaining 17 cases and investigate the recorded poll-picker timing failure in the end-to-end phase. Keep its two-second bound and retain the failing observation. No browser retry or ignore was added.
2. **Broader coercion/date sampling:** signed/long years, expanded ISO, further exceptional grammar/messages, odd lookup shapes on other endpoints, structured pager/link/option/file inputs and unprobed user filtering remain. Current 359 builder samples, 88 slash date samples and 88 actual saved/scheduled HTTP/dispatch pairs remain green; the new 41 user cases do not prove universal Ruby coercion. #179 viewer-zone rendering and cache keys are retained exactly.
3. **Additional provider/runtime proof:** populated complete search/older-window pages beyond the existing samples; uncached provider callback combinations; constant-query poll/pin-list measurements; the periodic poll-close job's actual runtime socket delivery remain unproved. Main's production callbacks and the sampled private endpoints are integrated, with no read-only stand-in left in their place.
4. **WS13 owner boundary:** this main has internal huddle/gateway, moderation and stage APIs plus real readiness; public room huddle join/show/configured-header mounting remains absent. The unconfigured browser command paths pass; configured execution is not claimed. WS13 owns that public adapter.
5. **WS14g owner boundary:** room/thread composers consume the real Drive configuration/grant facts, root messages accept Drive file IDs, and browser Files rows/filtering pass. Google/Drive OAuth, enhanced Picker/share network endpoints are absent from this main snapshot and remain WS14g's work. This report does not substitute metadata rows for those network endpoints.
6. **Other owner APIs:** WS11 human command invocation/auth are integrated; the broader agent REST/MCP controller work is absent from this main snapshot and remains WS11-owned. WS17 production reminder transport is integrated; no physical-send stand-in is still awaiting WS17.

No new product decision is needed. Scratch logs are retained in `.scratch/main-merge/`; completed browser app/forwarder containers and processes are stopped, and the extra scratch Cargo target is deleted after final verification. The machine rustc throttle was not modified and the Python model server was never touched. The tracked report and the authorized external report are kept byte-identical. This remains partial because both owned and owner-dependent work is outstanding.
