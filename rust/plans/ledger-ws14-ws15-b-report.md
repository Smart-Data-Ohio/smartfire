# WS14 / WS15 acceptance continuation

Stacked on PR #239 head `ca92288e839e9dd8e97f83dc236ae8f889ddd3c4`; branch `rust/ledger-ws14-ws15-b`. No commits from this continuation are pushed to the parent branch. Batches follow the remaining ledger's Rails file/declaration order. Each new named test reproduces the original setup and discriminating assertions through the real Rust model/HTTP/job path. Duplicate declarations in the two inventories share the same named test and are counted explicitly as ledger records.

## Batch 1: test/models/event_test.rb

Twelve open declarations are closed by twelve named tests in `crates/db/src/tests/calendar_event_test/cutover_event_test.rs`. Setup uses the real User, Membership, Room, CalendarEvent and ActivityItem APIs, the Rails fixtures and a FrozenClock. Cases distinguish early/equal/no end, bot/outsider organizers, a deleted venue, each notification involvement, going/maybe/declined responses, unread state, duplicates, title-only edits, cancellation, membership accessibility and complete room deletion through its registered domain consumer. All pass without a production change.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_event_test::)' --no-fail-fast
```

```text
Summary [   0.755s] 12 tests run: 12 passed, 1384 skipped
```

The original unmodified Rails file also runs in the reference container with network disabled and four processes:

```text
18 runs, 74 assertions, 0 failures, 0 errors, 0 skips
```

The image tag `ws14e-reference:d7c7de92` has a later revision environment label, so it is not trusted by tag alone. The actual `app/models/event.rb` and `test/models/event_test.rb` SHA-256 hashes match `git show d7c7de92:<path>` byte-for-byte; shared model sources are checked separately. Rails execution receipts are tied to the verified source bytes. Native checks use the canonical Rust 1.98.1/libvips toolchain image, the existing machine-wide compiler slots, build jobs 2 and nextest workers 4. No real Google HTTP or scanner-shaped tokens.

## Batch 2: Calendar entry and channel timeline lifecycles

Seven new named tests close ten ledger rows (the three entry declarations appear in both WS14e and WS14g). Added the missing direct `google_entry::destroy` model API using the existing `RemoteDeleteJob` type, with captured arguments published after commit. Added the callback-free association delete_all API and made the real disconnect producer consume it. Reconciliation's existing `delete` continues to skip remote work. Timeline tests assert exact singleton announcement count, creator, title/relative URL, reference rows, no edit/cancel posts, no message inbox items, and deletion of references without deleting the message.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_entry_test::) | test(cutover_timeline_test::)' --no-fail-fast
```

```text
Summary [   0.303s] 7 tests run: 7 passed, 1396 skipped
```

Original Rails files: `10 runs, 40 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **22 ledger records closed; 199 remain**.

## Batch 3: recurrence guards, ordering, uniqueness and local edits

Nine original recurrence declarations are closed. Eight model tests cover nil singleton series/neighbors, head update range limits and rollback, unchanged recurrence form values, the active-series-slot unique index (including a real rejected SQLite write), equal-time cancelled ordering, over-cap rule rollback, a retimed head-only series and a local description edit. The ninth runs a guard-flag injection through real HTTP (422, weekly unchanged); a compile-fail EventChanges doctest additionally proves the unknown attribute cannot be supplied to the typed model. Rust cannot dynamically assign unknown model fields as ActiveRecord does; this checks both its typed boundary and the observable request boundary.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_recurrence_test::)' --no-fail-fast
cargo nextest run -p campfire -j 4 -E 'test(cutover_recurrence_guard_flag_injection)' --no-fail-fast
```

```text
Summary [   0.325s] 8 tests run: 8 passed, 1403 skipped
Summary [   0.896s] 1 test run: 1 passed, 2853 skipped
```

Original Rails recurrence file (source bytes checked against d7c7de92): `63 runs, 360 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **31 records closed; 190 remain**.

## Batch 4: references, reminder occurrences and venues

Eighteen named model tests close the next eighteen declarations. References check missing/no links, exact event identity on edit, and message deletion preserving the event. Reminders use the real due-ID scan and per-event transactional claim, with frozen midnight and consecutive occurrence clocks. Venues cover optional/voice/stage, rejected text/DM/nonmember rooms and exact error, self-room venue, retained links after leaving, real room deletion, all/local propagation, clearing, and no inbox mutation for venue-only edits. No production divergence found.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_reference_test::) | test(cutover_reminder_test::) | test(cutover_venue_test::)' --no-fail-fast
```

```text
Summary [   0.726s] 18 tests run: 18 passed, 1411 skipped
```

