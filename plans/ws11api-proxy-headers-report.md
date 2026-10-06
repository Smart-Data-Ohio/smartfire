# WS11-api: proxy-header parity and remaining-scope audit

Branch: rust/ws11api-proxy-headers, created from current main
`4fd0a74ccb0d596a0687557f3cc1f52a5a58ad08` after #192 merged. Tested production
source: `52989204f05fd13ff3b5f5037772191de2b759d6`. Commit `569f6080a` adds only the
scope inventory; the final report commit also changes documentation only. The explicit
checkpoint git merge --no-ff origin/main was already up to date. Locked metadata passes.
#202's rust/ws11api-work-writes remains at `05bdf0f7ce31af9d989ec99e9b31e37a6545f243`;
it was neither edited nor pushed. No stash or rebase was used.

## Completed fix

The pinned ActiveStorage::Streaming#send_blob_stream uses send_stream, which omits
Content-Transfer-Encoding. Rust used kit send_file/send_data, which adds binary.
Both HTTP 200 and handled empty HTTP 404 representation proxies therefore drifted.
The shared streaming helper now removes that one header from both response branches.
Streaming blob proxies share the correction. Generic kit data/file responses, disk
downloads and byte-range send_data responses retain their existing headers.

The oracle now compares nine headers: Content-Type, Cache-Control, Content-Disposition,
Content-Length, Location, Last-Modified, ETag, Accept-Ranges and Content-Transfer-Encoding.
No comparison was removed or masked. Three real signed JPEG/video scenarios retain 21
exact bodies/header sets, including healthy controls, empty HTTP 404, missing-preview HTTP 200,
redirect/disk follow-ups and repeats. Existing full domain/media/job and file snapshots
still prove no regeneration or persisted-state change. Media sizes/checksums remain exact.

## Changes by file

- crates/campfire/src/active_storage.rs: omit transfer encoding only from send_blob_stream.
- controllers/agent_review_r5_tests.rs: require the new oracle field and compare this
  independent header before native media-derived lengths/bodies; retain all other assertions.
- reference-tools/agents/review192r5_missing_representations.rb and
  vectors/agent_review192r5_representations.json: recapture the ninth header from pinned Rails.
- reference-tools/agents/test-http-vector-verifier.py: reject binary header insertion and
  oracle-field removal on healthy and post-removal proxy responses in every scenario.
- reference-tools/agents/proxy-headers-failing-first.txt: committed six-case red receipt.
- plans/ws11api-remaining-scope.md: every remaining API path, reason, owner and all 81
  outstanding domain case names, plus the complete REST/MCP/token/webhook source audit.

No agent model/domain/queue or reviewed #202 code was changed. WS12's work_threads.rs
and board presenter are untouched. No parity allowlist was added or widened.

## Failing-first proof

Production active_storage.rs was unchanged from merged main while only the recorder,
golden and test assertions were introduced. All six tests failed on binary versus absent
Content-Transfer-Encoding, including HTTP 200 and empty HTTP 404 proxies. Three native video
checks in an earlier diagnostic run hit Content-Length first; prioritizing the new header
made the final red run independent of media-version drift. No media check was disabled.
The committed receipt preserves the actual command and assertions:

```text
Proxy-header failing-first receipt
Production baseline: origin/main 4fd0a74cc (PR192 merged; active_storage.rs unchanged).
Only the oracle and regression assertions changed before this run.
All six regressions failed on Content-Transfer-Encoding, before media-derived fields.
HTTP success proxies and handled empty 404 proxies both returned binary; pinned Rails omits it.
No body, header, size or checksum is masked.

Command (expected exit101):
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/proxy-headers-target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/proxy-headers/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire pr192_r5_ -- --nocapture --test-threads=8

Raw summary and assertions:
    Finished `test` profile [unoptimized] target(s) in 50.11s
thread 'controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_proxy' (3452546) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: jpeg_variant proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect' (3452549) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: positive proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
test controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_proxy ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect ... FAILED
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect' (3452551) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: positive proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy' (3452548) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: video_preview proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
thread 'controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_redirect' (3452547) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: positive proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_redirect ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect ... FAILED
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy' (3452550) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:65:5:
assertion `left == right` failed: video_variant proxy: Content-Transfer-Encoding
  left: Some("binary")
 right: None
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy ... FAILED
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 2534 filtered out; finished in 1.30s
```

## Remaining scope

The full list is plans/ws11api-remaining-scope.md. Main has four REST and five MCP
valid work-write paths still calling a generic REST 500/-32603 seam: REST board-post create,
work update, result write/clear, handoff; MCP create_board_post, update_work,
update_board_post, set_result, handoff_work. They are already implemented and verified
on #202 and are held for its review/merge by the user's instruction. WS12 supplies the
needed services; no new service gap exists. Their denial paths are already on main.

