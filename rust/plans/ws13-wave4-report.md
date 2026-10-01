# WS13 timing continuation — verified slice, wider parity partial

Pushed implementation: 13dfb4a7c1b3625291208c5d0596391081a283b8. Commits: d23f2894 (inbox owner/route handoff), 04f5d30a (completed RTP sampling), 7e4f4c5a (complete-page navigation and diagnostics), 13dfb4a7 (stable browser namespace and opaque packet forwarding). The report-only commit follows. No merge of main or PR #172 occurred in this continuation. The baseline remains the previously merged main 20dc8ea3. No PR was opened; the lead will authorize that after #172 lands.

## Readiness root causes and comparison

**Initial device-picker presence-idle timeout:** the controller could not become idle because it never connected. Diagnostics reproduced the same ten-second guard failure with readyState complete, presence markup present but no controller and no aggregate fetch, while Chromium reported ERR_NETWORK_CHANGED on the huddle_presence_controller import and other lazy modules. A second startup had an empty sidebar frame after its request was cancelled by the same error. The media browser shared the host network namespace; Docker veth creation/removal on the shared machine invalidates Chromium requests. The existing parity HTTP forwarder documents and avoids this failure class already. This is a transport/startup failure, not Rust reporting presence readiness earlier than Rails.

The unchanged original Rails device-picker declaration also fails under controlled host bridge-interface churn at its initial sign-in navigation (Designers never appears); the accompanying server-enforcement declaration passes. It passes all three unperturbed original comparisons against the same server. Waiting longer cannot revive failed lazy module imports. The repair gives real-media Chromium its own stable Docker bridge namespace, like the ordinary parity browser's existing isolated namespace. Opaque ICE/DTLS/SRTP UDP packets traverse local Unix IPC to per-peer UDP sockets bound to 127.0.0.1 and the same local LiveKit UDP 7882. Gateway TCP uses the existing Unix loopback forwarder. No provider response, track, RTP counter or frame is synthesized or rewritten. A native 35/35 diagnostic run passed during 12 controlled host veth creation/removal cycles and logged 8,089 outbound / 4,732 inbound datagrams. Independent-peer/byte-preservation tests pass, and deliberate byte corruption is rejected.

An additional adapter mismatch was corrected: goto DOMContentLoaded had returned before full page load, whereas original Selenium uses normal/readyState complete. A delayed-controller regression fails at interactive and passes at complete. That correction alone did not fix the network outage: the preserved earlier fresh sequence was 35/35, 35/35, 33/35, with the final two failures furnishing the ERR_NETWORK_CHANGED evidence. It is not counted as the final clean streak. Navigation now waits for load; the 30-second navigation, 15-second Cable, ten-second presence-idle and two-second huddle-controller deadlines remain unchanged. No fixture assertion was removed. Future failures retain readiness, controller, request and page-error diagnostics.

**Zero inbound audio before server enforcement:** the pinned Playwright 1.63 waitForFunction implementation tests predicate truthiness before awaiting it. Our async getStats predicate returned a truthy Promise; polling stopped after the first sample even when it resolved false. This was false readiness in the adapter, not an observed premature Rust frontend signal. Rails' evaluate_async_script passes the resolved boolean to its callback and its outer Ruby loop retries. `pollBrowser` now awaits every evaluate sample and retries its resolved result with the original 20-second deadline and 100 ms interval. No-byte and delayed-positive regressions both failed before the change (0/2), then passed. The original pre-revocation positive-byte assertion remains, and all original enforcement interactions run.

The unchanged original Rails declarations passed three consecutive two-case runs (55 assertions each) on this server. The read-only probe also observes positive native RTP byte counts immediately after Rails' original media helper. No Rails test or application change was necessary. Shared huddle, presence and Stimulus controller sources match d7c7de92 byte for byte; Rust production readiness/render/domain/transport signatures were not changed.

Both runtimes used the same project-local LiveKit 1.13.7 process, loopback signaling/admin 7880 and UDP 7882. Rust's unchanged gateway used held WS13 ports; Rails used its original 7884 gateway and 3001 Capybara listener. External IP discovery and TURN remained disabled. Native WebRTC, decoded audio/video, microphone processing, screen capture, token-refresh/full reconnects, membership/session revocation and server enforcement are exercised. No recorded responses substitute for media and no pixel work was done.

## Exact remaining work and owner

The two open original invitation declarations remain:

1. `the recipient sees an incoming huddle banner and dismissing it marks the item read`: needs GET /activity/unread_count.json (`ActivityItemsController#unread_count`) and PATCH /activity/:id/read (`#read`; absent state defaults to read).
2. `joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel`: needs PATCH /activity/:id/handled (`#handled`; absent state defaults to handled).

