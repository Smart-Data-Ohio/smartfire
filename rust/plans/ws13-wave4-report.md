# WS13 wave 4 report — partial

Pushed implementation: ac7d68cdf6859f3df4835ff0667eb8beecc234ac; catalogue continuation: 553c2eb4. Final report-only commit follows. origin/main was merged with commit 864d37ac from 20dc8ea3ce04818f3fc51236e7957b9a892cb88f, including #174 Events and #178 Rails deflakes. Both main and WS13 behaviors were retained; PR #172 was not merged.

The original 75 remaining declarations are all implemented. Seventy-three have complete assertion coverage across the runs: 30 ordinary join/presence/voice cases, eight conditional invitations, and all 35 real LiveKit declarations. Two invitation cases fail at unported inbox routes. The latest fresh real-media batch is 34/35, so this is not a claim of a green E2E batch. All 548 original titles remain in `ws13-deferred-tests.md`; WS13 covers 330 of its 332 titles, while WS13b's 216 assigned domain titles are unscored here and belong in its own report.

## Delivered

The ordinary fixture uses signed Rust sessions, genuine CSRF/Cable, and production grant issuance, sighting, broadcast, membership/session revocation and reconnect effects. No synthetic backend frames replace those effects. Join notices cover sounds, live regions, batching, mute cycles across navigation, genuine leave/rejoin, banners and sidebar pills. Presence covers expiry, in-flight polling, reconnects, removed stacks and hidden tabs. Voice covers public CRUD/member removal, chat, leave/presence, request sharing and narrow-phone geometry.

The real-media adapter uses the project's pinned LiveKit 1.13.7 server, its unchanged gateway, actual Rust internal endpoints and native browser WebRTC. It checks audio/video RTP, decoded camera/screen tracks, permanent huddle state across Turbo navigation, screen controls, device retargeting, capture constraints, RNNoise, meters, native statistics, refreshed-token full reconnects, original/refreshed token rejection after revocation and server enforcement against a browser that ignores access polling. Screen capture uses the original Rails browser-canvas helper. No recorded administrative responses substitute for these assertions; no pixel comparison was performed.

Both Rails and Rust used the same locally started server process. Signaling/admin are 127.0.0.1:7880, media is loopback UDP 7882, public gateway listeners are held ports in WS13's range, external-IP discovery is disabled and TURN is disabled. The fixture's SDK URL and real CSP read the same LIVEKIT_URL. Returning-user browser permissions match Rails' granted fake-media driver; fresh-permission cases retain the original explicit permission-query stub.

Existing controllers, 38 complete room-page goldens (including Designers), approved #163 layout reconciliation, domain and WS17 production signatures remain unchanged in this continuation. No new model/job/service Rails declaration was ported.

## WS13b dependency and exact remaining cases

The tested API snapshot is PR #172 branch `rust/ws13b-huddle-domain`, cd9f5a854910934ec619560ed78376cbef6d3b73. The tracked runner overlays exactly activity_item.rs, huddle_grant.rs, huddle_invitations.rs, huddle_notices.rs and jobs/huddle.rs in the isolated fresh checkout. The later current branch head 35ed3a80 has identical bytes for all six API/compile-dependency files (verified after final fetch). Its unchanged ring_matrix_tests.rs is copied only to satisfy module compilation; none of WS13b's domain tests is executed or counted. Every file is restored/removed in finally, and git diff is clean.

All ten titles in `test/system/huddle_invitations_test.rb` depend on WS13b's issuance/ring/state callbacks:

- the recipient sees an incoming huddle banner and dismissing it marks the item read
- joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel
- the banner flips to caller-left when the starter hangs up, then dismisses
- a ring stops itself after the ring timeout
- a banner-only ring stops itself after the ring timeout
- the banner stays hidden while already in the room's huddle
- an incoming call rings audibly until it is answered
- a silent invitation shows the banner without any sound
- a hidden tab raises a system notification for the call
- the ring stops when the caller leaves

Eight pass conditionally; the first two remain failing:

1. **the recipient sees an incoming huddle banner and dismissing it marks the item read**: GET /activity/unread_count is unported (501); its unread badge never reaches 1. PATCH /activity/:id/read is also unported and needed for the later original read assertion.
2. **joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel**: navigation and huddle:join pass, but PATCH /activity/:id/handled is unported (501); handled_at remains null.

