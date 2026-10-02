# WS11-api PR192 correctness fixes — PARTIAL wave-4 handoff, 2026-10-02

Verified code commit: `16b9063352b30544af6d02e5393142b00ae4573a` on `rust/ws11api-rest-mcp`. A report-only commit follows. This supersedes the prior verification section. The original review at `6ad21be091402e5d2db151a487310d2c252ab4cc` and read-only reviewer receipts (`/home/riels/.cache/rust-port/ws11apir/review-192/review-evidence.md` and `summary.txt`) were read before editing.

All four requested review defects are fixed. Main was merged with merge commits `98eaac743c1b2fd5fc20d4877a92432217841e3c` and `560dabab76818a741c7b360d4f2d75c55002d964`; the latter contains origin/main `573762b5987522edadbdb855532c5dc14a88b526` (reviewed agent UI). Conflicts retained main's activity/profile/directory/history and approval controllers alongside this branch's REST/MCP adapters. Shared WS11 models and main's WS15g accounts/repository policy remain authoritative; no private GitHub access implementation was added. No stash, rebase, WS12 branch merge, test-concurrency reduction or timing-threshold change occurred.

## Fixes and failing-first evidence

Only the new regression module, its test registration and captured Rails vectors were copied into a clone at **6ad21be0**. Its production code was unchanged. All **eight new tests failed there before the fixes**. Raw evidence is committed in `rust/reference-tools/agents/review-192-failing-first.txt`; the complete baseline log is `.scratch/pr192/logs/failing-first.log`.

1. **Thread attachment rollback.** Thread attachment analysis, metadata/touches, preview/variant inserts and jobs now run inside the same writer transaction as posting/reopening. Staged files are retained only after commit. The rejected-variant-insert regression returned 500 before and after: before it left one posted message and cleared `closed_at`; after it leaves zero posted rows and keeps the original closed timestamp. It also compares all rows of 12 domain/media/queue tables and the storage file list before/after.
2. **Reader batching.** Owned work and board filters select authorized, filtered, bounded SQL windows before presentation. Work payloads batch rooms/users/tags/links/pull requests/events; message/context payloads batch threads, memberships/counts, authors, attachments, Drive IDs and icons. Shared policy helpers preserve membership/grant and work-viewer checks; WS15g account snapshot/revalidation remains in use. Each query regression checks returned rows and response bytes at **5 and 50**, with the job runner disabled using main's `TestApp::without_job_runner()`.
3. **Approval deadlines.** Approval creation uses the authenticated user's zone and the existing shared Rails-style calendar time parser. All **24 REST/MCP differentials** pass: New York, UTC, Kolkata, the Rails Eastern alias, date-only inputs, explicit offset, invalid text and `expires_in` precedence. The New York spring-gap case succeeds and persists 07:30Z for local 02:30; the fall-fold and Lord Howe gap inputs reject with the seven-day TTL bound in this frozen-clock corpus, so they are not proof of persisted fold/half-hour-gap conversion. The regression checks persisted UTC deadlines as well as exact response/status/header bytes. Before the fix, 12 passed and 12 failed.
4. **Array IDs.** Context lookup uses Active Record-style flattened `IN` conditions for message/thread arrays and preserves error precedence for a supplied thread constraint. The **35 real context/history/board ID vectors** include REST query arrays, MCP arrays/nested arrays, valid-plus-missing IDs, empty/missing/hash shapes, dual context IDs, room/thread history IDs, cursors and board IDs. Fourteen context cases failed against 6ad21be0. The final audit replaced six mistakenly named nonexistent `get_work` tool requests with real `read_messages` cases; those six old unknown-tool responses are not evidence about work IDs. The immutable baseline receipt still contains its original 21-pass/14-fail ID line; the final corpus has 35 real ID cases. REST work-show IDs are scalar path params; WS12 write IDs remain at the flagged seam.

Reader SELECTs (reader-pool capture; absolute counts differ from Rails, but stay flat):

| Endpoint | Before, 5 → 50 rows | After, 5 → 50 rows |
|---|---:|---:|
| REST owned work | 40 → 355 | 12 → 12 |
| MCP owned work | 41 → 356 | 13 → 13 |
| REST board filters | 40 → 310 | 14 → 14 |
| MCP board filters | 35 → 305 | 9 → 9 |
| MCP root history | 52 → 412 | 28 → 28 |
| MCP thread history | 113 → 1013 | 36 → 36 |
| REST thread context | 114 → 1014 | 38 → 38 |

