# WS13 — PR #185 merge verification

Verified implementation: `eaa8d9529ab923c9f5f54ab310564fe5c12cfd99` on `rust/ws13-huddles`; the report-only commit follows. The merge has parents `f26e460ebac939b3869883e5bfc258845c64cdba` and main `2b05160758307effa035b7d2581618014c46b755`. All requested suites and gates pass on this same implementation from a new no-hardlinks clone. The lead opened PR #185; this continuation updates that branch. The shared origin/main ref advanced again to 7442031d (#176) during verification; this report covers the requested 2b051607 baseline, not that later merge.

## Merge decisions and changed files

- `controllers/channel_threads/page_tests.rs`: the only content conflict was the same worker-stop fix on each side. Both stop the fixture job runner before durable queue assertions. Main #180 uses a one-second stop timeout; WS13 used two seconds. The file now matches main byte for byte, retaining every dedupe, rollback and queue-failure assertion and dropping the duplicate WS13 fix.
- `ci/cargo.sh`: reverted WS13’s parameterized two-job default. It changed CI behavior when the caller did not set CARGO_BUILD_JOBS, so it was inappropriate for a local resource rule. The entire file now matches main byte for byte and unconditionally sets four CI build jobs. Local compiler throttling remains in the configured machine-wide pool; the untracked WS13 verification adapter uses two local jobs without changing CI source.
- #179’s `presenters/message_cache.rs`, `presenters/page.rs`, `presenters/github.rs`, `presenters/room_list.rs`, message/thread/event/refresh request paths and `rooms/native_integration_tests.rs` match main. Viewer-zone message creation, request card dates, timestamp components in fragment keys without a zone-name suffix, actor-zone shared broadcasts, alias-zone sharing, DST and invalid-zone fallbacks are preserved exactly. The full production Presenter struct and implementation match main. `presenters/room_native.rs` matches main plus the existing configured-huddle navigation field; its persisted viewer-zone selection is intact. Presenter module declarations and voice/Stage RoomKind variants keep WS13 wiring alongside main’s changes.
- Main’s event/card-write/cache-zone fixtures and reference tools are merged unchanged. No Rails source, HTML golden expectation, domain signature, push transport seam, timing threshold or test concurrency was changed by this merge. No new security/domain implementation was added; the existing PR’s security regressions run in the workspace suite.

## Exact remaining declarations and owners

Only two nonblocking declarations in `test/system/huddle_invitations_test.rb` remain deferred, by the lead’s ruling:

| Original declaration | Required public actions | Owner |
| --- | --- | --- |
| the recipient sees an incoming huddle banner and dismissing it marks the item read | GET /activity/unread_count.json; PATCH /activity/:id/read | WS11-UI, rust/ws11ui-agent-pages |
| joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel | PATCH /activity/:id/handled | WS11-UI, rust/ws11ui-agent-pages |

Their complete test code remains ready to enable with WS13_ENABLE_INBOX_CASES=1. The eight independent invitations pass directly on main’s WS13b APIs, with no overlay script or private cfg. `rust/plans/ws13-inbox-route-dependencies.md` records their Rails routes/controllers. No LiveKit case or pixel work is deferred.

The catalogue remains 548 titles across 33 files: WS13 owns 332 (226 controller/integration and 106 system), with 330 assertion-covered and the two above deferred. WS13b’s 216 declarations remain owned and unscored here. Per-file counts and every title remain in `rust/plans/ws13-deferred-tests.md`. WS13b and WS17 seams are unchanged.

## Fresh clone and command environment

`git clone --no-hardlinks --branch rust/ws13-huddles . .scratch/fresh-ws13-pr185` created a new clone from the merged branch. No target, local seed or fixture input was copied. Both seeds were rebuilt from the pinned tracked inputs. It was clean before and after all gates; CI=1 makes missing seeds fail.