After the header fix, no additional unported WS11-API implementation was found in the
route/service/token/webhook audit. The 35 non-MCP JSON actions, six by-bot actions and
three MCP transport routes have real bindings; all 38 tools have dispatch. Only the
five work-write operations reach the generic pending executor. Real webhook job handlers,
retry/recovery, signing, encrypted secrets, guarded transport, sync replies and token scrub
are installed; token/auth/grant/rate boundaries have the existing Rails vectors.

The broader domain ledger remains explicitly partial: 81 individually named comparisons
in eight files are still unmapped (WS11 domain, WS11-ui, WS8 and WS12/14/15 peer ownership).
Each exact name and its file-level reason is retained in the scope list. This is evidence
debt, not a claim that 81 production behaviors are missing. The tracked domain mapping
is unchanged, and API vectors are not falsely credited as closing those named tests.
The old absent-credential API-boundary deferral is stale: the installed Bearer parser and
REST/MCP credential vectors already exercise that boundary.

Only the #202 review/merge hold remains for known API implementation gaps. Broader named
domain evidence is still partial with explicit owners. WS12's /work.json growth fix in
#201 is peer-owned and was not touched. Existing approved media crash/retention differences
remain recorded separately; they are not hidden pending API paths.

## Fresh-clone verification

The clone is at the committed production fix. All 13 local workspace packages were cleaned
before rebuilding, so no earlier worktree application artifact was counted as a fresh test.
Default, first_run and agents_ui seeds were rebuilt inside the clone. All commands below
were run for this checkpoint. Cargo used two jobs with the configured rustc throttle,
one test runner at a time with eight threads, and only assigned 52900–52949 ports.
No timing threshold or concurrency setting changed.

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-proxy-headers . .scratch/proxy-headers-fresh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/proxy-headers-fresh/rust/Cargo.toml --format-version 1 > .scratch/proxy-headers/fresh-metadata.json
env PARITY_NAMESPACE=ws11api-proxy-headers-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/proxy-headers-fresh/rust/parity/bin/seed build default first_run agents_ui > .scratch/proxy-headers/logs/fresh-seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/proxy-headers-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/proxy-headers/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/proxy-headers-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/proxy-headers/logs/fresh-workspace.log 2>&1
python3 .scratch/proxy-headers-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/proxy-headers/logs/fresh-workspace.log > .scratch/proxy-headers/logs/fresh-workspace-summary.log 2>&1
```

Full native workspace exit 101: the seven media and version comparisons also identified
on reviewed 928531fa in Astra's read-only review-192-r6 evidence. They remain counted
as failures. The unchanged freshly built binaries pass those comparisons in the pinned
runtime below; there are no response/header/size/checksum masks. Every raw workspace summary:

```text
    Finished `test` profile [unoptimized] target(s) in 9m 45s
test result: FAILED. 2527 passed; 6 failed; 7 ignored; 0 measured; 0 filtered out; finished in 602.35s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.64s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.95s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1282 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 129.35s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.60s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.32s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.72s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.61s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
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
WS11-api cargo totals: 4536 passed; 7 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

Native failing tests:

- controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers
- controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect
- pipeline_matches_the_reference

Native libvips 8.18.6/ffmpeg 9.0.2 differs from pinned libvips 8.16.1/ffmpeg 7.1.5.
The strengthened video regressions now expose native Content-Length drift before body bytes;
the independent transfer-encoding assertion passes. Both JPEG regressions pass natively.
The storage guard is the seventh native failure. No actual seeded test silently skips;
the intentional missing-seed unit diagnostic is reported separately. Existing ignored tests
remain counted, and the two polling comparisons also run explicitly below.

## Strict clippy and release inputs

Both exit 0 from the fresh clone.

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/proxy-headers-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/proxy-headers/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/proxy-headers-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/proxy-headers/logs/fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 42s
```

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/proxy-headers-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/proxy-headers/tmp" mise exec rust@1.98.1 -- bash .scratch/proxy-headers-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/proxy-headers/logs/fresh-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 54s
```

## Pinned-runtime and explicit polling tests

The unchanged fresh-clone binaries run with read-only source/test inputs and only the
owned TMPDIR writable. Runners are sequential; all exit 0. The31-test PR192 batch includes
all six new header regressions and their exact media/state comparisons.

```bash
docker run --rm --name ws11api-proxy-headers-pinned-pr192 --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/proxy-headers/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/proxy-headers-fresh:$PWD/.scratch/proxy-headers-fresh:ro" -v "$PWD/.scratch/proxy-headers/tmp:$PWD/.scratch/proxy-headers/tmp" --entrypoint "$PWD/.scratch/proxy-headers-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8 > .scratch/proxy-headers/logs/pinned-pr192.log 2>&1
```

```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 2509 filtered out; finished in 24.79s
```

```bash
docker run --rm --name ws11api-proxy-headers-pinned-logo --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/proxy-headers/tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/proxy-headers-fresh:$PWD/.scratch/proxy-headers-fresh:ro" -v "$PWD/.scratch/proxy-headers/tmp:$PWD/.scratch/proxy-headers/tmp" --entrypoint "$PWD/.scratch/proxy-headers-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/proxy-headers/logs/pinned-logo.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2539 filtered out; finished in 0.98s
```

