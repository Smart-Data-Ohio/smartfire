# WS11 API PR214 review fixes

Tested source: `92d7706bb29cadc46240ca217db262934f25a71f`; current main `15653de605ca608d56003d5645cfcf762c99c33a` merged with `94b107f15`. Started with no tracked WIP; existing scratch receipts were preserved. Work remained on `rust/ws11api-next-3`. The final report commit changes documentation only. Cargo metadata passed locked after the merge; the final fetch confirms zero unmerged main commits. Fresh clone: `.scratch/pr214-fresh` (`git clone --no-hardlinks --single-branch`); all three pinned Rails seeds regenerated. Rust source tests were executed before changing production code, then again after the repair.

## Findings fixed

1. **P2 ID casting:** `controllers/agents/reads.rs` restores Ruby vertical-tab whitespace and signed `0d`/`0D` decimal prefixes. Empty/invalid decimal continuations retain numeric zero (`0d_1`); other radix prefixes retain decimal `to_i` behavior rather than switching bases. Non-numeric strings and overflow stay rejected. The repaired array/singleton/null/hash semantics and scoped selection remain unchanged.
2. **P3 scheduling timestamp:** `models/agent_delivery.rs` captures the UPDATE timestamp once and returns it. `models/agent_work_events.rs` returns the successful conditional UPDATE's timestamp to its caller; no clock reread or database reread occurs. Existing permission checks, webhook status transitions and atomic jobs remain intact. Two real advancing-clock tests in `tests/agent_event_clock_test.rs` compare returned/stored values against freshly executed Rails equality/delta receipts; both use 1µs per call.
3. **P3 blank step parent:** `controllers/agents.rs` applies Rails presence before integer coercion, so `[]` is absent. The two MCP cases now return the exact required-parent error with unprocessable_entity service status (422), rather than not_found (404). REST controls retain 422. No step or job writes occur; normal agent/credential authentication usage timestamps match Rails. The existing step service retains authorization and validation ownership.

No approval mask, timing threshold, worker limit or permission rule was widened. The generators and comparisons are committed under `reference-tools/agents` and `vectors`; the standard HTTP recapture/verification scripts include the new corpus and clock receipts.

## Corpus and differentials

`pr214_id_corpus.rb` executes real Active Record predicates against rows including zero, signed-i64 endpoints, negative Alpha, Alpha and Beta. It crosses eleven positions (scalar, singleton, null/singleton, nested singleton, either array position, mixed object, mixed nested, nested multi, and nested-null singleton) with 2,681 scalar strings: **29,491 freshly executed predicates**. Whitespace classes, signs, 0d/0D/0x/0X/0b/0B/0o/0O, underscores, decimal tails and overflow are combined. This is a finite enumerated corpus, not a claim to exhaust arbitrary strings.

The committed oracle preserves literal inputs and all selected IDs. The original parser has 6,426 mismatches; the repaired parser has zero. The extended HTTP matrix has **139 cases / 145 responses**, zero strict differences. All response bytes/status/selected headers, every column of 20 projected tables and jobs are compared, without masks. Both original #210 repaired array patterns remain in that matrix.

Fresh Rails timestamp receipts retain both exact timestamps, status and total clock calls. Rust and Rails both have zero returned/stored delta; runtime call counts are observed, not forced to match. Rust's two scheduling regressions also compare metadata/creation timestamps after rereading the stored row.

Fresh Rails and source-check receipts:

```text
PR214 Rails ID corpus: 2681 scalar forms; 11 positions; 29491 executed predicates
WS11 array-shape oracle: 139 cases; 145 responses; all state columns; lossless shared snapshots
WS11 source case files: 26 pinned Git files matched; 0 checkout mismatches
```

## Failing first

Regression-only commit `3be55eb7e` preserved production code from `bbfdf9630` plus current main. These executed failures precede repair `b482e2092`:

```text
PR214 ID corpus: 2681 scalar forms; 11 positions; 29491 cases; 6426 mismatches
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2643 filtered out; finished in 0.23s
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1319 filtered out; finished in 0.15s
```

```text
PR214 advancing-clock returned=Some(2027-01-15 08:00:00.000002) stored=Some(2027-01-15 08:00:00.000001) receipt={"kind":"approval_decided","webhook_status":"pending","equal":false,"delta_microseconds":1}
PR214 advancing-clock returned=Some(2027-01-15 08:00:00.000002) stored=Some(2027-01-15 08:00:00.000001) receipt={"kind":"work_assigned","webhook_status":"pending","equal":false,"delta_microseconds":1}
```

```text
WS11 next3 array shapes: 139 cases; 20 strict differences
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2643 filtered out; finished in 60.26s
```