Commands run from that clone with mise Rust 1.98.1, CI=1, CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0, and CAMPFIRE_REFERENCE/CARGO_TARGET_DIR pointing into it. TMPDIR and npm cache use its scratch; Cable ports 52300–52349 and mail ports 52350–52399 are retained. All Rust test-thread and explicit Node concurrency flags are eight. Rails remains pinned to d7c7de92 plus approved #163 drift. The machine-wide compiler throttle configuration and slot count were not changed; no python model server was touched.

`rust/parity/bin/seed build default first_run`:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

`mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null` from fresh/rust; strict tomllib parse of workspace.dependencies (duplicate keys are rejected):

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

Source identity checks compare fresh files/Presenter bodies with `git show 2b051607:<path>`. `python3 rust/reference-tools/ws13_verify_reference.py` and `python3 rust/reference-tools/ws13_verify_declarations.py`:

```text
Main source identity: rust/ci/cargo.sh matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/channel_threads/page_tests.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/messages.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/presenters/message_cache.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/presenters/page.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/presenters/github.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/presenters/room_list.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/rooms/native_integration_tests.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/rooms/events.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/rooms/refreshes.rs matches 2b051607
Main source identity: rust/crates/campfire/src/controllers/channel_threads.rs matches 2b051607
WS13b overlay removed: no script or private cfg
Viewer-zone Presenter identity: struct and entire production implementation match 2b051607
Viewer-zone adapter identity: rust/crates/campfire/src/controllers/presenters/room_native.rs matches 2b051607 plus existing huddle navigation field
Viewer-zone adapter identity: rust/crates/campfire/src/controllers/presenters/events.rs matches 2b051607
Reference identity: 97 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
```

## Acceptance suites

Invitations once: `WS13_INVITATIONS_ONLY=1 rust/parity/system/ws13`. The two owner-deferred cases are explicitly skipped; every other invitation assertion runs:

```text
    Finished `test` profile [unoptimized] target(s) in 7m 44s
ℹ tests 10
ℹ pass 8
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 32211.879591
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1738 filtered out; finished in 34.87s
```

Real media once: `rust/parity/system/ws13-livekit`. The project-local LiveKit process uses loopback signaling/admin 7880 and media UDP 7882, with external discovery/TURN disabled. The isolated browser forwards opaque local packets; real WebRTC tracks, audio/video decode and RTP statistics, refresh/full reconnects and server enforcement are asserted. Four polling/transport regressions precede the 35 original media interactions:

```text
WS13 local media datagrams: 2 sent; 2 received
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 1495.002241
    Finished `test` profile [unoptimized] target(s) in 0.37s
WS13 local media datagrams: 10043 sent; 5632 received
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 286093.962099
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1738 filtered out; finished in 289.10s
```

Ordinary browser once: `rust/parity/system/ws13`:

```text
    Finished `test` profile [unoptimized] target(s) in 0.65s
ℹ tests 71
ℹ pass 69
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 315929.750664
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1738 filtered out; finished in 319.23s
```

Gateway’s own Node suite once against Rust: `mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8`:

```text
    Finished `test` profile [unoptimized] target(s) in 1.04s
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 17863.206517
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1738 filtered out; finished in 18.35s
```

## Full workspace, clippy and sealed build

`mise exec rust@1.98.1 -- cargo --config 'target.x86_64-unknown-linux-gnu.runner=<JSON runner array>' test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8`. The runner is `docker run --rm --name ws13-pinned-workspace-test --label com.smartfire.rust-parity.owner=ws13 --cpus 2 --network none --user 1000:1000 --volume <absolute-fresh>:<absolute-fresh> --workdir <absolute-fresh> --env CI=1 --env TMPDIR=<absolute-fresh>/.scratch --env CABLE_TEST_PORT_RANGE=52300-52349 --env MAIL_TEST_PORT_RANGE=52350-52399 --entrypoint /usr/bin/env ws13-reference:d7c7de92`. Linked fresh binaries run with pinned media libraries and their original absolute reference paths. Every target/doctest summary follows:

