# WS13 — PR #185 Astra fixes and fresh-clone verification

Verified implementation: `0ebb2e9b005c98000a529a79a8ef90b8be0ab818` on `rust/ws13-huddles`; the report-only commit follows. Main was merged first in a47b0ebf79737f55c077b2cb5ed6a1746bd3cecd, with parents 02fcb2d6 and 59ad94de3d0c53dfc15cae95901453f6506a7e83. The fetched main contains requested 7442031d plus #183's fixture-runner changes. All five P2 findings and the P3 CI default are addressed. The lead's PR #185 is updated by the pushed branch.

## Fixes and merge resolutions

- Room deletion now commits WS8a/WS13b's begin_destroy transaction before recording the audit in a second transaction. Rails' recorded audit failure returns 500 with deleted=true and zero memberships; Rust now matches. Durable deletion, grant revocation and stream cleanup remain in the original deletion transaction.
- Stage navigation is rendered even without LiveKit configuration. Show stage, the dialog and roster remain present; the media launcher stays gated. The persisted viewer-zone lookup is unchanged.
- Full sidebars preload all visible rooms' participants in one query and live Stage streams/presenters in one query. Participants remain device-deduplicated and sorted by case-insensitive name. The 100-quiet-room regression originally counted 112 participant and 112 stream queries including seed rooms; it now requires at most Rails' recorded one of each. Callback row adapters retain their existing signatures and behavior.
- Configured room navigation uses main's Presenter room view and HeaderIdentity. Open, Closed, Direct, Voice, Stage and Board preserve their existing kind labels and header IDs; Board remains Board/header_rooms_board_*.
- All eight stale mutation anchors are repaired. The mutation runner preflights the whole selected catalogue, reports every missing anchor without editing source, verifies each selected baseline passes and runs tests, and continues to collect every compile failure or survivor. Compiler errors never count as detected behavior regressions. Cache/privacy tests now exercise the shared production cache and actual room-page path; the warm-cache fixture uses main's frozen clock so a later edit is actually later than the source's creation. No wait or timing threshold was widened.
- ci/cargo.sh uses CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}": four by default, explicit local overrides honored. The local run uses two jobs and the configured machine-wide rustc throttle unchanged.
- Main's agent payload presenter, user lifecycle module and without_job_runner helper are retained. WS13 fixtures adopt the main helper; the merged agent recovery test drops a needless borrow identified by strict clippy. Main's channel_threads/page_tests.rs (#180), viewer-zone/cache files (#179), events and refresh paths match main byte for byte. WS13b/WS17 domain and transport signatures remain unchanged. No overlay script or private cfg remains.

## Failing-first regressions and Rails values

The four new HTTP/query regressions were added before the behavior fixes and failed against the merged baseline. The initial broad review_ filter also ran 46 existing review regressions. Command: mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire review_ -- --nocapture --test-threads=8.

```text
    Finished `test` profile [unoptimized] target(s) in 2m 22s
test result: FAILED. 46 passed; 4 failed; 0 ignored; 0 measured; 1819 filtered out; finished in 23.86s
```

The mutation-anchor regression failed with eight missing anchors before repairing them. Command: python3 rust/reference-tools/test_huddle_discrimination.py.

```text
First list contains 8 additional elements.
First extra element 0:
Ran 2 tests in 0.012s
FAILED (failures=1)
```

Rails production requests and query notifications were recorded from the pinned local reference image using the tracked recorder:

PARITY_NAMESPACE=ws13 PARITY_OWNER=ws13 PARITY_CPUS=2 PARITY_IMAGE=ws13-reference:d7c7de92 rust/parity/bin/reference runner --seed default -e RAILS_LOG_LEVEL=fatal rust/reference-tools/huddle_review_fixes.rb > rust/vectors/huddle_review_fixes.json

Recorded audit, Stage and sidebar values:

```text
{
  "reference_pin": "d7c7de92",
  "audit_failure": {
    "status": 500,
    "deleted": true,
    "memberships": 0
  },
  "unconfigured_stage": {
    "status": 200,
    "show_stage": true,
    "dialog": true,
    "roster": true,
    "media_launcher": false
  },
  "quiet_sidebar": {
    "status": 200,
    "participant_queries": 1,
    "stream_queries": 1
  }
}
```

The same recording retains all six room-kind labels and header ID patterns, including Board/header_rooms_board_:id. No Rails fixture expectations were adjusted to fit Rust.

Fresh regression command: mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire controllers::rooms::review_tests -- --test-threads=8.

```text
    Finished `test` profile [unoptimized] target(s) in 7m 49s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 1.22s
```

Fresh final-source mutation catalogue preflight: python3 rust/reference-tools/test_huddle_discrimination.py.

```text
...
----------------------------------------------------------------------
Ran 3 tests in 0.008s

OK
```

Baseline guard against the prior runner (the negative check intentionally fails):

```text
Ran 3 tests in 0.460s
FAILED (failures=2)
```

All 102 final mutation definitions compile and are rejected by their selected assertion failures. The full default sweep reported 100 rejections and two unproven entries: a type-invalid blank-sidebar mutation and a deleted-room mutation that left a redundant guard intact. Those two were corrected and rerun successfully. One reported rejection was then found to be a false positive caused by the prototype full-page test's stale input comparison. After fixing that comparison, the baseline passed but that mutation survived: it targeted the native template while the selected test used a prototype template. Retargeting to the native HTTP page golden gives a passing baseline and a compiled assertion failure in all four unchanged Rails fixtures. This is a 99+2+1 proof, not a claim of one clean 102-entry default run. The runner now explicitly rejects unhealthy or empty selected baselines before editing source; its guard regression fails against the prior runner. The runner reports unproven entries and returns nonzero; it does not abort on the first one. The 102 individual raw summaries are committed in rust/plans/ws13-review-mutations.txt.

Commands: python3 rust/reference-tools/huddle_discrimination.py; then python3 rust/reference-tools/huddle_discrimination.py --only '^(full-sidebar-request-composition-bypassed|public-deleted-room-exposed)$'. The final native-page mutation rerun uses python3 rust/reference-tools/huddle_discrimination.py --only '^room-populated-collection-rendering-bypassed$'. Runs use CI=1, two local build jobs, CARGO_INCREMENTAL=1, the existing compiler throttle, the isolated port ranges and eight Rust test threads. Sources were restored after every mutation.

```text
WS13 discrimination: 100/102 compiled regressions detected; sources restored
Unproven mutations: ['full-sidebar-request-composition-bypassed', 'public-deleted-room-exposed']
full-sidebar-request-composition-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 1.05s
public-deleted-room-exposed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 17.92s
WS13 discrimination: 2/2 compiled regressions detected; sources restored
BASELINE PASSED: campfire full_native_room_pages_match_four_complete_rails_pages
room-populated-collection-rendering-bypassed: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 1.19s
WS13 discrimination: 1/1 compiled regressions detected; sources restored
Compiled mutation aggregate: 102/102 rejected; sources restored
```

CI docker-argument capture (the real CI script, with an argument-only docker stub) verifies both default and explicit override:

```text
CI default: CARGO_BUILD_JOBS=4
Local override: CARGO_BUILD_JOBS=2
```

## Exact remaining declarations and owners

Only two nonblocking declarations in test/system/huddle_invitations_test.rb remain deferred, under the lead's ruling:

| Original declaration | Required public actions | Owner |
| --- | --- | --- |
| the recipient sees an incoming huddle banner and dismissing it marks the item read | GET /activity/unread_count.json; PATCH /activity/:id/read | WS11-UI, rust/ws11ui-agent-pages |
| joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel | PATCH /activity/:id/handled | WS11-UI, rust/ws11ui-agent-pages |

Astra confirmed these public actions return 501. Their complete test code enables with WS13_ENABLE_INBOX_CASES=1. The eight other invitations pass through main's merged WS13b APIs without any overlay. No LiveKit case remains deferred. The 548-title/33-file catalogue retains WS13's 332 declarations: 330 assertion-covered and the two above deferred. WS13b's 216 declarations are owned and unscored here; its results belong to its own report. Exact per-file counts and titles remain in rust/plans/ws13-deferred-tests.md.

## Fresh clone and environment

A new no-hardlinks clone was created at .scratch/fresh-ws13-review185 from pushed 6aa3d584, and both seeds rebuilt from tracked pinned inputs; no target, seed or local fixture was copied. Invitation/media/browser/gateway acceptance and focused regressions ran there on 6aa3d584. It was then fast-forwarded to final source 0ebb2e9b before the final full workspace, clippy and sealed build. Since 6aa3d584 only the mutation tools and prototype full-page adapter test changed; the Rust production implementation and acceptance inputs are byte-identical. The earlier full run was stopped after finding the stale prototype input comparison. Its isolated reproduction fails; the strengthened test renders production navigation and passes all 38 unchanged Rails response fixtures. The full suite is rerun after that correction. Tracked sources are clean before and after verification; scratch output is generated by these commands.

Commands use mise Rust 1.98.1, CI=1, CARGO_BUILD_JOBS=2, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0, with CAMPFIRE_REFERENCE/CARGO_TARGET_DIR inside that clone. TMPDIR and npm cache use its scratch; Cable ports are 52300–52349 and mail ports 52350–52399. Test threads and explicit Node concurrency stay at eight. Rails is pinned to d7c7de92 plus approved #163 drift. No compiler-throttle configuration, timing threshold, production domain seam, Rails expectation or python model server was changed. No pixel comparisons were performed.

rust/parity/bin/seed build default first_run:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 >/dev/null from fresh/rust; strict tomllib parse of workspace.dependencies (duplicates are rejected):

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

Source identity checks against git show 59ad94de:<path>; python3 rust/reference-tools/ws13_verify_reference.py; python3 rust/reference-tools/ws13_verify_declarations.py:

```text
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
Reference identity: 97 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
```

## Acceptance suites

Invitations: WS13_INVITATIONS_ONLY=1 rust/parity/system/ws13. Only the two owner-deferred cases are skipped.

```text
    Finished `test` profile [unoptimized] target(s) in 0.40s
ℹ tests 10
ℹ pass 8
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 23126.207754
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 25.24s
```

Real media: rust/parity/system/ws13-livekit. The actual project-local LiveKit server uses loopback signaling/admin 7880 and media UDP 7882, external discovery/TURN disabled. Real WebRTC tracks, audio/video decode, RTP statistics, token-refresh/full reconnects and server enforcement are asserted. Four polling/transport regressions run first.

```text
WS13 local media datagrams: 2 sent; 2 received
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 1057.032193
    Finished `test` profile [unoptimized] target(s) in 0.14s
WS13 local media datagrams: 7665 sent; 4406 received
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 203068.638897
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 206.04s
```

Ordinary browser: rust/parity/system/ws13.

```text
    Finished `test` profile [unoptimized] target(s) in 0.22s
ℹ tests 71
ℹ pass 69
ℹ fail 0
ℹ skipped 2
ℹ duration_ms 268443.281824
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 271.32s
```

Gateway own Node suite against Rust: mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8.

```text
    Finished `test` profile [unoptimized] target(s) in 0.22s
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ skipped 0
ℹ duration_ms 17602.291593
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 18.11s
```

Additional full-page test verification: the isolated original adapter test fails, then all 38 unchanged response fixtures pass after rendering the real navigation adapter. Both runs use the same pinned runner and eight-thread flag.

```text
    Finished `test` profile [unoptimized] target(s) in 0.47s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 2.64s
    Finished `test` profile [unoptimized] target(s) in 1m 55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 6.89s
```

## Full workspace, strict clippy and sealed build

mise exec rust@1.98.1 -- cargo --config 'target.x86_64-unknown-linux-gnu.runner=["docker", "run", "--rm", "--name", "ws13-pinned-workspace-test", "--label", "com.smartfire.rust-parity.owner=ws13", "--cpus", "2", "--network", "none", "--user", "1000:1000", "--volume", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185:/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185", "--workdir", "/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185", "--env", "CI=1", "--env", "TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185/.scratch", "--env", "CABLE_TEST_PORT_RANGE=52300-52349", "--env", "MAIL_TEST_PORT_RANGE=52350-52399", "--entrypoint", "/usr/bin/env", "ws13-reference:d7c7de92"]' test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8.

The runner is docker run --rm --name ws13-pinned-workspace-test --label com.smartfire.rust-parity.owner=ws13 --cpus 2 --network none --user 1000:1000 --volume /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185:/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185 --workdir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185 --env CI=1 --env TMPDIR=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-review185/.scratch --env CABLE_TEST_PORT_RANGE=52300-52349 --env MAIL_TEST_PORT_RANGE=52350-52399 --entrypoint /usr/bin/env ws13-reference:d7c7de92. Fresh binaries run with pinned libvips/ffmpeg and their original absolute reference paths. Every target/doctest raw summary follows:

```text
    Finished `test` profile [unoptimized] target(s) in 1m 14s
test result: ok. 1864 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1233.58s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.09s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 172.00s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.83s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.90s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.30s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.98s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.75s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.95s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.17s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.26s
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
Workspace aggregate: 3718 passed; 0 failed; 14 ignored (unit, integration and doctest summaries)
```

Strict clippy: mise exec rust@1.98.1 -- cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings.

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 09s
```

Release-input build: bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins. RUNNER_TEMP and the sealed inputs are in the fresh clone's scratch; RUST_CI_CONTAINER_PREFIX=ws13 and RUST_CI_IMAGE=sha256:80bed826ce3b998e8ba75b85b25d18055066760982413e4483dd9779c5f053b2 select the local pinned toolchain. CARGO_TARGET_DIR=/src/rust/target. The untracked local Docker adapter only mounts the configured rustc slot wrapper/shared lock pool into this local container; it does not rewrite the CI job argument or change the throttle configuration. The source CI default remains four; the explicit local override is two.

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 44s
```

All gates exited zero; fresh git diff --exit-code passed before and after. The owned project-local LiveKit process was stopped and verified gone. The fresh scratch cargo target was measured and removed after all gates; the permanent worktree target was retained. No other process or target was touched.

```text
Project-local LiveKit: stopped WS13 server PID 1549613; exit 0; verified PID gone
     Removed 10541 files, 5.5GiB total
Fresh scratch cargo target: deleted
```