Current specific owner: **WS11-UI**, branch rust/ws11ui-agent-pages. Its current wave4/ws11ui-report.md records inbox index/count and controllers/activity_items.rs at line 10, and explicitly lists read/handled plus HuddleGrant presentation as its inbox continuation at line 24. WS11's report leaves pages to that worker. The WS8b-m brief enumerates message controllers and its report leaves activity/work/board 501 seams. Decisions contains no reassignment. No WS12 brief exists in the delegation tree; the old generic WS12 source/plan label was historical. The full audit and Rails route/controller/client requirements are in [ws13-inbox-route-dependencies.md](ws13-inbox-route-dependencies.md). No foreign-owner endpoint was implemented here.

All ten original invitation declarations still depend on WS13b's issuance/ring/state API. The eight conditional passes remain inventoried as conditional; they were not rerun or claimed as main-only passes in this continuation. Once the lead authorizes merging main after #172, rerun all ten without the overlay; land the three inbox endpoints through WS11-UI and rerun both full open declarations. Then open the WS13 PR when authorized. The overlay runner is retained unchanged; no overlay was active for any fresh verification below. WS13b and WS17 seams stay unchanged. The WS17 producer signatures remain `enqueue_huddle_push(tx: &mut Tx<'_>, request: &PushRequest)` and `prepare_push(tx: &mut Tx<'_>, request: &PushRequest, policy_allowed: bool) -> Result<Option<PushDelivery>>`.

The 548-title catalogue keeps every declaration: WS13 has 226 controller/integration declarations and 106 system declarations, with 330 assertion-covered and the above two open. WS13b's 216 owned titles are unscored here; no historical WS13b pass counts are copied. The 35 real-media cases now have three consecutive clean fresh-clone batches. Existing full page/Designers/sidebar/chrome/CSP work is unchanged and remains exercised by the seeded workspace tests.

## Fresh-clone verification and raw summaries

A new no-hardlinks clone of the WS13 branch was created at `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws13/.scratch/fresh-ws13-timing-20261001`. It was fast-forwarded only on WS13 through 13dfb4a7c1b3625291208c5d0596391081a283b8. Its default and first_run seeds were built from the pinned tracked seed inputs. No untracked fixture or pre-existing target data was copied. `git diff --exit-code` was clean before and after verification. CI=1 requires the seeds; no missing-seed skip occurred. CARGO_BUILD_JOBS=2, dev/test debug=0, incremental=0; test threads/concurrency=8. Rails PARALLEL_WORKERS=1 is its original LiveKit configuration. CAMPFIRE_REFERENCE and CARGO_TARGET_DIR point into the fresh clone; TMPDIR/npm cache use its scratch. The local LiveKit env is runtime configuration generated by bin/livekit-local setup.

`git clone --no-hardlinks --branch rust/ws13-huddles <WS13 worktree> .scratch/fresh-ws13-timing-20261001`; `rust/parity/bin/seed build default first_run` (pinned seed construction):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

`cargo metadata --locked --format-version 1 >/dev/null`; strict tomllib workspace dependency-key parse:

```text
Locked cargo metadata: ok
Workspace dependency keys: 77 unique; duplicates: []
```

`python3 rust/reference-tools/ws13_verify_reference.py`; `ws13_verify_declarations.py`:

```text
Reference identity: 97 files match d7c7de92
Post-#163 sidebar source: tracked SHA256 matches 2e20b24c
Post-#163 application layout: tracked source and oracle image SHA256 match 2e20b24c
Rails declaration catalogue: 548 titles retained; WS13 330 passed / 2 open; WS13b 216 owned, unscored; 33 files; source titles match
```

Original Rails comparison: source project-local env; docker image ws13-reference:system-d7c7de92, host network, cpus=2, uid/gid=1000, original gateway dependencies and private tmp; `bundle exec bin/rails db:setup`; three invocations of `bundle exec bin/rails test test/system/huddles_test.rb -n "/device.pickers.list.the.fake.devices.and.switching.keeps.media.flowing|server.enforcement.removes.a.revoked.participant/"`, LIVEKIT_SYSTEM_TESTS=1, PARALLEL_WORKERS=1:

```text
Finished in 11.529551s, 0.1735 runs/s, 4.7704 assertions/s.
2 runs, 55 assertions, 0 failures, 0 errors, 0 skips
Finished in 11.450953s, 0.1747 runs/s, 4.8031 assertions/s.
2 runs, 55 assertions, 0 failures, 0 errors, 0 skips
Finished in 11.358896s, 0.1761 runs/s, 4.8420 assertions/s.
2 runs, 55 assertions, 0 failures, 0 errors, 0 skips
```

Same runtime/env, read-only tracked probe: `bundle exec ruby -Itest /rails/rust/reference-tools/ws13_media_readiness_probe.rb -n "/device.pickers.list.the.fake.devices.and.switching.keeps.media.flowing|server.enforcement.removes.a.revoked.participant/"`:

```text
WS13 Rails navigation: {"presence_controller":true,"presence_in_flight":false,"ready_state":"complete","page_load_strategy":"normal"}
WS13 Rails navigation: {"presence_controller":true,"presence_in_flight":false,"ready_state":"complete","page_load_strategy":"normal"}
WS13 Rails completed RTP sample: {"kind":"audio","bytes":345}
WS13 Rails navigation: {"presence_controller":true,"presence_in_flight":false,"ready_state":"complete","page_load_strategy":"normal"}
WS13 Rails completed RTP sample: {"kind":"audio","bytes":3796}
WS13 Rails completed RTP sample: {"kind":"audio","bytes":5544}
WS13 Rails completed RTP sample: {"kind":"audio","bytes":7638}
Finished in 14.035947s, 0.1425 runs/s, 3.9185 assertions/s.
2 runs, 55 assertions, 0 failures, 0 errors, 0 skips
```

Native-browser failing-first regressions, same pinned Playwright image and test-concurrency=8. Original Promise sampling:

```text
ℹ tests 2
ℹ pass 0
ℹ fail 2
ℹ duration_ms 690.12999
```

After RTP repair but before navigation repair:

```text
ℹ tests 3
ℹ pass 2
ℹ fail 1
ℹ duration_ms 1244.229499
```

Final navigation + asynchronous sampling regressions:

```text
ℹ tests 3
ℹ pass 3
ℹ fail 0
ℹ duration_ms 1417.773916
```

Original Rails under controlled host interface churn (same original two-case pattern, same LiveKit process):

```text
Finished in 21.963709s, 0.0911 runs/s, 1.2293 assertions/s.
2 runs, 27 assertions, 1 failures, 0 errors, 0 skips
```

Isolation diagnostic fault injection and byte-corruption gate (not final fresh acceptance):

```text
WS13 isolation fault injection: 12 host veth creation/removal cycles during native media batch
```

```text
ℹ tests 3
ℹ pass 3
ℹ fail 0
ℹ duration_ms 1226.065446
WS13 local media datagrams: 8089 sent; 4732 received
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ duration_ms 214863.494622
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 218.20s
```

```text
ℹ tests 1
ℹ pass 0
ℹ fail 1
ℹ duration_ms 38.24217
```

```text
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ duration_ms 1320.485818
```

Fresh real-media batches: `rust/parity/system/ws13-livekit` three consecutive invocations. Each first runs the four standalone regressions; then the ignored Rust wrapper launches the original 35 native-media interactions. No filtered retry/reset of the successful streak:

Batch 1:

```text
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ duration_ms 896.071422
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ duration_ms 168250.185402
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 170.75s
```

Batch 2:

```text
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ duration_ms 957.361382
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ duration_ms 161983.314736
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 165.21s
```

Batch 3:

```text
ℹ tests 4
ℹ pass 4
ℹ fail 0
ℹ duration_ms 997.880353
ℹ tests 35
ℹ pass 35
ℹ fail 0
ℹ duration_ms 165377.075621
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 167.53s
```

Main-only seeded workspace: mise Rust 1.98.1 `cargo test --locked --workspace --no-fail-fast -- --test-threads=8`, using target.x86_64-unknown-linux-gnu.runner to run the same fresh binaries in ws13-reference:d7c7de92 (--network none, cpus=2, uid/gid=1000, fresh source mounted at its identical absolute path). No ws13b_domain_api cfg or source overlay:

```text
test result: ok. 1054 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 900.95s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.91s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.45s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 723 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 110.33s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.57s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.53s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.76s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.56s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.45s
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

Gateway acceptance: `cargo test --locked -p campfire huddle_gateway_own_node_suite_against_rust_endpoints -- --ignored --nocapture --test-threads=8`:

```text
ℹ tests 16
ℹ pass 16
ℹ fail 0
ℹ duration_ms 14492.440328
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 14.81s
```

Main-only ordinary browser: `rust/parity/system/ws13`:

```text
ℹ tests 61
ℹ pass 61
ℹ fail 0
ℹ duration_ms 193808.426444
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 195.53s
```

Strict main-only clippy: mise Rust 1.98.1 `cargo clippy --locked --workspace --all-targets -- -D warnings`:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 02s
```

Normal production binary, no overlay: mise Rust 1.98.1 `cargo build --locked -p campfire`:

```text
    Finished `dev` profile [unoptimized] target(s) in 58.80s
```

Cleanup: fresh `cargo clean` and project-local LiveKit termination; no scratch target or own LiveKit process remains:

```text
Project-local LiveKit: SIGTERM sent to verified WS13 server PID 3387686
Project-local LiveKit: exit 0; verified server PID is gone
     Removed 7517 files, 3.4GiB total
Fresh scratch cargo target: deleted
```

Verification logs remain under this worktree's .scratch/timing-logs and .scratch/timing-isolated-fresh-logs. Failed diagnostic and earlier fresh batches are retained. The diagnostic isolated run had a trailing shell error because its wrapper was edited while active; the 35 native cases passed, but that invocation is not claimed as a green wrapper. Final verification uses immutable committed fresh sources. The relay unit initially triggered a Node callback-scope abort with a 109-byte Unix socket address; a short relative IPC address avoids the Linux 108-byte address limit, including in deeply nested fresh clones. Coredump evidence showed SIGABRT, not OOM; no core was extracted. Final native/regression batches use the pinned browser Node runtime. No waits, thresholds, parity masks, test concurrency or production seams were weakened.
