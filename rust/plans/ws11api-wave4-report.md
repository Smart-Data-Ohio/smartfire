# WS11-api wave 4 — PARTIAL continuation, 2026-10-01

Verified code head: `10aa8f39a97867b18e32a6c4b68a09485cd69b9a`, pushed on `rust/ws11api-rest-mcp`. A report-only commit follows it.
Verification: the complete fresh-clone native workspace ran all targets (3694 passed, 2 media environment failures, 12 existing ignores). Both failures reproduce on fresh main. The unchanged full application and storage-vector targets pass in the pinned runtime: 1851 + 10 passed, zero failures. Strict workspace clippy passes. No actual seed-dependent test skipped.

This report replaces the previous continuation report; prior evidence remains in branch history.

Merged `origin/main` at `7442031d`, then `59ad94de` including #183, with merge commits. No stash or rebase. Main's real agent, callback, lifecycle, message-payload, Fizzy and GitHub implementations replace imported stand-ins. WS15g's installed repository-access resolver and its ownership boundary remain as main supplies them.

This continuation adds **8 REST successes and 8 MCP successes**: room/history/board/work reads, pin/unpin, poll create/show and four kinds of GitHub approval request. Selected success coverage is now **31/35 REST actions and 32/38 MCP tools**, excluding the separate bot action count. All **839 committed wire vectors** assert raw response bytes, status and selected header presence/absence. All 84 base MCP vectors are enabled. Coverage remains selected, not exhaustive or a file-equivalent port of every Rails test.

**Not only owner-blocked items remain.** The API worker still owes react, exhaustive validation and attachment/bot/deferred permission cases. Four REST mutation successes and five MCP mutation successes remain WS11/WS12 domain seams.

## Coherent pushed slices

| Commit | Change |
| --- | --- |
| `d2ec873d` | Merge main; use its services and payload presenter; remove duplicate GitHub API preflight and repository resolver; share GitHub throttle/no-store |
| `d9150ac7` | Five read operations, 60 new raw REST/MCP vectors; enable deferred base list_rooms/list_work pairs |
| `af0dac5d` | Merge #183 and preserve its exact `TestApp::without_job_runner()` helper |
| `bff2e2cb` | Pin/unpin and 18 vectors; upgrade older bodies/header absence checks; four GitHub approval success vectors; correct empty membership 404 content types |
| `8dee9c63` | Poll create/show and 76 vectors; shared Poll Ruby whitespace fix; oracle recorder |
| `b6941194` | Two compiled permission mutations; restore guards; compiler jobs limited to 2 |
| `f9bdb52f` | Fix brand avatar URL regression in main's shared JSON presenter |
| `10aa8f39` | Group injected boot transports, retain main's constructor entry point, fix strict clippy's eight-argument error |

## Files and design

Controller, app and integration paths below are relative to `rust/crates/campfire/src/`; other paths are relative to `rust/`.

