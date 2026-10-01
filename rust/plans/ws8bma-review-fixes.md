# PR #182 review fixes

Reviewed input: `9883d3fa1d2f797f84fd3e96e9e7e8472435cdb9`.
The PR was conflicting, so `origin/main` was merged in `2139a2b6` before the fixes.
The one thread-header conflict retains the owner's adapter and main's render-zone behavior.
Rails oracle: `d7c7de92`, with the approved #163 layout/assets drift unchanged.

## Changes

- Shared message fragments retain the exact Rails collection-helper key and add the
  exact `MessagesController#index` validator composition over the message and its
  rendered reply source. This covers source edits, legacy `edited_at`, and both
  creators' updates without changing database touches. The original collection-key
  goldens remain unchanged. Cache witnesses use the actual stored key.
- Reaction resolution and classification use the shared Ruby `String#strip`
  implementation. The human controller and database resolution path were audited;
  their shortcode pattern rejects Unicode whitespace before icon lookup. The remaining
  `trim_matches(':')` in the reaction module handles registry alt text, not whitespace.
- The token-leak mutation anchors the `Arc` conversion independently of surrounding
  formatting. The GitHub omission mutation targets the actual preloaded renderer.
  Mutation checks honor the caller's test port ranges.

The new `cache-reaction-review.rb` oracle records actual Rails room/edit/profile
requests, both exact key compositions, and eight repeated boost inputs. The committed
vector was regenerated from fresh seeds and reproduced byte for byte. Its reply has
a different creator from the source. Source edits and source-author renames occur
within one second, exercising microsecond keys. Both new regressions failed before
their fixes; the stronger different-author case also failed before adding the source
to the validator's records.

Public room-list/composer inputs are unchanged. This is a bounded review-fix slice;
the existing messaging behavior ledger's deferred flows retain their previous status.

## Verification

The final checks use Rust 1.98.1 in the pinned media toolchain, freshly generated
`default` and `first_run` seeds, `CI=1`, two build jobs, eight test threads, and ports
54000–54049. Docker compilers share the unchanged host rustc slot pool. The release
input build runs the requested CI wrapper with one compiler under one host slot.
The fixture additions contain no scanner-shaped secret strings.

All required gates passed. The full workspace run includes `html5ever` and doctests:
**3375 passed, 0 failed, 12 existing ignores**, across 60 summaries. No missing-seed
skips occurred. Locked metadata completed with exit 0. Both named mutations reached
their intended assertions and were rejected; production sources were restored.

Commands:

```sh
cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo metadata --locked --format-version 1
bash rust/ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
PARITY_IMAGE=triage-reference-d7c7de92 python3 rust/reference-tools/messaging/check-goldens.py cache-reaction-review
python3 rust/reference-tools/messaging/check-owned-mutations.py cached-token-leak github-card-omitted
```

Raw summary lines (terminal color escapes removed from the CI build line):


Before the P2 fixes:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.27s
```

Before including a different source author:

```text
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.09s
```

Final P2 regressions:

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1714 filtered out; finished in 2.11s
```

Final mutation rejections:

```text
cached-token-leak: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1715 filtered out; finished in 2.09s
github-card-omitted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1715 filtered out; finished in 0.81s
WS8bm owned mutations: 2 rejected; 0 survived; production files restored
```

Full workspace tests and doctests:

```text
test result: ok. 1713 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 847.70s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.88s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 951 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 85.49s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.46s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 53 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.97s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.32s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.96s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.97s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.18s
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.41s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Strict Clippy:

```text
    Finished `dev` profile [unoptimized] target(s) in 57.50s
```

Release-input build:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 39s
```

Rails oracle reproduction:

```text
WS8bm golden check: 1 Rails oracles re-run; 1 golden files byte-identical
```

Pinned Rails source verification:

```text
WS8bm reference source check: 63 controller, model, helper, template and icon files match d7c7de92
WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected
```
