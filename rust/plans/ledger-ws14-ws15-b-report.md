# WS14 / WS15 acceptance continuation

Stacked on PR #239 head `ca92288e839e9dd8e97f83dc236ae8f889ddd3c4`; branch `rust/ledger-ws14-ws15-b`. Parent branch is unchanged. Batches follow the remaining ledger's Rails file/declaration order. Each new named test reproduces the original setup and discriminating assertions through the real Rust model/HTTP/job path. Duplicate declarations in the two inventories share the same named test and are counted explicitly as ledger records.

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
