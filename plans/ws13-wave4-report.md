# WS13 — historical PR #185 second Astra recheck

This is the preceding P2 review record. PR #188 has since merged: both inbox invitation cases now run unconditionally, and all ten pass from a fresh clone. The two deferrals below describe that earlier baseline and are closed. Current verification and raw summaries are in the delegation report `wave4/ws13-report.md`; the declaration catalogue and route dependency record are updated alongside this note.

Verified implementation: `52b338b9e0a1f715ba22bf379e55c0536edb5e6b` on `rust/ws13-huddles`; the report-only commit follows. The two remaining P2 findings are addressed. No new main merge was requested or performed; main's viewer-zone behavior, the original Astra fixes, CI's four-job default, and WS13b/WS17 APIs remain unchanged.

## Changes

The configured sidebar loads DM memberships in their original SQLite association order, then loads every distinct member in one user query. Group labels retain their separate sorting; avatar order does not change. The regression traces all SELECT executions across the actual HTTP request before and after five new peer/group-DM pairs, with the periodic/job runner disabled. It compares total query growth to the pinned Rails recording. Astra's independent full-request Rails result was 20→20. The additional tracked Rails recorder measures a warmed sidebar-frame response at 13→13 SELECT notifications. These request/counting scopes differ, but both require zero growth; the regression does not enforce identical absolute counts between applications.

Board pages select their own Rails navigation regardless of LiveKit configuration. The Board nav retains New post, Events, Files, settings, the bell and overflow menu; it adds neither chat controls nor a huddle launcher/participant stack. RoomView uses persisted header metadata for Board DOM IDs and settings routes. Board's overflow menu omits chat Threads/Pins actions, matching Rails. The huddle navigation factory excludes Boards and leaves Stage's unconfigured controls present.

Two negative checks fail before fixes: the total sidebar-growth and full Board-nav HTTP regressions. A third browser regression fails on the old configured Board: it sights a real grant, waits for the actual production `replace:header_voice_participants_rooms_board_*` Cable frame to reach Turbo, and inspects the final DOM. The fixed Board has no header participant target, exactly as Rails, so the replacement leaves its nav intact. No broadcast target or domain behavior is rewritten.

## Exact deferred items

Only these two invitation declarations remain deferred to WS11-UI (`rust/ws11ui-agent-pages`), whose endpoints are not in this branch's main baseline:

- `the recipient sees an incoming huddle banner and dismissing it marks the item read`: `GET /activity/unread_count.json`, `PATCH /activity/:id/read`.
- `joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel`: `PATCH /activity/:id/handled`.

Complete test code remains available through `WS13_ENABLE_INBOX_CASES=1`. Eight other invitations use main's WS13b APIs and pass without any overlay. The catalogue remains 548 original declarations in 33 files: WS13 owns 332 (330 covered, two deferred); WS13b owns 216, unscored here. The supplementary Board review regression does not change the Rails declaration catalogue. All 35 real LiveKit cases are enabled; no pixel work remains.

## Files and boundaries

- `controllers/users/sidebars/composition.rs`: batched `User::where_ids` preload, membership-order reassembly, missing-user error preserved.
- `controllers/presenters/room_native.rs`: exclude Boards from the call-navigation adapter; retain unconfigured Stage navigation and main's viewer-zone setup.
- `views/src/rooms.rs`, `views/templates/rooms/show.html`, new `rooms/boards/_nav.html`, and `rooms/show/_header_overflow.html`: Board nav selection, persisted room-type IDs/edit route, exact Board overflow conditions.
- `controllers/rooms/review_tests.rs`: total SELECT-growth and complete Board HTTP-nav comparisons, configured and unconfigured.
- `controllers/rooms/system_browser_tests.rs`, `parity/system/ws13-board-review.test.mjs`, `ws13-stage.test.mjs`: private Board fixture and supplementary real Cable/Turbo replay; original interactions and wait budgets unchanged.
- `reference-tools/huddle_review_recheck.rb`, `vectors/huddle_review_recheck.json`: actual pinned Rails sidebar queries, complete Board navigation, no launcher/participant stack, one New post link, and real production presence broadcast. `ws13_verify_reference.py` also hashes the Board nav against the pin.

WS13b's grant/membership/stream and WS17's policy/transport signatures stay unchanged. The overlay remains absent. The original 99+2+1 mutation evidence remains in `rust/plans/ws13-review-mutations.txt`; Astra confirmed it, and the full current anchor preflight passes. This turn does not claim a new 102-mutation sweep. Main's original #179/#180 protected source files match the merged baseline byte for byte. No Rails application source, expectation, compiler throttle, CI default, timing threshold, or pixel comparison was changed.

