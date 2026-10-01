# WS11-api wave 4 — PARTIAL continuation, 2026-10-01

Code head: `2703c7f7ada1867e1b42411428b79fc47e6e1cfb`, pushed on `rust/ws11api-rest-mcp`; a report-only commit follows. Four coherent slices were committed and pushed. Selected successes remain **31/35 REST actions and 33/38 MCP tools**. The nine missing writes still call the explicit WS12 seam; WS15g's installed private-PR access seam is unchanged. No main merge, stash or rebase occurred this turn; the branch retains main through `59ad94de` (#183).

**Not only owner-blocked items remain.** The unblocked API work below is still outstanding. This report supersedes the previous continuation report and does not claim exhaustive or one-for-one Rails-file coverage.

## Pushed slices and files

| Commit | Result |
| --- | --- |
| `e2a46734` | 168 poll REST/MCP vectors; broad date parsing, day/month/year precedence, Parameters versus Hash coercion |
| `14664358` | 71 bot vectors; page windows/Link headers, malformed cursors, icon fields and invalid reply/bot/human keys |
| `ff236a7e` | 64 attachment vectors; signed image/video reuse, binary upload, analysis failures and full persisted blob metadata |
| `2703c7f7` | 63 warmed-read policy transitions; three compiled mutations; 16 artifacts and 76 source pins |

Paths are relative to `rust/`.

- `crates/campfire/src/controllers/agents/{polls,pending}.rs`: both transports reuse main's `parse_calendar_time` instead of a second date parser. REST options objects are one Parameters element, while MCP Hash objects become pairs. Existing auth/grant/budget/validation ordering remains covered.
- `crates/db/src/slash_commands/time_parser.rs`: small cross-workstream correction to main's shared Calendar/slash parser: a day-bearing form (`3 Mar 2026`) wins over its embedded month/year. No WS11 model or service was duplicated.
- `crates/campfire/src/controllers/bot_http_tests.rs`, `reference-tools/agents/bot_http_contract.rb`, `vectors/agent_bot_http.json`: raw JSON/redirect/500 bodies, statuses, header presence/absence, exact page windows, persisted rows, stale/tampered/wrong-room replies and invalid keys. Test fixtures use main's `TestApp::without_job_runner()`.
- `crates/campfire/src/controllers/agent_reads_tests.rs`, `reference-tools/agents/attachments_http_contract.rb`, `vectors/agent_attachments_http.json`: full blob metadata assertions, existing signed image/video blobs, replay/budget/grant/board outcomes, binary image upload and actual corrupt-media processing. The committed Rust moon bytes are checked against the pinned Rails fixture. Corrupt media retains the committed message/blob and returns production 500 in both apps. These tests call the existing posting/storage paths; no production media implementation was added.
- `crates/campfire/src/controllers/agent_permissions_tests.rs`, `crates/campfire/src/controllers.rs`, `reference-tools/agents/permissions_http_contract.rb`, `vectors/agent_permissions_http.json`: successful raw responses followed by grant/membership removal, credential revocation/expiry, suspension, owner deactivation or banning. Four readers use both transports; history is MCP-only. Owner transitions call main's real lifecycle APIs.
- `reference-tools/agents/{check-permission-media-mutations,check-http-reference,record-http-vectors,verify-http-vectors}.py`: compiled negative proofs for current read grants, reply-room binding and synchronous analysis; source restoration in `finally`; exact-byte oracle registration and four added source/fixture pins.

The corpus grew by **205 cases** to **1,250 cases across 16 artifacts**, with **70 explicitly asserted successful warm bodies**. No masks, tolerances, ignores, assertions, test concurrency or timing thresholds were loosened. WS14g's previously pushed polling HTTP shape remains available and unchanged this turn.

## Failing-first and selected positive evidence

New poll cases failed before implementation on `March 3, 2026 5pm` (422 versus Rails success). Reusing main's parser then exposed `3 Mar 2026` being parsed as March 1; its precedence was corrected.


```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 26.94s
```


```bash
python3 rust/reference-tools/agents/check-permission-media-mutations.py
```


