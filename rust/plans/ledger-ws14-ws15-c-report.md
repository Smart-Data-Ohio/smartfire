# WS14 / WS15 continuation C — partial

Branch `rust/ledger-ws14-ws15-c`, stacked on PR #243. Step 1 of the review request is complete on #243 at `e02e8d61d1efed439f2f749ee990035dead55a22`: all 81 closures have 271 individual Rails assertion mappings; 19 distinct native mutation tests killed their production-behavior mutations; strict clippy and the full workspace passed (5,034 tests, zero failures, 20 existing skips). The parent review head `ec35cd92` is retained through the earlier merge; the fix239b audit has subsequently landed and is merged through #243 head `31b767757`.

## Current slice

Fifty open declarations are now covered by fifty named native tests, following the remaining ledger's Rails controller/integration/job/model file order. [The assertion map](ledger-ws14-ws15-c-assertions.md) enumerates **141 original Rails assertion and Mocha expectation calls**, each with its exact discriminating Rust assertion file and line. Repeated loop inputs and shared DOM assertions are identified. [The structured ledger](ledger-ws14-ws15.json) preserves previous receipts, including the two reopens, and adds the new disposition in place.

- **WS14e-051/057:** the registered durable ReminderPushJob reaches the real Web Push pool and a local TLS fake push service; its encrypted payload is decrypted and compared. Venue body is exactly `Starts in 15 minutes: Launch party planning in Lounge`. Both the original old-ended case and a recent-ended case are suppressed, with a still-running delivery control. A pinned Rails probe also checks those additional end-time controls.
- **WS14g-004/005:** real message show/edit requests compare the consent-independent chip DOM, two removable chips and each of the two file inputs plus the blank sentinel.
- **WS14g-007/011/015/022/025/027/034:** actual sudo/password/audit/read/re-sign-in/replay requests, registered verifier membership, nested role DOM and the shared rate-limit store reset. Sessions are created after loading the original Rails fixtures. Ten failed requests and the eleventh 429 discriminate the store clear; no timing threshold is widened.
- **WS14g-035–041:** configured/unconfigured room composers, every public picker meta, legacy/enhanced selection, menu item text/counts, dialog attributes and signed-out redirect. These close the Rails HTTP integration assertions, not the later browser callback/caret/interaction declarations.
- **WS14g-053/054/056/058/059/062/063 and 066–074/077–080/083/084:** registered Calendar cleanup and remote-delete jobs, refresh/revoke ordering, exact method/path/token requests, retries and unchanged payloads, terminal log levels, token/argument log suppression and no-request controls. Source mutations use the real Event/Room APIs and durable-job observer. An independent queue reader proves before-commit invisibility; the writer still inserts the intent atomically under WS17's contract. Series RSVP executes three real Google inserts and checks three distinct stored remote identities. Room destruction and series shrink execute the captured remote delete identities through the real registered runner.
- **WS14g-121–128/184–185:** unsaved Drive-only/textless/invalid/over-cap association validation, saved scoped duplicates, distinct parent messages, real destruction, exact open URL and persisted account connected/usable/expiry/scope predicates.

The job tests use FrozenClock, an explicit commit observer/drain and the existing five-second deadlines. Google HTTP stays behind the injected recorded client. No real Google request, browser pixel comparison, JS/CSS change or asset digest change is involved.

## Production changes and first-failure evidence

The new unreadable-cleanup regression failed against the unchanged #243 Calendar implementation at the assertion requiring the Rails warning's literal account ID:

```text
assertion failed: log.lines().any(|line| line.contains("WARN") && line.contains(&format!("account {account_id}")))
```

The previous structured log field did not contain Rails' `account <id>` text. The cleanup handler now logs `Calendar::DisconnectCleanupJob skipped cleanup for account <id>: credentials expired or unreadable`, retaining the normal no-op result and token redaction. The native regression and original pinned Rails case pass.

