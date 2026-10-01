# WS13 — main WS13b integration and PR handoff

Verified implementation: `638d746c89959d35a9f7912469ad657489680650`, pushed on `rust/ws13-huddles`; the report-only commit follows. Merge commit `fd57e46b3edde778f885af25f252b23ca263a368` merges main `b908ebc28f5b13039ce4dcca427bb1fb314b3d88` into WS13 (first parent `b542f3a0`). No stash, rebase or PR creation occurred. The lead opens the PR.

The full workspace and all acceptance batches passed at `de7f1ba57debe5f3445e76b1ac28970a256b3412` in the fresh clone. Subsequent changes restore main's production OOO presenter availability, collapse an equivalent conditional, and limit five fixture-only helpers to test builds. Those changes leave test bodies and expectations unchanged; affected composition/OOO tests, strict clippy, locked metadata and the sealed production build pass at the verified implementation above. All eight independent invitation declarations pass directly on main's WS13b APIs. The two original inbox declarations are nonblocking and deferred to WS11-UI by the lead's ruling; their assertions remain ready to enable.

## Changes and merge decisions

- `rust/parity/system/ws13-domain-system.py` is deleted; the `ws13b_domain_api` cfg, its lint allowance and `WS13_WS13B_API` dispatch are removed. The ordinary browser entry always includes invitations and uses main's queued-ring API. Five formerly overlaid production files and WS13b's ring-matrix test module match main byte for byte. No WS13b model, job or service test was ported or changed.
- Shared controller wiring (`controllers/mod.rs`, `concerns.rs`, `rooms.rs`, `rooms/room_native.rs`, `users/sidebars.rs`) retains main's authorization, audit and native room/sidebar behavior while adding configured huddle, voice and Stage navigation and room-destruction callbacks. Main's user/agent lifecycle owns revocation; the obsolete unreferenced `db/models/user/lifecycle.rs` duplicate is deleted. Domain signatures stay unchanged.
- Shared views retain main's room/sidebar data structures and post-#163 layout/preferences/Picker adapters. The recorded complete WS13 composition remains in `views/rooms/composition_page.rs`, `views/users/sidebar_composition.rs` and their templates. The huddle navigation wrapper accounts for main's notification partial trailing newline. The full-room fixture supplies main's presenter an origin without a trailing slash. The old quote fixture supplies main's Card renderer Rails' 200-character excerpt facts; recorded HTML expectations remain unchanged. Main's native full-page goldens and WS13's 28 headers / 38 full-room pages all pass.
- `channels/room_composition.rs` leaves unconfigured directory rows/headers with main's renderer. Configured DM callbacks use main's membership association avatar order, filtered by the original descriptor's selected members, before adding huddle controls. A new controller/Cable regression compares the avatar sequence with the unchanged Rails directory fixture; it failed before the fix and passes afterward.
- Real DM media startup now follows the original Rails test's session order: the second participant opens the room after the first joins. An inline invitation from main had correctly covered its header Join control when both pages were opened prematurely. Delayed-session media instrumentation is installed before its real visit; it does not evaluate mediaDevices on about:blank. No media assertions, readiness signals, test concurrency or deadlines were weakened. The unchanged original Rails DM declaration passed against the same earlier project-local LiveKit process.
- Main's `controllers/channel_threads/page_tests.rs` enqueue-count fixture reproduced its live-worker race on a clean main checkout. It now stops its own job runner before setting up the durable enqueue-count assertions. Dedupe, queue-insert failure and rollback assertions remain unchanged; production behavior is untouched. `views/tests/room_shell.rs` initializes the new optional navigation field. `ci/cargo.sh` respects `CARGO_BUILD_JOBS`, defaulting to the machine-required two.

WS13b and WS17 production seams are unchanged. Main owns policy and transport; retained core helper signatures are `enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest)` and `prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool) -> Result<Option<PushDelivery>>`. No transport or domain follow-up is assigned to WS13.