```text
WS11-api current_read_grant mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 1.20s
WS11-api reply_room_scope mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 3.60s
WS11-api media_analysis mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1868 filtered out; finished in 37.72s
WS11-api permission/media mutations: 3 broken guards rejected; sources restored
```

All three mutations compiled and failed at runtime assertions; their sources were restored. Selected positives below ran before the mutations, with the same restored behavior subsequently rebuilt in the fresh clone.


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_polls -- --nocapture
CI=1 TMPDIR="$PWD/.scratch/continue8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 rust/target/debug/deps/campfire-55d4acca837cdb4b bot_http --nocapture
CI=1 TMPDIR="$PWD/.scratch/continue8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 rust/target/debug/deps/campfire-55d4acca837cdb4b agent_attachments --nocapture
CI=1 TMPDIR="$PWD/.scratch/continue8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 rust/target/debug/deps/campfire-55d4acca837cdb4b agent_permissions --nocapture
```


```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1867 filtered out; finished in 83.14s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1857 filtered out; finished in 59.16s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1864 filtered out; finished in 31.79s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1867 filtered out; finished in 31.67s
```

## Fresh-clone suite and strict clippy

The clone was made from the pushed code head; both seeds were rebuilt from the pin. After owned build/test processes exited, the one target was moved into the clone as a compiler cache. Changed source/reference paths forced fresh app/crate compilation. Cargo used two jobs, the configured rustc throttle and unchanged default test concurrency.


```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/fresh8
PARITY_NAMESPACE=ws11api-fresh8 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh8/rust/parity/bin/seed build default first_run
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh8-tmp" mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh8/rust/Cargo.toml --format-version 1
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh8/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture
python3 .scratch/fresh8/rust/reference-tools/agents/summarize-http-tests.py .scratch/continue8/fresh-workspace.log
```

Locked metadata exited 0. Raw seed and aggregate lines:


```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
WS11-api cargo totals: 3708 passed; 3 failed; 12 ignored; 58 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

All native target summaries (native workspace exits 101):


```text
test result: FAILED. 1865 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 450.26s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.73s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 280.79s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 8.09s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.97s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.89s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.75s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.47s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.42s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Two native failures are the account-logo PNG byte comparison and storage's pinned-media version gate. Both exact fresh-clone targets pass unchanged in the pinned runtime below. The preceding delivered report baseline-confirmed these same failures; that earlier baseline run is not re-claimed here. No golden or test assertion changed.


```bash
vips --version
ffmpeg -version
docker run --rm --name ws11api-check8-vips-version --network none --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"'
docker run --rm --name ws11api-check8-ffmpeg-version --network none --entrypoint /usr/bin/ffmpeg ws11api-reference:d7c7de92 -version
```


```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

The initial native run also failed the unchanged richtext timing guard: 200 nested attachments took 5.740984219 seconds against its five-second bound. Its complete unchanged target passed on one rerun below, with default concurrency and the original threshold. The initial failure is retained in the raw native summaries; no inherited-failure claim is made for it.


```bash
CI=1 TMPDIR="$PWD/.scratch/fresh8-tmp" .scratch/fresh8/rust/target/debug/deps/hardening-ad131644ca4fd322 --nocapture
```


```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.43s
```


```bash
docker run --rm --name ws11api-fresh8-pinned-app --network none --user "$(id -u):$(id -g)" -e CI=1 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/fresh8-tmp" -v "$PWD/.scratch/fresh8:$PWD/.scratch/fresh8" -v "$PWD/.scratch/fresh8-tmp:$PWD/.scratch/fresh8-tmp" --entrypoint "$PWD/.scratch/fresh8/rust/target/debug/deps/campfire-abe1b035fe2b45bc" ws11api-reference:d7c7de92 --nocapture
docker run --rm --name ws11api-fresh8-pinned-storage --network none --user "$(id -u):$(id -g)" -e CI=1 -e TMPDIR="$PWD/.scratch/fresh8-tmp" -v "$PWD/.scratch/fresh8:$PWD/.scratch/fresh8:ro" -v "$PWD/.scratch/fresh8-tmp:$PWD/.scratch/fresh8-tmp" --entrypoint "$PWD/.scratch/fresh8/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture
```


```text
test result: ok. 1866 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 629.49s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 13.38s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh8-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh8/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```