The fixture uses a bot owner and a different human message creator, so its original thread counts differ from Astra's fixture. No exact Rails query-count equality is claimed.

## Failing-first baseline run (exit 101)

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-baseline/rust/Cargo.toml -p campfire pr192_ -- --nocapture --test-threads=8 > .scratch/pr192/logs/failing-first.log 2>&1
```

```text
WS11-api review reader: work_rest; size=5; returned=5; SELECTs=40
WS11-api review reader: board_rest; size=5; returned=5; SELECTs=40
WS11-api review reader: work_mcp; size=5; returned=5; SELECTs=41
WS11-api review reader: board_mcp; size=5; returned=5; SELECTs=35
WS11-api review reader: context_thread; size=5; returned=5; SELECTs=114
WS11-api review reader: work_rest; size=50; returned=50; SELECTs=355
WS11-api review reader: history_root; size=5; returned=5; SELECTs=52
WS11-api review reader: history_thread; size=5; returned=5; SELECTs=113
WS11-api review reader: board_rest; size=50; returned=50; SELECTs=310
WS11-api review reader: work_mcp; size=50; returned=50; SELECTs=356
test controllers::agent_review_tests::pr192_owned_work_query_count_is_flat ... FAILED
WS11-api review reader: board_mcp; size=50; returned=50; SELECTs=305
test controllers::agent_review_tests::pr192_board_filters_query_count_is_flat ... FAILED
WS11-api review reader: history_root; size=50; returned=50; SELECTs=412
test controllers::agent_review_tests::pr192_history_root_query_count_is_flat ... FAILED
WS11-api review reader: context_thread; size=50; returned=50; SELECTs=1014
test controllers::agent_review_tests::pr192_context_thread_query_count_is_flat ... FAILED
WS11-api review thread attachment: status=500; posted_rows=1; closed_at=None
test controllers::agent_review_tests::pr192_failed_thread_attachment_rolls_back_post_reopen_and_queue ... FAILED
WS11-api review reader: history_thread; size=50; returned=50; SELECTs=1013
test controllers::agent_review_tests::pr192_history_thread_query_count_is_flat ... FAILED
WS11-api review approval zone differential: 12 passed; 12 failed
test controllers::agent_review_tests::pr192_approval_deadlines_match_rails_zones_dates_and_dst ... FAILED
WS11-api review ID coercion differential: 21 passed; 14 failed
test controllers::agent_review_tests::pr192_context_and_reader_id_shapes_match_rails ... FAILED
test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 2174 filtered out; finished in 15.58s
```

## Complete versus flagged

The prior API surface, stateless MCP transport, all 38 tool names, shared throttling, selected error/Retry-After matrices and selected **31/35 REST / 33/38 MCP success paths** remain. The committed corpus now contains **1,491 Rails request/response pairs in 18 artifacts**: the original 1,432 are unchanged, plus 59 review vectors (24 deadlines and 35 real ID cases). Responses compare complete JSON bytes, status and the explicitly captured headers; this is selected coverage, not every possible input or header.

**Nine WS12 writes remain pending:** REST board create and work update/result/handoff; MCP `create_board_post`, `update_board_post`, `update_work`, `set_result`, `handoff_work`. Valid requests reach the flagged `agent_api_pending::execute` seam and return **generic REST 500 / MCP -32603**. They do not return a purpose-built unavailable response or successful domain result. Their authorization/validation paths remain implemented; the pending-path tests verify unchanged domain and queue tables. PR193's agent work services were not merged from its branch or wired here.

**Not only owner-blocked items remain.** The broader wave-4 task remains partial: exhaustive input/coercion/length/callback-precedence matrices for other endpoint/tool families, broader compact/partial date and zone grammar, remaining work/filter/cursor combinations, legacy boosts without Agent rows, multi-hop bot/legacy bounce and reply-source chains, root/thread replay and attachment callbacks, unshared purge execution, additional MIME/representation/viewer/cache cases, concurrent revocation versus writes/finalization, and remaining viewer/credential/account snapshot and delivery/finalization failure interactions. The four PR192 defects above are complete. Live delivery/network cases remain with their domain/integration owners. This turn recaptured Rails vectors but did not rerun the Rails controller-file runner or claim new controller-file pass counts.

## Final fresh clone and seeds

The final code and test corpus came from a clean Git clone at the verified commit. Only the previously built Cargo cache was moved into that clone; all local crates rebuilt using the clone's source paths. The default, first_run and agents_ui seeds were built there from the pinned image. Cargo used two jobs, the existing rustc throttle and at most eight test threads. One owned scratch target was used and removed after the final checks; logs, seeds and source clones were preserved. No Python model-server process or files were touched.

## Fresh clone and three seeds

```bash
git clone --no-hardlinks --no-local --branch rust/ws11api-rest-mcp . .scratch/pr192-final-fresh > .scratch/pr192/logs/final-clone.log 2>&1
PARITY_NAMESPACE=ws11api-pr192-final-fresh PARITY_IMAGE=ws11api-reference:d7c7de92 .scratch/pr192-final-fresh/rust/parity/bin/seed build default first_run agents_ui > .scratch/pr192/logs/final-seeds.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

