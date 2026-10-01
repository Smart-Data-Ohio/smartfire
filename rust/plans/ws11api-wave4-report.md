# WS11-api wave 4 — PARTIAL continuation, 2026-10-01

Final code head: `6ecbf9fec9207757a9d6c0371395df2884e94b4f`, pushed on `rust/ws11api-rest-mcp`. A report-only commit follows. This report replaces the previous continuation report; its earlier evidence remains in branch history.

Seven coherent slices were committed and pushed. Selected success coverage is now **31/35 REST actions and 33/38 MCP tools**. The newly completed MCP success is `react`. The four REST and five MCP mutation successes still call the explicit WS12 seam; successful board/work writes were not implemented here. WS15g's installed private-PR repository-access resolver is unchanged.

**Not only owner-blocked items remain.** Exhaustive validation and the remaining bot/attachment/permission combinations still require API work. This is not a claim of full file-equivalent Rails test coverage.

The branch still includes main through `59ad94de` (#183); the start-of-turn fetch found no additional main commit to merge. No stash or rebase was used.

## Pushed slices

| Commit | Result |
| --- | --- |
| `92eae2f9` | MCP react and bot boost parity using shared Boost/icon primitives; grouped reaction replacement broadcasts; 50 initial raw HTTP/broadcast vectors |
| `43790a11` | 99 work-update/result/handoff validation and permission-denial vectors ahead of the flagged WS12 write seam |
| `7ffba163` | 12 REST/MCP polling comparisons; message-event field order and Drive attachment shape; cursor/cache headers and permission filtering |
| `c9b8a742` | REST multipart/signed attachment staging through main's posting service; initial 17 raw response/state vectors; queue/denial rollback checks |
| `bd00a293` | Expand attachment coverage to 38 vectors; correct root attachment/Drive validation order while retaining thread ordering |
| `8d7a2fec` | Seven successful-request-then-permission-change vectors; two compiled security mutations; all 15 oracle artifacts and 72 source pins included |
| `6ecbf9fe` | Strict clippy correction: legacy single-boost removal helper becomes test-only; its channel wire coverage stays |

## Changed files and design

Paths are relative to `rust/`.

- `crates/campfire/src/controllers/agents/reactions.rs`, `agents/pending.rs`, `agents.rs`: one MCP reaction adapter over main's Boost, icon resolution, membership and capability APIs. Replay returns the existing normalized boost; bot requests continue stacking rows. Array IDs retain Rails IN lookup semantics. The adapter emits the shared grouped reaction partial after the writer commits.
- `crates/campfire/src/channels/broadcasts.rs`: the legacy single-boost removal primitive is test-only after the real controllers switch to grouped replacement. No lint suppression was added.
- `controllers/messages/boosts.rs`, `boosts/by_bots.rs`, `crates/views/src/messages.rs`: render the existing reactions partial and replace the grouped reaction target for bot create and shared destroy. The human create/toggle implementation stays with the messaging owner. Recorded Rails broadcast strings are compared against messages received on actual authenticated Action Cable sockets.
- `controllers/agents/work_validation.rs`, `agents/pending.rs`: before the WS12 write seam, match update status/note/run-url/tag validation order, nested REST argument precedence, result length limits, and receiver existence/activity/membership/post/read/manage grants. Field errors call main's `ChannelThread::validate` and tag normalization. No board/work/result/handoff mutation or callback implementation is duplicated. Replace this denial adapter with the complete owner service when it lands.
- `crates/db/src/models/agent_event_polling.rs`: preserve Rails message-event JSON insertion order (`hop` before `room`/`actor`). The underlying filtering and private-PR resolver are unchanged. The polling fixtures use real persisted mention events and main's message presenter; they do not replace WS14g's producer/delivery integration tests.
- `controllers/agents/conversations.rs`, `controllers/messages.rs`, `crates/db/src/models/agent_posting.rs`: the app stages attachments and calls the same posting service as MCP. A small additive preparation hook inserts the storage rows inside the message savepoint; validation/queue failure rolls back rows, callbacks and staged files. Existing callers use a no-op preparation hook. Replay/budget and current membership/grant checks happen before staging and again in the writer. Root posts resolve attachments before Drive validation, while thread posts validate Drive first, as observed in Rails. This storage integration hook is a cross-workstream touch to WS11's service; it does not add another posting implementation.
- `controllers/{agent_reactions_tests,agent_polling_tests,agent_work_validation_tests,agent_attachments_tests}.rs`, `agent_reads_tests.rs`, `controllers.rs`: 206 new cases, bringing the committed artifacts to **1,045 cases**, plus seven explicitly asserted warm response bodies. Assertions compare raw JSON/body strings, status, selected header presence/absence (including Retry-After), persisted attachment metadata and actual reaction frames. Seven transitions change policy after a successful request: grant revocation, membership removal, credential revocation/expiry, agent suspension, owner deactivation and owner banning. The last two call main's real lifecycle APIs. Queue tests use main's `TestApp::without_job_runner()`; no sleeps, concurrency reductions or timing threshold changes were introduced.
- `reference-tools/agents/{reactions,bot_reactions,polling,work_validation,attachments}_http_contract.rb`, recorder/verifier/source checker, `check-reaction-attachment-mutations.py`, and five vector files: capture production Rails requests from private pinned seeds, compare artifacts byte for byte, and compile/reject broken security guards. The upload cases commit normally so attachment presentation and metadata reflect the actual save/analysis flow.

## Precisely remaining

- **WS12 owner-blocked successes:** REST board-post create; work update, result and handoff. MCP create_board_post, update_board_post, update_work, set_result, handoff_work. The API continues to call `agent_api_pending::execute`; preflight permissions and selected validations are implemented, successful writes/callbacks are not. Board creation's remaining field/owner validation and handoff context-package validation must be wired to the complete owner APIs.
- **API validation still owed:** full field/ID/coercion/length boundary and callback precedence matrices across older endpoint/tool families, broader poll date/expiry grammar, and remaining list/filter/cursor permutations. The 99 new work denial cases are selected evidence, not a declaration of exhaustive coverage.
- **API attachment/bot cases still owed:** existing direct-upload blob success, image/video representation and analysis-error combinations; root/thread reply and replay/analysis/fanout interactions beyond the 38 captured cases; complete bot paging, viewer/cache and callback parity. The new queue tests cover row/file rollback and silent reactions; they do not prove every remote delivery path.
- **API deferred permissions still owed:** the remaining race/transition combinations for read/viewer redaction, owned work and PR snapshots, reply-token scope and finalization/delivery failures. The seven new sequential transitions and older security matrices pass, but the complete Rails permission files have not been ported one for one.
- **Integration owners:** live private-PR access and network execution remain WS15g; full external delivery/network safety stays with the relevant domain/integration owners. No live GitHub access was implemented here.

WS14g can use the pushed `GET /agents/events?envelope=1` shape: Drive attachment objects, `next_since`, `X-Smartfire-Next-Since`, and no-store headers are covered by raw REST/MCP comparisons. The polling fix was pushed early at `7ffba163`.

## Failing-first evidence

This turn observed the missing reaction service and work validation fail before implementation; bot reactions initially appended a single boost instead of replacing the group. The first REST upload case exposed the discarded multipart file. Expanding to 38 attachment cases exposed wrong signed-blob/Drive error precedence (422 instead of Rails 500). Polling failed on JSON insertion order before its correction. All of those mismatches were corrected.

The following security mutation command was run against the current tests. It restores each file in `finally`, rejects compile-only failures, and requires an assertion failure at runtime:

```bash
python3 rust/reference-tools/agents/check-reaction-attachment-mutations.py
```

```text
WS11-api reaction_grant mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 1.29s
WS11-api attachment_file_rollback mutation: test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1864 filtered out; finished in 2.25s
WS11-api reaction/attachment mutations: 2 broken guards rejected; sources restored
```

Positive checks after restoring both guards:

```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/continue7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire agent_attachment -- --nocapture
CI=1 TMPDIR="$PWD/.scratch/continue7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 rust/target/debug/deps/campfire-55d4acca837cdb4b agent_reaction_permission_transitions --nocapture
```

```text
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 1861 filtered out; finished in 35.28s
```
```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1865 filtered out; finished in 10.17s
```

## Fresh-clone verification

Commands below were run from the assigned root. The clone is inside its `.scratch/`. It was fast-forwarded from `c9b8a742` to the pushed code. Seeds were rebuilt in the clone from the pinned image, not copied from the original worktree. The existing compiler target was moved into the clone as a cache after all owned processes exited; there was only one target. The configured rustc throttle stayed in effect, with two cargo jobs and default test concurrency.

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/fresh7
PARITY_NAMESPACE=ws11api-fresh7 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh7/rust/parity/bin/seed build default first_run
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh7/rust/Cargo.toml --format-version 1
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh7/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture
python3 .scratch/fresh7/rust/reference-tools/agents/summarize-http-tests.py .scratch/continue7/fresh-workspace.log
```

The complete native and pinned suites ran at `8d7a2fec`. The subsequent `6ecbf9fe` change only adds `#[cfg(test)]` to an unused production helper; its test body remains present and unchanged. Final boost tests and strict clippy run at that final head. The source trees have no tracked changes.

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```
```text
WS11-api cargo totals: 3706 passed; 2 failed; 12 ignored; 58 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```
All 58 native target summaries (exit 101):

```text
test result: FAILED. 1862 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 419.29s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.51s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 123.37s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.55s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.06s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.93s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.43s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.10s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.38s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
```

The two native failures are the account-logo PNG byte comparison and storage's pinned-media gate. Native libvips is 8.18.6 and ffmpeg is 9.0.2; the pin has 8.16.1 and 7.1.5. Both exact fresh-clone executables pass unchanged in the pinned runtime below. No golden, assertion, ignore, concurrency or timing threshold changed. The previous delivered report baseline-confirmed these same two failures; this report does not present that earlier baseline run as new evidence.

```bash
vips --version
ffmpeg -version
docker run --rm --name ws11api-fresh7-vips-version --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"'
docker run --rm --name ws11api-fresh7-media-versions --entrypoint sh ws11api-reference:d7c7de92 -c 'vips --version; ffmpeg -version'
```

The image has no `vips` CLI; the Ruby binding reports its installed library version. Raw version lines:

```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

### Pinned runtime, exact executables

The app clone is writable because some tests produce their own ignored scratch/target artifacts. No Rust rebuild or rustc process runs in these Docker commands; no test concurrency override is passed.

```bash
docker run --rm --name ws11api-fresh7-pinned-app --network none --user "$(id -u):$(id -g)" -e CI=1 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/fresh7-tmp" -v "$PWD/.scratch/fresh7:$PWD/.scratch/fresh7" -v "$PWD/.scratch/fresh7-tmp:$PWD/.scratch/fresh7-tmp" --entrypoint "$PWD/.scratch/fresh7/rust/target/debug/deps/campfire-abe1b035fe2b45bc" ws11api-reference:d7c7de92 --nocapture
docker run --rm --name ws11api-fresh7-pinned-storage --network none --user "$(id -u):$(id -g)" -e CI=1 -e TMPDIR="$PWD/.scratch/fresh7-tmp" -v "$PWD/.scratch/fresh7:$PWD/.scratch/fresh7:ro" -v "$PWD/.scratch/fresh7-tmp:$PWD/.scratch/fresh7-tmp" --entrypoint "$PWD/.scratch/fresh7/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture
```

```text
test result: ok. 1863 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 446.20s
```
```text
byte-identical: ["moon.jpg", "moon-thumb", "earth.png", "earth-thumb", "black_hole.jpg", "black_hole-thumb", "alpha-centuri.mov", "alpha-centuri.mov preview_image", "alpha-centuri-preview-webp", "alpha-centuri-poster", "alpha-centuri-poster-from-key", "pixel.bmp", "earth.png", "earth-avatar", "moon.jpg", "moon-avatar", "black_hole.jpg", "black_hole-logo-large", "black_hole-logo-small"]
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.96s
```

Both complete pinned targets exited 0. The remaining native targets passed; the native workspace was not green.

### Final-head boost checks and strict clippy

```bash
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh7/rust/Cargo.toml -p campfire boost -- --nocapture
CI=1 CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch/fresh7-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh7/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1860 filtered out; finished in 14.44s
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.62s
```

The first strict clippy run failed with `error: method boost_remove is never used`; `6ecbf9fe` corrects it with test-only compilation, preserving its only caller in the channel tests. Final strict clippy exits 0.

## Fresh Rails oracle and verifier

```bash
PARITY_NAMESPACE=ws11api-fresh7 python3 .scratch/fresh7/rust/reference-tools/agents/record-http-vectors.py .scratch/fresh7-oracles
python3 .scratch/fresh7/rust/reference-tools/agents/verify-http-vectors.py .scratch/fresh7-oracles
python3 .scratch/fresh7/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/fresh7/rust/reference-tools/agents/test-http-vector-verifier.py
```

Every artifact was freshly captured; no output mask or tolerance was added. Raw summaries:

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 269 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 47 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh readers oracle: 60 request/response pairs; byte-identical committed vectors
WS11-api fresh pins oracle: 18 request/response pairs; byte-identical committed vectors
WS11-api fresh polls oracle: 76 request/response pairs; byte-identical committed vectors
WS11-api fresh polling oracle: 12 request/response pairs; byte-identical committed vectors
WS11-api fresh reactions oracle: 44 request/response pairs; byte-identical committed vectors
WS11-api fresh bot reactions oracle: 13 request/response pairs; byte-identical committed vectors
WS11-api fresh work validation oracle: 99 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 38 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```
```text
WS11-api reference sources: 72 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```
```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.004s

OK
```

## Rails files and remaining file-equivalent ports

These pinned source files were rerun with the existing runner, grouped by file, with its normal behavior. Reference passes establish the source behavior; they do not mean every source test has a Rust counterpart.

```bash
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agents/messages_controller_test.rb test/controllers/agents/events_controller_test.rb test/controllers/agents/mcp_controller_test.rb test/controllers/agents/work_controller_test.rb test/controllers/agents/posts_controller_test.rb test/controllers/agents/work_handoff_test.rb test/controllers/agents/mcp_handoff_test.rb test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb test/controllers/messages/by_bots_controller_test.rb test/controllers/messages/boosts/by_bots_controller_test.rb
```

12 files, 294 runs and 1769 assertions; raw file summaries:

```text
test/controllers/agents/messages_controller_test.rb: 15 runs, 54 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/events_controller_test.rb: 43 runs, 206 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_controller_test.rb: 71 runs, 660 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_handoff_test.rb: 13 runs, 106 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_handoff_test.rb: 8 runs, 95 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

| Rails files | Selected Rust parity; explicitly deferred |
| --- | --- |
| agents/messages_controller | Root/thread posts, existing conversation vectors, 28 REST attachment cases, error precedence, atomic rows/files; broader attachment/media/fanout and input boundaries remain API/owners |
| agents/events_controller | Existing poll envelopes plus 12 new REST/MCP mention/Drive/header/permission pairs; full producer/delivery and PR/work snapshot cases remain WS11/WS14g/WS15g, with remaining wire matrices API |
| agents/mcp_controller | Existing 84 base vectors, protocol/auth/rate matrix and newly successful react with raw frames; five write successes require WS12; remaining exhaustive per-tool validations stay API |
| agents/work_controller; agents/posts_controller; agents/work_handoff; agents/mcp_handoff | Existing reads and 99 new update/result/receiver/permission errors; successful writes/callbacks remain WS12; board creation and context-package validations need owner APIs; broader request coercions stay API |
| agent_capability; agent_revocation_endpoints; agent_owner_deactivation | Existing boundaries/domain suites plus seven successful-then-denied HTTP transitions and a compiled missing-grant mutation; remaining races and cross-capability/viewer paths stay API/domain owners |
| messages/by_bots; messages/boosts/by_bots | Existing 47 wire cases, 13 grouped reaction cases, 10 multipart attachment cases, bot queue rollback; complete paging/viewer/cache, reply/thread and callback combinations remain API/owners |
| Other controller/MCP and domain files | Existing committed selected vectors and merged owner suites run in the fresh workspace; their earlier Rails runner counts are not re-claimed here. Exhaustive source-file ports remain as listed above |

No approval or clarification is pending. This remains a partial API handoff, with unblocked API work still outstanding. Logs, oracle copies and source clones are retained under the assigned worktree's `.scratch/`.

```text
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```