```text
    Finished `test` profile [unoptimized] target(s) in 2m 12s
test result: ok. 1734 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1567.97s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.84s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.51s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 187.65s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.55s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.35s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.55s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.76s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.25s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.15s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
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
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.53s
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
Workspace aggregate: 3396 passed; 0 failed; 14 ignored (unit, integration and doctest summaries)
```

#179’s unchanged behavior regressions exercised within the full suite:

```text
test controllers::rooms::native_integration_tests::native_component_capture_matches_rails_root_selection ... ok
test controllers::rooms::native_integration_tests::native_room_anchor_mounts_owner_messages_around_the_root ... ok
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_both_eastern_dst_transitions ... ok
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_hawaii_and_warm_viewer_changes ... ok
test controllers::rooms::native_integration_tests::native_room_event_cards_match_rails_utc_and_invalid_zone_fallbacks ... ok
test controllers::rooms::native_integration_tests::native_room_lists_are_per_viewer_even_when_fragments_are_warm ... ok
test controllers::rooms::native_integration_tests::native_room_page_mounts_the_selected_owner_list_and_composer ... ok
test controllers::rooms::native_integration_tests::native_thread_header_matches_rails_viewer_zones ... ok
test controllers::rooms::native_integration_tests::native_room_page_provider_cards_match_rails_bytes ... ok
test controllers::rooms::native_integration_tests::zone_audit_event_broadcast_matches_rails_actor_zone_for_every_recipient ... ok
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_honolulu_share_github_fragments_like_rails ... ok
test controllers::rooms::native_integration_tests::zone_cache_hawaii_and_india_share_ordinary_fragments_like_rails ... ok
test controllers::rooms::native_integration_tests::zone_audit_github_message_cards_match_rails_with_warm_zones ... ok
test controllers::rooms::native_integration_tests::zone_cache_timestamp_components_match_rails_across_dst_and_fractional_zones ... ok
test controllers::rooms::native_integration_tests::zone_audit_message_creation_matches_rails_and_reload ... ok
test controllers::rooms::native_integration_tests::zone_audit_utc_and_invalid_zone_fallbacks_cover_all_three_paths ... ok
```

Gateway/browser/LiveKit Rust harness wrappers are ignored in the normal unit run and explicitly exercised above. Remaining workspace ignores are inherited recording/export/differential helpers or measurements; their counts are preserved and are not additional WS13 deferrals.

Strict clippy: `mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings`:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 03s
```

Exact sealed-input command: `bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins`. RUNNER_TEMP is fresh/.scratch/ci-runner, RUST_CI_CONTAINER_PREFIX=ws13, CARGO_TARGET_DIR=/src/rust/target and RUST_CI_IMAGE=sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 (the available Rust 1.98.1 CI toolchain with mold and pinned media). A WS13-only untracked Docker CLI adapter changes the local container job setting to two, mounts the unchanged machine slot-count file/lock directory, and applies the existing compiler throttle loop to container compiles across the separate PID namespace. CI source stays at four jobs and is byte-identical to main. The sealed input tree contains only builder source/asset inputs, with no parity/vector/reference-tool files:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 14s
```

## Cleanup

`mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml` with the fresh absolute CARGO_TARGET_DIR, followed by scratch-target deletion verification. The own project-local LiveKit process was stopped immediately after its batch. No WS13 test/build process remains, no other worker target/process was touched, and no python model server was touched:

```text
Project-local LiveKit: stopped WS13 server PID 3680847; exit 0; verified PID gone
     Removed 10541 files, 5.4GiB total
Fresh scratch cargo target: deleted .scratch/fresh-ws13-pr185/rust/target; no other target touched
Project-local LiveKit: original WS13 PID 3680847 remains gone
```

Current evidence logs: `.scratch/pr185-main/logs`; coordinator progress: `.scratch/pr185-main/progress.log`. This report cites only commands rerun for this merge. Previous acceptance/failing-first evidence remains in branch history and the retained earlier scratch logs. The only remaining work is the two explicitly owner-deferred inbox declarations above; the requested PR merge verification is complete.
