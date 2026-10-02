# WS11-api PR192 third-review fixes — PARTIAL wave-4 handoff, 2026-10-02

Verified Rust source/tests/vector commit: `c180d07f26e61f2f5ae2573e29a57e2d88902686`, branch `rust/ws11api-rest-mcp`. The final report commit changes documentation only. The requested read-only review at `/home/riels/.cache/rust-port/ws11apir/review-192-r3/review-evidence.md` was read before editing.

## Fixes, ruling and failing-first evidence

1. **Fresh video thumbnail retention.** `crates/campfire/src/active_storage.rs` now decides the approved JPEG discard from the **original source's** content type, before a video is replaced by its JPEG preview. Non-JPEG attachments retain their staged variants after commit and enqueue variant analysis atomically. Successful fresh video has source blob 15, JPEG preview 16 and WebP variant 17, all with files. Reposting reuses the same usable variant. Injected variant insertion and analysis-queue failures return 500, restore every domain/media/queue row and remove all staged preview/variant files.

   The exact Rails exception is **`ActiveStorage::FileNotFoundError: ActiveStorage::FileNotFoundError`**, raised in **`ActiveStorage::Service::DiskService#stream`, `activestorage/lib/active_storage/service/disk_service.rb:152`**, rescuing `Errno::ENOENT`. Gem revision is `1a02651ac37f`. `ActiveStorage::Preview#processed` requests its variant at `preview.rb:54`; `VariantWithRecord#transform_blob` opens the newly recorded preview at `variant_with_record.rb:48` before its deferred upload. The probe captures **one open transaction**. Rails returns 500 and rolls back the message, reopening, preview/variant rows and jobs. The separately uploaded source survives.

   **The pre-commit defect branch of the maintainer's ruling applies.** Rust does not reproduce the crash: it returns **201** with consistent media and two logical jobs, `ActiveStorage::AnalyzeJob(blob_id:17)` and `ChannelThread::PushMessageJob(thread_id:1900700030,message_id:935962058)`. `reference-tools/agents/review192r3_attachment_diagnosis.rb` first records the unchanged failing Rails request. It then uses ordinary Rails preprocessing in committed transactions (including preview analysis) and posts the same source to produce a valid successful response/state oracle. No Rails request or failure path is patched.

   `vectors/agent_review192r3_attachment.json` retains both outcomes and the Rails rollback/exception, all nonrandom approved message/thread/blob attributes, attachment relationships, file presence/sizes and logical job arguments. `plans/ws11api-approved-differences.md` records the rationale beside the JPEG approval. The verifier compares every artifact byte and has injections rejecting status, rollback, file, job, response and header drift.

   **JPEG remains the explicit state exception.** The already-approved fresh JPEG vector still asserts its absent variant file, one push job and Rust201/Rails500. That is the only deliberate missing-file exception; this report does not claim an unqualified invariant that every JPEG variant has a file. Existing JPEG variants with files remain usable. This preserves the instruction to scope the discard to the approved JPEG case.

2. **HTML timestamp excluded from JSON.** `crates/db/src/models/message_rendering.rs` loads `github_pull_request_threads` aggregate timestamps only in the full HTML loader, never `load_payload`. At both 5/50 plain root messages, MCP reader SELECTs fall **19→18**, and this unused query falls **1→0**. Both response SHA-256 hashes are identical before/after. All 1,491 existing wire cases are unchanged.

The separate baseline clone remains at **15f5536f9869d1261568e9153dfcd1f1bae6c985**. Only the six-test module, registration/helper visibility and vector were added. **Four regressions failed first:** fresh video retained no variant file, reuse trusted that missing file, no variant analysis was enqueued, and MCP queried HTML timestamps at both sizes. **Two controls already passed:** reused JPEG file retention and injected variant-insert rollback. The committed receipt is `reference-tools/agents/review-192-r3-failing-first.txt`. The existing approved JPEG and original attachment rollback regressions remain in the pinned 19-test review run. No mock replaces the router, writer, media processor or queue.

## Merge and cross-workstream boundaries