## Exact deferred declarations and owners

| Original declaration in test/system/huddle_invitations_test.rb | Required public actions | Owner |
| --- | --- | --- |
| the recipient sees an incoming huddle banner and dismissing it marks the item read | GET /activity/unread_count.json; PATCH /activity/:id/read | WS11-UI, rust/ws11ui-agent-pages |
| joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel | PATCH /activity/:id/handled | WS11-UI, rust/ws11ui-agent-pages |

These are the only WS13 deferrals. The public actions still return 501 on this main baseline. Set `WS13_ENABLE_INBOX_CASES=1` to run their retained complete interactions when the routes land. No inbox endpoint was implemented here. Route/controller ownership is documented in `rust/plans/ws13-inbox-route-dependencies.md`.

The title catalogue retains 548 declarations across 33 files. WS13 owns 332: 226 controller/integration declarations and 106 system declarations; 330 are assertion-covered and the two above deferred. System acceptance is 69 ordinary cases plus 35 real-media cases, with two inbox cases skipped. WS13b's 216 owned declarations are unscored in this report. Per-file counts and every title remain in `rust/plans/ws13-deferred-tests.md`. No pixel work or LiveKit deferral remains.

## Fresh-clone setup and command environment

`git clone --no-hardlinks --branch rust/ws13-huddles <WS13 worktree> .scratch/fresh-ws13-main-172` created a new source clone with no target or local fixture inputs copied. It was fast-forwarded only on WS13 through the verified implementation. Default and first_run seeds were rebuilt from pinned tracked inputs. CI=1 makes missing seeds fail. It was clean before and after all gates. A temporary clean detached checkout of main b908ebc2 in this same clone provided the baseline comparison, then its WS13 branch was restored clean; no source overlay was applied.

Commands run from that fresh clone with mise Rust 1.98.1, CI=1, CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0 and CARGO_INCREMENTAL=0. CAMPFIRE_REFERENCE and CARGO_TARGET_DIR point into that clone; TMPDIR and npm cache use its scratch. Cable ports are 52300–52349 and mail ports 52350–52399. All test-thread/concurrency flags are eight. The machine-wide rustc throttle remains configured. No python model server was touched.

`rust/parity/bin/seed build default first_run`:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

`mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null` from fresh/rust; strict `tomllib.loads` parse of `[workspace.dependencies]` (duplicate keys are rejected):

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

Source identity checks (`git show b908ebc2:<path>` compared with fresh file bytes, overlay script/cfg absence); `python3 rust/reference-tools/ws13_verify_reference.py`; `python3 rust/reference-tools/ws13_verify_declarations.py`:

```text
Main WS13b source identity: rust/crates/db/src/models/activity_item.rs matches b908ebc2
Main WS13b source identity: rust/crates/db/src/models/huddle_grant.rs matches b908ebc2
Main WS13b source identity: rust/crates/db/src/models/huddle_invitations.rs matches b908ebc2
Main WS13b source identity: rust/crates/db/src/models/huddle_notices.rs matches b908ebc2
Main WS13b source identity: rust/crates/campfire/src/jobs/huddle.rs matches b908ebc2
Main WS13b source identity: rust/crates/campfire/src/jobs/huddle/ring_matrix_tests.rs matches b908ebc2
WS13b overlay removed: no script or private cfg
Reference identity: 97 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
```

## Failing-first and merge regressions

Initial seeded workspace run before the rendering corrections (complete no-fail-fast run; other crates passed):

```text
test result: FAILED. 1718 passed; 4 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1700.77s
```

Clean main baseline: `cargo test --locked -p campfire controllers::channel_threads::page_tests:: -- --test-threads=8`, using the pinned Docker runtime runner described below:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 24s
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 1651 filtered out; finished in 37.83s
```

The unchanged enqueue fixture read zero queued jobs instead of one on both branches. No wider waits or lower test concurrency were used.

Configured DM avatar regression: `cargo test --locked -p campfire configured_direct_callback_keeps_rails_membership_avatar_order -- --test-threads=8`, pinned runtime runner:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 40s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 1.45s
```

