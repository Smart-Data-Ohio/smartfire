# WS14 / WS15 continuation C — partial

Branch `rust/ledger-ws14-ws15-c`, stacked on PR #243. Step 1 of the review request is complete on #243 at `e02e8d61d1efed439f2f749ee990035dead55a22`: all 81 closures have 271 individual Rails assertion mappings; 19 distinct native mutation tests killed their production-behavior mutations; strict clippy and the full workspace passed (5,034 tests, zero failures, 20 existing skips). The parent review head `ec35cd92` is retained through the earlier merge; the separate fix239b audit has not landed at this receipt.

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

**94 records remain open: 49 WS14g and 45 WS15g; zero WS14e.** This slice stops at the complete HTTP/job/model block before the Drive browser system files. [The exact remaining list](ledger-ws14-ws15-remaining.md) retains each ID, current inventory file:line, Rails declaration and missing assertion. None is waived or called owner-blocked. In particular, the two remaining parent reopens **WS15g-056/057** still require the real registered GitHub fetch-PR broadcast/no-broadcast checks and belong to this continuation. The fix239b parent audit is still pending and must be merged when it lands.