```bash
docker run --rm --name ws11api-proxy-headers-pinned-storage --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/proxy-headers/tmp" -v "$PWD/.scratch/proxy-headers-fresh:$PWD/.scratch/proxy-headers-fresh:ro" -v "$PWD/.scratch/proxy-headers/tmp:$PWD/.scratch/proxy-headers/tmp" --entrypoint "$PWD/.scratch/proxy-headers-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/proxy-headers/logs/pinned-storage.log 2>&1
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.58s
```

```text
PR192_R5_REPRESENTATION jpeg_variant redirect status=302 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION jpeg_variant proxy status=404 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_preview redirect status=302 disk_status=200 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_preview proxy status=200 disk_status=200 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_variant proxy status=404 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_variant redirect status=302 disk_status=404 rows/files/jobs=unchanged regenerated=false
```

```bash
env CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/proxy-headers/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/proxy-headers-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/proxy-headers/logs/polling-explicit.log 2>&1
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2538 filtered out; finished in 0.89s
```

## Rails vector and named-case evidence

All 21 HTTP oracle artifacts were recaptured from private pinned seeds. Only the
representation oracle changed, to add its ninth header. Its existing bodies/statuses,
other eight headers, state and jobs retain their values. Two approved media differences
remain explicit, not relabeled as identical Rails/Rust statuses. Source/verifier scripts
also ran from the fresh clone; fourteen planted-mutation verifier checks pass.

```bash
env PARITY_NAMESPACE=ws11api-proxy-headers-all python3 rust/reference-tools/agents/record-http-vectors.py .scratch/proxy-headers/fresh-oracles > .scratch/proxy-headers/logs/fresh-oracles.log 2>&1
python3 .scratch/proxy-headers-fresh/rust/reference-tools/agents/check-http-reference.py > .scratch/proxy-headers/logs/fresh-source-check.log 2>&1
python3 .scratch/proxy-headers-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/proxy-headers/fresh-oracles > .scratch/proxy-headers/logs/fresh-vector-verification.log 2>&1
python3 .scratch/proxy-headers-fresh/rust/reference-tools/agents/test-http-vector-verifier.py > .scratch/proxy-headers/logs/fresh-verifier-injections.log 2>&1
```

```text
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```

```text
WS11-api fresh HTTP oracle: 33 request/response pairs; byte-identical committed vectors
WS11-api fresh MCP oracle: 84 request/response pairs; byte-identical committed vectors
WS11-api fresh surface oracle: 269 request/response pairs; byte-identical committed vectors
WS11-api fresh bot oracle: 71 request/response pairs; byte-identical committed vectors
WS11-api fresh legacy bot/fanout/replacement oracle: 50 request/response pairs; byte-identical committed vectors
WS11-api fresh conversation oracle: 66 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy reads oracle: 54 request/response pairs; byte-identical committed vectors
WS11-api fresh Fizzy approvals oracle: 132 request/response pairs; byte-identical committed vectors
WS11-api fresh readers oracle: 192 request/response pairs; byte-identical committed vectors
WS11-api fresh pins oracle: 18 request/response pairs; byte-identical committed vectors
WS11-api fresh polls oracle: 168 request/response pairs; byte-identical committed vectors
WS11-api fresh polling oracle: 12 request/response pairs; byte-identical committed vectors
WS11-api fresh reactions oracle: 44 request/response pairs; byte-identical committed vectors
WS11-api fresh bot reactions oracle: 13 request/response pairs; byte-identical committed vectors
WS11-api fresh work validation oracle: 99 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 zones and ID shapes oracle: 59 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved JPEG difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved video difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 handled missing representations oracle: 3 request/response pairs; byte-identical committed vectors
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```

```text
..............
----------------------------------------------------------------------
Ran 14 tests in 0.016s

OK
```

```bash
python3 .scratch/proxy-headers-fresh/rust/reference-tools/agents/named-case-pass-counts.py .scratch/proxy-headers/logs/fresh-workspace.log > .scratch/proxy-headers/logs/named-case-counts.log 2>&1
```

```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 27 passed; 0 failed; 2 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 0 passed; 0 failed; 29 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 19 passed; 0 failed; 4 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 9 passed; 0 failed; 6 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 9 passed; 0 failed; 2 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 9 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 7 passed; 0 failed; 1 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 297 passed; 0 failed; 81 deferred
```

## Cleanup and retained evidence

All owned build/test/container processes finished. The sole extra target directory was
deleted after verification; logs, regenerated vectors, seeds and fresh-clone source are
retained under .scratch/proxy-headers and .scratch/proxy-headers-fresh. The Python model
server was not touched. No external message or PR was created. Cleanup receipt:

```text
11G	/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws11api/.scratch/proxy-headers-fresh/rust/target
WS11-api scratch target cleanup: complete; logs, vectors and fresh-clone source retained
```
