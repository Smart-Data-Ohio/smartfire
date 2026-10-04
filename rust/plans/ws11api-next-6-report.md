# WS11 API next-6 checkpoint

Branch: `rust/ws11api-next-6`, stacked on next-5 `491b9425`.
Verified code snapshot: `886425ef0` after merging main `7e35a5fd6` with merge commit `f61de2c57`; the earlier main merge was `b493ad024`. No pushed history was rewritten.

## Complete and remaining

**31 newly closed broader API declarations; 51/51 total, 0 pending.** All 378 domain named comparisons in 26 pinned files remain mapped. The fresh clone gates below require their actual passes; the ledger alone is not execution evidence.

Fresh Rails produced 182 HTTP responses plus four real delivery/inbox outputs for the 31 declarations, and 24 HTTP responses for 18 two-size cases. REST/MCP body bytes, status and selected headers, complete committed work/history/ledger/handoff/audit facts and logical queue class/argument facts are compared. Human history and DOM projections check the original JSON fields, selectors, text and links, using real sessions/CSRF, a human HTTP post, an agent reply and result update. Webhook cases run the durable claim/post/finish path through a locally routed recording endpoint.

The only flagged integration in this scope belongs to **WS12, `rust/ws12-board-automations-4`**: `rust/crates/db/src/models/activity_item/recorder.rs:377` rejects AgentBudgetNotice without the explicit owning-domain reader; its contract is `recording_source.rs:47`. The typed WS11 reader and presenter consumers already exist. No WS12 board-automation files are edited. **Only this owner-blocked item remains; no unblocked WS11-API named declaration remains.**

| Pinned API file | Newly closed | Total closed | Pending |
| --- | ---: | ---: | ---: |
| agents/mcp_handoff_test.rb | 1 | 3 | 0 |
| agents/posts_controller_test.rb | 8 | 10 | 0 |
| agents/work_controller_test.rb | 8 | 22 | 0 |
| agents/work_delivery_test.rb | 9 | 9 | 0 |
| agents/work_handoff_test.rb | 4 | 6 | 0 |
| integration/agent_boards_test.rb | 1 | 1 | 0 |

## Changes and design

- `agent_next6_named_tests.rs`, `next6_named.rb`, the two input JSON files and two vectors: individually audited request/producer sequences with complete committed facts. The named positive-premise checker is wired through the existing CI named-case checker.
- `integrations/agent_jobs/next6_named.rs`: test-only external dial substitution, using the actual durable job runner. Unrelated agents' hooks are scheduled for later test steps; logical job payloads stay compared.
- `db/models/agent_event_polling.rs`: batch rooms, actors, threads and work payload associations. `agent_payloads.rs` exposes an event-scoped batch without reusing another occurrence's private allowance. `integrations/agent_repositories.rs` batches ledger/message/thread candidate discovery and private-link preflight by surface, preserving event/link order, current identity guards and Accounts-owned permission reads. Empty event batches retain their original zero candidate reads.
- `agent_work_writes_tests.rs`: validate every generated chain UUID and the shared handoff pair; independent model operations may have different chains. Only the message-rate declaration compares jobs as a full multiset, including duplicates and every class/argument fact. Rails' test adapter and the atomic Rust queue register push/delivery jobs in different order; this original declaration does not assert execution order. No job fact is dropped, and execution-order parity is not claimed.
- `ws11api-named-api-cases.json`, deferral inventory and remaining-scope plan: all 51 broader declarations now point to their own test/vector/recipe/receipt. The checker retains main's audited-inventory, one-to-one test/vector and selector/receipt validation, plus the new positive-premise check. Each fresh named-case vector now records its exact producer selector.

Shared DB polling/payload modules are touched to remove API read growth. No WS12 grants, ledger, audit, handoff/writer transaction rules or board-automation implementation are reimplemented. No WS15 account or network-access service is changed. Incoming main also supplies its reviewed persisted-second-owner assertion and the stronger evidence-checker regressions; both sides are retained.

## Failing-first evidence