Three original Rails files (source bytes matched d7c7de92): `35 runs, 117 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **49 records closed; 172 remain**.

## Batch 5: remaining Event HTTP, attendance, interaction and card assertions

Thirty-two named HTTP tests close the next thirty-two WS14e declarations: 24 event controller cases, two attendance propagation cases, three scheduling/inbox/card interaction cases and three card integration cases. Setup uses the original Rails fixtures, fresh real sessions, the real CSRF/router path and a frozen September 22 clock. Non-fixture seed rows are cleared before fixture loading; a seed-issued cookie is not reused after replacing its sessions.

The two whole-request index query guards exposed missing preloads. Before the fix the occurrence-count case reads **47 then 57** statements, and the shared-venue case reads **38 then 78**. Both original Rails guards pass. The index now loads events and all presentation associations in batches (organizers, attendance/counts, venues, venue membership/live streams, calendar copies and series neighbors), keeping the show reader intact. Both guards now pass without count growth; the room/event-link guard already passes against the owner renderer. No JS/CSS or asset fingerprints changed.

The interaction ports exercise the server assertions behind the original system flows: links and form fields, schedule redirects and heading/description/current response, series list labels, invitation identity/count, inbox open redirect, persisted responses, lazy card attendance frame and an in-frame response without navigation. A pinned Rails HTTP probe runs those same assertions: `reference-tools/cutover/events-interactions.rb`. It uses real Rails controllers/models and fixture users, not generated expectations from Rust. It passes **3 runs, 114 assertions, 0 failures, 0 errors, 0 skips**. No browser/pixel run is claimed.

The three original controller/attendance/card files also run unmodified in Rails: **79 runs, 498 assertions, 0 failures, 0 errors, 0 skips**. Their installed source bytes match d7c7de92. Targeted Rust rerun also includes the prior attendance-shape and event-page regressions:

```sh
cargo nextest run -p campfire -j 4 -E 'test(tests::cutover::) | test(pr174_) | test(event_pages)' --no-fail-fast
```

```text
Summary [  28.141s] 36 tests run: 36 passed, 2850 skipped
```

Total continuation at the original starting head: **81 ledger records closed by 78 named native tests; 140 remained**. The exact open declarations remain in the gate and are listed in `ledger-ws14-ws15-remaining.md`.

## Parent review reconciliation

While final verification was running, the parent PR advanced to `ec35cd92c74a07addc2f91e9f67207098c8b7d64`. Merge commit `553eb2196fe265346d75dca688f57112e921964a` retains its strengthened tests and receipt corrections, together with all 81 closures above. No continuation commits were pushed to the parent branch.

The parent review reopened four earlier receipt claims, which are preserved as open assertions rather than folded into this slice:

- **WS14e-051**: execute the registered reminder-push job with the Lounge venue and assert the queued push suffix.
- **WS14e-057**: execute the registered reminder-push job for a recently started but already ended event, with a still-running control.
- **WS15g-056**: execute the registered GitHub fetch-PR job for a mapped PR and assert the thread card/header broadcast title and file.
- **WS15g-057**: execute that job with neither references nor mappings and assert zero broadcasts plus the committed fetch.

Current disposition: **81 assertions closed in this slice; 144 remain (2 WS14e, 97 WS14g, 45 WS15g)**. This is the requested coherent stopping point at about 80 closures. The ledger retains the precise Rails declarations, missing assertions and current row locations for all remaining records.


## Final validation

Verified code: `553eb2196fe265346d75dca688f57112e921964a`, in a fresh local clone created with `git clone --no-hardlinks`, then advanced to the merged head. Only this report changes after that code head. Commands below run inside the canonical `campfire-toolchain-ci-rust-speedups:latest` image with Rust 1.98.1, canonical libvips, `CI=true`, build jobs 2, the existing machine-wide rustc slot throttle and nextest workers 4. The shared `rust/target` cache is retained; checkout inputs come exclusively from the fresh clone. No test timing threshold or concurrency setting changed.

All three current-schema seeds were rebuilt and validated by Rails before being copied into the fresh clone:

```sh
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed check-image
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed build
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed validate
```

```text
  "passed": 29,
  "failed": 0
ci-seed: default validated
  "passed": 4,
  "failed": 0
ci-seed: first_run validated
  "passed": 40,
  "failed": 0
ci-seed: agents_ui validated
```

Final full-workspace run:

```sh
cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast
```

```text
     Summary [1876.631s] 5034 tests run: 5034 passed (7 slow), 20 skipped
```

The earlier fresh-clone run before merging the parent review corrections also passed: `Summary [1748.683s] 5030 tests run: 5030 passed (7 slow), 20 skipped`. The final PASS log contains **78/78** unique test names from the 81 updated records; none of this slice's tests is skipped. These tests run under the existing blocking db/app CI package selectors in `.github/workflows/rust.yml`. The canonical `avatar_bot_logo_uploads_match_pinned_rails` comparison passes. No inherited flake appeared.

Strict lint check on the merged code:

```sh
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized] target(s) in 43.01s
```

Exit status 0. Two test-only helper lints encountered earlier were fixed by moving the regex outside its loop and returning the query count directly; no lint suppression was added. The affected HTTP tests were rerun:

```sh
cargo nextest run -p campfire -j 4 -E 'test(tests::cutover::)' --no-fail-fast
```

```text
     Summary [  20.911s] 33 tests run: 33 passed, 2853 skipped
```

Workspace doctests (the recurrence guard compile-fail case passes; the two existing kit examples remain ignored):

```sh
cargo test --workspace --exclude html5ever --doc --no-fail-fast -- --test-threads=4
```

```text
test crates/db/src/models/calendar_event/changes.rs - models::calendar_event::changes::EventChanges (line 20) - compile fail ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Ledger validation:

```sh
python3 rust/reference-tools/cutover/check-ws14-ws15.py
```

```text
Acceptance ledger: 445 records checked; 181 baseline-CI passed; 119 new assertions; 1 test-only outside gate; 144 explicitly open
```

Production-input binary build, excluding test vectors/reference tools and Rails source outside the explicit asset inputs:

```sh
bash ci/with-release-inputs.sh cargo build --workspace --bins
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 20s
```

Exit status 0. Own fresh-clone/Rails storage/scratch output is removed after the PR is opened; the shared target cache and inherited worktree output are preserved.