Merge commit `c180d07f26e61f2f5ae2573e29a57e2d88902686` includes fetched main **5f908337a953b8672472317b03bd0cd5de34194e**, including #185, #194, #195 and #197. Conflicts were resolved by retaining both the agent endpoint declarations/payload preloads and main's huddle endpoints and batched event/GitHub provider loaders. No board presenter was manually changed; main's reviewed WS14g changes arrived through the merge. Main's real WS15g repository/account services remain authoritative. Cargo metadata passes with `--locked` after the merge. No Rails source, dependency, throttle, timing threshold or comparison mask was changed. The only new parity exception is the explicitly authorized fresh-video outcome; the JPEG approval is unchanged.

## Complete versus flagged

Both requested review findings are fixed. Twenty exact recaptured artifacts contain **1,491 unchanged wire cases plus two approved media-difference cases**. All 38 MCP tools remain dispatched; selected success coverage remains 31/35 REST and 33/38 MCP. The boundary and validation matrices cover selected cases, not every possible input/header.

**Nine WS12 writes remain pending:** REST board create and work update/result/handoff; MCP `create_board_post`, `update_board_post`, `update_work`, `set_result`, `handoff_work`. They return **generic REST500 / MCP-32603**, with unchanged domain/queue tables; no fake success or special unavailable response is claimed. No WS12 branch/service was wired in this review pass.

**Not only owner-blocked items remain in the broader wave-4 scope.** Exhaustive coercion/length/callback precedence, broader date grammar, work/filter/cursor combinations, legacy boosts without Agent rows, bot bounce/reply-source chains, further media replay/purge/MIME/representation cases, and concurrent revocation/viewer/delivery/finalization interactions remain partial. This requested review pass is complete; no new Rails controller-file pass counts are claimed.

## Verification and native media limits

A fresh Git clone of the merged commit ran the entire workspace, with default, first_run and agents_ui seeds rebuilt. All 13 local workspace packages were cleaned before rebuilding from fresh source paths; third-party compilation cache was retained. Cargo used two jobs and the configured rustc throttle. Tests used at most eight threads and ports 52900–52949. The Python model server was not touched; no stash/rebase, new ignore, reduced concurrency or widened timing threshold was used.

The native suite exits **101**: **4,465 passed, three failed, 16 ignored, zero actual seed skips**. Two failures are the accepted native logo and storage-version checks (also reproduced on clean main by the read-only review). The third is the **new strict video state oracle**, whose native preview/WebP sizes/checksums differ: JPEG11787 versus pinned11788 bytes, WebP3336 versus pinned3326. Message/thread/source metadata, attachments, jobs and file presence match; no fields are masked. The same binary passes that complete state comparison in the pinned runtime. Pinned logo, all ten storage vectors and all 19 PR192 regressions pass. Both normally ignored WS14g polling HTTP comparisons were explicitly run and pass.

Strict clippy, release-input build, all exact oracle comparisons, 84 source pins and 11 verifier injection tests exit **0**. The full workspace's other ignored tests remain deferred; they are not counted as passes. Raw commands and summary lines follow.

## Failing-first baseline (exit 101)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r3-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-r3-target" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked --manifest-path .scratch/pr192-r3-baseline/rust/Cargo.toml -p campfire --bin campfire pr192_r3 -- --nocapture --test-threads=8 > .scratch/pr192-r3/logs/failing-first-final.log 2>&1
```

```text
Baseline: 15f5536f9869d1261568e9153dfcd1f1bae6c985
Only regression module, registration, helper visibility and vector added; no production edits.
PR192_R3_CACHE size=5 SELECTs=19 HTML_timestamp_SELECTs=1 body_sha256=e5496bb86574510f488746563a320c83942db1b1d06fc29c761f3210b055324a
test controllers::agent_review_r3_tests::pr192_r3_reused_jpeg_variant_retains_its_existing_file ... ok
PR192_R3_CACHE size=50 SELECTs=19 HTML_timestamp_SELECTs=1 body_sha256=91bee85e195e08c6912e83111854ff26b6f123d88f942ff1df256dc51803a1fe
assertion `left == right` failed: JSON payloads must not read HTML-only cache stamps
  left: [1, 1]
 right: [0, 0]
test controllers::agent_review_r3_tests::pr192_r3_mcp_payload_omits_html_cache_timestamps_at_both_sizes ... FAILED
test controllers::agent_review_r3_tests::pr192_r3_video_insert_failure_rolls_back_all_rows_and_files ... ok
PR192_R3_VIDEO status=201 variant_files=[(17, false, 0)]
assertion `left == right` failed: successful video must enqueue its variant analysis atomically
  left: 201
 right: 500
test controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files ... FAILED
test controllers::agent_review_r3_tests::pr192_r3_video_analysis_enqueue_failure_rolls_back_all_rows_and_files ... FAILED
test controllers::agent_review_r3_tests::pr192_r3_reused_video_variant_keeps_the_same_usable_file ... FAILED
test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 2370 filtered out; finished in 1.02s
```

## Fresh clone, merge lockfile and seeds (exit 0)

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/pr192-r3-final-fresh
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path .scratch/pr192-r3-final-fresh/rust/Cargo.toml --format-version 1 > .scratch/pr192-r3/final-metadata.json
git log -1 --format='%H%n%P'
PARITY_NAMESPACE=ws11api-r3-final-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-r3-final-fresh/rust/parity/bin/seed build default first_run agents_ui
```

```text
c180d07f26e61f2f5ae2573e29a57e2d88902686
d7d3390fd99c9cebae1cf42bc2b841f6af305a39 5f908337a953b8672472317b03bd0cd5de34194e
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

## Full workspace (exit 101: three native media comparisons)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r3-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r3-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/pr192-r3/logs/final-workspace.log 2>&1
python3 .scratch/pr192-r3-final-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/pr192-r3/logs/final-workspace.log
```

```text
    Finished `test` profile [unoptimized] target(s) in 3m 01s
test result: FAILED. 2481 passed; 2 failed; 7 ignored; 0 measured; 0 filtered out; finished in 519.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.77s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1257 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 123.10s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.33s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.41s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s
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
WS11-api cargo totals: 4465 passed; 3 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

## Controller/review receipts from the full run

```bash
python3 .scratch/pr192-r3/receipts.py
```

```text
WS11-api compiled agent/bot/review groups: 97 passed; 1 failed; 0 ignored
PR192_R2_PAYLOAD context=true SELECTs=27
PR192_R2_CONTEXT author_batch_SELECTs=1
PR192_R2_JPEG Rust_status=201 Rails_status=500 jobs=1 variant_file_exists=false
PR192_R2_PAYLOAD context=false SELECTs=18
PR192_R2_PRIVATE interface=mcp_private_work size=5 SELECTs=22
PR192_R2_PRIVATE interface=mcp_private_work size=50 SELECTs=22
PR192_R2_PRIVATE interface=rest_private_work size=5 SELECTs=21
PR192_R2_PRIVATE interface=rest_private_work size=50 SELECTs=21
PR192_R3_VIDEO status=201 variant_files=[(17, true, 3336)]
PR192_R3_CACHE size=5 SELECTs=18 HTML_timestamp_SELECTs=0 body_sha256=e5496bb86574510f488746563a320c83942db1b1d06fc29c761f3210b055324a
PR192_R3_CACHE size=50 SELECTs=18 HTML_timestamp_SELECTs=0 body_sha256=91bee85e195e08c6912e83111854ff26b6f123d88f942ff1df256dc51803a1fe
WS11-api review reader: board_rest; size=5; returned=5; SELECTs=14
WS11-api review reader: board_mcp; size=5; returned=5; SELECTs=9
WS11-api review reader: board_rest; size=50; returned=50; SELECTs=14
WS11-api review reader: board_mcp; size=50; returned=50; SELECTs=9
WS11-api review reader: context_thread; size=5; returned=5; SELECTs=27
WS11-api review reader: context_thread; size=50; returned=50; SELECTs=27
WS11-api review thread attachment: status=500; posted_rows=0; closed_at=Some("2026-03-01 16:00:00")
WS11-api review reader: history_root; size=5; returned=5; SELECTs=18
WS11-api review reader: history_root; size=50; returned=50; SELECTs=18
WS11-api review reader: history_thread; size=5; returned=5; SELECTs=26
WS11-api review reader: history_thread; size=50; returned=50; SELECTs=26
WS11-api review reader: work_rest; size=5; returned=5; SELECTs=12
WS11-api review reader: work_mcp; size=5; returned=5; SELECTs=13
WS11-api review reader: work_rest; size=50; returned=50; SELECTs=12
WS11-api review reader: work_mcp; size=50; returned=50; SELECTs=13
```

## Strict clippy (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r3-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/pr192-r3-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/pr192-r3/logs/final-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 2m 18s
```