The existing ActivityItem module assigns inbox accessibility/grouping/read/handled work to WS12. Implement/land these three public endpoints through that owner, then rerun both full declarations. Rerun all ten conditional invitations when #172 lands. No main-only acceptance is claimed for the eight passing conditional cases.

The current WS13b issuance broadcasts synchronously after commit. The adapter honors delivered/cancelled/superseded envelopes and calls `publish_queued_ring(tx: &mut Tx<'_>, id: i64) -> Result<()>` or `publish_ring_with_policy(tx: &mut Tx<'_>, request: &RingRequest, quiet_check: Option<&dyn Fn(&UserStatusSettings) -> bool>) -> Result<()>` for retained envelopes. Rails' explicit quiet-check decision is supplied through WS17's existing manual-DND input before synchronous issuance, without editing a broadcast frame. The isolated mixed API build emits an unused source_registry warning because its dispatcher is not overlaid; strict clippy below is for the restored WS13 branch, not that mixed build.

WS17 push seams remain `enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest)` and `prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool) -> Result<Option<PushDelivery>>`. Its policy/transport integration is untouched.

## Timing failures retained

The worktree 35-case run passed 34 and failed the device-picker case during the fixture's initial ten-second presence-idle readiness check; it had passed in the preceding 30-case run and passed again in the final fresh clone.

The final fresh 35-case run passed 34 and failed **server enforcement removes a revoked participant that ignores browser access checks** at its original pre-revocation assertion: the initial inbound audio-byte sample was zero immediately after media-ready polling. That entire declaration passed in the worktree 35-case run, including actual server removal and stable inbound-byte samples. The failure occurred before revocation, so the fresh run did not exercise its remaining enforcement assertions.

Neither wait was widened, no test concurrency was lowered, no assertion was removed or skipped, and neither timing failure was hidden by a filtered retry. Investigate these timing failures and obtain a clean 35-case real-media batch in the E2E continuation; all native interactions and assertions remain in the runnable suite.

## Fresh-clone verification

The checkout was created with `git clone --no-hardlinks --branch rust/ws13-huddles <WS13 worktree> .scratch/fresh-ws13-ordinary-20261001` and fast-forwarded only along WS13's pushed branch. Final Rust/browser inputs are ac7d68cd; subsequent commits change catalogue/report prose only. The WS13b overlay was restored before final main-only runs. Fixtures, goldens, scripts and compile dependencies come from tracked branch/git content; the local LiveKit environment is runtime configuration generated by bin/livekit-local setup.

Environment: CI=1, CARGO_BUILD_JOBS=2, dev/test debug=0, incremental=0, CAMPFIRE_REFERENCE and CARGO_TARGET_DIR point into that checkout, TMPDIR and npm cache are its .scratch, CABLE_TEST_PORT_RANGE=52300-52349, MAIL_TEST_PORT_RANGE=52350-52399. Rust is invoked through mise exec rust@1.98.1. Workspace test executables use ws13-reference:d7c7de92, --network none, --cpus 2, uid/gid 1000:1000 and the fresh source mounted at the same absolute path. No extra jobs were set; all Rust test commands use --test-threads=8, browser wrappers --test-concurrency=8 and Rails PARALLEL_WORKERS=1.

`cargo metadata --locked --format-version 1 >/dev/null`; strict tomllib workspace dependency-key parse:

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

`rust/parity/bin/seed build default first_run` (fresh clone):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

`python3 rust/reference-tools/huddle_sidebar_reference.py` (approved private Rails overlay), `ws13_verify_reference.py`, `ws13_verify_declarations.py`, `ws13_verify_corpora.py`:

```text
Sidebar reference: tracked #163 source SHA256 verified; isolated image built
```

```text
Reference identity: 97 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
```

```text
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
```

