# WS11-api PR192 second-review fixes — PARTIAL wave-4 handoff, 2026-10-02

Verified Rust code/test commit: `d6e60d85b51d7ce5470cceeca39d897058dc5a46`, branch `rust/ws11api-rest-mcp`. The final commit adds this report and strengthens Rails job-argument extraction without changing Rust code, tests or vector bytes. The read-only review at `/home/riels/.cache/rust-port/ws11apir/review-192-r2/review-evidence.md` was read before editing.

Main was merged with merge commit `11f0950af729fd176fa62aff0125b3495277597e`, containing origin/main `056ab49acffd52007334356ada0bdba3774c0fe2` (#191). Both main's fragment-cache facts and this branch's payload thread preloads were retained. The shared calendar parser replaced the old fallback stand-in. WS15g's main account/repository service remains authoritative and is called for every repository occurrence. `presenters/boards.rs` was not changed; its aged-board N+1 remains with WS14g.

## Fixes and failing-first evidence

A separate clone stayed at **b79f376e7c24979d8713b425c63be0d835bf173e**. Only the new regression module, its registration/helper visibility and vector were added. All **five regressions failed there before any production fix**. The receipt is `rust/reference-tools/agents/review-192-r2-failing-first.txt`; the complete final baseline log is `.scratch/pr192-r2/logs/failing-first-final.log`.

1. **Fresh JPEG / deliberate status difference.** The exact pinned Rails exception is **`IOError: closed stream`**, raised by `IO.copy_stream` in `activestorage/lib/active_storage/service/disk_service.rb:23`. Rails gem revision is `1a02651ac37f`. `ActiveStorage::VariantWithRecord#transform_blob` yields its output at `app/models/active_storage/variant_with_record.rb:49–52`; that IO closes before the outer `ChannelThread#post_message!` transaction at `app/models/channel_thread.rb:489` commits. Deferred `CreateOne#upload` then uses the closed stream. This is an accidental after-commit defect, not intentional validation.

   Following the maintainer's ruling, Rust retains **201**, versus Rails **500**. The successful body and captured headers equal Rails' idempotent replay of the committed message. Rust now matches the agreed state: committed message, reopened thread, analyzed source metadata, variant/blob/attachment rows, absent variant file, and exactly one `ChannelThread::PushMessageJob` with the correct logical thread/message arguments. No source or variant analysis job remains. The regression checks all message/thread columns, blob attributes other than its random storage key, variant digest/count, attachment relationships/counts, file absence and all job classes/logical arguments. Nonexposed attachment/variant surrogate IDs are omitted explicitly in the vector notes. Before: two jobs and a kept variant file; after: one job and no variant file. The existing injected insert-failure regression still returns 500 and rolls back message, reopen, media rows/files and queues. Approval and exception details live in `rust/vectors/agent_review192r2_attachment.json` and `rust/plans/ws11api-approved-differences.md`.

2. **Private-linked work.** Request-local identity snapshots replace four repeated identity/account SELECTs per permitted link. Every in-process writer job invalidates the snapshot, including rollback, panic and after-commit work; an active writer's snapshot is never reused. Repository permission results are never cached by this adapter, no connection is held across network awaits, and final owner/account fingerprint sealing remains fresh. Both REST and MCP regressions measure 5/50 rows, verify every private title/branch and identical response bytes, and assert 5/50 permission-service calls. Existing disconnect, transient retry, repeated-event, stale-snapshot and failure-order tests pass. A DB regression also checks invalidation through rollback and after-commit callbacks.

3. **Unused payload reads.** JSON payload loading skips HTML fragment/cache dependencies, boosts, pins, steps, polls/options/votes, quote-card sources and Fizzy/link/event/GitHub card references. Actual reply sources, bodies, users, rooms, attachments, Drive data, rich-text resolution and thread policy facts remain. HTML rendering retains its full loader. Context shares the payload's preloaded authors instead of loading them again. The two new regressions failed on unnecessary queries and two author batches, respectively; after the fix no unused table is read and context has one author batch.

Counts use the same fixtures before and after, at **5 → 50** rows:

| Interface | b79f376e Rust | Fixed Rust | Fresh Rails |
|---|---:|---:|---:|
| REST private-linked work | 41 → 221 | 21 → 21 | 12 → 12 |
| MCP private-linked work | 42 → 222 | 22 → 22 | 14 → 14 |
| MCP root messages | 28 → 28 | 19 → 19 | — |
| MCP thread messages | 36 → 36 | 27 → 27 | — |
| REST thread context | 38 → 38 | 28 → 28 | — |

Context author batches fall from 2 to 1. Rust counts are reader-pool SELECTs; Rails counts include all uncached request SELECTs. Absolute cross-runtime equality is not claimed. Job runners are disabled with `TestApp::without_job_runner()`. Rust plain fixtures include a work owner and differ from Astra's 33/35 thread/context totals.

## Complete versus flagged

All three requested second-review fixes are complete. **1,491 existing Rails wire vectors remain unchanged**, plus **one approved status-difference/state vector**, in 19 artifacts. All 38 MCP names and selected 31/35 REST and 33/38 MCP success paths remain covered. Selected body/status/header and validation matrices are not exhaustive coverage of every possible input or header.

**Nine WS12 writes remain pending:** REST board create and work update/result/handoff; MCP `create_board_post`, `update_board_post`, `update_work`, `set_result`, `handoff_work`. Valid requests return **generic REST 500 / MCP -32603**, with unchanged domain and queue tables. They are neither fake successes nor purpose-built unavailable responses. No WS12 branch or agent-work service was wired here. Live private-PR access stays with main's WS15g implementation. The board-row N+1 remains with WS14g.

**Not only owner-blocked items remain in the broader wave-4 task.** Exhaustive coercion/length/callback-precedence coverage, broader compact/partial date grammar, work/filter/cursor combinations, legacy boosts without Agent rows, additional bot bounce/reply-source chains, media replay/purge/MIME/representation cases, and further concurrent revocation/viewer/delivery/finalization interactions remain partial as previously reported. This requested review pass is complete. No new Rails controller-file pass counts are claimed.

## Final verification

The full workspace ran from the clean Git clone at the verified Rust code/test commit, with rebuilt default, first_run and agents_ui seeds. A single Cargo cache was moved into it; all local crates rebuilt from fresh-clone source paths. Cargo used two jobs and the configured rustc throttle; test commands used at most eight threads. No concurrency or timing threshold was lowered, no new ignore was added, no stash/rebase was used, and the Python model server was not touched. The only owned target was removed after all checks.

The native workspace exits **101** for exactly the two previously accepted native media-version failures: account logo bytes and `campfire_storage::pipeline_matches_the_reference`. The same binaries pass in the pinned runtime. Strict clippy, release-input build, source pins and vector checks exit **0**.

## Baseline regressions (exit 101)

```bash
CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 CARGO_TARGET_DIR="$PWD/.scratch/pr192-r2-target" TMPDIR="$PWD/.scratch/pr192-r2-tmp" mise exec rust@1.98.1 -- cargo test --locked --manifest-path .scratch/pr192-r2-baseline/rust/Cargo.toml -p campfire --bin campfire pr192_r2 -- --nocapture --test-threads=8 > .scratch/pr192-r2/logs/failing-first-final.log 2>&1
```

```text
Baseline b79f376e7c24979d8713b425c63be0d835bf173e; only regression registration, test helpers and vectors added.
PR192_R2_PRIVATE interface=rest_private_work size=5 SELECTs=41
PR192_R2_PRIVATE interface=mcp_private_work size=5 SELECTs=42
PR192_R2_PRIVATE interface=mcp_private_work size=50 SELECTs=222
test controllers::agent_review_r2_tests::pr192_r2_private_mcp_query_count_is_flat ... FAILED
PR192_R2_PRIVATE interface=rest_private_work size=50 SELECTs=221
test controllers::agent_review_r2_tests::pr192_r2_private_rest_query_count_is_flat ... FAILED
PR192_R2_PAYLOAD context=false SELECTs=28
test controllers::agent_review_r2_tests::pr192_r2_plain_payload_reads_only_rendered_facts ... FAILED
PR192_R2_PAYLOAD context=true SELECTs=38
PR192_R2_CONTEXT author_batch_SELECTs=2
test controllers::agent_review_r2_tests::pr192_r2_context_reuses_preloaded_authors ... FAILED
PR192_R2_JPEG Rust_status=201 Rails_status=500 jobs=2 variant_file_exists=true
test controllers::agent_review_r2_tests::pr192_r2_fresh_jpeg_approved_status_and_committed_state ... FAILED
test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 2319 filtered out; finished in 0.80s
```

## Fresh clone and seeds (exit 0)

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/pr192-r2-final-fresh
PARITY_NAMESPACE=ws11api-r2-final-seeds PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-r2-final-fresh/rust/parity/bin/seed build default first_run agents_ui
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

## Full workspace (exit 101, two native media failures)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r2-tmp" CABLE_TEST_PORT_RANGE=53500-53519 INTEGRATION_TEST_PORT_RANGE=53520-53559 MAIL_TEST_PORT_RANGE=53520-53559 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-r2-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/pr192-r2/logs/final-workspace.log 2>&1
python3 .scratch/pr192-r2-final-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/pr192-r2/logs/final-workspace.log
```

```text
    Finished `test` profile [unoptimized] target(s) in 4m 02s
test result: FAILED. 2364 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 484.87s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.27s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.02s
test result: ok. 1255 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 163.91s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.37s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.15s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.35s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.50s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.94s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.85s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.19s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s
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
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.17s
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
WS11-api cargo totals: 4346 passed; 2 failed; 14 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

## API / reader receipts from that full run

```text
WS11-api compiled agent/bot controller groups: 92 passed; 0 failed; 0 ignored
WS11-api PR192 R2 regressions: 5 passed; 0 failed; 0 ignored
PR192_R2_PAYLOAD context=true SELECTs=28
PR192_R2_CONTEXT author_batch_SELECTs=1
PR192_R2_JPEG Rust_status=201 Rails_status=500 jobs=1 variant_file_exists=false
PR192_R2_PAYLOAD context=false SELECTs=19
PR192_R2_PRIVATE interface=mcp_private_work size=5 SELECTs=22
PR192_R2_PRIVATE interface=mcp_private_work size=50 SELECTs=22
PR192_R2_PRIVATE interface=rest_private_work size=5 SELECTs=21
PR192_R2_PRIVATE interface=rest_private_work size=50 SELECTs=21
WS11-api review reader: board_rest; size=5; returned=5; SELECTs=14
WS11-api review reader: board_mcp; size=5; returned=5; SELECTs=9
WS11-api review reader: board_rest; size=50; returned=50; SELECTs=14
WS11-api review reader: board_mcp; size=50; returned=50; SELECTs=9
WS11-api review reader: context_thread; size=5; returned=5; SELECTs=28
WS11-api review reader: context_thread; size=50; returned=50; SELECTs=28
WS11-api review reader: history_root; size=5; returned=5; SELECTs=19
WS11-api review reader: history_root; size=50; returned=50; SELECTs=19
WS11-api review reader: history_thread; size=5; returned=5; SELECTs=27
WS11-api review reader: history_thread; size=50; returned=50; SELECTs=27
WS11-api review reader: work_rest; size=5; returned=5; SELECTs=12
WS11-api review reader: work_mcp; size=5; returned=5; SELECTs=13
WS11-api review reader: work_rest; size=50; returned=50; SELECTs=12
WS11-api review reader: work_mcp; size=50; returned=50; SELECTs=13
```

## Fresh Rails private-link counts (exit 0)

```bash
PARITY_NAMESPACE=ws11api-r2-private PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/review192r2_private_queries.rb > .scratch/pr192-r2/logs/rails-private-queries.log 2> .scratch/pr192-r2/logs/rails-private-queries.err
```

```text
PR192_PRIVATE_RAILS label=rest_private_work size=5 returned=5 select_count=12 reader_calls=5
PR192_PRIVATE_RAILS label=mcp_private_work size=5 returned=5 select_count=14 reader_calls=5
PR192_PRIVATE_RAILS label=rest_private_work size=50 returned=50 select_count=12 reader_calls=50
PR192_PRIVATE_RAILS label=mcp_private_work size=50 returned=50 select_count=14 reader_calls=50
```

## Strict clippy (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r2-tmp" CABLE_TEST_PORT_RANGE=53500-53519 INTEGRATION_TEST_PORT_RANGE=53520-53559 MAIL_TEST_PORT_RANGE=53520-53559 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/pr192-r2-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/pr192-r2/logs/final-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 39s
```

## Release inputs (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r2-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-r2-final-fresh/rust/target" mise exec rust@1.98.1 -- bash .scratch/pr192-r2-final-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/pr192-r2/logs/final-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 52s
```

## Recaptured vector suites, pins and verifier injections (all exit 0)

```bash
PARITY_NAMESPACE=ws11api-r2-final-oracles RUST_TEST_THREADS=8 python3 .scratch/pr192-r2-final-fresh/rust/reference-tools/agents/record-http-vectors.py .scratch/pr192-r2/final-oracles
python3 .scratch/pr192-r2-final-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/pr192-r2/final-oracles
python3 .scratch/pr192-r2-final-fresh/rust/reference-tools/agents/check-http-reference.py
python3 .scratch/pr192-r2-final-fresh/rust/reference-tools/agents/test-http-vector-verifier.py
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
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.........
----------------------------------------------------------------------
Ran 9 tests in 0.004s

OK
```

The final Rails helper derives named job IDs from the actual queued Global IDs, rather than fixture constants. Its output was regenerated independently and compared without masks:

```bash
PARITY_NAMESPACE=ws11api-r2-queue-proof PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/review192r2_attachment_diagnosis.rb > .scratch/pr192-r2/attachment-derived-jobs.json 2> .scratch/pr192-r2/logs/attachment-derived-jobs.err
cmp rust/vectors/agent_review192r2_attachment.json .scratch/pr192-r2/attachment-derived-jobs.json
```

```text
WS11-api approved JPEG oracle: actual queued Global IDs map to identical logical job arguments; byte-identical committed vector
```

## Pinned logo (exit 0)

```bash
docker run --rm --name ws11api-r2-logo --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=53500-53519 -e INTEGRATION_TEST_PORT_RANGE=53520-53559 -e MAIL_TEST_PORT_RANGE=53520-53559 -e TMPDIR="$PWD/.scratch/pr192-r2-tmp" -v "$PWD/.scratch/pr192-r2-final-fresh:$PWD/.scratch/pr192-r2-final-fresh:ro" -v "$PWD/.scratch/pr192-r2-tmp:$PWD/.scratch/pr192-r2-tmp" --entrypoint "$PWD/.scratch/pr192-r2-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2369 filtered out; finished in 1.11s
```

## Pinned storage vectors (exit 0)

```bash
docker run --rm --name ws11api-r2-storage --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e TMPDIR="$PWD/.scratch/pr192-r2-tmp" -v "$PWD/.scratch/pr192-r2-final-fresh:$PWD/.scratch/pr192-r2-final-fresh:ro" -v "$PWD/.scratch/pr192-r2-tmp:$PWD/.scratch/pr192-r2-tmp" --entrypoint "$PWD/.scratch/pr192-r2-final-fresh/rust/target/debug/deps/vectors-377b245d4f8eeec0" ws11api-reference:d7c7de92 --nocapture --test-threads=8
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.57s
```

## Pinned PR192 regressions (exit 0)

```bash
docker run --rm --name ws11api-r2-review-tests --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=53500-53519 -e INTEGRATION_TEST_PORT_RANGE=53520-53559 -e MAIL_TEST_PORT_RANGE=53520-53559 -e TMPDIR="$PWD/.scratch/pr192-r2-tmp" -v "$PWD/.scratch/pr192-r2-final-fresh:$PWD/.scratch/pr192-r2-final-fresh:ro" -v "$PWD/.scratch/pr192-r2-tmp:$PWD/.scratch/pr192-r2-tmp" --entrypoint "$PWD/.scratch/pr192-r2-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57" ws11api-reference:d7c7de92 pr192 --nocapture --test-threads=8
```

```text
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 2357 filtered out; finished in 17.51s
```

## WS14g ignored polling comparisons, explicitly run (exit 0)

```bash
CI=1 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-r2-tmp" CABLE_TEST_PORT_RANGE=53500-53519 INTEGRATION_TEST_PORT_RANGE=53520-53559 MAIL_TEST_PORT_RANGE=53520-53559 .scratch/pr192-r2-final-fresh/rust/target/debug/deps/campfire-d38ff26cb5c6fd57 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/pr192-r2/logs/final-ws14g-polling.log 2>&1
```

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2368 filtered out; finished in 1.09s
```

## Observed media versions and cleanup

```text

ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
WS11-api cleanup: removed owned scratch target .scratch/pr192-r2-final-fresh/rust/target (15422982240 bytes); 0 owned target directories remain
```

Logs, seeds and source clones remain under the owned `.scratch/pr192-r2*` paths. The final report/oracle-helper commit does not change the verified Rust source, test corpus or vector bytes. The pushed head SHA is reported in the final reply.