Its actual sequence was Jason, Kevin, JZ; the unchanged Rails fixture specifies Jason, JZ, Kevin. After using main association ordering, the configured callback and ten related merge regressions pass. The exact command passes these OR filters after Cargo’s `--`: `configured_direct_callback_keeps_rails_membership_avatar_order`, `channels::tests::hub_test::directory::`, `complete_call_headers_match_twenty_eight_rails_renders`, `full_room_pages_match_thirty_eight_complete_rails_pages`, `pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure`, `composed_sidebar_`, and `--test-threads=8`:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 40s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 1717 filtered out; finished in 10.61s
```

Original Rails DM comparison: LIVEKIT_SYSTEM_TESTS=1, PARALLEL_WORKERS=1, ws13-reference:system-d7c7de92, local host networking, private scratch tmp and tracked-lock gateway dependencies; `bundle exec bin/rails db:setup` then `bundle exec bin/rails test test/system/huddles_test.rb -n "/two.direct.message.participants.exchange.audio.and.screen.while.navigating.and.reconnecting/"`:

```text
Finished in 9.057030s, 0.1104 runs/s, 3.8644 assertions/s.
1 runs, 35 assertions, 0 failures, 0 errors, 0 skips
```

## Acceptance at de7f1ba5 in the fresh clone

Explicit full ten-case diagnostic: `WS13_INVITATIONS_ONLY=1 WS13_ENABLE_INBOX_CASES=1 rust/parity/system/ws13`. Only the two WS11-UI cases fail; this is not a passing gate or an invitation-domain failure:

```text
    Finished `test` profile [unoptimized] target(s) in 0.14s
ℹ tests 10
ℹ pass 8
ℹ fail 2
ℹ skipped 0
ℹ duration_ms 34469.570354
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 37.45s
```

Nonblocking invitation acceptance: `WS13_INVITATIONS_ONLY=1 rust/parity/system/ws13`:

```text
    Finished `test` profile [unoptimized] target(s) in 0.10s
ℹ tests 10
ℹ pass 8
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 20826.286648
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 23.40s
```

Real media: `rust/parity/system/ws13-livekit`. The script sources the generated project-local env, checks readiness and installs gateway dependencies from the tracked lock. LiveKit 1.13.7 runs only on loopback signaling/admin 7880 and UDP 7882, external discovery/TURN disabled. Browser transport remains isolated and forwards opaque local media packets. The four asynchronous-sampling/transport regressions precede the 35 original real-media interactions. Native audio/video tracks, decode, RTP statistics, token-refresh/full reconnects and server enforcement are asserted; no recorded media substitutes:

```text
WS13 local media datagrams: 2 sent; 2 received
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 1098.693917
    Finished `test` profile [unoptimized] target(s) in 0.17s
WS13 local media datagrams: 8194 sent; 4800 received
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 224169.594759
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 227.55s
```

Ordinary browser: `rust/parity/system/ws13`:

```text
    Finished `test` profile [unoptimized] target(s) in 0.42s
ℹ tests 71
ℹ pass 69
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 256321.012802
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 261.88s
```

Gateway’s own Node suite against Rust endpoints: `mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8`:

```text
    Finished `test` profile [unoptimized] target(s) in 0.86s
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 15574.715098
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1727 filtered out; finished in 16.08s
```

## Workspace, clippy and sealed release-input build

`mise exec rust@1.98.1 -- cargo --config 'target.x86_64-unknown-linux-gnu.runner=<JSON runner array>' test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8`. The array is `docker run --rm --name ws13-pinned-workspace-test --label com.smartfire.rust-parity.owner=ws13 --cpus 2 --network none --user 1000:1000 --volume <absolute-fresh>:<absolute-fresh> --workdir <absolute-fresh> --env CI=1 --env TMPDIR=<absolute-fresh>/.scratch --env CABLE_TEST_PORT_RANGE=52300-52349 --env MAIL_TEST_PORT_RANGE=52350-52399 --entrypoint /usr/bin/env ws13-reference:d7c7de92`. This runs the linked fresh binaries with the pinned libvips/ffmpeg while preserving their absolute reference paths. Raw summary lines from every target and doctest:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 48s
test result: ok. 1723 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1648.10s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.64s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 153.16s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.95s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.59s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.34s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.38s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.48s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.35s
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
Workspace aggregate: 3385 passed; 0 failed; 14 ignored (unit, integration and doctest summaries)
```