## Failing-first evidence and Rails recording

Both new HTTP regressions fail on the preceding implementation; the four existing review regressions pass. Command from the assigned worktree: `mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire controllers::rooms::review_tests -- --nocapture --test-threads=8`.

```text
    Finished `test` profile [unoptimized] target(s) in 0.18s
DM sidebar total SELECTs: Rust 27 -> 32; Rails 13 -> 13 after five new peers/group DMs
test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 1.18s
```

The supplementary browser regression first consumes the real Board-targeted Cable frame and then rejects the old huddle launcher. Command: `WS13_BOARD_REVIEW_ONLY=1 rust/parity/system/ws13`.

```text
    Finished `test` profile [unoptimized] target(s) in 0.10s
ℹ tests 1
ℹ pass 0
ℹ fail 1
ℹ skipped 0
ℹ duration_ms 2114.648643
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 4.84s
```

Pinned Rails recording command (output redirected to the tracked vector):

`PARITY_NAMESPACE=ws13 PARITY_OWNER=ws13 PARITY_CPUS=2 PARITY_IMAGE=ws13-reference:d7c7de92 rust/parity/bin/reference runner --seed default -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_review_recheck.rb > rust/vectors/huddle_review_recheck.json`

The recorder uses warmed `Turbo-Frame: user_sidebar` requests and counts all SELECT notifications, including Rails query-cache notifications. Astra's separate full-request measurement is 20→20; this additional scope is 13→13. Rust traces every actual SELECT execution in the full HTTP request and compares total growth, rather than hiding user reads behind per-room counters. The recorded Board response contains the complete nav exactly, no huddle launcher/participant IDs, and one New post link. The real Rails replacement target is `header_voice_participants_rooms_board_699448332`, absent from that nav by design.

```text
{
  "reference_pin": "d7c7de92",
  "direct_sidebar": {
    "peers_added": 5,
    "before_selects": 13,
    "after_selects": 13
  },
  "board": {
    "room_id": 699448332,
    "status": 200,
    "media_launchers": 0,
    "new_posts": 1,
    "participant_ids": []
  }
}
```

The fixed focused checks pass from the worktree with the same commands:

```text
    Finished `test` profile [unoptimized] target(s) in 2m 07s
DM sidebar total SELECTs: Rust 24 -> 24; Rails 13 -> 13 after five new peers/group DMs
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 1.81s
    Finished `test` profile [unoptimized] target(s) in 0.47s
ℹ tests 1
ℹ pass 1
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 1767.260502
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 4.13s
```

## Fresh-clone verification

A no-hardlinks clone at `.scratch/fresh-ws13-recheck2` was created from this branch; its source was fast-forwarded to pushed implementation 52b338b9 before all verification gates. Both seeds were built there from tracked inputs; no seed, target or fixture was copied from the assigned worktree. Dependency binaries were precompiled there while the machine test queue was busy. All reported final gates ran at the same source SHA with no tracked-source edits. The clone has only one scratch cargo target, removed after verification.

Environment: mise Rust 1.98.1, CI=1, CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0; CAMPFIRE_REFERENCE and CARGO_TARGET_DIR point into the fresh clone. TMPDIR and npm cache are under its scratch. Cable ports 52300–52349 and mail ports 52350–52399 are isolated to WS13. Every Rust test command uses `--test-threads=8`; the Node commands retain concurrency eight. Gates wait for machine capacity; clippy and the sealed build ran before the full workspace gate while another workstream occupied test slots. No concurrency setting or wait was reduced or widened. Only owned idle verification coordinators were stopped to reorder these gates; the local model server was untouched.

`rust/parity/bin/seed build default first_run`:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

`mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null` from the clone’s `rust/`; strict `tomllib` parsing rejects duplicate workspace dependencies:

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

`python3 rust/reference-tools/test_huddle_discrimination.py` (current catalogue preflight and baseline guards):

```text
...
----------------------------------------------------------------------
Ran 3 tests in 0.003s

OK
```

`python3 rust/reference-tools/ws13_verify_reference.py`; `python3 rust/reference-tools/ws13_verify_declarations.py`; byte identity against `git show 59ad94de:<path>`:

