# WS14 / WS15 cutover reconciliation — partial

Original execution baseline: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. The full closure audit starts at `ec35cd92c74a07addc2f91e9f67207098c8b7d64` and cleanly merges `origin/main` at `ea94edeaabe689c3fd0b7d6d451d5aa1fc694827` (#236) before changing assertions. Target branch: `rust/ledger-ws14-ws15`. This audit's Rails oracles use the refreshed pin `78b9b1546bdab4c6c1c9b8ddb94512f661289112` and schema through `20261003180000`; earlier receipts below used `d7c7de92`. No Rails source, assets, CodeQL configuration or compiler throttle was changed.

## Full closure audit after re-review (2026-10-04)

Every previously closed record in the three inventories was opened against its complete Rails declaration and cited native tests: **219 closures plus the one test-harness classification**. The machine ledger records **555 Rails assertions or mock predicates**, with **487 mapped to exact current Rust checks**. The 67 unmapped in-gate assertions belong to reopened records; the remaining unmapped assertion belongs to the existing WebMock teardown classification. Every retained closure maps all its Rails assertions. Shared corpus receipts identify the active real path and discriminate its scenario inputs; detached renderers or isolated callbacks do not close producer/HTTP assertions.

| Ledger | Closed records audited | Retained closures | With new assertions | Reopened | Currently open |
|---|---:|---:|---:|---:|---:|
| WS14e | 28 | 13 | 11 | 15 | 95 |
| WS14g | 172 | 162 | 63 | 10 | 110 |
| WS15g | 19 | 12 | 4 | 7 | 52 |
| Total | 219 | 187 | 78 | 32 | 257 |

WS14g-268 was independently reviewed as the additional 220th record and retains its test-only classification. Of the retained closures, **78 acquired new assertions and 109 already had corresponding assertions**. No previously open record is silently waived. All 32 reopened records preserve their old receipts and list the exact missing assertion, its Rails line, and why the former evidence did not cover that scenario in the JSON and remaining list.

The four new findings are fixed through their real paths:

- WS14g-006: the actual unenrolled password user's HTTP prompt asserts zero Google forms, along with the password form target and input type.
- WS14g-028: a real non-replayable secret-changing POST and confirmation exercise an external Referer; redirect status/location select the safe root, with no replay form. Missing and same-host Referers are explicit controls.
- WS14g-099: the installed dispatcher runs twice while a dedicated SQLite writer trace observes every UPDATE, including zero-row conditional statements. The comparison checks one first-tick update and zero steady-state updates.
- WS14g-187: persisted encrypted credentials feed cleanup_snapshot and decryption; exact refresh token, access token and expiry are checked. Corrupting persisted ciphertext and reloading yields no snapshot.

The broader audit strengthens actual reminder dispatch/durable encrypted push delivery, singleton reminder rearming and cancellation timestamps, persisted encryption/tampering, credential rotation, Google endpoint hosts, Calendar status wrappers and real GitHub controller responses. Google oracles were regenerated from the pinned current reference image: **49 client cases, six credential combinations and 12 API token cases**. No production divergence required a new source change in this audit. Separate review checked every worker's assertion mappings and retained/reopened scenarios.

Compact inventory mappings use `file:line,line`; the JSON supplies the individual Rails assertion to full Rust `file:line` lists, scenario selectors, and precise missing contracts. The integrity checker verifies complete explicit Rails assertion-call coverage, physical Rust check locations, matching compact rows, exact remaining IDs, and historical execution receipts. Semantic path coverage was assessed by reading the declarations and implementations separately.

Current dispositions are **86 baseline-CI passed + 101 implemented or strengthened + 1 test-only outside gate + 257 open = 445**. The original CI receipt below never certifies assertions added on this branch. Final-source execution is recorded at the end of this report.

## Original PR dispositions before the full closure audit

| Ledger | Originally open | Exact baseline-CI test receipt | Newly implemented assertion | Test-only outside gate | Still open |
|---|---:|---:|---:|---:|---:|
| WS14e | 108 | 27 | 1 | 0 | 80 |
| WS14g | 273 | 137 | 35 | 1 | 100 |
| WS15g | 64 | 17 | 2 | 0 | 45 |
| Total | 445 | 181 | 38 | 1 | 225 |

Each original declaration is annotated **in place**, with its historical receipt retained. [The structured ledger](ledger-ws14-ws15.json) retains stable IDs, original/current ledger lines, the actual Rails declaration, exact test/source evidence and individual CI log lines. [Run 37200618245](https://github.com/Smart-Data-Ohio/smartfire/actions/runs/37200618245) is successful at exactly the baseline SHA: 4,948 distinct nextest PASS entries, zero FAIL entries. Every test used to close an old record was checked against that log, rather than inferred from the aggregate count or an owner report.

The two Drive polling HTTP comparisons are running and passed on main; their 501/ignored/owner-held descriptions are historical. The real GitHub thread page and its public/private/unknown body comparisons are also running and passed. Both stale seam descriptions are corrected explicitly. New Rust tests are registered by their modules and run under the unchanged db/app package selectors in `.github/workflows/rust.yml:212` and `:227`; they are not attributed to the older CI run.

Historical receipt preservation was checked separately: **445/445** original receipts are still present in their annotated inventories.

Only WS14g-268 is outside the gate: `test/system/drive_teardown_race_test.rb:18` deliberately tests WebMock stub lifetime across Rails test teardown, using the test-only `DriveTeardownRaceGate` prepend. There is no production teardown/stub behavior to port. The actual Drive fetch/error behavior remains in the gate. Browser interactions and behavior belonging to another original worker were not waived.

## Implemented behavior and failing-first evidence

### Removed room membership schedules Calendar reconciliation atomically

Rails `Membership#sync_removed_room_calendar_entries` (`app/models/membership.rb:282`) queues `Calendar::SyncEntryJob` for this user's entries in the removed room after commit. Rust's membership deletion had no producer, leaving remote copies stale despite an existing sync consumer.

`Membership::destroy` now reads final transaction state in a before-commit hook, persists the durable sync intents with the removal, and publishes after commit. The existing consumer and Event APIs are reused. Seven pinned Rails cases cover matching entries, no entries, other room/user isolation, entries inserted/deleted later in the transaction and rollback. A separate installed-app test rejects the job insert with a SQLite trigger and proves that membership deletion and queue persistence are atomic.

Failing first on the unchanged baseline implementation:

```text
membership_calendar_callback_matches_pinned_rails_final_state: matching_entries
left: []
right: [["Calendar::SyncEntryJob", [390339825, 127326141]]]
Summary [   0.749s] 1 test run: 0 passed, 1 failed, 1383 skipped
```

The exact test invocation was `cargo nextest run -p campfire_db -j 4 -E 'test(membership_calendar_callback_matches_pinned_rails_final_state)'`. The job-persistence regression is `jobs::tests::event_tests::membership_removal_calendar_job_is_durable_and_atomic`.

### Recorded Google client acceptance and revoke transport message

A new pinned Rails oracle records 42 client scenarios plus five credential combinations. It compares exact HTTP method/path/form/body/headers, returned JSON, exception class/message/retry classification and credential rotation/disconnection. It covers Calendar/Drive 403/404/409/410/429/500, refresh 429/503 and a persistent 401, pagination cap, blank/recent Drive listings, expired tokens, code exchange, revoke statuses and open/read/refused/reset/EOF transport failures. Both oracle and native tests prohibit unrecorded Google calls. The native app uses a FrozenClock.

This exposed a real mismatch: a revoke transport exception used the Calendar-request prefix in Rust. The revoke path now returns Rails' `Google token revoke failed (Net::OpenTimeout)`; refresh and code-exchange prefixes keep their own Rails behavior.

Failing first on the unchanged client implementation:

```text
google_client_errors_exchange_revoke_and_paging_match_pinned_rails: revoke_timeout
left:  Google Calendar request failed (Net::OpenTimeout)
right: Google token revoke failed (Net::OpenTimeout)
Summary [   0.741s] 2 tests run: 1 passed, 1 failed, 2851 skipped
```

The exact test invocation was `cargo nextest run -p campfire -j 4 -E 'test(acceptance_cases::)'`. Configuration already agreed with Rails; the acceptance matrix failed on the revoke message before the fix.

The four new tests then passed together:

```sh
cargo nextest run -p campfire -p campfire_db -j 4 -E 'test(membership_calendar_callback_matches_pinned_rails_final_state) | test(membership_removal_calendar_job_is_durable_and_atomic) | test(acceptance_cases::)'
```

```text
Summary [   0.701s] 4 tests run: 4 passed, 4233 skipped
```

## Original-head validation (ca92288e)

The commands and counts in this section describe the original reconciliation head. Review-correction execution is recorded below.

Rust checks use `campfire-toolchain-ci-rust-speedups:latest` (Rust 1.98.1, canonical libvips/ffmpeg), `CI=true`, the existing machine-wide rustc slot locks, `CARGO_BUILD_JOBS=2`, and at most four nextest workers. The host throttle configuration is unchanged. Container children use a disposable copy of the existing throttle wrapper with container-parent detection enabled. No real Google endpoint is called.

The first workspace attempt exposed stale local seed schema, not a production regression: the old seeds lacked migration `20261003180000`. All three seeds were rebuilt and Rails-validated using the already available `ws11ui-cutover-reference:current-schema` image (same Rails pin plus current schema overlay). A subsequent intermediate run was stopped before running additional tests; it is not counted as a full pass.

Seed commands (from repository root):

```sh
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed check-image
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed build
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed validate
```

Raw validation counts: default `"passed": 29, "failed": 0`; first_run `"passed": 4, "failed": 0`; agents_ui `"passed": 40, "failed": 0` (73 passed, zero failed).

Oracle regeneration commands (from repository root):

```sh
rust/reference-tools/events/membership_calendar.sh
PARITY_NAMESPACE=ledger-ws14-ws15 PARITY_OWNER=ledger-ws14-ws15 PARITY_IMAGE=ws14e-reference:d7c7de92 rust/parity/bin/reference runner --seed default rust/reference-tools/google/client-acceptance.rb
```

Both regenerated files are byte-identical to the committed vectors:

```text
Pinned Rails membership Calendar callback: 7 cases; no Google HTTP
Pinned Rails Google client acceptance: 42 recorded cases; 5 credential combinations; no Google network
rust/crates/db/src/models/calendar_event/membership_calendar.json: OK
rust/vectors/google_client_acceptance.json: OK
```

Ledger integrity/CI-receipt verification:

```sh
gh run view 37200618245 --log > ci-run.log
python3 rust/reference-tools/cutover/check-ws14-ws15.py --ci-log ci-run.log
```

```text
Acceptance ledger: 445 records checked; 191 baseline-CI passed; 32 new assertions; 1 test-only outside gate; 221 explicitly open
CI receipt verification: 4948 PASS entries; 0 FAIL entries; all cited names/lines match run 37200618245
```

Full Rust commands (from `rust/`, in the toolchain container):

```sh
cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
cargo test --workspace --exclude html5ever --doc --no-fail-fast -- --test-threads=4
bash ci/with-release-inputs.sh cargo build --workspace --bins
```

The strict lint and release-input build completed successfully (exit 0):

```text
Finished `dev` profile [unoptimized] target(s) in 2m 51s
Finished `dev` profile [unoptimized] target(s) in 1m 47s
```

The complete workspace run passed (exit 0):

```text
Summary [1590.942s] 4952 tests run: 4952 passed (6 slow), 20 skipped
```

The 20 skips are the unchanged baseline ignores (4 db, 13 app, 3 other); neither polling comparison nor any new test is skipped. The canonical logo byte comparisons passed in this run. No timing threshold or concurrency setting was widened to obtain the pass.

Doctests completed with exit 0: zero active doctests, zero failures, two unchanged kit ignores. All ten other crate summaries report zero passed/failed/ignored.

```text
Finished `test` profile [unoptimized] target(s) in 26.78s
Doc-tests campfire_kit
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## What remains

**This PR does not clear the cutover gate.** [The remaining list](ledger-ws14-ws15-remaining.md) names all **257** exact records with both current ledger file:line and Rails file:line: 95 WS14e, 110 WS14g and 52 WS15g. The 32 newly reopened records state their precise missing contracts. An open record means this reconciliation lacks a complete discriminating acceptance receipt; it does not assert that every corresponding production path is absent. Grouped model tests were not substituted for a distinct HTTP or browser assertion, and render-only query checks were not substituted for complete-request preload counts.

Closing these remaining invitations/recurrence/venue/controller/reminder cases, Calendar deletion/sync failure paths, Drive model/picker/interactions, GitHub delivery/cache/controller/logging cases and browser interactions is larger than this coherent reconciliation PR. They remain in the gate with exact declarations, without an owner-held waiver or a new allowlist. Historical validation and review-correction receipts below describe their respective earlier heads; the final full audit has its own execution receipts.

## Earlier PR #239 review correction (ec35cd92)

All closures citing the six challenged test functions were rechecked: 19 structured records plus four historical mapped rows (event show/series index and PR room routing/public-private cards). Historical receipts remain intact. The series-index test now checks the head URL, title/repeat label and absence of later-occurrence URLs; the callback socket test subscribes an unrelated room and checks exactly two publications only to the referencing room/mapped thread. The six retained pusher payload receipts describe only their actual countdown/title/start-age inputs, without claiming venue or end-time coverage.

Four new registered app tests close six records:

- WS14e-085: bot-key POST create and bot-session GET index both return 403; no event is persisted.
- WS14g-003: persisted Drive attachments reach authorized message HTTP show with status 200 and the complete pinned generic chip block.
- WS14g-052 and WS14g-055: actual durable cleanup jobs compare both DELETEs and snapshot bearer tokens, refresh-token revoke form, committed drain, and no retry after a literal DELETE 500.
- WS15g-046 and WS15g-051: actual thread GET renders two populated files with statuses/counts/more line; populated private files and the title stay hidden behind the exact lazy-frame URL.

WS14g-057 now cites the real durable retry/reporting test (baseline PASS line 3004, independently read from the raw log). Supplemental assertions on this head compare preserved ids/snapshot/account arguments at every committed retry, in addition to the existing future schedules and DELETE-only calls.

Four records are reopened with precise missing acceptance assertions in both the machine ledger and remaining list: WS14e-051 (venue suffix through the reminder job/pool), WS14e-057 (past end distinguished from old start through that job), WS15g-056 (mapped header replacement from the actual fetch job), and WS15g-057 (silence without references/mappings from the actual fetch job). These require additional reminder transport or fetch-job/publication fixtures; existing source-only/direct-callback tests do not supply those receipts. No production divergence is claimed for reopened records.

Totals at the earlier review-correction head: **445 records = 181 baseline-CI passed + 38 implemented + 1 test-only outside gate + 225 open**. The remaining list has exactly the 225 unsupported IDs, with no duplicate, omitted or closed ID. The baseline raw-log hash and all name/line receipts pass `reference-tools/cutover/check-ws14-ws15.py --ci-log`.

Review-correction validation uses scoped nextest (`-j 4`) and strict workspace clippy. Default/first_run/agents_ui seeds are private copies of the current-schema Rails seeds, freshly Rails-validated here: **73 passed, 0 failed** (29 + 4 + 40). No seed-dependent test is allowed to skip (`CI=true`). The existing toolchain image and shared rustc slot locks remain in use, with two build jobs and four CPUs. Production code and CI selectors are unchanged.

Exact final-source nextest selection (from `rust/`, in the existing toolchain container):

```sh
cargo nextest run --locked -p campfire -p campfire_db -j 4 --profile ci -E 'test(controllers::rooms::events::tests::) | test(controllers::messages::drive_tests::) | test(controllers::channel_threads::github_tests::) | test(app::google_calendar_job_tests::) | test(app::google_reporting_tests::) | test(integrations::github::pull_requests::tests::) | test(reminder_push_payload_and_staleness_match_rails_vectors)'
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
Summary [  32.234s] 44 tests run: 44 passed, 4197 skipped
Strict workspace clippy: exit 0; Finished `dev` profile [unoptimized] target(s) in 1m 55s
```

This is an affected-suite run, not a new full-workspace pass; all four new tests and the three strengthened shared tests executed. The 4,197 tests outside this selection/ignored are not counted as executed. The initial focused run had three passes and two test-expectation failures: DELETE Content-Type needed Rails' application/json, and preserved retry arguments needed the queue inspector to unwrap its metadata envelope. Both expectations were corrected using Rails source/existing queue decoding; no production change or test-policy relaxation was needed. The final 44-test run passed at the final Rust sources.

## Final full-audit execution

The final checks use `campfire-toolchain-ci-rust-speedups:latest` (Rust 1.98.1 and canonical media libraries), `CI=true`, four CPUs, two build jobs and the existing shared rustc slot locks. The throttle, CI package selectors, ignores and timing policies are unchanged. The refreshed reference image is `review236-reference:78b9b1546`; default, first_run and agents_ui seeds were rebuilt and Rails-validated: **73 passed, zero failed** (29 + 4 + 40).

The first complete audit run executed 4,960 tests: 4,959 passed and the new fixed-date OOO socket test failed before its assertions because the March-issued seed admin session had expired at its September clock. The fixture now creates a fresh signed session at that clock; the September 24 date, note, real dispatcher and exact stream counts remain asserted. All **66** affected status/notice tests passed after correction. Strict lint also caught repeated regex construction in the notice scenario loop; the same regex now compiles once outside the loop.

Final commands, from `rust/` in that container:

```sh
cargo nextest run --locked --offline -p campfire -j 4 --profile ci -E 'test(controllers::rooms::ws17_ooo_tests::) | test(controllers::users::statuses::)'
cargo nextest run --locked --offline --workspace --exclude html5ever -j 4 --profile ci --no-fail-fast
cargo clippy --locked --offline --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
Focused status/notice: 66 tests run, 66 passed, 2792 filtered/ignored
Summary [1767.782s] 4960 tests run: 4960 passed (10 slow), 20 skipped
Strict workspace clippy: exit 0; Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 07s
```

The final full run has **4,960 distinct named PASS receipts, zero failures and 20 unchanged baseline skips**. All **108 distinct cited native tests** executed, covering every one of the **187 retained closure records**. No skipped test closes a record. Baseline CI and local assertion execution remain separate in the machine ledger; raw local log hashes and command/count receipts are recorded under `closure_audit.verification`.

```sh
python3 rust/reference-tools/cutover/check-ws14-ws15.py --ci-log ci-baseline.log --nextest-log nextest.log
```

```text
Acceptance ledger: 445 records checked; 86 baseline-CI passed; 101 current implementation receipts; 1 test-only outside gate; 257 explicitly open
Full closure audit: 220 records; 555 Rails assertions/predicates; 487 mapped; 78 strengthened closures; 32 reopened
CI receipt verification: 4948 PASS entries; 0 FAIL entries; all cited names/lines match run 37200618245
Final local run: 4960 distinct PASS entries; 0 FAIL entries; 20 skipped; all 187 retained closures have named PASS receipts
```

All 16 changed Rust files pass rustfmt; whitespace and exact inventory/remaining-row integrity checks pass. Independent review approved all four audit partitions, the integrated mappings and the final fixture/lint corrections.
