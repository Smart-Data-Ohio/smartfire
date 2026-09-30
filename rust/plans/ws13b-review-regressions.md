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