## Full workspace tests from final fresh clone

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/pr192-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8 > .scratch/pr192/logs/final-workspace.log 2>&1
python3 .scratch/pr192-final-fresh/rust/reference-tools/agents/summarize-http-tests.py .scratch/pr192/logs/final-workspace.log > .scratch/pr192/logs/final-workspace-summary.log
```

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7m 04s
test result: FAILED. 2313 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 507.99s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.93s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.03s
test result: ok. 1253 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 142.06s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.53s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.16s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.79s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.89s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.99s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s
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
WS11-api cargo totals: 4293 passed; 2 failed; 14 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```

The native workspace command exits 101 with exactly two known media-version failures: `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers` and storage `pipeline_matches_the_reference`. Both pass using the same final test binaries in the pinned runtime below. This does not claim the whole workspace was run in Docker. No test was weakened or newly ignored. Main's two ignored WS14g polling HTTP comparisons were explicitly run and pass separately.

## Final compiled agent/bot controller groups and PR192 receipts

```bash
python3 .scratch/pr192/summarize-final-review.py
```

```text
WS11-api compiled agent/bot controller groups: 87 passed; 0 failed; 0 ignored
WS11-api PR192 regressions: 8 passed; 0 failed; 0 ignored
WS11-api review reader: board_rest; size=5; returned=5; SELECTs=14
WS11-api review reader: board_mcp; size=5; returned=5; SELECTs=9
WS11-api review reader: board_rest; size=50; returned=50; SELECTs=14
WS11-api review reader: board_mcp; size=50; returned=50; SELECTs=9
WS11-api review approval zone differential: 24 passed; 0 failed
WS11-api review reader: context_thread; size=5; returned=5; SELECTs=38
WS11-api review reader: context_thread; size=50; returned=50; SELECTs=38
WS11-api review thread attachment: status=500; posted_rows=0; closed_at=Some("2026-03-01 16:00:00")
WS11-api review reader: history_root; size=5; returned=5; SELECTs=28
WS11-api review reader: history_root; size=50; returned=50; SELECTs=28
WS11-api review reader: history_thread; size=5; returned=5; SELECTs=36
WS11-api read wire case WS11-api review reader: history_thread; size=50; returned=50; SELECTs=36
WS11-api review reader: work_rest; size=5; returned=5; SELECTs=12
WS11-api review reader: work_mcp; size=5; returned=5; SELECTs=13
WS11-api review reader: work_rest; size=50; returned=50; SELECTs=12
WS11-api review reader: work_mcp; size=50; returned=50; SELECTs=13
WS11-api review ID coercion differential: 35 passed; 0 failed
```

## Strict clippy (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/pr192-final-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings > .scratch/pr192/logs/final-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 59.95s
```

## Release-input-only binary build (exit 0)

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-tmp" CARGO_TARGET_DIR="$PWD/.scratch/pr192-final-fresh/rust/target" mise exec rust@1.98.1 -- bash .scratch/pr192-final-fresh/rust/ci/with-release-inputs.sh cargo build --locked --workspace --bins -j2 > .scratch/pr192/logs/final-release-inputs.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 21s
```

## Final fresh Rails vector recapture and source pin (all exit 0)