```text
Reference identity: 98 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
Main source identity: rust/crates/campfire/src/controllers/channel_threads/page_tests.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/messages.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/presenters/message_cache.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/presenters/page.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/presenters/github.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/presenters/room_list.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/rooms/native_integration_tests.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/rooms/events.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/rooms/refreshes.rs matches 59ad94de
Main source identity: rust/crates/campfire/src/controllers/channel_threads.rs matches 59ad94de
WS13b overlay removed: no script or private cfg
```

Fresh focused command: `mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire controllers::rooms::review_tests -- --nocapture --test-threads=8`.

```text
    Finished `test` profile [unoptimized] target(s) in 1m 31s
DM sidebar total SELECTs: Rust 24 -> 24; Rails 13 -> 13 after five new peers/group DMs
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 1.65s
```

Invitations: `WS13_INVITATIONS_ONLY=1 rust/parity/system/ws13`.

```text
    Finished `test` profile [unoptimized] target(s) in 0.23s
ℹ tests 10
ℹ pass 8
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 18629.981394
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 20.31s
```

Real media: `rust/parity/system/ws13-livekit`. One clean 35/35 batch, preceded by all four polling/network regressions. The real project-local LiveKit server uses loopback signaling/admin and local media forwarding, with no external LiveKit or TURN endpoints. The owned process was stopped after this batch.

```text
WS13 local media datagrams: 2 sent; 2 received
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 889.257275
    Finished `test` profile [unoptimized] target(s) in 0.10s
WS13 local media datagrams: 7137 sent; 4151 received
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 166441.512152
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 167.98s
```

Ordinary browser: `rust/parity/system/ws13`. Its 72 titles comprise the original 71 plus the supplementary Board review regression; 70 pass and two WS11-UI cases skip.

```text
    Finished `test` profile [unoptimized] target(s) in 0.20s
ℹ tests 72
ℹ pass 70
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 210382.731403
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 211.99s
```

Gateway own suite against Rust: `mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8`.

```text
    Finished `test` profile [unoptimized] target(s) in 0.19s
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 13714.029459
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1870 filtered out; finished in 13.97s
```

## Full workspace, strict clippy and release-input build

Full workspace command: `mise exec rust@1.98.1 -- cargo --config 'target.x86_64-unknown-linux-gnu.runner=<runner>' test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8`.

The exact `<runner>` JSON array follows. It executes fresh binaries with the pinned libvips/ffmpeg and original absolute reference paths; no test was removed or filtered:

```text
["docker", "run", "--rm", "--name", "ws13-pinned-workspace-test", "--label", "com.smartfire.rust-parity.owner=ws13", "--cpus", "2", "--network", "none", "--user", "1000:1000", "--volume", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-recheck2:/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-recheck2", "--workdir", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-recheck2", "--env", "CI=1", "--env", "TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-recheck2/.scratch", "--env", "CABLE_TEST_PORT_RANGE=52300-52349", "--env", "MAIL_TEST_PORT_RANGE=52350-52399", "--entrypoint", "/usr/bin/env", "ws13-reference:d7c7de92"]
```

```text
    Finished `test` profile [unoptimized] target(s) in 1m 08s
test result: ok. 1866 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1098.99s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.64s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 168.61s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.51s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.38s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.62s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.53s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.71s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.73s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.33s
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

Workspace aggregate: 3720 passed; 0 failed; 14 ignored (sum of the raw target/doctest summaries)

Strict clippy: `mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`.

```text
    Finished `dev` profile [unoptimized] target(s) in 2m 50s
```

Release-input build: `bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins`.

RUNNER_TEMP is the clone’s scratch/ci-runner; RUST_CI_CONTAINER_PREFIX=ws13, RUST_CI_IMAGE=sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2, CARGO_TARGET_DIR=/src/rust/target. The local Docker adapter mounts the existing rustc slot configuration and lock pool into this local container; it does not rewrite CI's jobs argument. CI still defaults to four; the explicit local override stays two. Sealed inputs are removed by the build script’s trap.

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 23s
```

All final gates exit zero; `git diff --exit-code` passes before and after. Normal-suite ignored targets are retained in the summaries above; the three WS13 acceptance harnesses were run explicitly with `--ignored`, as listed above. Only the two named invitation declarations remain deferred in WS13's inventory.

After all gates, the owned LiveKit PID was verified gone and the fresh scratch target was removed. The permanent worktree target and other workers' targets/processes were retained.

```text
Project-local LiveKit: stopped WS13 server PID 1079062; exit 0; verified PID gone
     Removed 10541 files, 5.5GiB total
Fresh scratch cargo target: deleted; permanent worktree target retained
```
