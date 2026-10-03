# WS11 API next-2 report

Verified code checkpoint: `1c6b6bfbd61eb1317d79cc39c69900012768c474`; main merged through `21be05f47f3c0967f97eefcc35b057df4e2268c4` (#196, #201,
#204 and the lead's #205 merge). Work branch: `rust/ws11api-next-2`, originally
stacked on 45bffea35. No pushed history was rewritten and neither review branch
was modified by this worker. The report commit only adds documentation.

## Outcome

All three assigned parts are complete. Thread/cursor/reaction array candidates
are resolved in one scoped SQL lookup, all proxy headers are compared with only
the maintainer's explicit differences, and all 13 WS11-owned named comparisons
have pinned Rails evidence and negative controls. There is no missing WS12
service or unmerged WS11 dependency. Only 53 peer-owned named evidence cases
remain: 35 WS8 slash commands, 16 WS12 assignment/validation names and 2 WS11-ui
rendered broadcast names. Exact names and reasons remain in
`ws11api-remaining-scope.md` and `deferred-domain-cases.json`.

## Array reads

`agent_reading.rs` uses one JSON bind through SQLite json_each, including global
thread/message lookup and conversation-scoped cursors. Global first-match order
is primary-key order, matching pinned Rails' IN lookup; unauthorized threads do
not fall back to another candidate. Room-scoped lookups and permission-before-
validation order are preserved. The same helpers serve REST context and work
reads. Reaction IDs were the additional per-ID loop found in the sweep and are
now batched. Board room filters use a preloaded relation; work writer ID arrays
already use JSON-backed IN queries. ack_events deliberately retains Rails' own
per-event acknowledgment/policy loop with its existing 100-item bound. Drive ID
normalization has no database lookup per element.

| Lookup | Rust before, 5/50 | Rust after, 5/50 | Fresh Rails, 5/50 |
| --- | --- | --- | --- |
| MCP thread |31/76|26/26|29/29|
| MCP before |32/77|27/27|30/30|
| MCP after |32/77|27/27|30/30|
| MCP reaction |16/61|11/11|9/9|
| MCP context message |27/27|27/27|34/34|
| MCP context thread |26/26|26/26|27/27|
| REST context message |28/28|28/28|33/33|
| REST context thread |27/27|27/27|26/26|

These are SELECT counts after warmup. Rust reader counts use reader connections;
reactions include the writer. The fresh Rails recorder explicitly disables the
query cache. The supplied independent review reports Rails thread/before at
26/27 rather than this recorder's 29/30; both measurement windows are flat.
No claimed count is substituted from the review for the fresh measurement.
The 24 vectors include multiple valid candidates, conversation-only cursors,
global selection before membership, revoked grants before invalid limits and
inactive auth before missing rows. A 40,000-candidate control uses at most five
binds and exceeds SQLite's ordinary parameter ceiling without an IN expansion.

The new two-size test failed before batching on this stack plus its main merge:
```text
WS11-api array lookup: thread; missing=5; SELECTs=31
WS11-api array lookup: thread; missing=50; SELECTs=76
WS11-api array lookup: before; missing=5; SELECTs=32
WS11-api array lookup: before; missing=50; SELECTs=77
WS11-api array lookup: after; missing=5; SELECTs=32
WS11-api array lookup: after; missing=50; SELECTs=77
WS11-api array lookup: react; missing=5; SELECTs=16
WS11-api array lookup: react; missing=50; SELECTs=61
WS11-api array lookup: context_message; missing=5; SELECTs=27
WS11-api array lookup: context_message; missing=50; SELECTs=27
WS11-api array lookup: context_thread; missing=5; SELECTs=26
WS11-api array lookup: context_thread; missing=50; SELECTs=26
WS11-api array lookup: rest_context_message; missing=5; SELECTs=28
WS11-api array lookup: rest_context_message; missing=50; SELECTs=28
WS11-api array lookup: rest_context_thread; missing=5; SELECTs=27
WS11-api array lookup: rest_context_thread; missing=50; SELECTs=27
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2573 filtered out; finished in 1.47s
```

Final fresh counts:
```text
WS11-api array lookup: thread; missing=5; SELECTs=26
WS11-api array lookup: thread; missing=50; SELECTs=26
WS11-api array lookup: before; missing=5; SELECTs=27
WS11-api array lookup: before; missing=50; SELECTs=27
WS11-api array lookup: after; missing=5; SELECTs=27
WS11-api array lookup: after; missing=50; SELECTs=27
WS11-api array lookup: react; missing=5; SELECTs=11
WS11-api array lookup: react; missing=50; SELECTs=11
WS11-api array lookup: context_message; missing=5; SELECTs=27
WS11-api array lookup: context_message; missing=50; SELECTs=27
WS11-api array lookup: context_thread; missing=5; SELECTs=26
WS11-api array lookup: context_thread; missing=50; SELECTs=26
WS11-api array lookup: rest_context_message; missing=5; SELECTs=28
WS11-api array lookup: rest_context_message; missing=50; SELECTs=28
WS11-api array lookup: rest_context_thread; missing=5; SELECTs=27
WS11-api array lookup: rest_context_thread; missing=50; SELECTs=27
```

## Proxy oracle

Rails is entered through Rack::Builder.parse_file(config.ru), so Rack::Deflater
runs. SERVER_PROTOCOL is explicitly asserted as HTTP/1.1 and retained in every
receipt. 21 representation responses plus 4 blob controls (200/206/416/404) keep
all headers, each value, statuses, exact bodies and unchanged stored rows/jobs.
There is no fixed header projection. Case-insensitive duplicate names and
multiple values fail. Date, UUIDv4 request IDs and six-place runtimes must have
matching name presence and valid single values; only their values may vary.

Rust keeps the six approved security defaults, with exact values checked:
Permissions-Policy, Referrer-Policy, X-Content-Type-Options, X-Frame-Options,
X-Permitted-Cross-Domain-Policies and X-XSS-Protection. No other extra/missing
name or changed value is accepted. CSP is byte-identical: nonce-generator
entropy is fixed as a test input on both sides, with no output/header mask and
no change to production randomness. The former exact-header inventory claim
is corrected. ws11api-approved-differences.md records this approval alongside
the existing JPEG/video and committed-file-retention state differences.

The new all-header checker rejected the old nine-name projection before
recapture. The later dynamic-name negative control loads the previous comparator
from git and proves it accepted a missing/extra request header; the stricter
comparator rejects it. Full verifier tests also challenge ordinary values,
unexpected names, duplicate values, changed security values and wrong routes.
```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2569 filtered out; finished in 0.87s
```
```text
F
======================================================================
FAIL: test_per_request_name_presence_still_matches (__main__.FullHeaders.test_per_request_name_presence_still_matches)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "rust/reference-tools/agents/test-proxy-headers.py", line 21, in test_per_request_name_presence_still_matches
    with self.assertRaises(AssertionError):compare_headers(rust,rails,True)
         ~~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^
AssertionError: AssertionError not raised

----------------------------------------------------------------------
Ran 1 test in 0.000s

FAILED (failures=1)
```

Final verifier injection summaries:
```text
.......
----------------------------------------------------------------------
Ran 7 tests in 0.001s

OK
```
```text
..............
----------------------------------------------------------------------
Ran 14 tests in 0.017s

OK
```

## Thirteen newly closed comparisons

| Rails file | Case | Rust regression |
| --- | --- | --- |
| test/models/message_streaming_test.rb | stream start broadcasts the append without the unread broadcast | ws11_next2_stream_start_without_unreads |
| test/models/message_streaming_test.rb | finalize fires every side effect exactly once | ws11_next2_stream_finalize_every_side_effect_once |
| test/models/message_streaming_test.rb | appends re-render without firing side effects | ws11_next2_stream_appends_render_without_side_effects |
| test/models/message_streaming_test.rb | a coalesced update sends a trailing broadcast with the final text | ws11_next2_stream_trailing_final_text |
| test/models/user/bot_test.rb | create bot | ws11_next2_bot_factory_fixed_entropy |
| test/models/user/bot_test.rb | reset bot key | ws11_next2_bot_reset_fixed_entropy |
| test/models/user/bot_test.rb | deliver message by webhook | ws11_next2_bot_actual_queued_delivery |
| test/models/channel_thread_agent_assignment_test.rb | two agents assigning posts to each other stop at the hop limit | ws11_next2_assignment_agent_hops_stop_at_limit |
| test/models/channel_thread_agent_assignment_test.rb | deleting a thread without an agent owner emits nothing | ws11_next2_assignment_delete_no_agent_emits_nothing |
| test/models/channel_thread_agent_assignment_test.rb | a human assignment starts a new root at hop 0 | ws11_next2_assignment_human_starts_root_zero |
| test/models/channel_thread_agent_assignment_test.rb | deleting an agent-owned thread emits work_unassigned | ws11_next2_assignment_deleted_owner_snapshot_delivery |
| test/models/agent_budgets_test.rb | a stranger cannot read another agent's budget item | ws11_next2_budget_stranger_cannot_read_owner_item |
| test/models/agent_budgets_test.rb | an agent-created handoff counts toward no budget, even with caps exhausted | ws11_next2_budget_handoff_ignores_exhausted_caps |

Streaming uses actual callbacks, complete real WebSocket frames, both human
subscribers' unread broadcasts, index/activity/ledger projections and logical
queued arguments. Finalization is invoked twice and all effects remain single.
Trailing delivery uses the registered handler's production implementation and
a clock advance, without timing-threshold changes.

Factory/reset fixtures control the actual entropy generator and retain call
lengths, auth results, digests, role/status and stored-key secrecy. Queued bot
and deletion comparisons use actual durable claims and production handlers;
only the DNS/dial boundary is replaced, as in the Rails Net::HTTP fixture.
Requests keep exact bodies, timestamps and HMACs. Deletion retains the owned
work snapshot and assigned/deleted ledger statuses. Hop/root comparisons retain
history, every ledger projection and chain equality. Budget cases use merged
WS12 ActivityItem.accessible_to and agent_work::handoff_work unchanged. The
original handoff case sets all three caps but consumes the message cap; this
evidence does not pretend three separate usage buckets were exhausted.

No new production mismatch was found in these 13 cases. Each comparator was
challenged with an independently incorrect observed field (name/digest/hop,
foreign-viewer count, usage, HMAC/deletion snapshot or rendered frame). All 13
failed and the exact original oracle bytes were restored in finally:
```text
WS11 negative controls campfire_db: test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 1299 filtered out; finished in 1.33s
WS11 negative controls campfire: test result: FAILED. 1 passed; 6 failed; 0 ignored; 0 measured; 2573 filtered out; finished in 2.82s
WS11 remaining comparison negative controls: 13 expected failures; exact oracle bytes restored
```

Executed final named counts by Rails file:
```text
WS11 domain inventory: 26 pinned Rails files; 378 source cases; 0 source cases claimed as run
WS11 named comparisons: test/models/agent_test.rb: 39 passed; 0 failed; 2 deferred
WS11 named comparisons: test/services/slash_commands/dispatcher_test.rb: 5 passed; 0 failed; 35 deferred
WS11 named comparisons: test/jobs/agent/delivery_job_test.rb: 29 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/channel_thread_agent_assignment_test.rb: 13 passed; 0 failed; 16 deferred
WS11 named comparisons: test/lib/restricted_http/private_network_guard_test.rb: 28 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message_streaming_test.rb: 23 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_test.rb: 22 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_approval_test.rb: 20 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/event_webhook_job_test.rb: 17 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/user/bot_test.rb: 15 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent/delivery_recovery_test.rb: 13 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_grant_test.rb: 12 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_budgets_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_credential_test.rb: 11 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_event_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_step_test.rb: 10 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/webhook_agent_key_test.rb: 9 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_kill_switch_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_revocation_test.rb: 8 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_slash_command_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_working_presence_test.rb: 6 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agents/work_payload_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/services/bots/clear_plaintext_tokens_test.rb: 3 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/message/bot_webhook_fanout_test.rb: 2 passed; 0 failed; 0 deferred
WS11 named comparisons: test/jobs/agent/delivery_concurrency_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparisons: test/models/agent_backfill_test.rb: 1 passed; 0 failed; 0 deferred
WS11 named comparison totals: 325 passed; 0 failed; 53 deferred
```

## Verification from the fresh clone

The clone was created with no local hardlinks, all 13 local packages were cleaned
(including the patched vendored html5ever), and all three seeds were regenerated.
Registry dependency output alone was retained. The clone was fast-forwarded to
the final main merge and all gates rerun. CI=1 prevents genuine seed skips.
The vendored html5ever is excluded from tests/clippy as rust/AGENTS.md requires.
Machine-wide rustc throttling stayed configured; Cargo used two jobs and tests
used at most eight threads. No model server was touched and no git stash was used.

Environment and exact commands (from the worktree):
```sh
export CI=1 CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/.scratch/next-2-fresh/rust/target" CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8 TMPDIR="$PWD/.scratch/next-2/tmp" CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949 MAIL_TEST_PORT_RANGE=52920-52949
unset CAMPFIRE_REFERENCE
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path .scratch/next-2-fresh/rust/Cargo.toml
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path .scratch/next-2-fresh/rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --nocapture --test-threads=8
```
```text
WS11-api cargo totals: 4634 passed; 7 failed; 16 ignored; 59 result summaries
WS11-api seed skips: 0 actual; 1 intentional missing-seed unit notice
```
```text
test result: FAILED. 2604 passed; 6 failed; 7 ignored; 0 measured; 0 filtered out; finished in 557.31s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.48s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.08s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1302 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 146.42s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.22s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.04s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.31s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.42s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.99s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.90s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.55s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.38s
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

Native failures (media bytes/version; the same affected gates pass below in the pinned runtime):

- controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers
- controllers::agent_review_r3_tests::pr192_r3_fresh_video_retains_preview_and_variant_files
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_preview_redirect
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_proxy
- controllers::agent_review_r5_tests::pr192_r5_video_missing_variant_redirect
- pipeline_matches_the_reference
```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path .scratch/next-2-fresh/rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash .scratch/next-2-fresh/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --workspace --bins -j2
```
Strict clippy:
```text
Finished `dev` profile [unoptimized] target(s) in 1m 10s
```
Release-input guard build:
```text
Finished `dev` profile [unoptimized] target(s) in 1m 12s
```
```sh
bash .scratch/next-2/pinned-checks.sh
```
The preserved script executes the final fresh app/storage test executables
in ws11api-reference:d7c7de92, read-only source, writable owned TMPDIR, no network,
two CPUs and eight threads; it then explicitly runs the two ignored polling cases.
```text
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 2586 filtered out; finished in 23.49s
```
```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2616 filtered out; finished in 0.83s
```
```text
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.88s
```
```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2615 filtered out; finished in 0.74s
```
```sh
bash .scratch/next-2/fresh-oracles.sh
```
The preserved script recaptures all 24 API artifacts and the 13 new and 15 previous-round
named cases in fresh private seed copies, then runs source, exact vector and verifier checks.
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
WS11-api fresh work writes oracle: 244 request/response pairs; byte-identical committed vectors
WS11-api fresh attachments oracle: 64 request/response pairs; byte-identical committed vectors
WS11-api fresh permissions oracle: 63 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 zones and ID shapes oracle: 59 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved JPEG difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 approved video difference oracle: 1 request/response pairs; byte-identical committed vectors
WS11-api fresh PR192 handled missing representations oracle: 3 request/response pairs; byte-identical committed vectors
WS11-api fresh array lookups oracle: 24 request/response pairs; byte-identical committed vectors
WS11-api fresh blob proxy all headers oracle: 4 request/response pairs; byte-identical committed vectors
WS11-api all-header oracle: 25 responses; every header name/value/cardinality; config.ru HTTP/1.1; only 6 named security additions and 3 per-request names approved
WS11-api MCP dispatch: 38 explicit tool names; 29 throttled tools
WS11-api base MCP coverage: 84 asserted vectors; 0 deferred base vectors
WS11-api boundary matrix: 35 REST invalid credentials; 35 REST human sessions; 29 MCP tool overflows
```
```text
WS11-api new named sources: 4 pinned test files matched checkout; test sources are not shipped in the Rails image
WS11-api reference sources: 93 pinned files matched; 0 image or checkout mismatches (d7c7de92)
```
```text
WS11 fresh remaining named oracle: agents_next2_models.json: 7 cases; byte-identical committed vectors
WS11 fresh remaining named oracle: agents_next2_jobs.json: 2 cases; byte-identical committed vectors
WS11 fresh remaining named oracle: agents_stream_remaining.json: 4 cases; byte-identical committed vectors
WS11 previous-round named oracle: agents_next_named.json: 6 cases; byte-identical committed vectors
WS11 previous-round named oracle: agents_assignment_named.json: 9 cases; byte-identical committed vectors
WS11 fresh remaining named oracles: 13 cases; 0 byte differences
```

## Files, boundaries and remainder

Production edits are confined to the shared array lookup models and WS11 read/
reaction adapters. security.rs and db/sql.rs add scoped cfg(test)-only entropy
inputs. New regressions are in agent_array_read_tests.rs, agent_review_r5_tests.rs,
channels/sink/stream_remaining_cases.rs, integrations/agent_jobs/remaining_cases.rs
and db/tests/agent_next2_cases_test.rs. Reference recorders, vectors and the case
inventory carry all new evidence. No WS12 writer, grant, ledger, audit, handoff or
tag service was reimplemented. WS15 clients and account access remain unchanged.

Only peer-owned named evidence remains:

| Count | Owner | Remainder |
| --- | --- | --- |
|35|WS8|Built-in slash-command names in dispatcher_test.rb|
|16|WS12|Owner eligibility/viewer and work mutation/validation names in channel_thread_agent_assignment_test.rb|
|2|WS11-ui|Status badge/directory replacement and no-secrets rendered broadcast names in agent_test.rb|

These are evidence obligations, not genuinely unmerged service blockers. Exact
names and individual reasons are in ws11api-remaining-scope.md. WS12 also retains
remaining fixed service/render read costs; this branch fixes candidate growth
without changing those owned boundaries. WS15g/WS15e retain their clients.
No WS11-owned deferred comparison or pending work-write adapter remains.

The owned 23 GiB target was removed after all verification processes finished; logs, raw
receipts and disposable source are retained under .scratch/next-2 for review.
