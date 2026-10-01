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