```text
quote_children: reference rerun byte-identical; 3 cases
row_broadcasts: reference rerun byte-identical; 16 cases
full_rooms: reference rerun byte-identical; 38 cases
room_shell: reference rerun byte-identical; 38 cases
room_composition: reference rerun byte-identical; 30 cases
chrome: reference rerun byte-identical; 23 cases
protocol: reference rerun byte-identical
cleanup: reference rerun byte-identical
grants: reference rerun byte-identical
gateway: reference rerun byte-identical; 39 cases
presence: reference rerun byte-identical; 50 cases
notices: reference rerun byte-identical; 67 cases
pushes: reference rerun byte-identical; 36 cases
issuance: reference rerun byte-identical; 49 cases
resolver: reference rerun byte-identical; 29 cases
stale_streams: reference rerun byte-identical; 16 cases
hands: reference rerun byte-identical; 17 cases
stream_lifecycle: reference rerun byte-identical; 20 cases
moderation: reference rerun byte-identical; 29 cases
stage_views: reference rerun byte-identical; 50 cases
participation: reference rerun byte-identical; 34 cases
stage_note: reference rerun byte-identical
call_channels: reference rerun byte-identical; 47 cases
call_views: reference rerun byte-identical; 16 cases
form_pages: reference rerun byte-identical; 14 cases
sidebar_views: reference rerun byte-identical; 8 cases
page_views: reference rerun byte-identical; 28 cases
public: reference rerun byte-identical; 65 cases
full_sidebar: reference rerun byte-identical; 17 cases
WS13 corpora: 29 complete files regenerated byte-identically
```

`cargo test --locked --manifest-path rust/Cargo.toml --workspace --no-fail-fast -- --test-threads=8` with the pinned Docker target runner above (all raw result lines):

```text
test result: ok. 1054 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 779.71s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.30s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 723 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 88.39s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.28s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.95s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.67s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.83s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.09s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.73s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.62s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.06s
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

`cargo test --locked --manifest-path rust/Cargo.toml -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8`:

```text
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 12053.155006
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 12.29s
```

`rust/parity/system/ws13` (main-only ordinary cases):

```text
ℹ tests 61
ℹ pass 61
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 167527.489157
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 169.94s
```

`python3 rust/parity/system/ws13-domain-system.py --domain-sha cd9f5a854910934ec619560ed78376cbef6d3b73` (fresh checkout, expected two public-route gaps):

```text
ℹ tests 71
ℹ pass 69
ℹ fail 2
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 208368.86407
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1065 filtered out; finished in 210.12s
```

`WS13b API overlay restored; WS13 source clean` is the runner's final restoration line.

`bin/livekit-local setup`, `source .bundle/livekit/env`, `bin/livekit-local start`; `WS13_LIVEKIT_ENV=<WS13 runtime env> rust/parity/system/ws13-livekit` from the fresh checkout:

```text
ℹ tests 35
ℹ pass 34
ℹ fail 1
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 171571.084164
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 173.33s
```

Original Rails baseline: ws13-reference:system-d7c7de92 (tracked system-reference Dockerfile from the approved #163 oracle), --network host, --cpus 2, --ipc host, uid/gid 1000:1000, RAILS_ENV=test, sourced LIVEKIT_* environment and PARALLEL_WORKERS=1. `bundle exec bin/rails db:setup && bundle exec bin/rails test test/system/stage_test.rb test/system/huddles_test.rb`, LIVEKIT_SYSTEM_TESTS=1:

```text
Finished in 221.915423s, 0.2073 runs/s, 3.6410 assertions/s.
46 runs, 808 assertions, 0 failures, 0 errors, 0 skips
```

Same image/environment/server, LIVEKIT_SYSTEM_TESTS=0: `bundle exec bin/rails db:setup && bundle exec bin/rails test test/system/huddle_join_notices_test.rb test/system/huddle_invitations_test.rb test/system/huddle_presence_test.rb test/system/voice_channels_test.rb`:

```text
Finished in 158.399792s, 0.2778 runs/s, 2.0518 assertions/s.
44 runs, 325 assertions, 0 failures, 0 errors, 0 skips
```

`cargo clippy --locked --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings` (restored WS13, exit 0):

```text
    Finished `dev` profile [unoptimized] target(s) in 21.51s
```

`git diff --exit-code` before and after final runs: exit 0. `cargo clean --manifest-path rust/Cargo.toml --target-dir <fresh>/rust/target` after validation:

```text
     Removed 6424 files, 3.0GiB total
```

Scratch fresh-clone targets remaining: []. The owned project-local LiveKit server was terminated after all runs; no foreign process was stopped.