- `controllers/agents/{reads,pins,polls,pending}.rs`: shared REST/MCP adapters over main's Room/ChannelThread/Message/MessagePin/Poll models, capability/budget readers, WorkPayload and repository-access resolver. Readers enforce room scope and cursor membership; work lists retain ownership/read gates. Pin replay precedes capacity checks through the existing atomic model. Poll rows and their message are created in one writer transaction; existing human-poll broadcast/webhook APIs handle delivery. Date parsing uses the existing shared helper; broader Rails Date._parse grammar remains unimplemented.
- `controllers/{agent_reads_tests,agent_pins_tests,agent_polls_tests}.rs`: 154 new wire vectors including scopes, revoked grants, membership removal, work ownership, cursors, coercions, pin replay/capacity, poll votes/anonymous/closed state and validation precedence. Fixtures use main's `without_job_runner()` without sleeps or timing adjustments.
- `controllers/{agent_http_tests,agent_mcp_tests,agent_surface_tests}.rs` and their oracle scripts: add raw body equality and absent-header assertions. Surface vectors add four successful GitHub write-request kinds through main's approval service. They request approval and do not prove remote execution.
- `controllers/github/agent_actions.rs`: keep main's transport/domain implementation; replace its duplicated throttle/no-store with the shared concern (including missing Pragma); match Rails' HTML content type for empty pre-render 404s.
- `controllers/presenters/agent_payload.rs`, `controllers/messages/payload.rs`, `controllers/messages/boosts/by_bots.rs`, `controllers/presenters.rs`: main's canonical message/user presentation replaces duplicate bot JSON. Resolve brand icons through the asset manifest in the shared user helper. The first full run caught the logical-path regression, and the final full run tests its fix.
- `crates/db/src/models/poll.rs`: one shared normalization correction: Ruby strip trims NUL and ASCII whitespace, preserves non-breaking spaces around nonblank labels, and rejects Unicode-blank labels. Both wire transports failed before this fix. No alternate Poll implementation was added.
- `app.rs`, `controllers/presenters/test_support.rs`, `integrations/fizzy/mod.rs`: retain the per-app injectable Fizzy transport on main's full service implementation; group boot transports without changing main's `boot_with_all_services` callers. Production uses the system transport. The helper merge preserves #183's implementation.
- `integrations/fizzy/agent_requests.rs`: retain previously proven AR external-ID replay coercions on the real owner service, including flattened array/NULL matching, boolean t/f and Hash failure. No copied model/service remains alongside main.
- Main owns the merged database lifecycle, callback, streaming/posting, user-removal, integration, job and scheduler implementations and their tests. The obsolete duplicated API scheduler oracle/check was removed; main's scheduler vectors/tests are authoritative. No scheduler intervals or test timing thresholds changed.
- `reference-tools/agents/{reads,pins,polls}_http_contract.rb`, `record-http-vectors.py`, `verify-http-vectors.py`, `check-http-reference.py`, `check-read-pin-mutations.py`; `vectors/agent_*`: pinned production Rails capture, source pin checks, exact artifact comparison and compiled fail-first proof.

## Precisely remaining

- **4 REST successes:** board posts create; work update, result and handoff. Their domain writes/callbacks are still explicit `agent_api_pending::execute` seams requiring WS11/WS12 APIs.
- **6 MCP successes:** react (available shared Boost primitives; API adapter still owed); create_board_post, update_board_post, update_work, set_result, handoff_work (domain seams).
- **Exhaustive validation:** remaining field shapes, ID/coercion boundaries, lengths, broad date/expiry grammar, pagination/filter/cursor combinations, and validation/callback precedence across older families and the remaining seams. The new poll cases cover selected boundaries, not all Ruby time parsing.
- **Attachments/bots:** agent REST multipart/signed-blob staging; root/thread bot attachment presentation, persisted rows, analysis/delivery and enqueue rollback; complete bot pagination, work-viewer/cache and callback behavior. Existing 47 bot wire vectors now use shared presentation and pass, but full file-equivalent attachment coverage is unfinished.
- **Deferred permission/security:** full transition/race matrix for membership/grant/credential/owner changes, reply-token restrictions, owned work/PR redaction, attachment and finalization callback errors. Earlier fail-first credential/protocol/rate proofs remain in history; this continuation adds two new compiled permission proofs and runs the positive security tests in the fresh suite. Live private-PR access and network/SSRF/DNS proof remain with WS15g and the respective integration owners.
- **Native media environment:** account-logo PNG bytes and storage's pinned-media requirement fail natively, identically on a fresh `origin/main` checkout at `59ad94de`. Native libvips 8.18.6 / ffmpeg 9.0.2 differ from the pin's 8.16.1 / 7.1.5. Both failed tests pass unchanged in the pinned reference image, confirming the native-runtime boundary. No golden, ignore or version gate was changed.

## Fail-first evidence

Four reader groups and both pin/poll groups failed against missing services before implementation. The expanded poll cases then failed on non-breaking-space label bytes before the shared normalization fix. Complete header assertions exposed the pre-render 404 Content-Type mismatches and missing GitHub Pragma. The initial fresh run found the bot brand-URL regression; strict clippy found the eight-argument boot function. All API issues were corrected; the two native media failures are baseline-confirmed and pass in the pinned runtime.

