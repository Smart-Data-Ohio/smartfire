# WS14 / WS15 cutover reconciliation — partial

Baseline: fresh `origin/main` at `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Branch: `rust/ledger-ws14-ws15`. Rails behavioral oracles: pinned `d7c7de92`; the CI seeds include main's schema overlay through `20261003180000`. No Rails source, assets, CodeQL configuration or compiler throttle was changed.

## Disposition of every originally open record

| Ledger | Originally open | Exact baseline-CI test receipt | Newly implemented assertion | Test-only outside gate | Still open |
|---|---:|---:|---:|---:|---:|
| WS14e | 108 | 30 | 0 | 0 | 78 |
| WS14g | 273 | 140 | 32 | 1 | 100 |
| WS15g | 64 | 21 | 0 | 0 | 43 |
| Total | 445 | 191 | 32 | 1 | 221 |

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

## Validation

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

**This PR does not clear the cutover gate.** [The remaining list](ledger-ws14-ws15-remaining.md) names all **221** exact records with both current ledger file:line and Rails file:line: 78 WS14e, 100 WS14g and 43 WS15g. There are 166 domain/HTTP/job/assertion records and 55 browser-interaction records. An open record means this reconciliation lacks a complete discriminating acceptance receipt; it does not assert that every corresponding production path is absent. Grouped model tests were not substituted for a distinct HTTP or browser assertion, and render-only query checks were not substituted for complete-request preload counts.

Closing these remaining invitations/recurrence/venue/controller/reminder cases, Calendar deletion/sync failure paths, Drive model/picker/interactions, GitHub delivery/cache/controller/logging cases and browser interactions is larger than this coherent reconciliation PR. They remain in the gate with exact declarations, without an owner-held waiver or a new allowlist. Scratch databases/reference archives and raw CI logs created for this slice are removed after extracting the receipts; `target/` is retained as permitted.
