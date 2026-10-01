# PR #172 failing-first evidence

Base implementation: `b38bbedd8bad5859b56dbf189a58baeea2497f8b`. Before any production edits, only regression tests and the independently generated Rails oracle were added. Both commands used eight test threads. All eight tests reached their assertions and failed; there were no compilation failures in these recorded runs.

```sh
CARGO_BUILD_JOBS=2 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_db --lib ws13b_review -- --test-threads=8 --nocapture
CARGO_BUILD_JOBS=2 CI=1 CABLE_TEST_PORT_RANGE=53000-53049 MAIL_TEST_PORT_RANGE=53050-53099 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire --bin campfire ws13b_review -- --test-threads=8 --nocapture
```

```text
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 803 filtered out; finished in 0.08s
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 869 filtered out; finished in 0.49s
```

- Queued ring after recipient removal published room/caller metadata. After issuer revocation, a queued ring published `huddle_started/unread` after the immediate `huddle_ended` frame. The real worker retaining pre-leave arguments also restarted an explicitly ended call. The two domain ring tests were subsequently moved, unchanged, from `huddle_grant_test.rs` to `huddle_ring_revocation_test.rs`.
- Rejecting the invitation INSERT left zero grants, against Rails' one committed grant. Reissuing an existing grant retained `12:00:00`, against Rails' committed `12:03:00` issuance.
- The offset parser returned `17:00:00Z` against Rails' `12:00:00Z`; the HTTP request cleared a `12:00:01Z` rejoin. `24:01:00` normalized to the following day instead of being rejected.

The independent probe calls actual Rails issuance, lifecycle callbacks, and the private gateway parser. Rails rings synchronously; it has no deferred ring job. The Rust ordering regression therefore checks that its additional asynchronous step cannot emit a started frame after Rails' ended/removal sequence. Actual Rails job-enqueue notifications establish that no ring job is queued.

Additional pinned probes retain Date._parse's actual zone/offset fields. Compact `+0560` normalizes to six hours, colon offsets with invalid minute/second components have no offset, and total offsets of one day are rejected by Time.new. These extend the original failing-first parser regression to 22 timestamps without changing its original failure on `b38bbedd`.

Fresh verification at `64ec5b18` also exposed a producer-assertion race in the existing gateway vector test. With the real job registry running, `removed_member` observed zero pending jobs against Rails' one cleanup enqueue after a worker consumed the row. The test now audits durable INSERTs transactionally in its private database, preserving rollback behavior and normal worker/test concurrency. No production code, threshold, or expected Rails vector changed for this test correction.

## Re-review: superseded invitation generations

At `e9ddf5d920ff9363ec68c42c08623576ebedd4d9`, the reviewer's unchanged `tests::huddle_grant_test::reviewer_delayed_handled_job_must_not_become_a_second_retry_ring` reached its assertion and failed with `left: 2`, `right: 1`: the queued handled update became another unread retry ring after the same grant was reissued 181 seconds later.

```sh
CARGO_BUILD_JOBS=2 TMPDIR="$PWD/.scratch" mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml -p campfire_db --lib reviewer_delayed_handled_job_must_not_become_a_second_retry_ring -- --test-threads=8 --nocapture
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 808 filtered out; finished in 0.10s
```

`ws13b_ring_generations.rb` independently probes six actual Rails retry sequences. Each emits one unread retry frame. Rails resets item `created_at` when refreshing the row; banner-only invitations use the grant's refreshed `last_issued_at`. Rust jobs now capture that generation in UTC microseconds and reject superseded or unversioned requests before rebuilding metadata. An older state/type update also cannot be promoted into another state's ring. The new corpus checks both drain orders, while the reviewer's post-commit and legitimate-retry-after-end probes remain executable.

## Third-pass pending banner, failed first on 3f82457a

Before production edits, the reviewer's exact real-handler regression was copied unchanged from the read-only review clone. It retained the queued job's original arguments, reissued the grant one second later without a replacement enqueue, then drained through the real handler and ActivityChannel.

```sh
CARGO_BUILD_JOBS=2 CI=1 TMPDIR="$PWD/.scratch" CABLE_TEST_PORT_RANGE=53000-53049 MAIL_TEST_PORT_RANGE=53050-53099 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever reviewer_r3_real_handler_keeps_pending_ring_after_deduped_reissue -- --test-threads=8 --nocapture
```

```text
assertion `left == right` failed: suppressed=true: a deduped issuance discarded the only durable ring
  left: 0
 right: 1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1000 filtered out; finished in 1.80s
```

The delivery rule was written first in `ws13b-ring-delivery-rule.md`. Rails does not have an asynchronous in-app ring job: its actual asynchronous invitation job is a push job identified by item ID. The immutable Rust queue row identifies its extra deferred emission. Only an actual later invitation emission supersedes that row, atomically with the new enqueue. Dedupe is a no-op for that identity; banner last_issued_at is not an invitation generation.