Two missing direct model/concern APIs are also added: the ordered, duplicate-free app-owned extra-verifier registry (TOTP remains registered at boot; registration alone supplies no implementation), and the metadata-free DriveAttachment reader/validator/create/open-URL API. Creation validates scoped uniqueness in a savepoint and touches the real Message. Existing Message bulk autosave and the Google/sudo adapters remain in use.

The first native setup used seed-issued cookies after replacing seed sessions with Rails fixtures. Those unrelated redirects were corrected by signing in through the real session API after fixture loading. They are not claimed as production divergences.

## Rails receipts

The original ten test files are byte-identical to `git show d7c7de92:<file>`. They run in the current-schema reference image with network disabled, the pinned test directory and test environment mounted read-only, and four Rails workers. The additional recent-ended/live-control probe uses the same image and reference models.

Reproducible commands from the repository root (after `parity/bin/ci-seed prepare`):

```sh
python3 rust/reference-tools/cutover/rails-cases.py rust/plans/ledger-ws14-ws15-c-assertions.json --storage .scratch/ledger-c/rails-storage
```

For the extra control, run `bundle exec rails runner /probe/reminder-ended-control.rb` in the same network-disabled reference image with the probe mounted from `rust/reference-tools/cutover/reminder-ended-control.rb`, and the same test/test-environment/storage mounts as `rails-cases.py`.

Raw summary lines:

```text
50 runs, 204 assertions, 0 failures, 0 errors, 0 skips
1 runs, 5 assertions, 0 failures, 0 errors, 0 skips
```

## Validation

Native targeted rerun, with `CI=true`, four nextest workers and the three Rails-validated current-schema seeds:

```sh
cargo nextest run --workspace --exclude html5ever -j 4 -E 'test(cutover_c_)' --no-fail-fast
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
     Summary [   7.950s] 50 tests run: 50 passed, 5054 skipped
    Finished `dev` profile [unoptimized] target(s) in 40.67s
```

Exit status 0 for both. Final full-workspace/fresh-clone receipts will be appended after execution. No new native test is attributed to baseline CI run 37200618245. The unchanged db/app CI selectors execute these modules; the PASS log is checked against every mapped test name.

## Precisely remaining

**126 records remain open: 59 WS14g, 52 WS15g and 15 WS14e.** This slice stops at the complete HTTP/job/model block before the Drive browser system files. [The exact remaining list](ledger-ws14-ws15-remaining.md) retains each ID, current inventory file:line, Rails declaration and missing assertion. None is waived or called owner-blocked. In particular, the two remaining parent reopens **WS15g-056/057** still require the real registered GitHub fetch-PR broadcast/no-broadcast checks and belong to this continuation. The fix239b parent audit at `6880979a` is now merged through #243 head `31b767757`; all 32 further reopened contracts are retained.

## Full parent audit merge

The parent moved to `6880979acaf433e294d6eb8031e989aa9a394484` during this slice. It is merged into #243 at `31b767757`, then into C with both sides retained. The reviewed reference pin is now `78b9b1546`; the parent audit reopens 32 additional records. All original parent assertion mappings, stronger tests and historical validation hashes remain intact. The continuation verifier additionally validates B/C's exact assertion maps; its own full-run receipt is kept separately. Current C still closes the same 50 declarations, and the remaining count is **126**, not the earlier 94.

## Fresh-clone integration receipts

Production code `056c56b0b6870e327ee62aba5297ae49127b525f` was verified in a clone made with `git clone --no-hardlinks`. Its tracked tree is clean and matches that commit. The three seeds were rebuilt and Rails-validated for `review236-reference:78b9b1546` before copying into the clone: default 29/29, first_run 4/4, agents_ui 40/40 (73 passed, zero failed). The canonical Rust 1.98.1/libvips image, build jobs 2, shared machine-wide compiler slot throttle and `CI=true` are retained. Only the shared target cache is reused; all source inputs come from the fresh clone.

```sh
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed prepare
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed check-image
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed build
PARITY_IMAGE=review236-reference:78b9b1546 rust/parity/bin/ci-seed validate
cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
bash ci/with-release-inputs.sh cargo build --workspace --bins
```

