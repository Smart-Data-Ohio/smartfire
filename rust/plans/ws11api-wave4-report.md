# WS11-api: PR192 fourth-review fixes

Verified source: `2676ac9a5fb68d2e26ab17bc67cc99f244ea18bb`, following main merge `06f0685e6dc45919ae36b20e831da4fd89dddc7e`. The merge includes main `033ab0ca93a567bb7ddaf239ca29db9d1253a9ff` (#193). Cargo metadata succeeds with `--locked`. The separate `rust/ws11api-work-writes` branch remains exactly at `ceb1229b16ad40978a9fc5a97072469a9c5e9f4f`; no work-write adapter changes were made.

## Fixes and failing-first evidence

| Item | Against unchanged 03342475 | Fixed behavior |
|---|---|---|
| Committed video ownership | An earlier MessageActivity after-commit failure commits the message, reopened thread, media rows and two jobs, but deletes preview 16 and variant 17 files | HTTP500 and committed state remain; both generated files survive with exact row/file sizes and checksums, and representation reuse returns a usable file |
| Missing-file representation lookup | Existing missing variant/preview records return a dangling blob | Matching transformation lookup returns a clean error; the exact signed HTTP representation returns 500 with no redirect and unchanged rows/files/jobs |
| Video attachment targets | The oracle has no record_id and the new target assertion fails | Rails extraction includes record_id and record_type; all three exact Message/source/preview/variant relationships and row deltas (+2 blobs, +3 attachments, +1 variant) are checked |

The six-test final baseline batch has **1 passing pre-commit rollback control and 5 failures**, including both missing-file cases. Production source in the baseline clone is exactly 03342475; only tests, module registration and helper visibility were copied. The committed raw receipt is `reference-tools/agents/review-192-r4-failing-first.txt`.

The signed HTTP key converts a Ruby format symbol into a string and produces a different Marshal digest, as Rails does. An initial extra HTTP assertion removed the posting variant but requested a distinct URL variant, which correctly regenerated and returned 302. The fixture now materializes/removes the exact URL transformation before asserting its failure. The first full attempt was stopped to correct that test. A second complete run identified the approved JPEG preparation regression and an obsolete dispatch-test assumption after main completed the last parameterless unported GET. The shared messaging runtime also required retaining metadata-only lookup semantics. The final fresh clone runs the six new regressions, all six legacy JPEG response/state vectors and the dispatch test before the whole workspace. Superseded runs are not counted as final verification. The final full workspace run below is from a new clone of the corrected commit.

## Changes by file

- `crates/db/src/database.rs`: an infallible `on_commit_success` resource-finalization queue runs immediately after successful COMMIT and before ordinary model callbacks. Savepoint rollback truncates it, and failed writes/COMMIT drop captured staged guards. Three database regressions cover callback failure, savepoint boundaries and actual deferred-foreign-key COMMIT failure.
- `crates/campfire/src/active_storage.rs`: staged-file retention uses that queue for originals, previews and variants. The approved original-JPEG discard remains unchanged.
- `crates/storage/src/storage.rs`: serving/transformation lookups require the recorded file while metadata lookup retains Rails processed-record semantics; absent files fail cleanly instead of returning a dangling blob. No synthetic success, automatic metadata rewrite or additional variant record is returned for the missing transformation.
- `crates/campfire/src/controllers/agent_review_r4_tests.rs`: real HTTP/media/writer regressions for an earlier fallible callback, successful file reuse, two missing-file cases, exact foreign keys/counts and three late pre-commit faults. Existing R3 helper visibility/module registration supports these tests. The added approved-JPEG regression verifies two posts reuse one metadata record while missing-file serving fails.
- `crates/campfire/src/controllers/messages.rs`: preserve the approved JPEG metadata reuse in attachment preparation; it returns no image for serving. Agent thread preparation preserves the same boundary.
- `crates/campfire/src/controllers.rs`: main completes the last parameterless unported GET; the dispatch regression now exercises every remaining unported GET, including parameterized routes, and retains the missing-action/unknown-path404 checks.
- `reference-tools/agents/review192r3_attachment_diagnosis.rb` and its committed video vector: retain attachment target IDs; every other byte, including sizes/checksums and record types, is unchanged.
- `reference-tools/agents/{verify-http-vectors.py,test-http-vector-verifier.py}`: validate exact target tuples and prove target/type mutations fail comparison. The injection suite is now 12 tests.
- `reference-tools/agents/review192r4_commit_callbacks.rb` and its recorded JSON: pinned Rails callback experiment, without changing the Rails app or its callbacks.

## After-commit audit and Rails semantics

Pinned Rails gem 1a02651ac37f `ActiveRecord::ConnectionAdapters::Transaction#commit_records` (abstract/transaction.rb:308) stops executing callbacks on the first exception. Its ensure path only calls `committed!(should_run_callbacks: false)` on remaining records; transaction after_commit callbacks are also skipped. The real Rails probe raises `RuntimeError: PR192 injected Rails after-commit error`, records only `first:1900700040`, retains both records 1900700040/1900700041 and reports zero open transactions. It recaptures identically to the committed receipt.

Rust preserves this cancellation behavior for ordinary hooks and broadcasts. Production `Message::create` registers fallible `ChannelThread::broadcast_thread_indicators` reads before media retention, so ownership cannot depend on their success. Finalizing already-written files is resource bookkeeping, not a new Rails-style side effect.

The other work that must survive this failure is committed durable job notification: thread push, variant analysis and any committed purge/delivery jobs. Main already persists these inside the source transaction and wakes remaining committed jobs after an earlier callback error. The existing `ws11_r4_callback_failure_stops_hooks_but_wakes_committed_jobs` regression is rerun below, and the HTTP fault regression asserts the exact two committed job payloads. No additional change to job handling was needed. Later ordinary indicator/publication/model hooks remain skipped as in Rails. Transactional index/unread/reference/activity bookkeeping remains committed; pre-commit media/attachment/job faults still roll it all back.

The two approved Rails crash differences remain narrow: fresh JPEG Rust201/Rails500 with its explicitly approved missing variant file; fresh video Rust201/Rails500 with usable generated files. The fresh-video diagnosis remains the genuine pre-commit unuploaded JPEG preview defect, not a missing original upload. No masks, timings, concurrency settings, dependency or Rails source changed.

## Verification and limits

All commands below were run for this report. The final clone rebuilt default, first_run and agents_ui seeds, cleaned all 13 local workspace packages and rebuilt from its own source paths. Only one owned target existed at a time; cargo used two jobs with the configured rustc throttle and tests used eight threads/ports52900–52949. The Python model server was not touched; no stash/rebase or new ignore was used.

The native workspace retains the previously established media-version differences: logo output, the storage version guard and the exact fresh-video byte golden. These are not masked or counted as passing. The same current test binaries pass the relevant full assertions in the pinned runtime. Other ignored tests remain ignored and are not passes. There are zero actual seed skips.

## Failing-first command (expected exit101)


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-r4-target" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r4-baseline/rust/Cargo.toml -p campfire --bin campfire pr192_r4 -- --nocapture --test-threads=8 > .scratch/pr192-r4/logs/final-failing-first.log 2>&1
```

```text
Baseline 03342475c6bc61e44d1ce3cc39d6799aafa2df58; production source unchanged.
The final six-test module, module registration and existing-helper visibility were copied; the old oracle remains unchanged.
    Finished `test` profile [unoptimized] target(s) in 2m 31s
thread 'controllers::agent_review_r4_tests::pr192_r4_video_oracle_preserves_attachment_targets_and_exact_row_counts' (1372923) panicked at crates/campfire/src/controllers/agent_review_r4_tests.rs:187:9:
oracle omits attachment target: {"blob_id":16,"created_at":"2026-03-02 16:00:00 UTC","name":"preview_image","record_type":"ActiveStorage::Blob"}
test controllers::agent_review_r4_tests::pr192_r4_video_oracle_preserves_attachment_targets_and_exact_row_counts ... FAILED
thread 'controllers::agent_review_r4_tests::pr192_r4_approved_jpeg_metadata_reuse_does_not_serve_missing_files' (1372918) panicked at crates/campfire/src/controllers/agent_review_r4_tests.rs:296:5:
approved missing JPEG file must never be returned for serving: Ok(Blob { id: 16, key: "q66e086hralkmjzn8z0e1ftl169h", filename: Filename("moon.jpg"), content_type: Some("image/jpeg"), metadata: Object([("identified", Bool(true))]), service_name: "local", byte_size: 13036, checksum: Some("p7Xvr8seqToD36kDFhsGng=="), created_at: "2026-03-02 16:00:00" })
test controllers::agent_review_r4_tests::pr192_r4_approved_jpeg_metadata_reuse_does_not_serve_missing_files ... FAILED
thread 'controllers::agent_review_r4_tests::pr192_r4_earlier_after_commit_error_preserves_committed_video_files_and_jobs' (1372919) panicked at crates/campfire/src/controllers/agent_review_r4_tests.rs:30:5:
committed blob 16 has lost its file
test controllers::agent_review_r4_tests::pr192_r4_earlier_after_commit_error_preserves_committed_video_files_and_jobs ... FAILED
thread 'controllers::agent_review_r4_tests::pr192_r4_missing_preview_file_fails_cleanly_without_returning_a_blob' (1372921) panicked at crates/campfire/src/controllers/agent_review_r4_tests.rs:157:5:
missing media file returned dangling blob: Ok(Blob { id: 18, key: "aqx8i2kd4q6c0j3042yvgyi5318r", filename: Filename("alpha-centuri.webp"), content_type: Some("image/webp"), metadata: Object([("identified", Bool(true))]), service_name: "local", byte_size: 3336, checksum: Some("e8tykSPtDcnTAIo2FuqAJw=="), created_at: "2026-03-02 16:00:00" })
thread 'controllers::agent_review_r4_tests::pr192_r4_missing_variant_file_fails_cleanly_without_returning_a_blob' (1372922) panicked at crates/campfire/src/controllers/agent_review_r4_tests.rs:157:5:
missing media file returned dangling blob: Ok(Blob { id: 18, key: "2hmgujm8nb7u87aykx93g6g987w5", filename: Filename("alpha-centuri.webp"), content_type: Some("image/webp"), metadata: Object([("identified", Bool(true))]), service_name: "local", byte_size: 3336, checksum: Some("e8tykSPtDcnTAIo2FuqAJw=="), created_at: "2026-03-02 16:00:00" })
test controllers::agent_review_r4_tests::pr192_r4_missing_variant_file_fails_cleanly_without_returning_a_blob ... FAILED
test controllers::agent_review_r4_tests::pr192_r4_missing_preview_file_fails_cleanly_without_returning_a_blob ... FAILED
PR192_R4_ROLLBACK preview_attachment/variant_attachment/push_job rows/files=unchanged
test controllers::agent_review_r4_tests::pr192_r4_late_media_and_job_failures_roll_back_rows_and_files ... ok
    controllers::agent_review_r4_tests::pr192_r4_approved_jpeg_metadata_reuse_does_not_serve_missing_files
    controllers::agent_review_r4_tests::pr192_r4_earlier_after_commit_error_preserves_committed_video_files_and_jobs
    controllers::agent_review_r4_tests::pr192_r4_missing_preview_file_fails_cleanly_without_returning_a_blob
    controllers::agent_review_r4_tests::pr192_r4_missing_variant_file_fails_cleanly_without_returning_a_blob
    controllers::agent_review_r4_tests::pr192_r4_video_oracle_preserves_attachment_targets_and_exact_row_counts
test result: FAILED. 1 passed; 5 failed; 0 ignored; 0 measured; 2490 filtered out; finished in 4.19s
```

## Fresh-clone workspace


```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/pr192-r4-done-fresh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml --format-version 1 > .scratch/pr192-r4/done-metadata.json
PARITY_NAMESPACE=ws11api-r4-done-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-r4-done-fresh/rust/parity/bin/seed build default first_run agents_ui
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
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/pr192-r4/logs/done-workspace.log 2>&1
python3 .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/pr192-r4/logs/done-workspace.log
```

```text
test result: FAILED. 2512 passed; 2 failed; 7 ignored; 0 measured; 0 filtered out; finished in 484.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.42s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1279 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 107.80s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.84s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.41s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.30s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.15s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.75s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.35s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.73s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
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
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
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
WS11-api cargo totals: 4518 passed; 3 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

## Targeted preflight (all exit0)


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml -p campfire --bin campfire pr192_r4 -- --nocapture --test-threads=8
```

```text
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2515 filtered out; finished in 2.58s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml -p campfire --bin campfire controllers::messages::review_tests::jpeg_new_and_reused_variants_match_rails_rows_files_and_lifecycle -- --exact --nocapture --test-threads=8
```

```text
WS8bm JPEG boundaries: 6 Rails responses byte-identical; root success; initial/reply 500 then reuse 201; committed variant rows and missing files match
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2520 filtered out; finished in 2.09s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml -p campfire --bin campfire controllers::tests::unported_actions_say_so_instead_of_404ing -- --exact --nocapture --test-threads=8
```

```text
Dispatch unported GET routes: 1 checked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2520 filtered out; finished in 0.28s
```

## New invariants and database callback controls


```bash
rg '^PR192_R4|^test database::tests::commit_finalizers|^test tests::agent_work_events_test::ws11_r4_callback_failure_stops_hooks_but_wakes_committed_jobs' .scratch/pr192-r4/logs/done-workspace.log
```

```text
PR192_R4_JPEG reuse_statuses=201/201 messages=2 variants=1 missing_file_serving=clean_error
PR192_R4_AFTER_COMMIT status=500 message/thread/media/two_jobs=committed preview/variant_files=retained later_model_callback=skipped
PR192_R4_MISSING_FILE preview_missing=true clean_error=true rows/files/jobs=unchanged
PR192_R4_MISSING_FILE preview_missing=false clean_error=true rows/files/jobs=unchanged
PR192_R4_ROLLBACK preview_attachment/variant_attachment/push_job rows/files=unchanged
PR192_R4_RELATIONS blobs=+2 attachments=+3 variants=+1 three_exact_foreign_keys=true target_injection=detected
test database::tests::commit_finalizers_follow_savepoint_rollback_boundaries ... ok
test database::tests::commit_finalizers_do_not_run_when_commit_itself_fails ... ok
test database::tests::commit_finalizers_precede_fallible_model_callbacks ... ok
test tests::agent_work_events_test::ws11_r4_callback_failure_stops_hooks_but_wakes_committed_jobs ... ok
```

## Strict clippy and release inputs (both exit0)


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/pr192-r4-done-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/pr192-r4/logs/verified-clippy.log 2>&1
```

```text
Finished `dev` profile [unoptimized] target(s) in 1m 26s
```


```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r4-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-r4-done-fresh/rust/target" mise exec rust@1.98.1 -- bash .scratch/pr192-r4-done-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/pr192-r4/logs/verified-release-inputs.log 2>&1
```

```text
Finished `dev` profile [unoptimized] target(s) in 1m 27s
```

## Exact vectors, source pins and verifier injections (exit0)


```bash
PARITY_NAMESPACE=ws11api-r4-done-vectors python3 .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/record-http-vectors.py .scratch/pr192-r4/done-vectors
python3 .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/pr192-r4/done-vectors
PARITY_NAMESPACE=ws11api-r4-done-sources python3 .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/test-http-vector-verifier.py
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
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
............
----------------------------------------------------------------------
Ran 12 tests in 0.009s

OK
WS11-api vector totals: 1491 unchanged wire cases; 2 approved media differences; 20 exact oracle artifacts
```

## Pinned callbacks and pinned media regressions (exit0)


```bash
PARITY_NAMESPACE=ws11api-r4-done-callbacks PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-r4-done-fresh/rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/review192r4_commit_callbacks.rb > .scratch/pr192-r4/verified-rails-callbacks.json
cmp .scratch/pr192-r4/verified-rails-callbacks.json .scratch/pr192-r4-done-fresh/rust/reference-tools/agents/review-192-r4-callbacks.json
```

```text
{
  "exception": {
    "class": "RuntimeError",
    "message": "PR192 injected Rails after-commit error",
    "open_transactions": 0
  },
  "trace": [
    "first:1900700040"
  ],
  "committed_ids": [
    1900700040,
    1900700041
  ],
  "commit_records_source": [
    "/usr/local/bundle/ruby/3.4.0/bundler/gems/rails-1a02651ac37f/activerecord/lib/active_record/connection_adapters/abstract/transaction.rb",
    308
  ]
}
```


```bash
python3 .scratch/pr192-r4/run-pinned.py
```

```text
review:
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 2496 filtered out; finished in 19.85s
logo:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2520 filtered out; finished in 0.94s
storage:
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.06s
polling:
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2519 filtered out; finished in 0.69s
```

## Remaining scope and cleanup

Both requested fourth-review fixes are complete. Nine work writes remain explicitly pending (four REST, five MCP): valid requests still return generic REST500 or MCP-32603 with no domain/queue writes. Their WS12 services are now merged, but adapter wiring remains pending and its run was paused by the maintainer on the preserved work-writes branch; these are no longer owner-blocked. No work-write service/domain logic was reimplemented here. All other prior API coverage is retained.



```bash
python3 .scratch/pr192-r4/cleanup-target.py
```

```text
WS11-api cleanup: removed owned scratch target .scratch/pr192-r4-done-fresh/rust/target (11995767141 bytes); 0 owned PR192-R4 target directories remain
```