Logs under `.scratch/merge-main/` retain each failed run. Raw summary lines:

```text
# reads-before.log
test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 1843 filtered out; finished in 20.95s
# reads-after.log
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1845 filtered out; finished in 15.75s
# pins-before.log
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1850 filtered out; finished in 1.06s
# polls-before.log
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1852 filtered out; finished in 1.17s
# polls-normalization-before.log
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1852 filtered out; finished in 15.83s
# polls-final-after.log
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1852 filtered out; finished in 25.22s
# raw-and-pins-after.log
test result: FAILED. 44 passed; 2 failed; 0 ignored; 0 measured; 1806 filtered out; finished in 199.47s
# raw-headers-fixed.log
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 1842 filtered out; finished in 192.77s
# gh-success-after.log
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1851 filtered out; finished in 71.33s
# read-pin-mutations.log
WS11-api room_visibility mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1853 filtered out; finished in 2.27s
# read-pin-mutations.log
WS11-api pin_grant mutation: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1853 filtered out; finished in 5.01s
# read-pin-mutations.log
WS11-api read/pin mutations: 2 broken guards rejected; sources restored
# fresh-workspace.log
test result: FAILED. 1849 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 339.58s
```

The mutation command was run with the restored code after positive poll verification:

```bash
CARGO_BUILD_JOBS=2 python3 rust/reference-tools/agents/check-read-pin-mutations.py
```

Compilation errors do not count as mutation detection. Both altered guards produced assertion failures and the script restored source bytes in `finally`; the fresh checkout contains the restored guards.

## Commands and fresh-clone verification

All Rust compilation uses mise Rust 1.98.1, `CARGO_BUILD_JOBS=2` and `-j2`, with the configured machine rustc throttle. Default libtest concurrency and timing limits are unchanged. One owned target cache was moved into the fresh clone and reused for the baseline; no second scratch target was created. It was removed after verification; the cleanup found no remaining owned test/build processes. Neither the Python model server nor another worktree was touched.

From the assigned root:

```bash
git clone --no-hardlinks --single-branch --branch rust/ws11api-rest-mcp . .scratch/fresh6
mkdir -p .scratch/fresh6-tmp .scratch/fresh6-oracles
PARITY_NAMESPACE=ws11api-fresh6 PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/fresh6/rust/parity/bin/seed build default first_run
```

The fresh clone was fast-forwarded to the pushed code head after the permission-proof and verification-fix commits. Its source tree is clean. These are fresh seed builds, not copies from the original worktree:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

Rust environment for the fresh suite:

```text
CI=1
CARGO_BUILD_JOBS=2
TMPDIR=$PWD/.scratch/fresh6-tmp
CABLE_TEST_PORT_RANGE=52900-52919
INTEGRATION_TEST_PORT_RANGE=52920-52949
MAIL_TEST_PORT_RANGE=52920-52949
```

```bash
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/fresh6/rust/Cargo.toml --format-version 1
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/fresh6/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/fresh6/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
python3 .scratch/fresh6/rust/reference-tools/agents/summarize-http-tests.py .scratch/merge-main/fresh-workspace-final.log
```

Native full-workspace run: exit 101 because of the two media environment failures. Strict clippy: exit 0. All 58 raw test summaries:

```text
test result: FAILED. 1850 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 269.35s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.19s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1143 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 72.26s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.09s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.62s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.20s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.20s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.92s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
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
```

```text
WS11-api cargo totals: 3694 passed; 2 failed; 12 ignored; 58 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
cargo metadata --locked: exit 0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 50s
```

Twelve pre-existing ignores (including two doctests) remain; none was added. The CI pinned-media assertion failed rather than silently skipping media bytes. There were no actual missing-seed skips.

### Fresh main baseline

```bash
git clone --no-hardlinks --single-branch --branch rust/ws11api-rest-mcp . .scratch/main-baseline
git -C .scratch/main-baseline checkout --detach 59ad94de
PARITY_NAMESPACE=ws11api-main-baseline PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/main-baseline/rust/parity/bin/seed build default first_run
```