The new two-size work-event regression failed against next-5's production code (`491b9425`), at its intended cost assertion. Raw excerpt from `.scratch/next-6/baseline-2.log`:

```text
WS11_NEXT6_READS cost_events_5 response=0 Rust=61 Rails=17
WS11_NEXT6_READS cost_events_5 response=1 Rust=62 Rails=19
WS11_NEXT6_READS cost_events_50 response=0 Rust=466 Rails=17
WS11_NEXT6_READS cost_events_50 response=1 Rust=467 Rails=19
assertion `left == right` failed: events: request reads must remain flat at 5/50 rows
  left: [61, 62]
 right: [466, 467]
test controllers::agent_next6_named_tests::ws11_next6_request_reads_stay_flat_at_two_sizes ... FAILED
```

The initial harness smoke also exposed three test-fixture/job-order/independent-chain assumptions; those were corrected without changing their production behavior and are not credited as application defects.

All 31 named declarations have a separate producer control; the extra cost control restores per-event reads. Temporary changes were installed only in the independent clone. Vectors and assertions were unchanged. Each receipt requires a producer activation, nonzero exit, its exact assertion label and `0 passed; 1 failed; 0 ignored`. All producer sources were restored byte-identically before final gates. `ws11api-next-6-controls.json` contains raw summaries and SHA-256 values for every log.

| Declaration key | Producer selector | Hits | Intended assertion |
| --- | --- | ---: | --- |
| `posts_order` | `next6_posts_order` | 1 | rejected |
| `posts_reply` | `next6_posts_reply` | 1 | rejected |
| `posts_status` | `next6_posts_status` | 1 | rejected |
| `posts_owner` | `next6_posts_owner` | 1 | rejected |
| `posts_cap` | `next6_posts_cap` | 1 | rejected |
| `posts_legacy` | `next6_posts_legacy` | 1 | rejected |
| `posts_left` | `next6_posts_left` | 1 | rejected |
| `shared_payload` | `next6_shared_payload` | 1 | rejected |
| `work_order` | `next6_work_order` | 1 | rejected |
| `work_empty` | `next6_work_empty` | 1 | rejected |
| `work_links_show` | `next6_work_links_show` | 1 | rejected |
| `work_links_list` | `next6_work_links_list` | 1 | rejected |
| `work_note_history` | `next6_work_note_history` | 1 | rejected |
| `work_orphaned` | `next6_work_orphaned` | 1 | rejected |
| `result_permissions` | `next6_result_permissions` | 2 | rejected |
| `result_precedence` | `next6_result_precedence` | 1 | rejected |
| `assignment_poll` | `next6_assignment_poll` | 1 | rejected |
| `assignment_links` | `next6_assignment_links` | 1 | rejected |
| `unassignment_poll` | `next6_unassignment_poll` | 1 | rejected |
| `assignment_ack` | `next6_assignment_ack` | 1 | rejected |
| `assignment_webhook` | `next6_assignment_webhook` | 1 | rejected |
| `assignment_no_read` | `next6_assignment_no_read` | 1 | rejected |
| `assignment_revoked` | `next6_assignment_revoked` | 6 | rejected |
| `assignment_left` | `next6_assignment_left` | 6 | rejected |
| `work_message_rate` | `next6_work_message_rate` | 42 | rejected |
| `handoff_poll` | `next6_handoff_poll` | 1 | rejected |
| `handoff_ack` | `next6_handoff_ack` | 1 | rejected |
| `handoff_webhook` | `next6_handoff_webhook` | 1 | rejected |
| `handoff_throttle` | `next6_handoff_throttle` | 61 | rejected |
| `handoff_shared_throttle` | `next6_handoff_shared_throttle` | 61 | rejected |
| `board_flow` | `next6_board_flow` | 1 | rejected |
| `polling_reads` | `next6_polling_reads` | 110 | rejected |

Raw control completion:

```text
WS11 named API producer controls: 32 activated; 32 rejected at intended assertions; 0 unsupported credits
WS12_COVERAGE_RESTORE 15 production inputs byte-identical
```

Exact producer-control commands executed in the private clone, on the final merged code:

```sh
python3 rust/reference-tools/agents/next6-named-controls.py install --scratch ../next-6/merged-controls
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire ws11_next6 -- --test-threads=8 --nocapture
python3 rust/reference-tools/agents/next6-named-controls.py run --scratch ../next-6/merged-controls --binary "$CARGO_TARGET_DIR/debug/deps/campfire-51e126f6c4acd111"
python3 rust/reference-tools/agents/next6-named-controls.py restore --scratch ../next-6/merged-controls
```

## Fresh Rails and read costs

Fresh pinned image `ws11api-reference:d7c7de92`, one generator at a time. Requests disable the AR query cache at the executor boundary and reject any cache hit. The rerun agrees in all asserted observations and committed facts. Unused parent-harness last-response fields (random CSRF/client IDs on human pages) are removed from the generated format; all actually asserted responses remain in observations. No compared response field is masked.

Commands executed from the worktree (with the environment below and `PARITY_SEED_DIR=$PWD/.scratch/next-6-fresh/rust/parity/.seed`):

```sh
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/next6_named.rb
bash rust/parity/bin/reference runner --seed default rust/reference-tools/agents/next6_named.rb next6-query-inputs.json
```

```text
WS11 next6 Rails oracle: 31 cases; 182 HTTP responses; 4 producer outputs; 0 query cache hits
WS11 next6 Rails oracle: 18 cases; 24 HTTP responses; 0 producer outputs; 0 query cache hits
```

| Measured interface | Rust SELECTs 5/50 | Fresh Rails SELECTs 5/50 |
| --- | ---: | ---: |
| REST board list | 25/25 | 10/10 |
| MCP board list | 20/20 | 12/12 |
| REST owned work | 16/16 | 20/110 |
| MCP owned work | 17/17 | 22/112 |
| REST work-event poll | 25/25 | 17/17 |
| MCP work-event poll | 26/26 | 19/19 |
| REST linked work show | 19/19 | 13/13 |
| REST create | 107/107 | 64/64 |
| REST update | 42/42 | 37/37 |
| REST result | 25/25 | 27/27 |
| REST handoff | 68/68 | 62/62 |
| REST acknowledgment | 8/8 | 7/7 |

Work-event polling before/after: REST **61/466 → 25/25**, MCP **62/467 → 26/26**. Every measured Rust path is flat; Rails' owned-work growth is preserved as actual evidence, not replaced with a constant. These simple-body fixtures differ from the older tagged work-write corpus; their fixed costs must not be compared as the same request shape.

Raw current two-size output:

```text
WS11_NEXT6_READS cost_posts_5 response=0 Rust=25 Rails=10
WS11_NEXT6_READS cost_posts_5 response=1 Rust=20 Rails=12
WS11_NEXT6_READS cost_posts_50 response=0 Rust=25 Rails=10
WS11_NEXT6_READS cost_posts_50 response=1 Rust=20 Rails=12
WS11_NEXT6_READS cost_work_5 response=0 Rust=16 Rails=20
WS11_NEXT6_READS cost_work_5 response=1 Rust=17 Rails=22
WS11_NEXT6_READS cost_work_50 response=0 Rust=16 Rails=110
WS11_NEXT6_READS cost_work_50 response=1 Rust=17 Rails=112
WS11_NEXT6_READS cost_events_5 response=0 Rust=25 Rails=17
WS11_NEXT6_READS cost_events_5 response=1 Rust=26 Rails=19
WS11_NEXT6_READS cost_events_50 response=0 Rust=25 Rails=17
WS11_NEXT6_READS cost_events_50 response=1 Rust=26 Rails=19
WS11_NEXT6_READS cost_show_5 response=0 Rust=19 Rails=13
WS11_NEXT6_READS cost_show_50 response=0 Rust=19 Rails=13
WS11_NEXT6_READS cost_create_5 response=0 Rust=107 Rails=64
WS11_NEXT6_READS cost_create_50 response=0 Rust=107 Rails=64
WS11_NEXT6_READS cost_update_5 response=0 Rust=42 Rails=37
WS11_NEXT6_READS cost_update_50 response=0 Rust=42 Rails=37
WS11_NEXT6_READS cost_result_5 response=0 Rust=25 Rails=27
WS11_NEXT6_READS cost_result_50 response=0 Rust=25 Rails=27
WS11_NEXT6_READS cost_handoff_5 response=0 Rust=68 Rails=62
WS11_NEXT6_READS cost_handoff_50 response=0 Rust=68 Rails=62
WS11_NEXT6_READS cost_ack_5 response=0 Rust=8 Rails=7
WS11_NEXT6_READS cost_ack_50 response=0 Rust=8 Rails=7
```

