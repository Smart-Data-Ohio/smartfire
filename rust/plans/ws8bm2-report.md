# WS8b-m2 slice G checkpoint (partial)

Branch `rust/ws8bm2-message-features-g`, started from `699ea1c4a` (#216). No pushed history was rewritten. This checkpoint protects publication order, proves the real Calendar retry consumers, and extends exceptional input parity. It also records two owner boundaries without installing a workaround. No deployment or new browser result is claimed.

## Ordered publication versus receiver delivery

`cable::Hub::broadcast` has an inactive-by-default, `test-support`-feature observer. It records the real stream/payload under the existing sequence lock before subscription fanout. The scoped capture is weakly retained by the hub and enabled by TestApp subscriptions. Production builds do not record publications.

`comparison_support::published_frames` awaits the existing exact-envelope socket multiset first, then compares the unsorted actual publication batch with Rails. Receipt is the completion barrier: Hub records each publication before subscription fanout. All six changed modules use it: `older_embed_children_tests`, `older_embed_job_tests`, `mapped_provider_tests`, `older_provider_tests`, `older_owner_tests`, `older_calendar_tests`. No fixture value is used as the observed publication. The separate legal socket-reordering replay stays receiver-only.

Failing-first controls (`check_publication_mutants.py`): the reviewer's exact presenter-batch reversal passes the old receiver-only test while actual recorded order differs; it fails the new ordered assertion. A shared `message_batches` reversal is rejected by all six modules at `ordered publication differs from Rails`. Frame count, bytes, stream identity and multiplicity remain covered by the receiver comparator. No deadlines, retries, masks or allowlists changed.

The first fresh workspace run exposed a bug in my initial observer placement: three asynchronous job comparisons drained an empty buffer before jobs had published. Raw result: `test result: FAILED. 2719 passed; 3 failed; 5 ignored; 0 measured; 0 filtered out; finished in 918.21s`. This was an assertion lifecycle defect, not a product publication-order difference or a threshold flake. The added `ordered_capture_waits_for_actual_callback_publication_without_sleep` queues a real GitHub update on a current-thread runtime, so the callback cannot start before the comparator awaits. It deterministically fails with the early snapshot, passes after the receipt barrier, and `check_capture_completion.py` rejects restoring the early snapshot. The final full suite is from a second fresh clone. Strict clippy's first pass also found a needless struct default after all test entropy fields were specified; the fields are now applied after construction, with no production initialization change.

## Calendar retry and exhaustion

`calendar_retry_consumer_tests.rs`, `calendar_retry_consumers.rb` and its pinned vector execute actual registered `Calendar::InboundSyncJob` and `Calendar::SyncEntryJob` handlers through the durable runner. Thirty scenarios at 4/16 old references produce 156 real job executions: 429 recovery/exhaustion, open/read/write transport recovery, read-timeout exhaustion, and mixed transient failures ending in a permanent 503. The case label `mixed_recover` does not imply recovery after that 503: both apps stop there.

Each execution checks full persisted event, entry and attendance rows, actual HTTP method/path/body/credential, exact queue attempts and exception-group counters, persisted terminal error, ordered publications and wire silence. Test-only SQLite triggers observe committed ready/failed/deleted-success outcomes; no replacement job handler manufactures state. The next retry is explicitly made due, as Rails' test executes its deserialized retry. Production polynomial backoff and 15% jitter are unchanged; comparisons check its defined bounds, not a widened timing deadline.

`check_calendar_retry_mutants.py` rejects four actual producer faults at their intended assertions: omitted entry failure write (persisted row), grouped budget 8→7 (queue state), mutated PUT summary (owner exchange), extra actual root broadcast (ordered publication). These controls are documented beside the test.

| Calendar consumer / step | Rails reads 4 / 16 | Rust reads 4 / 16 |
| --- | --- | --- |
| Inbound initial transient | 7 / 7 | 6 / 6 |
| Inbound later transient or recovery | 3 / 3 | 6 / 6 |
| Upsert initial transient | 7 / 7 | 9 / 9 |
| Upsert later transient | 8 / 8 | 9 / 9 |
| Upsert recovery | 8 / 8 | 8 / 8 |
| Remove initial transient | 5 / 5 | 8 / 8 |
| Remove later transient | 0 / 0 | 8 / 8 |
| Remove recovery | 0 / 0 | 6 / 6 |

Counts include consumer physical SELECT/WITH reads; queue scheduling and the observation table are excluded on Rust, and Rails cached queries are excluded. Rails' warm association/query caches account for later-step decreases. Both sizes have identical counts at each corresponding step. No read count is replaced with a fixture count.

## Relative, split and container inputs

`relative_split_input_tests.rs` compares 184 actual inputs × parse/leading/trailing = 552 outcomes in UTC, New York, Lord Howe and Apia. It covers huge positive minute/hour/day/week durations, wide years, legacy regex boundaries and precedence, leap-second input, invalid clocks, NUL/NBSP/newline and Array/Hash/scalar coercion. Exact exceptional class/message values are compared. The pre-fix implementation panicked constructing `Span.days(2147483648)`; the new bounded Gregorian-cycle arithmetic retains Integer precision through the existing wide Timestamp representation.

No second Date parser was built. Absolute grammar and wide rendering use WS11's merged shared `rails_compat::date_parse`/`datetime` modules. Existing DST resolution and transition-boundary helpers resolve civil durations in the real target period. Existing normal-range scheduled JSON rendering, including #179's viewer zone, is retained.

`container_input_tests.rs` checks 84 root/nested container HTTP requests plus 56 unblocked relative-consumer requests at 4/16 references: actual status, Content-Type, body, Location, full persisted messages/rich-text/saved/scheduled tables, ordered publication and receiver bytes. Independent fixed UUID entropy is installed before boot via the database's existing fixture provider, parallel-app scoped; Rails uses its real `Random.uuid` callback. No expected UUID is copied into observed output.

Runtime fixes are limited to wide relative arithmetic, slash notice/event URL rendering and millisecond JSON output for wide stored times. Producer controls in `check_input_mutants.py` reject: minute multiplier 60→61, altered trailing title, real slash HTTP error text, saved-item default status, actual JSON timestamp, append→replace publication, and wide notice text, each at the intended parser/HTTP/row/publication assertion.

| HTTP consumer | Rails reads 4 / 16 | Rust reads 4 / 16 |
| --- | --- | --- |
| Saved root containers | 3 / 3 | 3 / 3 |
| Scheduled root containers | 5 / 5 | 5 / 5 |
| Slash root containers | 5 / 5 | 7 / 7 |
| Valid nested save / wide reminder | 6 / 6 | 13 / 13 |
| Wide scheduled create | 5 / 5 | 13 / 13 |
| Invalid reminder clock | 5 / 5 | 5 / 5 |
| Invalid scheduled clock | 5 / 5 | 6 / 6 |
| Slash reminder with a new message (UTC) | 26 / 26 | 109 / 109 |
| Slash event prefill | 5 / 5 | 10 / 10 |

The fixed slash cost exceeds Rails, but does not grow with old-reference count. CSRF preflight and state-observation queries are outside the production request count. No board/work files or inherited reader internals were changed.

## Flagged adapter boundary

`adapter_rejections.rb` pins 24 real Generic/LinkedIn after-commit callback permutations: adapter false return, `ActiveJob::EnqueueError`, and hard exception at sibling 1/2, both sizes. It records exact rows, actual adapter calls/queued jobs, exception and ordered frames. Rails soft refusals commit both claims, queue the other sibling and broadcast; hard exceptions retain the already committed metadata/claims and publish none.

Rust's durable queue has no equivalent soft-refusal adapter API. A durable job INSERT error must roll back the triggering metadata, claims and jobs under decisions.md decision 2. `probe_adapter_rejections.py` executes eight real first/second durable failures, compares complete actual rollback rows to the before-state and observes the real empty publication/queue. It explicitly reports the difference from Rails rather than claiming parity. Suppressing the real enqueue error is rejected at `adapter actual durable refusal`.

Blocking owners: **WS15e / WS3, with lead contract decision**. The boundary is `integrations/link_embed/store.rs:120` (final-state sibling claims before commit), `store.rs:214` (fetch emission), `jobs.rs:306`/`312` (transactional EventSink persistence and propagated queue error), `crates/jobs/src/queue.rs:47` (`Result<i64>` enqueue). Moving enqueue after commit or swallowing durable errors would violate the existing decision and is not implemented.

| Refusal / both providers | Rails hard-error reads 4 / 16 | Rust durable-error reads 4 / 16 |
| --- | --- | --- |
| First sibling | 7 / 7 | 15 / 15 |
| Second sibling | 7 / 7 | 14 / 14 |

The Rails soft-refusal callback reads 14 / 26 and emits 4 / 16 frames. These are different contracts; the flat Rust diagnostic counts are not substituted for the missing soft-adapter comparison.

## Remaining scope and owners

1. **WS8b-m shared message renderer:** 24 non-UTC slash-post cases in the full 80-request `relative_consumers.json` corpus remain blocked. `views/src/messages.rs:441` explicitly passes `Zone::utc()` to the day/permalink timestamp; `controllers/presenters.rs:432` builds `user_view` with the UTC avatar helper (`presenters.rs:93`). The detached context's viewer zone cannot affect those bytes. `probe_relative_renderer.py` replays the full corpus and retains the real first New York failure: UTC `16:00:00Z` / avatar `v=20260302160000` versus Rails `11:00:00-05:00` / `v=20260302110000`. The normal test explicitly selects the 56 unblocked cases; the full 80-case oracle is independently replayed, no bytes masked. The ineffective local timezone-guard attempt was removed rather than presented as a fix.
2. **WS15e / WS3 / lead:** after-commit soft adapter refusal contract described above. Real atomic durable failures are proved; Rails adapter byte/state parity is not claimed.
3. **WS8b-m2:** wide-year saved/scheduled HTML presentation remains unproven and still narrows through Jiff (`saved_items.rs:84`, `scheduled_messages.rs:133` and `:244`). This checkpoint proves wide JSON creation/output and persisted rows, not those HTML lists/forms. Periodic delivery of these newly covered wide dates is not replayed (`models/saved_item.rs:286`, `models/scheduled_message.rs:261`). Future extreme-duration inputs beyond the existing Timestamp representation are also not claimed exhaustively covered.
4. **Lead/system-test phase:** final both-app cutover browser run. Prior accepted 45/45 inventory retained below; no browsers or pixel checks run this checkpoint.
5. **WS12:** broader board/work parity remains with its owner. No changes to `presenters/boards.rs`, `channel_thread/board.rs`, `channel_thread/work.rs`, `board_posts.rs` or `work_threads.rs`.

Calendar retry/exhaustion consumer proof is complete. Physical push, agent invocation/auth, typed AgentBudgetNotice and huddles use the merged owner integrations. This is a coherent **partial, PR-ready checkpoint**, not an owner-blocked-only declaration.

## Changed files

| Files | Change |
| --- | --- |
| `cable/Cargo.toml`, `cable/src/pubsub.rs`, `cable/src/server.rs`; `campfire/Cargo.toml` | Test-only weak publication capture under the hub's sequence lock; dev feature wiring. |
| `presenters/test_support.rs`, `quote_integration_tests.rs`, `comparison_support.rs`, six older-provider modules | Scoped capture, completed unsorted producer assertions, exact receiver multisets and deterministic callback regression. |
| `calendar_retry_consumer_tests.rs`, `calendar_retry_consumers.rb` and vector | Real registered durable retry executions, full domain rows and actual queue/owner/publication observations. |
| `db/src/slash_commands/time_parser.rs`, `db/src/slash_commands.rs` | Wide relative arithmetic and shared-renderer notice/event URL output. |
| `controllers/message_features.rs`, `controllers/scheduled_messages.rs` | Millisecond wide JSON output; retain the normal #179 viewer-zone path. |
| `app.rs`, `campfire/src/test_support.rs` | Per-app test entropy copied into the existing database fixture provider. |
| `relative_split_input_tests.rs`, `container_input_tests.rs`; three corresponding Ruby generators/vectors | Parser/split, actual HTTP envelopes, full feature rows and ordered broadcasts; explicitly scoped owner-blocked cases. |
| `adapter_rejections.rb`, vector and `probe_adapter_rejections.py` | Rails soft/hard adapter behavior and the real Rust atomic boundary, marked diagnostic rather than parity. |
| Four `check_*` tools, `probe_relative_renderer.py`, `verify_oracles.py` | Restored producer fault controls, owner-blocked reproduction and 51 independent oracle replays. |

## Validation receipts

All native builds use the existing CI toolchain image, two build jobs and unchanged machine-wide rustc throttle. Test execution uses four threads, with the capacity guard checked after compilation. Existing compiler artifacts are reused; no test data or generated outputs are copied into the fresh clones. `CI=1` makes missing seeds fatal.

Verified code head: `c1cae80595d77fed13f2a71279f16be3e138bee6`, including merge `dec01a575` of main `f85fb4200`. The final report commit changes documentation only. The 51-oracle replay used the first fresh clone: generators, vectors and production behavior are unchanged by the subsequent observer-lifecycle and fixture-initialization fixes.

Commands run from a fresh clone (`git clone --no-hardlinks --single-branch --branch rust/ws8bm2-message-features-g <this-worktree> .scratch/ws8bm2-g/verified-fresh`):

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 PARITY_REFERENCE_REVISION=d7c7de9264c63015be398001d7a1094e7695a6db bash rust/parity/bin/seed build default first_run agents_ui
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 --no-run
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
cd rust
bash ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

Raw seed / metadata lines:

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
cargo metadata --locked: exit 0
```

Raw workspace summary lines (all 61 Cargo summary groups, including doctests):

```text
test result: ok. 2723 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 896.40s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.21s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1357 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 124.03s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.36s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.37s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.58s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.17s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.76s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.54s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
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
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.63s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
workspace test exit: 0
WS8bm2 fresh workspace aggregate: 4818 passed; 0 failed; 14 ignored; 0 measured; 0 filtered out; 61 Cargo summary groups
```

Fourteen existing ignored tests are disclosed below; none were added. There are no seeded integration skips. The single “skipping locally” diagnostic comes from the deliberate `missing_seed_may_skip_locally` unit test; `missing_seed_fails_in_ci` also passed its expected panic.

```text
test channels::tests::golden::record_reference ... ignored, needs a running reference app; see the module docs
test controllers::internal_huddle_tests::huddle_gateway_own_node_suite_against_rust_endpoints ... ignored, requires Node and the gateway pinned ws package; run explicitly with --ignored
test controllers::rooms::system_browser_tests::huddle_system_cases_in_real_browser ... ignored, requires Docker and the pinned Playwright image; run parity/system/ws13
test controllers::rooms::system_browser_tests::livekit_stage_system_cases_in_real_browser ... ignored, requires the project-local LiveKit server; run parity/system/ws13-livekit
test jobs::tests::push_latency ... ignored, a measurement, not a test
test record_reference ... ignored, needs a running reference app; see the module docs
test tests::differential_test::scenario_matches_ruby ... ignored, needs CAMPFIRE_RUBY_SCENARIO_DB, the reference app's database after scenario.rb
test tests::fixtures_test::export_database_for_rails ... ignored, writes a database to CAMPFIRE_EXPORT_DB for the Rails rollback check
test tests::fixtures_test::fixtures_match_ruby_row_for_row ... ignored, needs CAMPFIRE_RUBY_FIXTURES_DB, a database the reference app filled with `db:fixtures:load` at CAMPFIRE_FIXTURES_NOW
test tests::two_factor_rollback_test::read_rails_rollback_changes ... ignored, requires Rails to mutate the rollback fixture; run reference-tools/auth/rollback.sh
test acme_tls_alpn_certificate_cached_and_reused ... ignored, requires a local Pebble ACME CA, PEBBLE_MINICA root certificate and TLS ports 5001/5002
test export_for_rails ... ignored, exports a database for the Rails rollback check; requires CAMPFIRE_MAIL_EXPORT_DIR
test crates/kit/src/error.rs - error::halt (line 91) ... ignored
test crates/kit/src/lib.rs - (line 7) ... ignored
```

Raw strict clippy and release-input completion:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.21s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.53s
```

Oracles and source controls, from the first independently rebuilt clone:

```sh
python3 rust/reference-tools/messaging/features-reference-check.py --self-test
WS8BM2_ORACLE_SCRATCH=<owned-scratch>/fresh-oracles python3 rust/reference-tools/messaging/verify_oracles.py
```

```text
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 oracle replay: 51/51 independently replayed fixtures byte-identical
```

Five new oracles were independently regenerated a second time before the full replay, each matching the pinned bytes: `calendar_retry_consumers`, `relative_split_inputs`, `container_inputs`, `relative_consumers`, `adapter_rejections`.

Mutation/differential commands (configured Cargo runner supplies the same toolchain, throttle, compile-before-capacity check and four test threads):

```sh
python3 rust/reference-tools/messaging/check_publication_mutants.py --expect escape --producer presenter -- <cargo-runner>
python3 rust/reference-tools/messaging/check_publication_mutants.py --expect reject --producer presenter -- <cargo-runner>
python3 rust/reference-tools/messaging/check_publication_mutants.py --expect reject --producer batch -- <cargo-runner>
python3 rust/reference-tools/messaging/check_calendar_retry_mutants.py -- <cargo-runner>
python3 rust/reference-tools/messaging/check_input_mutants.py -- <cargo-runner>
python3 rust/reference-tools/messaging/probe_adapter_rejections.py --output <owned-scratch>/adapter-runtime.json -- <cargo-runner>
python3 rust/reference-tools/messaging/probe_adapter_rejections.py --control --output <owned-scratch>/adapter-control.json -- <cargo-runner>
python3 rust/reference-tools/messaging/probe_relative_renderer.py -- <cargo-runner>
python3 rust/reference-tools/messaging/check_capture_completion.py -- <cargo-runner>
```

Raw before/after and intended negative-control summary lines (failures below are deliberate):

```text
order-before.log:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2710 filtered out; finished in 10.07s
WS8bm2 publication mutant presenter/older_embed_children_tests::queued_stale: escape confirmed
capture-before-receive.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.45s
capture-after-receive.log:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.78s
capture-completion-mutant.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.91s
WS8bm2 capture completion mutant: rejected deterministically before actual callback publication
order-presenter-final.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.89s
WS8bm2 publication mutant presenter/older_embed_children_tests::queued_stale: reject confirmed
order-six-final.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.89s
WS8bm2 publication mutant batch/older_embed_children_tests::queued_stale: reject confirmed
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.83s
WS8bm2 publication mutant batch/older_embed_job_tests: reject confirmed
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2726 filtered out; finished in 0.89s
WS8bm2 publication mutant batch/mapped_provider_tests: reject confirmed
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2726 filtered out; finished in 0.84s
WS8bm2 publication mutant batch/older_provider_tests: reject confirmed
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2726 filtered out; finished in 1.07s
WS8bm2 publication mutant batch/older_owner_tests: reject confirmed
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 2724 filtered out; finished in 1.09s
WS8bm2 publication mutant batch/older_calendar_tests: reject confirmed
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2727 filtered out; finished in 0.49s
WS8bm2 publication mutant batch/comparison_support::ordered_capture_waits_for_actual_callback: reject confirmed
calendar-first.log:
WS8bm2 Calendar retry Rust: 156 real registered job executions; exact queue counters, owner exchanges, complete rows and broadcasts; flat reads
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2713 filtered out; finished in 50.59s
calendar-mutants.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2716 filtered out; finished in 8.79s
WS8bm2 Calendar producer mutant 1: rejected at Calendar persisted entry.last_error
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2716 filtered out; finished in 3.06s
WS8bm2 Calendar producer mutant 2: rejected at Calendar queue state
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2716 filtered out; finished in 8.76s
WS8bm2 Calendar producer mutant 3: rejected at Calendar owner exchanges
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2716 filtered out; finished in 0.46s
WS8bm2 Calendar producer mutant 4: rejected at ordered publication differs from Rails: Calendar retry publications
relative-first.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2713 filtered out; finished in 0.03s
consumer-first.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2714 filtered out; finished in 1.13s
input-final.log:
WS8bm2 relative/split Rust: 552 matched; 0 differed
WS8bm2 container Rust: 56 requests; 0 envelope differences; complete persisted tables, ordered publications and flat reads
WS8bm2 container Rust: 84 requests; 0 envelope differences; complete persisted tables, ordered publications and flat reads
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2709 filtered out; finished in 66.39s
input-mutants.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 0.07s
WS8bm2 input producer mutant 1: rejected at relative/split actual output differs from Rails
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 0.07s
WS8bm2 input producer mutant 2: rejected at relative/split actual output differs from Rails
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 54.71s
WS8bm2 input producer mutant 3: rejected at container actual HTTP envelope differs from Rails
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 16.44s
WS8bm2 input producer mutant 4: rejected at container persisted rows.status
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 43.42s
WS8bm2 input producer mutant 5: rejected at container actual HTTP envelope differs from Rails
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 1.12s
WS8bm2 input producer mutant 6: rejected at ordered publication differs from Rails: container actual publications
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2718 filtered out; finished in 38.15s
WS8bm2 input producer mutant 8: rejected at container actual HTTP envelope differs from Rails
adapter-runtime-summary.log:
WS8bm2 adapter owner probe: 8/8 atomic refusal executions; complete actual rows and empty publications; flat reads; Rails adapter parity remains owner-blocked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2715 filtered out; finished in 5.21s
adapter-control-summary.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2715 filtered out; finished in 0.52s
WS8bm2 adapter enqueue-error producer mutant: rejected at adapter actual durable refusal
relative-renderer-probe.log:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2715 filtered out; finished in 7.84s
WS8bm2 relative renderer: owner-blocked UTC timestamp/avatar mismatch reproduced against real Rails frames; no parity credit
```

Logs and complete observed adapter JSON remain under `.scratch/ws8bm2-g/`. Both fresh-clone `rust/target` directories contained only four generated security-output JSON files (40 KB each); those files and directories were deleted after the tests. The pre-existing owned compiler cache remains. No model-server operation, stash, deployment or new browser harness was used. No owned test/generator/server is left running after verification.


## Retained named Rails ports and browser inventory

All 35 named slash comparisons and 140 named controller behaviors remain inventoried, and their Rust ports run in the final workspace suite. This is not a claim that all Rails test files ran as suites; evidence is the independently replayed Rails vectors and named Rust tests. Existing earlier command mutations are historical evidence, not claimed as rerun this round.

| Rails name | Rust test name |
| --- | --- |
| registry holds every shipped command with metadata | `builtin_registry_holds_every_shipped_command_with_metadata` |
| command_text? matches slash commands but not escapes or play passthrough | `builtin_command_text_matches_slash_commands_but_not_escapes_or_play_passthrough` |
| huddle starts a call when configured | `builtin_huddle_starts_a_call_when_configured` |
| huddle errors when unconfigured | `builtin_huddle_errors_when_unconfigured` |
| event opens the prefilled form url | `builtin_event_opens_the_prefilled_form_url` |
| event without a time prefills the title only | `builtin_event_without_a_time_prefills_the_title_only` |
| event rejects past times | `builtin_event_rejects_past_times` |
| bare event opens the blank form | `builtin_bare_event_opens_the_blank_form` |
| poll opens the builder in channels but not threads | `builtin_poll_opens_the_builder_in_channels_but_not_threads` |
| remind posts and saves with a reminder | `builtin_remind_posts_and_saves_with_a_reminder` |
| remind rejects unusable input without posting | `builtin_remind_rejects_unusable_input_without_posting` |
| status sets emoji and text until end of day | `builtin_status_sets_emoji_and_text_until_end_of_day` |
| status rejects blank arguments | `builtin_status_rejects_blank_arguments` |
| dnd toggles, takes durations, and turns off | `builtin_dnd_toggles_takes_durations_and_turns_off` |
| dnd rejects garbage durations | `builtin_dnd_rejects_garbage_durations` |
| ooo sets an end with a note, and off clears it | `builtin_ooo_sets_an_end_with_a_note_and_off_clears_it` |
| ooo takes week durations, dates, and datetimes | `builtin_ooo_takes_week_durations_dates_and_datetimes` |
| ooo bare tomorrow and weekdays run to the end of the day | `builtin_ooo_bare_tomorrow_and_weekdays_run_to_the_end_of_the_day` |
| ooo bare dates run to the end of the day | `builtin_ooo_bare_dates_run_to_the_end_of_the_day` |
| ooo bare month dates roll to next year when this year's passed | `builtin_ooo_bare_month_dates_roll_to_next_year_when_this_year_s_passed` |
| ooo day durations stay exact | `builtin_ooo_day_durations_stay_exact` |
| ooo broadcasts the badge and the notice | `builtin_ooo_broadcasts_the_badge_and_the_notice` |
| ooo off while calendar OOO covers says the calendar still shows it | `builtin_ooo_off_while_calendar_ooo_covers_says_the_calendar_still_shows_it` |
| ooo rejects blank arguments, garbage, past times, and long notes | `builtin_ooo_rejects_blank_arguments_garbage_past_times_and_long_notes` |
| shrug posts with the shrug | `builtin_shrug_posts_with_the_shrug` |
| posting commands in a board answer an error without posting | `builtin_posting_commands_in_a_board_answer_an_error_without_posting` |
| posting commands in a board thread still post | `builtin_posting_commands_in_a_board_thread_still_post` |
| slash posts in threads skip the legacy webhook fanout | `builtin_slash_posts_in_threads_skip_the_legacy_webhook_fanout` |
| slash posts in channels fan out to legacy webhooks | `builtin_slash_posts_in_channels_fan_out_to_legacy_webhooks` |
| slash posts in threads process attachments once | `builtin_slash_posts_in_threads_process_attachments_once` |
| me posts an action line | `builtin_me_posts_an_action_line` |
| me requires an action | `builtin_me_requires_an_action` |
| play posts through the normal message path | `builtin_play_posts_through_the_normal_message_path` |
| slash posts never start a stream | `builtin_slash_posts_never_start_a_stream` |
| unknown commands error with the available list | `builtin_unknown_commands_error_with_the_available_list` |

| File under `test/controllers/` | Named behaviors covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16/16 |
| `messages/pins_controller_test.rb` | 6/6 |
| `rooms/pins_controller_test.rb` | 3/3 |
| `saved_items_controller_test.rb` | 12/12 |
| `scheduled_messages_controller_test.rb` | 19/19 |
| `searches_controller_test.rb` | 36/36 |
| `rooms/slash_commands_controller_test.rb` | 10/10 |
| `autocompletable/icons_controller_test.rb` | 6/6 |
| `autocompletable/slash_commands_controller_test.rb` | 7/7 |
| `autocompletable/users_controller_test.rb` | 5/5 |
| `rooms/message_links_controller_test.rb` | 12/12 |
| `rooms/files_controller_test.rb` | 8/8 |

| Browser file under `test/system/` | Prior accepted Rails/Rust behavior results |
| --- | --- |
| `polls_test.rb` | 4/4 each |
| `pins_saved_test.rb` | 7/7 each |
| `slash_commands_test.rb` | 26/26 each |
| `search_files_test.rb` | 4/4 each |
| `scheduled_messages_test.rb` | 4/4 each |
| Total | 45/45 each; not rerun this checkpoint |