```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 28s
```

## Fresh Rails oracle and controller files


```bash
PARITY_NAMESPACE=ws11api-check8 python3 rust/reference-tools/agents/record-http-vectors.py .scratch/continue8/oracles
python3 rust/reference-tools/agents/verify-http-vectors.py .scratch/continue8/oracles
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/test-http-vector-verifier.py
```


```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 269 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 71 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh readers oracle: 60 request/response pairs; byte-identical committed vectors
WS11-api fresh pins oracle: 18 request/response pairs; byte-identical committed vectors
WS11-api fresh polls oracle: 168 request/response pairs; byte-identical committed vectors
WS11-api fresh polling oracle: 12 request/response pairs; byte-identical committed vectors
WS11-api fresh reactions oracle: 44 request/response pairs; byte-identical committed vectors
WS11-api fresh bot reactions oracle: 13 request/response pairs; byte-identical committed vectors
WS11-api fresh work validation oracle: 99 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 76 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.005s

OK
```

Nine pinned controller files were rerun with the runner's unchanged behavior: 169 runs and 734 assertions. These reference passes establish the source behavior, not one-for-one Rust coverage.


```bash
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agents/polls_controller_test.rb test/controllers/agents/contexts_controller_test.rb test/controllers/agents/work_controller_test.rb test/controllers/agents/posts_controller_test.rb test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb test/controllers/messages/by_bots_controller_test.rb test/controllers/messages/boosts/by_bots_controller_test.rb
```


```text
test/controllers/agents/polls_controller_test.rb: 12 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/contexts_controller_test.rb: 13 runs, 50 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

| Source files | Selected Rust ports and deferrals |
| --- | --- |
| agents/polls_controller | 168 raw REST/MCP pairs; compact/partial date, broader DST and ID/input permutations remain API |
| agents/contexts_controller; work_controller; posts_controller | Existing reads and 63 warmed-read transitions; write successes and owner-dependent validation/callbacks remain WS12; other validation/viewer/race cases remain API |
| agent_capability; revocation_endpoints; owner_deactivation | Successful-then-denied reads and compiled removed-grant proof; concurrent write/finalization/delivery cases remain API/domain owners |
| messages/by_bots; messages/boosts/by_bots | 71 bot cases, prior grouped boosts, 23 bot attachment cases and queue rollback; remaining legacy, webhook/hop/DM, reply/thread and viewer/cache cases remain API/domain owners |
| Other API/MCP files | Existing selected oracle artifacts and the full Rust suite were rerun; older Rails-file runner counts are not re-claimed |

## Precisely remaining

- **WS12 owner-blocked, unchanged:** REST board-post create, work update/result/handoff; MCP create_board_post, update_board_post, update_work, set_result, handoff_work. They retain the explicit `agent_api_pending::execute` seam. Existing auth/grant/selected validation paths stay ported. Successful writes, remaining owner/context-package validations and callbacks must use WS12's complete services when available.
- **Unblocked API validation:** finish the older endpoint/tool field, ID/array/hash/coercion, length and callback-precedence matrices; remaining list/filter/cursor permutations; compact/partial dates and broader user-zone/DST grammar. The new poll matrix is selected evidence, not exhaustive validation.
- **Unblocked bot/media:** legacy bots without an Agent row; remaining webhook/self-mention/hop/DM fanout, update/replacement/purge, MIME/representation and root/thread reply/replay/callback interactions; remaining viewer/cache changes. Existing signed image/video success, full metadata and corrupt-media row retention are now covered.
- **Unblocked/dependent permissions:** concurrent revocation/write races, remaining viewer redaction and work/PR snapshots, finalization/delivery-failure interactions. The seven sequential authority changes now run across warmed readers; reply expiry/tamper/room binding are proven.
- **Integration owners:** live private-PR access/network execution stays WS15g through main's existing seam; full external delivery/network behavior stays with the domain/integration owners. No live GitHub access was implemented.

No approval or clarification is pending. This remains a partial API handoff, with unblocked work outstanding. Logs, oracle copies and source clones are retained under the assigned worktree's `.scratch/`. Owned targets were deleted after validation; the Python model server was not touched.


```text
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```