```bash
PARITY_NAMESPACE=ws11api-pr192-final-oracles RUST_TEST_THREADS=8 python3 .scratch/pr192-final-fresh/rust/reference-tools/agents/record-http-vectors.py .scratch/pr192/final-oracles > .scratch/pr192/logs/final-record-oracles.log 2>&1
python3 .scratch/pr192-final-fresh/rust/reference-tools/agents/verify-http-vectors.py .scratch/pr192/final-oracles > .scratch/pr192/logs/final-vectors.log 2>&1
python3 .scratch/pr192-final-fresh/rust/reference-tools/agents/check-http-reference.py > .scratch/pr192/logs/final-source-pin.log 2>&1
python3 .scratch/pr192-final-fresh/rust/reference-tools/agents/test-http-vector-verifier.py > .scratch/pr192/logs/final-verifier.log 2>&1
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
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
WS11-api reference sources: 84 pinned files matched; 0 image or checkout mismatches (d7c7de92)
.......
----------------------------------------------------------------------
Ran 7 tests in 0.002s

OK
```

## WS14g ignored polling comparisons, explicitly verified

```bash
CI=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/pr192-tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949 .scratch/pr192-final-fresh/rust/target/debug/deps/campfire-4cda423f68651c03 ws14g_agent_polling_http_ --ignored --nocapture --test-threads=8 > .scratch/pr192/logs/final-ws14g-polling.log 2>&1
```

```text
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_an_empty_drive_array ... ok
test integrations::agent_jobs::drive_attachment_cases::ws14g_agent_polling_http_carries_drive_file_ids_and_urls_only ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2317 filtered out; finished in 0.88s
```

## Native versus pinned media versions

```bash
vips --version > .scratch/pr192/logs/native-vips.log 2>&1
ffmpeg -version > .scratch/pr192/logs/native-ffmpeg.log 2>&1
docker run --rm --name ws11api-pr192-vips-version --network none --cpus 2 --entrypoint /usr/local/bin/bundle ws11api-reference:d7c7de92 exec ruby -rvips -e 'puts "vips-#{Vips.version_string}"' > .scratch/pr192/logs/pinned-vips.log 2>&1
docker run --rm --name ws11api-pr192-ffmpeg-version --network none --cpus 2 --entrypoint /usr/bin/ffmpeg ws11api-reference:d7c7de92 -version > .scratch/pr192/logs/pinned-ffmpeg.log 2>&1
```

```text
vips-8.18.6
ffmpeg version n9.0.2 Copyright (c) 2000-2026 the FFmpeg developers
vips-8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

## Pinned account logo (exit 0)

```bash
docker run --rm --name ws11api-pr192-final-campfire --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/pr192-tmp" -v "$PWD/.scratch/pr192-final-fresh:$PWD/.scratch/pr192-final-fresh:ro" -v "$PWD/.scratch/pr192-tmp:$PWD/.scratch/pr192-tmp" --entrypoint "$PWD/.scratch/pr192-final-fresh/rust/target/debug/deps/campfire-4cda423f68651c03" ws11api-reference:d7c7de92 controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers --exact --nocapture --test-threads=8 > .scratch/pr192/logs/final-pinned-logo.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2318 filtered out; finished in 0.91s
```

## Pinned storage vectors (exit 0)

```bash
docker run --rm --name ws11api-pr192-final-vectors --network none --cpus 2 --user "$(id -u):$(id -g)" -e CI=1 -e RUST_TEST_THREADS=8 -e CABLE_TEST_PORT_RANGE=52900-52919 -e INTEGRATION_TEST_PORT_RANGE=52920-52949 -e MAIL_TEST_PORT_RANGE=52920-52949 -e TMPDIR="$PWD/.scratch/pr192-tmp" -v "$PWD/.scratch/pr192-final-fresh:$PWD/.scratch/pr192-final-fresh:ro" -v "$PWD/.scratch/pr192-tmp:$PWD/.scratch/pr192-tmp" --entrypoint "$PWD/.scratch/pr192-final-fresh/rust/target/debug/deps/vectors-dcbe04fdcab19241" ws11api-reference:d7c7de92 --nocapture --test-threads=8 > .scratch/pr192/logs/final-pinned-storage.log 2>&1
```

```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.73s
```

## Owned scratch target cleanup

```bash
python3 .scratch/pr192/delete-owned-target.py > .scratch/pr192/logs/final-cleanup.log 2>&1
```

```text
WS11-api cleanup: removed owned scratch target .scratch/pr192-final-fresh/rust/target (32543736470 bytes); 0 owned target directories remain
```

Push verification: the report-only commit was pushed to `origin/rust/ws11api-rest-mcp`; its SHA is the final reply. The verified source/test commit above is unchanged by that report commit.