The 180-case Rails-generated matrix records actual synchronous emissions and push enqueues, full item state, and an explicitly documented deferred-delivery projection. Rust uses real queue persistence, retained worker arguments, its real ring handler and ActivityChannel. Initial and legitimate retry banners survive deduped reissues; genuinely newer emissions supersede old jobs; stale rings do not survive access removal or call end. No fixture masks or timing thresholds change.

## Fourth pass: cross-form and handled/end failures, first on 09bd4b62

The three unchanged real-handler reviewer probes were appended before production
edits or the main merge. `reviewer_r4_` failed in all three tests: a banner/item
retry delivered two rings in either order; dismissing the retry still delivered
an unread old banner; handling then ending delivered zero handled frames against
one. Command (at 09bd4b62 plus the new tests only):

```sh
CARGO_BUILD_JOBS=2 CI=1 TMPDIR="$PWD/.scratch" CABLE_TEST_PORT_RANGE=53000-53049 MAIL_TEST_PORT_RANGE=53050-53099 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever reviewer_r4_ -- --test-threads=8 --nocapture
```

```text
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1006 filtered out; finished in 2.83s
```

The former 180-case delivery projection is removed. Its named operation sequences
remain, but expectations now come exclusively from actual Rails Cable broadcasts
and actual serialized ActiveJob executions, including real push-pool handoffs.
The pinned Stimulus controller consumes those frames with timers after every
step. Rust now broadcasts after commit synchronously, matching Rails. Its marked
queue envelopes acknowledge without replay. Legacy persisted envelopes are
explicitly simulated by removing only the new delivered marker; their prior
regressions remain meaningful and executable.

## Random differential: identical-time handled callbacks

Seeds: `388013012` and `3620200082`. The random differential found one additional
production defect: a handled update that changes neither read_at nor handled_at
still broadcasts in Rust. Rails' `saved_change_to_*` guard emits no callback.
Every mismatch was delta-debugged against real Rails and the compiled pre-fix
Rust test executable. Ten minimized sequences (3–5 operations) are committed in
`reference-tools/ws13b_shrunk_sequences.json`, with original seed/index in each
name. They cover repeated handling directly, handling immediately after rejoin,
and switching from a banner to an item before repeating the answer.

The minimized observed corpus fails on the pre-fix executable:

```text
test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 893 filtered out; finished in 0.50s
```

All ten reach frame assertions, not compilation failures. Named tests additionally
identify the three minimal scenario families. `ActivityItem::mark_handled` now
matches Rails' timestamp-change guard (`activity_item.rb:192,215-218`).

The recorder also exposed two harness isolation errors: deleting SQLite rows did
not establish independent ID baselines, and restoring a database did not clear
Rails' query cache. Each case now restores a byte-identical fixture database,
clears query caches, Current and Rails cache. A 199-case prefix agrees exactly
with standalone replay. These were recorder repairs, not application fixes or
comparison masks. The comparator rejects five injected output corruptions:
lost/duplicate frames, changed metadata, changed push payload and banner state.

## Fifth pass: differential gate failures, first on 35ed3a80

No production mismatch was found. Two gate regressions were committed alone at
`9f9d05ea`, before implementation or the main merge. An actual duplicate
`PushInvitationJob` event was emitted into the test sink against the unchanged
Rails `issue → drain` case. The old gate accepted it. The banner-dismiss case
read a historical inbox item when the displayed banner had no read path.

```sh
CARGO_BUILD_JOBS=2 CI=1 TMPDIR="$PWD/.scratch" CABLE_TEST_PORT_RANGE=53000-53049 MAIL_TEST_PORT_RANGE=53050-53099 mise exec rust@1.98.1 -- cargo test --locked --manifest-path rust/Cargo.toml --workspace --exclude html5ever gate_review_r5_ -- --test-threads=8 --nocapture
```

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 904 filtered out; finished in 0.80s
```

The new gate compares every pending source job after every step, including class,
complete arguments and absolute scheduled time, before using the recorded drain
indices. Drains without a limit must select all jobs exactly once and leave none.
Named controls exercise both invariants. The committed mutation script additionally
duplicates the actual producer's enqueue, proves that two jobs versus Rails' one
fails at the issue step, and restores the source before proving the baseline passes.

The Rails recorder now runs the actual pinned Stimulus controller interactively.
Its actual fetch requests drive the accessible item read action; the Rust driver
replays those inputs and an independent controller replay on Rust frames must
produce the same requests. The historical-item/banner scenario observes no PATCH
and leaves the item unread. Dismissing a displayed item sends a PATCH and reads
that item; an explicit inbox read behind a banner is a distinct operation.
No driver infers the displayed invitation from the most recent database item.
Three new Rails-observed named cases cover those actions. Eleven corruption
controls now include queue and UI request changes. No production code or timing
threshold changed for these gate fixes.