With the same CI/ports/TMPDIR environment and `CARGO_TARGET_DIR=$PWD/.scratch/fresh6/rust/target`:

```bash
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/main-baseline/rust/Cargo.toml -p campfire --bin campfire stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers -- --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/main-baseline/rust/Cargo.toml -p campfire_storage --test vectors pipeline_matches_the_reference -- --nocapture
```

Both reproduce the native failure on main:

```text
assertion `left == right` failed: Some("moon.jpg") null complete Rails PNG bytes
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1794 filtered out; finished in 1.39s
pipeline_matches_the_reference: pinned media required: expected libvips "8.16.1" / ffmpeg "ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers", found libvips 8.18.6 / ffmpeg ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.05s
```

### Pinned media execution

The exact fresh-clone native test executables run in `ws11api-reference:d7c7de92`, with no rebuild, changed assertion, concurrency override or timing change. No rustc is launched by these Docker commands. The two formerly failing tests each passed there. The complete storage vector target also passed, including byte-identical processed media. The first whole-app Docker attempt mounted the clone read-only, which prevented tests from writing their own ignored scratch/target output; it was stopped and relaunched with the writable private clone under a new container name after an asynchronous Docker cleanup/name conflict. Its failed setup is retained in `pinned-app-full.log` and is not represented as application parity evidence.

```bash
docker run --rm --name ws11api-pinned-app-full2 --network none --user "$(id -u):$(id -g)" -e CI=1 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/fresh6-tmp" -v "$PWD/.scratch/fresh6:$PWD/.scratch/fresh6" -v "$PWD/.scratch/fresh6-tmp:$PWD/.scratch/fresh6-tmp" --entrypoint "$PWD/.scratch/fresh6/rust/target/debug/deps/campfire-abe1b035fe2b45bc" ws11api-reference:d7c7de92 --nocapture
docker run --rm --name ws11api-pinned-storage-full --network none --user "$(id -u):$(id -g)" -e CI=1 -e TMPDIR="$PWD/.scratch/fresh6-tmp" -v "$PWD/.scratch/fresh6:$PWD/.scratch/fresh6:ro" -v "$PWD/.scratch/fresh6-tmp:$PWD/.scratch/fresh6-tmp" --entrypoint "$PWD/.scratch/fresh6/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture
```

```text
test result: ok. 1851 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 267.91s
byte-identical: ["moon.jpg", "moon-thumb", "earth.png", "earth-thumb", "black_hole.jpg", "black_hole-thumb", "alpha-centuri.mov", "alpha-centuri.mov preview_image", "alpha-centuri-preview-webp", "alpha-centuri-poster", "alpha-centuri-poster-from-key", "pixel.bmp", "earth.png", "earth-avatar", "moon.jpg", "moon-avatar", "black_hole.jpg", "black_hole-logo-large", "black_hole-logo-small"]
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.93s
```

Both complete pinned-runtime targets exited 0. The remaining native targets passed; this is not a claim that the native media run was green.

## Rails files, grouped with current reference pass counts

These reference files were rerun against `d7c7de92`, using the existing runner and its default per-file test behavior. Their success is reference evidence; it does not claim every source test has a Rust equivalent.

```bash
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agents/pins_controller_test.rb test/controllers/agents/polls_controller_test.rb test/controllers/agents/posts_controller_test.rb test/controllers/agents/work_controller_test.rb test/controllers/agents/github/pull_request_actions_controller_test.rb test/controllers/agent_capability_test.rb test/controllers/agent_revocation_endpoints_test.rb test/controllers/agent_owner_deactivation_test.rb test/controllers/messages/by_bots_controller_test.rb test/controllers/messages/boosts/by_bots_controller_test.rb
python3 rust/reference-tools/agents/run-controller-reference.py test/controllers/agents/mcp_slash_polls_test.rb test/controllers/agents/mcp_handoff_test.rb test/controllers/agents/mcp_streaming_test.rb test/controllers/agents/mcp_fizzy_test.rb
```

Raw file summaries (248 runs, 1354 assertions total):

