# WS8b-m2 F: #216 review fixes

Source checkpoint: `dd5e29a0da01f8172e5a339c48aa91bcf5d16ca1`, branch `rust/ws8bm2-message-features-f`. Main `15653de60` was merged with merge commit `d2df88b8e`; `cargo metadata --locked` passed after resolution. Main advanced to `1ccf68733` (#212) while the first fresh gates ran; it was merged as `dd5e29a0d` without conflicts, metadata passed again, and all fresh gates were rerun after rebuilding seeds. The starting worktree was clean, so there was no WIP to commit. Both sides of the shared-parser and presenter conflicts are retained: bots use main's `rails_compat::datetime`; message features retain exceptional parse errors, plain-text preloads, and main's broadcast preloads. Both crates retain their identical transition fixtures. The manifest's automatically merged duplicate `bnum` key was removed before the metadata gate.

**All three review items are addressed. This is a coherent PR checkpoint, partial for cutover and not owner-blocked-only.** No timing threshold or test concurrency was changed. Rust ran with four test threads, Cargo with two build jobs, and the configured machine-wide rustc throttle. Test cohorts waited for capacity before starting. No stash, production deploy, browser harness, or model-server operation was used.

## Fixes and failing-first evidence

1. **Actual slash output, including routing.** `slash_named_tests.rs` captures `result.payload()` before serialization. Its 35 comparisons retain actual kind/message/url/notice, durable user/message/saved state, attachment processing and job counts. The OOO comparison now resolves the received wire identifier through independently registered subscriptions; it no longer copies the fixture's stream into the observed frame. Every message-feature comparison module was swept for actual values rebuilt from expected fixture data. These were the two substitution sites found; other fixture reads supply inputs, expected assertions, iteration counts or diagnostics. No masks were added.

   The comparison sources on the merged baseline were byte-identical to `cdc18779`. With a real `huddle_launch` mutation returning `room_id=-1` and a real `status_badge` mutation publishing the unchanged HTML on `ooo_notice`, the old 35 tests passed. With the strengthened assertions, precisely the huddle and OOO tests fail. The huddle compares `-1` against Rails' `654632876`; OOO observes the wrong stream rather than relabeling it. Source mutations were restored byte-for-byte and never committed.

2. **Complete Calendar rows and fatal writes.** `older_calendar_execution_tests.rs` now reads every column in `event_calendar_entries`, including `synced_at`, `last_error`, IDs and timestamps, or asserts the row is absent. `comparison_support.rs` normalizes timestamp storage spelling on both independently read rows, requiring valid timestamps; it neither invents columns nor reads expected values into actual output. The 26 ordinary parents and four actually dispatched children are retained. `sync_conflict` starts with both a NULL sync stamp and a prior error, proving that success writes the frozen 16:00 stamp and clears the error.

   The old execution matrix passed with `google_entry::success` a no-op. The strengthened matrix rejects that mutant at `sync_conflict.last_error` (prior error versus NULL); the complete expected row also requires `synced_at=2026-03-02 16:00:00`. The added six actual durable Calendar jobs exercise SQLite failures in success UPDATE, error UPDATE and DELETE at 4/16 references. Rails raises `ActiveRecord::StatementInvalid`; Rust must propagate the storage failure, retain a failed job at attempt 1, preserve the complete entry row, and emit no old-window frame. The no-op-success mutant also fails the fatal-write test. A separate real `entry_sync` mutation swallowing the success-write error is rejected by the committed mutation runner, proving the check detects lost error propagation, not just missing output fields.

3. **Sibling claims from final transaction state.** `link_embed/store.rs` registers the sibling request scan through `Tx::before_commit_record_latest`. It runs after the application closure finishes, inside the same writer transaction, before COMMIT. It retains bounded message batches and cross-batch fetch deduplication. Latest registrations coalesce in their first slot, and savepoint rollback restores earlier registrations. Queue insertion failures still roll back metadata, claims and jobs; after-commit broadcasting remains after COMMIT.

   `final_state_siblings.rb/json` pins Generic/LinkedIn × removed references, suppressed message, and a preceding rolled-back savepoint. Before the runtime fix, all six cases produced one unexpected sibling job and advanced its claim from 15:49 to 16:00 while Rails queued none. After the fix, all six compare complete newer-message and sibling rows, actual durable job arguments and exact frames. Two additional real queue-insertion rejection checks prove atomic rollback for both providers. That atomic enqueue contract is the explicit stronger SQLite guarantee in `decisions.md`, not a claim that Rails rolls back a primary save when a Redis after-commit enqueue fails. Two DB tests cover final state, latest registration/savepoint restoration and failed preparation rollback.

The committed `verify_review216_mutants.py` contains reproducible comparison, eager-claim and swallowed-error controls. The first two controls correspond to the served mutations recorded above; the swallowed-error mode was also executed through this runner. No fault switch remains in production code.

Raw negative-control and local regression lines, from `.scratch/ws8bm2-f-review216/` (ANSI escapes removed only):

`legacy-huddle-mutant.log`

```text
test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 2649 filtered out; finished in 3.88s
```

`fixed-comparison-mutants.log`

```text
test result: FAILED. 33 passed; 2 failed; 0 ignored; 0 measured; 2652 filtered out; finished in 3.45s
```

`legacy-calendar-mutant.log`

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2683 filtered out; finished in 16.47s
```

`calendar-persistence-mutant.log`

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 2685 filtered out; finished in 7.03s
```

`calendar-error-mutant.log`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2686 filtered out; finished in 0.48s
WS8bm2 review216 mutations: calendar-error rejected by actual-output regressions
```

`final-state-before.log`

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2686 filtered out; finished in 4.07s
```

`features-after.log`

```text
test result: ok. 238 passed; 0 failed; 0 ignored; 0 measured; 2449 filtered out; finished in 77.58s
```

`commit-preparation.log`

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1317 filtered out; finished in 0.00s
```

## Files and source provenance

- `controllers/message_features/{slash_named_tests,older_calendar_execution_tests,comparison_support,final_state_sibling_tests}.rs`, plus module registrations: actual result fields, actual stream identities, complete durable rows and failures.
- `db/src/database.rs`: final-state transaction preparation and savepoint/error tests; `integrations/link_embed/store.rs`: the consumer. This is the only runtime behavior change beyond the merge.
- `reference-tools/messaging/{older_calendar_execution,final_state_siblings}.rb` and their vectors: Rails row/failure/final-state evidence. The oracle list now has 46 entries.
- `verify_review216_mutants.py`: production source mutation controls with unconditional byte restoration on normal/error exits.
- The merge inherited #213's X quote-URL guard and its expanded card golden. An initial replay against the old pin correctly failed only the new non-HTTP quote case. `build-twitter-reference.sh` builds an **unmodified git archive of `955af4c3781bef07b97b7aefce12c376110a812c`** into a separate owned image. `verify_oracles.py` uses it only for `twitter_cards`; every other corpus keeps `d7c7de92`. The source checker verifies the consumed file set and bytes at each revision and rejects two injected drift controls. The inherited golden is unchanged; no reference image or Rails source was patched. Its inherited `reference: d7c7de92` metadata label remains unchanged, but this corpus's actual post-pin source revision is explicitly recorded here and enforced by the replay runner.

No own edits touch the WS12 prohibited board/work files, WS11's case ledger, or the read-only Astra evidence. All new/changed IN preloads reuse existing bounded batches/JSON single-bind lists; the preparation seam introduces no IN list.

## Physical reads

No result-count slope increases. The successful feature cohort and fresh workspace both execute the production render/cache and callback/job paths. The selected raw lines below come from the fresh suite; Calendar inspection reads occur after consumer capture stops.

| Calendar consumer | Rust 4 → 16 | Rails 4 → 16 |
| --- | --- | --- |
| `inbound_confirmed` | 6 → 6 | 7 → 7 |
| `inbound_cancelled` | 18 → 18 | 12 → 12 |
| `inbound_deleted` | 18 → 18 | 12 → 12 |
| `inbound_forbidden` | 6 → 6 | 7 → 7 |
| `inbound_unavailable` | 6 → 6 | 7 → 7 |
| `inbound_no_account` | 2 → 2 | 2 → 2 |
| `inbound_disconnected` | 2 → 2 | 2 → 2 |
| `inbound_departed` | 5 → 5 | 6 → 6 |
| `inbound_local_declined` | 6 → 6 | 7 → 7 |
| `sync_update` | 8 → 8 | 7 → 7 |
| `sync_conflict` | 9 → 9 | 8 → 8 |
| `sync_unavailable` | 9 → 9 | 7 → 7 |
| `sync_deleted` | 2 → 2 | 2 → 2 |
| `sync_write_failed` | 9 → 9 | 8 → 8 |
| `sync_error_write_failed` | 9 → 9 | 8 → 8 |
| `sync_delete_failed` | 6 → 6 | 5 → 5 |

| Other production path | Rust 4 → 16 | Rails 4 → 16 |
| --- | --- | --- |
| Pin list, plain/populated | 8 → 8 | 13 → 25 |
| Standalone named poll | 5 → 5 | 9 → 21 |
| Standalone anonymous poll | 5 → 5 | 5 → 5 |
| Generic/LinkedIn callback, ordinary/negative/empty | 6 → 6 | 7 → 19 |
| Generic/LinkedIn network parent job | 10 → 10 | 14 → 26 |
| Actually dispatched stale sibling job | 10 → 10 | 10 → 22 |

```text
WS8bm2 final-state siblings "generic"/"removed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state siblings "generic"/"suppressed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state sibling queue failures: 2/2 transactions rolled back metadata, claims, durable jobs and frames
WS8bm2 final-state siblings "generic"/"savepoint_then_removed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state siblings "linkedin"/"removed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state siblings "linkedin"/"suppressed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state siblings "linkedin"/"savepoint_then_removed": Rust jobs=[], Rails jobs=[]; Rust claim="2026-03-02 15:49:00", Rails claim="2026-03-02 15:49:00"; complete frames and newer message row match
WS8bm2 final-state siblings Rust: 6/6 transactions; complete persisted rows, durable sibling jobs and exact frames checked
WS8bm2 Calendar execution Rust sync_write_failed size=4: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_error_write_failed size=4: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_delete_failed size=4: 6 consumer reads; Rails=5
WS8bm2 Calendar execution Rust sync_write_failed size=16: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_error_write_failed size=16: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_delete_failed size=16: 6 consumer reads; Rails=5
WS8bm2 Calendar execution Rust sync_update size=4: 8 consumer reads; Rails=7
WS8bm2 Calendar execution Rust sync_conflict size=4: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_unavailable size=4: 9 consumer reads; Rails=7
WS8bm2 Calendar execution Rust sync_deleted size=4: 2 consumer reads; Rails=2
WS8bm2 Calendar execution Rust sync_update size=16: 8 consumer reads; Rails=7
WS8bm2 Calendar execution Rust sync_conflict size=16: 9 consumer reads; Rails=8
WS8bm2 Calendar execution Rust sync_unavailable size=16: 9 consumer reads; Rails=7
WS8bm2 Calendar execution Rust sync_deleted size=16: 2 consumer reads; Rails=2
WS8bm2 pin-list reads populated=false size=4: Rust=8; Rails=13
WS8bm2 standalone-poll reads size=4 anonymous=false: Rust=5; Rails=9
WS8bm2 standalone-poll reads size=4 anonymous=true: Rust=5; Rails=5
WS8bm2 pin-list reads populated=false size=16: Rust=8; Rails=25
WS8bm2 standalone-poll reads size=16 anonymous=false: Rust=5; Rails=21
WS8bm2 standalone-poll reads size=16 anonymous=true: Rust=5; Rails=5
WS8bm2 pin-list reads populated=true size=4: Rust=8; Rails=13
WS8bm2 standalone-poll reads size=4 anonymous=false: Rust=5; Rails=9
WS8bm2 standalone-poll reads size=4 anonymous=true: Rust=5; Rails=5
WS8bm2 standalone-poll reads size=16 anonymous=false: Rust=5; Rails=21
WS8bm2 standalone-poll reads size=16 anonymous=true: Rust=5; Rails=5
WS8bm2 pin-list reads populated=true size=16: Rust=8; Rails=25
```

## Commands and fresh-clone gates

All commands listed here were executed in this round. Cargo runs through `rust/ci/cargo.sh` in the configured 1.98.1 toolchain image, with the existing throttle, disk-backed scratch, `CARGO_BUILD_JOBS=2`, four test threads and this worker's port range. Builds/tests on the reused owned target are serialized. `.scratch/ws8bm2-f-review216/fresh` is a fresh `--no-hardlinks` clone of the branch, advanced only to the exact committed source checkpoint before testing. Its default, first_run and agents_ui seeds were rebuilt there from tracked inputs. The full workspace suite uses that clone's source and seeds; no source/fixture is borrowed from an untracked working file.

```sh
rust/parity/bin/seed build default first_run agents_ui
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j 2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j 2 -- -D warnings
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
WS8BM2_ORACLE_SCRATCH="$PWD/../fresh-oracles-after" python3 rust/reference-tools/messaging/verify_oracles.py
python3 rust/reference-tools/messaging/features-reference-check.py --self-test
bash rust/reference-tools/messaging/build-twitter-reference.sh
```

The first six commands run from the fresh clone (with the worker's documented CI mounts); the source check and reference build also ran in the worker checkout. The independent replay regenerated both changed/new Rails corpora a second time, matching the initially generated pinned files byte for byte. Source-image verification prints its actual commit; the ordinary source checker also verified the shared pin.

Local regression commands also executed:

```sh
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire --bin campfire slash_named_tests -j 2 -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire --bin campfire older_calendar_inbound_and_sync_jobs_execute -j 2 -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire --bin campfire older_calendar_execution_tests -j 2 -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire --bin campfire final_state_sibling_claims_match_rails -j 2 -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire --bin campfire controllers::message_features -j 2 -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-f-review216/run.sh test --locked -p campfire_db database::tests::before_commit -j 2 -- --test-threads=4 --nocapture
python3 rust/reference-tools/messaging/verify_review216_mutants.py calendar-error -- bash .scratch/ws8bm2-f-review216/run.sh
```

Derived aggregate across 61 Cargo summaries: **4750 passed; 0 failed; 14 ignored**. Every raw Cargo result line follows.

```text
test result: ok. 2697 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1006.91s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.73s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1315 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 173.01s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.63s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.99s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.08s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 31.20s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.69s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.11s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.20s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.66s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.88s
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
WS8bm2 full fresh workspace exit: 0
```

Strict clippy

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 04s
WS8bm2 fresh strict clippy exit: 0
```

Release-input build

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 59.11s
WS8bm2 fresh release input build exit: 0
```

Locked metadata: exit 0; parsed JSON has all 13 workspace members.

```text
WS8bm2 fresh locked metadata exit: 0
WS8bm2 reference source check: 118 controller, model, helper and template files match 955af4c3781bef07b97b7aefce12c376110a812c
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 oracle replay: 46/46 independently replayed fixtures byte-identical
WS8bm2 reference source check: 118 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Ignored tests are inherited; this round adds no ignore:

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

## Remaining for cutover, with owner

1. **WS8b-m2 / WS15e queue contract:** broader Rails enqueue-adapter rejection permutations. This round adds two actual SQLite enqueue rejection regressions and verifies metadata/claim/job rollback. The larger Rails differential matrix around failing enqueue adapters is still unported proof work; the explicit stronger SQLite atomicity decision applies.
2. **WS8b-m2 / WS14g:** InboundSync/SyncEntry transport and 429 retry/exhaustion permutations. Full ordinary/fatal entry rows, permanent 503 outcomes and dispatched children are covered; MeetLink retry evidence does not substitute for these two consumers' transient exhaustion cases.
3. **WS8b-m2:** exceptional relative-duration overflow, leading/trailing time-split inputs and unsampled top-level structured parameter containers. The existing absolute grammar/coercion/DST corpus is retained and rerun; no claim covers arbitrary Ruby strings or the legacy Option split paths.
4. **WS11-API:** typed AgentBudgetNotice facts reader remains the explicitly held named seam. Physical push, agent invocation/auth and huddles already use their owners' real integrations.
5. **Lead/system-test phase:** final both-app cutover browser run. The prior accepted 45/45 behavior results per app are retained as historical coverage; browsers were not rerun or counted in this review round.
6. **WS12:** broader board/work parity is outside this workstream; prohibited owner files were untouched except inherited merge content.

The three #216 requests are complete; cutover items 1–3 remain unblocked. This checkpoint is **not owner-blocked-only**.

## All 35 named built-in ports

Each row is a separate passed WS8 test in `controllers/message_features/slash_named_tests.rs`; all 35 also fail the served command mutations. Their original names are pinned from `test/services/slash_commands/dispatcher_test.rb`, independently of WS11's ledger.

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

## Retained Rails file coverage inventory

These are named Rails behaviors covered by grouped Rust tests/vectors, not a claim that the Rails controller suites were executed as suites in F. The full workspace suite reruns the Rust ports. Ordinary named controller coverage is **140/140**; the former single agent case is now integrated.

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
| Total | 45/45 each; not rerun in F |




Cleanup: fresh-clone generated build-output directories were removed after verification; the pre-existing owned compiler cache was reused, and no separate compiled Cargo target cache was created. Dependency source directories named `target` were retained. Raw logs and the reproducible reference images remain as evidence. No owned test/runner container or binary remains running.