## Fresh-clone gates

An independent `git clone --quiet --no-hardlinks . .scratch/next-6-fresh` was created for this round, then fast-forwarded to the verified code snapshot after restoring its copied inputs and all controls. Fresh default/first_run/agents_ui seeds were built there. Only own scratch files are untracked. One target, two cargo jobs, eight test threads, unchanged rustc throttle.

```sh
export CI=1 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 RUST_TEST_THREADS=8
export CARGO_TARGET_DIR="$PWD/rust/target" TMPDIR="$PWD/.scratch/tmp"
export CABLE_TEST_PORT_RANGE=52900-52919 INTEGRATION_TEST_PORT_RANGE=52920-52949
export MAIL_TEST_PORT_RANGE=52920-52949 GITHUB_TEST_PORT_RANGE=52920-52949
export PARITY_OWNER=ws11api-next6 PARITY_NAMESPACE=ws11api-next6-fresh PARITY_IMAGE=ws11api-reference:d7c7de92
```

Commands executed from `.scratch/next-6-fresh`:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -j2 --bin campfire
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
python3 rust/reference-tools/agents/summarize-tests.py ../next-6/workspace.log
python3 rust/reference-tools/agents/named-case-pass-counts.py ../next-6/workspace.log
python3 rust/reference-tools/agents/check-named-api-cases.py ../next-6/workspace.log
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/check-http-reference.py
python3 rust/reference-tools/agents/test-case-ports.py
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire ws11_next6 -- --test-threads=8 --nocapture
```

The last focused command selected no media cases, so it ran natively. It executes all 31 new named declarations plus the two-size regression. The vendor `html5ever` crate is excluded by the established workspace gate; all application crates and doc tests run.

Raw workspace/vector/source summaries:

```text
WS11 workspace totals: 4859 passed; 0 failed; 15 ignored; 60 result summaries
WS11 missing-seed skips: 0
WS11 next6 original assertion premises: 31 declarations; real positive selections, mutations, delivery and HTML/JSON projections; 0 missing clauses
WS11 broader named API assertions: 51 passed; 0 pending; owner WS11-API (unblocked)
WS11-api new named sources: 4 pinned test files matched checkout; test sources are not shipped in the Rails image
WS11-api reference sources: 93 pinned files matched; 0 image or checkout mismatches (d7c7de92)
WS11 named comparison totals: 378 passed; 0 failed; 0 deferred
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 2744 filtered out; finished in 11.75s
```

Raw strict-clippy completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 15.04s
```

Raw release-input completion:

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 09s
```

Checker regression:

```text
...........WS11 next6 original assertion premises: 31 declarations; real positive selections, mutations, delivery and HTML/JSON projections; 0 missing clauses
...
----------------------------------------------------------------------
Ran 14 tests in 2.657s

OK
```

The existing media runner executes all six byte-sensitive app cases and storage vectors in the pinned runtime, retaining size/checksum assertions. Other tests run natively. No concurrency or timing thresholds are reduced. The Python model server is untouched.

Cleanup executed in the fresh clone:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml
test ! -e "$CARGO_TARGET_DIR"
```

```text
Removed 24632 files, 19.1GiB total
```

The target is absent; no test process remains. Logs, controls, seeds and the independent source clone are retained under the authorized scratch root. No question remains for WS11-API; the owner-blocked Recorder source integration above is the only flag in this scope.