```text
     Summary [1711.105s] 5088 tests run: 5088 passed (7 slow), 20 skipped
    Finished `dev` profile [unoptimized] target(s) in 3m 13s
    Finished `dev` profile [unoptimized] target(s) in 3m 23s
```

All three commands exit 0. All 78 B and 50 C mapped native tests run and pass, including the production policy/transport checks. All 318 retained closure records have actual named PASS entries; none of them is skipped. The canonical logo comparison passes (`avatar_bot_logo_uploads_match_pinned_rails`, 10.513s). The inherited `pull_request_thread_without_starter_refreshes_once_and_survives_queue_failure` also passes; no inherited flake or timing-policy change is involved.

The 50 original Rails cases were rerun against the refreshed pin with `rails-cases.py --image review236-reference:78b9b1546`; the extra probe uses that same image. Their raw summaries remain:

```text
50 runs, 204 assertions, 0 failures, 0 errors, 0 skips
1 runs, 5 assertions, 0 failures, 0 errors, 0 skips
```

Post-run review strengthens the five new redirect declarations to compare status **and** Location; the continuing join-code control is strengthened too. Those are test-only changes after the full-run production head. A final fresh-clone scoped run covers every B/C mapped test after those changes, with strict clippy rerun. Its receipts are appended below rather than misattributed to the earlier full-run head.

The mutation utility's argument parser mistakenly consumed a trailing `--recheck` as its executable prefix. The restoration guard kept all production SHA-256 hashes unchanged. One scratch raw log was truncated; its committed historical receipt was preserved, and the organizer-bot sample was rerun through the fixed utility. All nineteen current raw logs now verify at their mapped native assertion lines. Compile/setup errors do not count as mutation kills.

```text
Mutation CLI parser: 3 cases passed; production source hashes unchanged
Mutation summary: 19 run; 19 killed; production restored=True
Assertion maps: 131 records; 412 original Rails assertion calls; 0 unmapped
Acceptance ledger: 445 records checked; 86 baseline-CI passed; 232 current implementation receipts; 1 test-only outside gate; 126 explicitly open
Full closure audit: 220 records; 555 Rails assertions/predicates; 487 mapped; 78 strengthened closures; 32 reopened
Final local run: 5088 distinct PASS entries; 0 FAIL entries; 20 skipped; all 318 retained closures have named PASS receipts
```

### Final test-only assertion verification (1fb679f8)

The fresh clone was advanced to `1fb679f89ed930e834dda8c82783c18a0ccbc9cf`. Every B/C mapped native test was rerun after the stronger redirect assertions; no production Rust file differs from the earlier full-run head.

```sh
cargo nextest run --workspace --exclude html5ever -j 4 -E 'test(cutover_c_) | test(cutover_event_) | test(cutover_entry_) | test(cutover_timeline_) | test(cutover_recurrence_) | test(cutover_reference_) | test(cutover_reminder_) | test(cutover_venue_) | test(tests::cutover::)' --no-fail-fast
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
cargo test --workspace --exclude html5ever --doc --no-fail-fast -- --test-threads=4
```

```text
     Summary [  16.016s] 128 tests run: 128 passed, 4980 skipped
    Finished `dev` profile [unoptimized] target(s) in 2m 35s
    Finished `test` profile [unoptimized] target(s) in 0.78s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All commands exit 0. The active doctest is the recurrence guard compile-fail assertion; the two kit examples remain ignored. Every other crate doctest summary is zero passed/failed/ignored. The scoped nextest skips are tests outside the selection plus the existing ignores; none of the 128 mapped tests is skipped.

Raw execution logs and the nineteen mutation logs are archived under `rust/target/ledger-ws14-ws15-c-receipts/` locally. The own 1.3GB scratch clone/Rails output and regenerated reference/seed input directories are removed after publishing. The inherited `.scratch/ws14g` output and shared build target are preserved. Only report and receipt metadata change after this verified code head.