## Release inputs (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r3-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-r3-final-fresh/rust/target" mise exec rust@1.98.1 -- bash .scratch/pr192-r3-final-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/pr192-r3/logs/final-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 58s
```

## Exact vector, source and injection suites (all exit 0)

```bash
PARITY_NAMESPACE=ws11api-r3-final-oracles RUST_TEST_THREADS=8 python3 .scratch/pr192-r3-final-fresh/rust/reference-tools/agents/record-http-vectors.py .scratch/pr192-r3/final-oracles
python3 .scratch/pr192-r3-final-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/pr192-r3/final-oracles
python3 .scratch/pr192-r3-final-fresh/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/pr192-r3-final-fresh/rust/reference-tools/agents/test-http-vector-verifier.py
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
...........
----------------------------------------------------------------------
Ran 11 tests in 0.007s

OK
WS11-api vector totals: 1491 unchanged wire cases; 2 approved media differences; 20 exact oracle artifacts
```

## Pinned PR192 review regressions (exit 0)

```bash
docker run --rm --name ws11api-r3-review-tests --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/pr192-r3-tmp" -v "$PWD/.scratch/pr192-r3-final-fresh:$PWD/.scratch/pr192-r3-final-fresh:ro" -v "$PWD/.scratch/pr192-r3-tmp:$PWD/.scratch/pr192-r3-tmp" --entrypoint "$PWD/.scratch/pr192-r3-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8
```

```text
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 2471 filtered out; finished in 18.65s
```

```text
PR192_R2_PRIVATE interface=mcp_private_work size=5 SELECTs=22
PR192_R2_PRIVATE interface=rest_private_work size=5 SELECTs=21
PR192_R3_CACHE size=5 SELECTs=18 HTML_timestamp_SELECTs=0 body_sha256=e5496bb86574510f488746563a320c83942db1b1d06fc29c761f3210b055324a
PR192_R2_PRIVATE interface=rest_private_work size=50 SELECTs=21
PR192_R2_JPEG Rust_status=201 Rails_status=500 jobs=1 variant_file_exists=false
PR192_R2_PRIVATE interface=mcp_private_work size=50 SELECTs=22
PR192_R2_PAYLOAD context=false SELECTs=18
PR192_R3_CACHE size=50 SELECTs=18 HTML_timestamp_SELECTs=0 body_sha256=91bee85e195e08c6912e83111854ff26b6f123d88f942ff1df256dc51803a1fe
PR192_R2_PAYLOAD context=true SELECTs=27
PR192_R2_CONTEXT author_batch_SELECTs=1
PR192_R3_VIDEO status=201 variant_files=[(17, true, 3326)]
```

## Pinned logo (exit 0)

```bash
docker run --rm --name ws11api-r3-logo --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8  -e TMPDIR="$PWD/.scratch/pr192-r3-tmp" -v "$PWD/.scratch/pr192-r3-final-fresh:$PWD/.scratch/pr192-r3-final-fresh:ro" -v "$PWD/.scratch/pr192-r3-tmp:$PWD/.scratch/pr192-r3-tmp" --entrypoint "$PWD/.scratch/pr192-r3-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2489 filtered out; finished in 2.29s
```

## Pinned storage vectors (exit 0)

```bash
docker run --rm --name ws11api-r3-storage --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8  -e TMPDIR="$PWD/.scratch/pr192-r3-tmp" -v "$PWD/.scratch/pr192-r3-final-fresh:$PWD/.scratch/pr192-r3-final-fresh:ro" -v "$PWD/.scratch/pr192-r3-tmp:$PWD/.scratch/pr192-r3-tmp" --entrypoint "$PWD/.scratch/pr192-r3-final-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.12s
```

## Explicit WS14g polling HTTP comparisons (exit 0)

```bash
CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r3-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/pr192-r3-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/pr192-r3/logs/final-ws14g-polling.log 2>&1
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2488 filtered out; finished in 0.85s
```

## Cleanup

```text
WS11-api cleanup: removed owned scratch target .scratch/pr192-r3-final-fresh/rust/target (11823586562 bytes); 0 owned PR192-R3 target directories remain
```

All test/build processes and review containers completed before target deletion. Logs, source clones and seeds remain in the owned `.scratch/pr192-r3*` paths. The final report commit changes no verified Rust source, tests, helpers or vector bytes. The pushed SHA is in the final reply.