```text
test/controllers/agents/polls_controller_test.rb: 12 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/pins_controller_test.rb: 10 runs, 29 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/posts_controller_test.rb: 24 runs, 169 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/work_controller_test.rb: 36 runs, 170 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/github/pull_request_actions_controller_test.rb: 21 runs, 82 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_capability_test.rb: 15 runs, 44 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_revocation_endpoints_test.rb: 4 runs, 36 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agent_owner_deactivation_test.rb: 7 runs, 20 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/by_bots_controller_test.rb: 40 runs, 144 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/messages/boosts/by_bots_controller_test.rb: 18 runs, 65 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_slash_polls_test.rb: 14 runs, 60 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_handoff_test.rb: 8 runs, 95 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_streaming_test.rb: 11 runs, 71 assertions, 0 failures, 0 errors, 0 skips; exit 0
test/controllers/agents/mcp_fizzy_test.rb: 28 runs, 333 assertions, 0 failures, 0 errors, 0 skips; exit 0
```

| Rails source file/group | Selected Rust coverage; precisely deferred |
| --- | --- |
| pins_controller | Shared pin/unpin successes, replay/capacity and authorization bytes; complete callback and race variants remain API/WS8 |
| polls_controller; mcp_slash_polls | 76 create/results vectors, ballots/anonymous/closed results, grant/budget/date/error precedence; broader dates and full malformed/length/boundary matrix remain API |
| posts_controller; work_controller; mcp_handoff | Read successes, ownership/read gates, room/board/filter denial bytes; post/work/result/handoff writes and callback integration remain WS11/WS12; exhaustive preflight errors remain API |
| github/pull_request_actions_controller | Main's real request service, all four request kinds, existing account/grant/replay/budget/rate wire cases; exhaustive field/relink/error variants remain API/WS15g; private-PR access and execution remain WS15g |
| agent_capability; agent_revocation_endpoints; agent_owner_deactivation | Main's domain suites plus positive HTTP boundary vectors and two new compiled permission mutations; full transition/race matrix remains API/WS11 |
| messages/by_bots; messages/boosts/by_bots | 47 raw wire/state vectors, reply-token scope and shared payload/icon fix; complete attachments/root/thread/bot callback, pagination and work-viewer variants remain API/owners |
| mcp_streaming | Main's streaming service plus existing conversation vectors; attachment, concurrent transition and full callback error combinations remain API/WS11 |
| mcp_fizzy | All nine tools, existing 186 Fizzy raw vectors and merged owner suites; exhaustive extra validation and remote execution/delivery proof remain API/WS15e |
| agents_controller; events; steps; slash_commands; approvals; contexts; messages; dms; streaming; api_throttle; budgets; concerns/agent_authentication; mcp_controller | Existing selected success/security vectors rerun in the fresh Rust suite and regenerated from Rails. Exhaustive file-equivalent validation/callback/security ports remain API/WS11; source file runner counts from older reports are not re-claimed here |
| approval/drive/github/slash/work/Fizzy delivery files; directory | Main's available domain/runtime tests rerun; complete delivery files stay with domain/integration owners; directory HTML stays WS11-ui |

## Fresh oracle and verifier evidence

From the assigned root, scripts are taken from the fresh clone:

```bash
PARITY_NAMESPACE=ws11api-fresh6 python3 .scratch/fresh6/rust/reference-tools/agents/record-http-vectors.py .scratch/fresh6-oracles
python3 .scratch/fresh6/rust/reference-tools/agents/verify-http-vectors.py .scratch/fresh6-oracles
python3 .scratch/fresh6/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/fresh6/rust/reference-tools/agents/test-http-vector-verifier.py
```

Every one of the ten artifacts was captured anew; no output mask or tolerance was added. Raw summaries:

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
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 69 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.004s

OK
```

No approval or clarification is pending. The outstanding API work above prevents an "only owner-blocked" handoff. Logs, oracle copies and fresh source checkouts remain under the assigned worktree's `.scratch/`; scratch targets were removed after all owned test processes exited.

```text
Fresh-clone test/build processes: []
WS11-api scratch targets: 0 remaining; owned test/build processes: 0
```