After the full workspace pass, the production/all-target lint gate exposed main OOO presenter functions incorrectly limited to tests, five retained fixture helpers compiled as unused production functions, and two style warnings. Main’s OOO presenter file is now byte-identical to b908ebc2 and remains available to production room pages. Only the fixture helpers are test-gated; no warning allowances were added. Affected tests rerun on the final implementation with the same pinned Docker runner and `--test-threads=8`, OR filters `controllers::rooms::room_shell_tests::`, `controllers::rooms::full_room_tests::`, `controllers::presenters::chrome_tests::`, `controllers::rooms::ws17_ooo_tests::`, `composed_sidebar_`, `configured_direct_callback_keeps_rails_membership_avatar_order`, and `ooo_notice`:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 38s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1706 filtered out; finished in 18.79s
```

Ignored Rust harness wrappers for gateway, ordinary browser and LiveKit were explicitly run above. Other workspace ignores are inherited recording/export/differential helpers or measurements, not additional WS13 deferrals; the raw ignored counts are preserved.

Strict clippy: `mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`:

```text
    Finished `dev` profile [unoptimized] target(s) in 53.54s
```

Exact release-input gate: `bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins`. RUNNER_TEMP is fresh/.scratch/ci-runner, RUST_CI_CONTAINER_PREFIX=ws13 and CARGO_TARGET_DIR=/src/rust/target, mapped to the same fresh target. The stale local default campfire-toolchain tag lacked mold and failed dependency build-script linking; its retained log is release-inputs-before-toolchain-fix.log. The passing run sets RUST_CI_IMAGE=sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2, the existing Rust 1.98.1 CI toolchain with mold and pinned media libraries. A WS13-only local Docker CLI adapter mounts the unchanged machine slot-count file and lock directory and sets a scratch RUSTC_WRAPPER: it uses the existing throttle loop for every container compile, accounting for the separate PID namespace. Host throttle configuration and slot count are unchanged. The wrapper copies only builder source/asset inputs and provides no parity/vector/reference-tool files. CARGO_BUILD_JOBS remains two:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 31s
```

## Cleanup and evidence

`cargo clean --manifest-path rust/Cargo.toml` with the fresh CARGO_TARGET_DIR; verified scratch target deletion. The own project-local LiveKit process was stopped immediately after media acceptance, before the ordinary/workspace runs. No WS13 test process remains; no other worker target/process or model server was touched:

```text
Project-local LiveKit: SIGTERM sent to verified WS13 server PID 1755477
Project-local LiveKit: exit 0; verified server PID is gone
     Removed 10541 files, 5.4GiB total
Fresh scratch cargo target: deleted .scratch/fresh-ws13-main-172/rust/target; no other target touched
Project-local LiveKit: original WS13 server PID 1755477 remains gone
```

Final logs are retained at `.scratch/merge-ws13b/final-verification-logs`; earlier failing-first, main baseline and diagnostic runs remain separately under `.scratch/merge-ws13b/accepted-logs`, `fresh-logs`, `final-logs` and `before-order-verification-logs`. The earlier verification coordinator was stopped before its workspace gate to include the configured-order regression; its completed browser/media runs are not substituted for final acceptance. No HTML golden expectations, waits, timing thresholds, test concurrency or domain/transport signatures were weakened.
