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
