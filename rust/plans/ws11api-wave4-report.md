# WS11-api: PR192 fifth-review fixes

Tested source: `e75bcd705af5afc912184921696b9397cf16cf97`. Main merge: `ac2a77d13a1650cf6cfe303c23a862137f8417f3`, including main `b98904b88032edfc8e06e2b765380a1d944e9afa` (#198 and #200, following #193). A final fetch confirms zero commits in HEAD..origin/main. Locked Cargo metadata succeeds. The final report commit only changes documentation.

The interrupted work-writes work was saved exactly as requested in local, unpushed `e068d0893521b789091311716d9b7de999c9b9b6` (`WIP: agent work writes (interrupted)`). That branch remains paused and unchanged. No stash was used.

## Fix and failing-first evidence

| Stored-file condition | Representation redirect | Signed disk follow-up | Representation proxy |
|---|---|---|---|
| Video intermediate JPEG missing, exact HTTP WebP still present | 302, exact signed Location and redirect body | 200, exact 3,326-byte WebP | 200, exact 3,326-byte WebP |
| Video WebP missing | 302, exact signed Location and redirect body | Empty 404 | Empty 404, exact image headers |
| JPEG variant missing (additional control) | 302, exact signed Location and redirect body | Empty 404 | Empty 404, exact image headers |

All three scenarios assert seven exact response bodies/header sets each: three positive controls, redirect, disk follow-up, proxy and repeated redirect. They use the real decoded signed HTTP transformation, including Rails' string-format digest. Fixed storage keys and mtimes are fixture inputs; no response, size, checksum or header is masked. Rails and Rust both leave rows/files/jobs unchanged and do not regenerate the removed file.

Before production changes, all six regressions failed against production `7f77fa5fa85ee46290f5cac44fcea7227c8cab56`, each with actual 500 rather than the expected handled response. The committed receipt is `reference-tools/agents/review-192-r5-failing-first.txt`. These failures were on status, before native media comparisons. Raw receipt:

```text
PR192 R5 failing-first receipt
Baseline production revision: 7f77fa5fa85ee46290f5cac44fcea7227c8cab56.
Only the new R5 regression module and its registration were added before this run; production active_storage.rs was unchanged. Six regressions fail on the handled response status, all with actual 500. No case failed on setup or native media bytes.

Command (expected exit101):
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/pr192-r5-target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r5-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire pr192_r5_ -- --nocapture --test-threads=8

Raw status assertions and summary lines:
    Finished `test` profile [unoptimized] target(s) in 2m 57s
thread 'controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_proxy' (2189665) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 404
test controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_proxy ... FAILED
thread 'controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_redirect' (2189666) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 302
test controllers::agent_review_r5_tests::pr192_r5_jpeg_missing_variant_redirect ... FAILED
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect' (2189668) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 302
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy' (2189669) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 404
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect' (2189670) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 302
thread 'controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy' (2189667) panicked at crates/campfire/src/controllers/agent_review_r5_tests.rs:26:5:
  left: 500
 right: 200
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy ... FAILED
test controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy ... FAILED
test result: FAILED. 0 passed; 6 failed; 0 ignored; 0 measured; 2521 filtered out; finished in 1.18s
```

## Changes by file and design

- `crates/campfire/src/active_storage.rs`: HTTP processing reuses recorded variant/preview metadata, as Rails' processed? does. A usable WebP needs no intermediate JPEG file. Missing final-file proxy responses retain the headers Rails sets before catching FileNotFoundError and return empty 404. The redirect continues to sign the existing record. File-aware attachment-processing helpers remain intact.
- `crates/campfire/src/controllers/agent_review_r5_tests.rs` and `controllers.rs`: six failing-first HTTP regressions, exact positive/repeat controls, and complete Rust domain/media/job snapshots plus file-list invariants.
- `crates/campfire/src/controllers/agent_review_r4_tests.rs`: replace the superseded missing-file 500 assumptions with Rails' handled 302/404 semantics; retain direct file-aware helper rejection, committed ownership, reuse, foreign-key/count checks and rollback injections.
- `reference-tools/agents/review192r5_missing_representations.rb`, `record-r5-representations.py`, `record-http-vectors.py` and `vectors/agent_review192r5_representations.json`: pinned Rails differential with three private fresh seeds and 21 exact response/header sets. Existing 20 artifacts retain their bytes.
- `reference-tools/agents/verify-http-vectors.py` and `test-http-vector-verifier.py`: compare the new artifact and reject planted status/body/header changes; 13 verifier tests pass.
- `plans/ws11api-approved-differences.md`: explicitly record the committed-file retention state difference and distinguish the ordinary handled missing-file responses.
- `crates/campfire/src/controllers/presenters/test_support.rs`: reconcile main's Fizzy helper extraction with the branch's injected GitHub read client. The first fresh-clone compile caught the client out of scope. Both client inputs now pass through the helper; no integration functionality was dropped. That failed compile is archived separately and is not counted as final verification.

## Deliberate media state difference

When an earlier after-commit callback raises, Rails can commit image/blob/attachment/variant records without the image file: its deferred upload callback never runs. Rust deliberately retains staged files whenever COMMIT succeeds, independently of ordinary callback failures. Both return 500 for the exception and skip later ordinary callbacks. This difference is now explicit alongside the previously approved JPEG/video crash differences in `plans/ws11api-approved-differences.md`.

The original fresh-JPEG missing-variant exception remains narrow. The genuine fresh-video pre-commit Rails defect remains separately documented. The R4 after-commit regression still asserts the committed message, reopened thread, exact media relations, two queued jobs, retained preview/variant files and skipped later model callback. Pre-commit faults still roll back rows/files/jobs. Missing-file serving is ordinary Rails parity and does not authorize regeneration or persisted-state changes.

## Fresh-clone verification

The fresh clone was created from the merge and fast-forwarded to the committed helper fix. All 13 local workspace packages were cleaned before its first build so no worktree-source artifact was reused. Default, first_run and agents_ui seeds were rebuilt inside the clone. Final tests, strict clippy and release-input build ran from that clone. Cargo used two jobs with the configured rustc throttle; tests used eight threads and the assigned 52900–52949 ranges. No thresholds or concurrency limits changed. The Python model server was not touched.

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/pr192-r5-final-fresh
git -C .scratch/pr192-r5-final-fresh pull --ff-only
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/pr192-r5-final-fresh/rust/Cargo.toml --format-version 1 > .scratch/pr192-r5/fresh-metadata.json
PARITY_NAMESPACE=ws11api-r5-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-r5-final-fresh/rust/parity/bin/seed build default first_run agents_ui
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
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/pr192-r5-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r5-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r5-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/pr192-r5/logs/fresh-workspace.log 2>&1
python3 .scratch/pr192-r5-final-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/pr192-r5/logs/fresh-workspace.log
```

Exit101: all seven failures are native media/version comparisons, listed below. They are counted as failures, not skipped or masked. All seven pass using the same freshly built binaries in the pinned runtime. Every raw workspace summary:

```text
    Finished `test` profile [unoptimized] target(s) in 3m 26s
test result: FAILED. 2527 passed; 6 failed; 7 ignored; 0 measured; 0 filtered out; finished in 450.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.55s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1282 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 109.09s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.43s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.77s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
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
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
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
WS11-api cargo totals: 4536 passed; 7 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

Native failing tests:

```text
controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers
controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files
controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy
controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect
controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy
controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect
pipeline_matches_the_reference
```

Native libvips8.18.6/ffmpeg9.0.2 differs from pinned libvips8.16.1/ffmpeg7.1.5. Failures are the PNG logo bytes, the approved-video media state size/checksum fields, four new video response-byte controls, and the storage pipeline version guard. Statuses, metadata and exact response bytes/headers are verified without masks by the pinned reruns below.

## Strict clippy and release inputs (both exit0)

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/pr192-r5-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r5-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/pr192-r5-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/pr192-r5/logs/fresh-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 18s```

```bash
env CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/pr192-r5-final-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r5-tmp" mise exec rust@1.98.1 -- bash .scratch/pr192-r5-final-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/pr192-r5/logs/fresh-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 30s```

## Pinned-runtime reruns (all exit0)

The unchanged fresh-clone test binaries run in ws11api-reference:d7c7de92; test inputs are read-only and TMPDIR is the only writable bind mount.

```bash
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/pr192-r5-tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/pr192-r5-final-fresh:$PWD/.scratch/pr192-r5-final-fresh:ro" -v "$PWD/.scratch/pr192-r5-tmp:$PWD/.scratch/pr192-r5-tmp" --name ws11api-r5-pinned-final --entrypoint "$PWD/.scratch/pr192-r5-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8 > .scratch/pr192-r5/logs/pinned-final-pr192.log 2>&1
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/pr192-r5-tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/pr192-r5-final-fresh:$PWD/.scratch/pr192-r5-final-fresh:ro" -v "$PWD/.scratch/pr192-r5-tmp:$PWD/.scratch/pr192-r5-tmp" --name ws11api-r5-pinned-logo --entrypoint "$PWD/.scratch/pr192-r5-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/pr192-r5/logs/pinned-final-logo.log 2>&1
docker run --rm --network none --cpus=2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/pr192-r5-tmp" -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -v "$PWD/.scratch/pr192-r5-final-fresh:$PWD/.scratch/pr192-r5-final-fresh:ro" -v "$PWD/.scratch/pr192-r5-tmp:$PWD/.scratch/pr192-r5-tmp" --name ws11api-r5-pinned-storage --entrypoint "$PWD/.scratch/pr192-r5-final-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/pr192-r5/logs/pinned-final-storage.log 2>&1
```

```text
PR192_R4_JPEG reuse_statuses=201/201 messages=2 variants=1 missing_file_serving=empty_404
PR192_R4_MISSING_FILE preview_missing=true processed_metadata=reused file_helper=clean_error redirect=302 rows/files/jobs=unchanged
PR192_R4_AFTER_COMMIT status=500 message/thread/media/two_jobs=committed preview/variant_files=retained later_model_callback=skipped
PR192_R4_MISSING_FILE preview_missing=false processed_metadata=reused file_helper=clean_error redirect=302 rows/files/jobs=unchanged
PR192_R5_REPRESENTATION jpeg_variant redirect status=302 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION jpeg_variant proxy status=404 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_preview proxy status=200 disk_status=200 rows/files/jobs=unchanged regenerated=false
PR192_R4_RELATIONS blobs=+2 attachments=+3 variants=+1 three_exact_foreign_keys=true target_injection=detected
PR192_R5_REPRESENTATION video_preview redirect status=302 disk_status=200 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_variant proxy status=404 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R5_REPRESENTATION video_variant redirect status=302 disk_status=404 rows/files/jobs=unchanged regenerated=false
PR192_R4_ROLLBACK preview_attachment/variant_attachment/push_job rows/files=unchanged
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 2509 filtered out; finished in 21.88s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2539 filtered out; finished in 0.80s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.08s```

The PR192 batch also reruns earlier flat readers, private-link redaction/counts, timezone/DST and array-ID differentials. WS12's separate /work.json growth was not changed.

## Explicit polling cases (exit0)

The full suite retains its 16 inherited ignores. The two WS14g HTTP polling cases pending this API branch were also executed explicitly; both pass. Their ignore attributes remain for the owner to remove when this branch lands.

```bash
env CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r5-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/pr192-r5-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/pr192-r5/logs/polling-explicit.log 2>&1
```

```text
running 2 tests
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ok
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2538 filtered out; finished in 0.71s
```

## Rails source pins, fresh vectors and planted-defect checks (all exit0)

```bash
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/record-http-vectors.py .scratch/pr192-r5/fresh-oracles
python3 .scratch/pr192-r5-final-fresh/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/pr192-r5-final-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/pr192-r5/fresh-oracles
python3 .scratch/pr192-r5-final-fresh/rust/reference-tools/agents/test-http-vector-verifier.py
```

```text
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
PR192 R5 Rails representations: 3 scenarios; 21 exact response bodies and header sets; 0 masks
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

.............
----------------------------------------------------------------------
Ran 13 tests in 0.013s

OK
```

The baseline 1,491 ordinary request/response vectors and two approved media artifacts remain unchanged. The three new scenario vectors add 21 exact subresponses. None of the new handled responses is an approved status difference.

## Deferred scope and cleanup

The nine work-write adapters remain pending here: four REST paths return generic 500, five MCP tools return -32603, and they do not write domain/queue state. WS12's services are now merged, but wiring them belongs to the paused work-writes task, whose WIP is saved locally and was not pushed. No grant, ledger, audit or queue domain logic was duplicated. No /work.json query-growth fix was attempted; WS12 owns it. No open service gap was investigated in this representation-only fix.

All owned test/container/build processes completed. Only one owned scratch target existed at a time. It was deleted after verification; fresh-clone source, seeds and raw logs remain for review.

```text
WS11-api scratch cleanup: removed 12025103519 target bytes; 0 owned scratch targets remain; evidence preserved
```