The 20 strict HTTP leaves occur in the eight parser reproductions and the two MCP empty-parent cases. REST controls already matched. No state/job differences were present. After repair:

```text
PR214 ID corpus: 2681 scalar forms; 11 positions; 29491 cases; 0 mismatches
```

```text
PR214 advancing-clock returned=Some(2027-01-15 08:00:00.000001) stored=Some(2027-01-15 08:00:00.000001) receipt={"kind":"work_assigned","webhook_status":"pending","equal":true,"delta_microseconds":0}
PR214 advancing-clock returned=Some(2027-01-15 08:00:00.000001) stored=Some(2027-01-15 08:00:00.000001) receipt={"kind":"approval_decided","webhook_status":"pending","equal":true,"delta_microseconds":0}
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2643 filtered out; finished in 0.21s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1319 filtered out; finished in 0.17s
```

```text
WS11 next3 array shapes: 139 cases; 0 strict differences
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2643 filtered out; finished in 69.77s
```

## Exact commands and raw gates

Fresh-clone environment: CI=1, CARGO_BUILD_JOBS=2, test/dev debug=0, RUST_TEST_THREADS=8, target `$PWD/rust/target`, TMPDIR `$PWD/.scratch/tmp`. Cable ports 52900–52919; integration/mail/GitHub ports 52920–52949. Existing machine-wide rustc throttle retained. Rails image `ws11api-reference:d7c7de92`; one Rails generator at a time.

Commands from the primary worktree:

```sh
mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1 --manifest-path rust/Cargo.toml
PARITY_IMAGE=ws11api-reference:d7c7de92 PARITY_NAMESPACE=ws11api-pr214 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/pr214_id_corpus.rb
PARITY_IMAGE=ws11api-reference:d7c7de92 PARITY_NAMESPACE=ws11api-pr214 rust/parity/bin/reference exec --seed default bin/rails runner /work/reference-tools/agents/pr214_event_clock.rb
PARITY_IMAGE=ws11api-reference:d7c7de92 PARITY_NAMESPACE=ws11api-pr214 python3 rust/reference-tools/agents/record-array-shapes.py .scratch/pr214/shapes
python3 rust/reference-tools/agents/check-case-ports.py
python3 rust/reference-tools/agents/test-case-ports.py
```

The initial ID generator fixture missed a required last_activity_at; that setup error is excluded from failing-first evidence. The corrected generator executed every reported predicate. Fresh-clone commands (focused tests executed before and after; other gates on the repaired source):

```sh
rust/parity/bin/ci-seed prepare
rust/parity/bin/seed build default first_run agents_ui
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire -p campfire_db --no-fail-fast pr214 -- --nocapture --test-threads=8
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml -p campfire ws11_next3_array_shapes_match_rails_full_state -- --nocapture --test-threads=8
mise exec rust@1.98.1 -- cargo clippy --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --all-targets -- -D warnings
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked -j2 --bin campfire
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="python3 $PWD/rust/reference-tools/agents/pinned-media-runner.py"
mise exec rust@1.98.1 -- cargo test --locked -j2 --manifest-path rust/Cargo.toml --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

The existing runner executes six native-version-sensitive application media tests and storage byte vectors in the pinned runtime; all other tests run natively. Every byte/size/checksum assertion remains. Existing ignores remain explicit in the raw summaries.

CI preparation and seeds:

```text
pin=d7c7de9264c63015be398001d7a1094e7695a6db
image_key=rust-parity-image-v1-a84a88b580bc609efb524959436495899ce3982988f435cc509b6ea4e51a7007
seed_key=rust-parity-seed-v1-297ed8903d9f24799ddc69f1654e63e1ad014aa45a96dfbdc2ba854cdf2c52e9
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

Full workspace raw summaries:

```text
test result: ok. 2631 passed; 0 failed; 7 ignored; 0 measured; 6 filtered out; finished in 493.54s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2638 filtered out; finished in 2.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.53s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1317 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 121.80s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.98s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.23s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.26s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.67s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.74s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.65s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.77s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.06s
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
```

```text
PR214 fresh workspace totals: 4683 passed; 0 failed; 16 ignored; 60 raw summaries
```

Strict clippy / release-input build:

```text
Finished `dev` profile [unoptimized] target(s) in 1m 24s
```

```text
Finished `dev` profile [unoptimized] target(s) in 2m 00s
```

Checker controls:

```text
..
----------------------------------------------------------------------
Ran 2 tests in 0.001s

OK
```

Remaining scope: none of the three review findings remains. Prior peer-owned evidence obligations are outside this review-fix scope; no peer service was replaced. The sole owned scratch target was cleaned, with no running owned tests or model-server changes.
