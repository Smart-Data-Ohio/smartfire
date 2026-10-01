# Deflake 2: isolated fixture sockets and the WS13b parser failure

Based on freshly fetched origin/main 2a7c8b9b. Branch: rust/deflake-2.

## Root causes and changes

The Fizzy message HTTP matrix launches isolated child processes to configure their API origin. Every child previously rebound 127.0.0.1:51598, also used by the Rails OpenGraph oracle. Concurrent worktrees or an oracle sharing the network namespace could cause AddrInUse before assertions ran. The parent now binds 127.0.0.1:0 and passes the live socket through stdin into the child, with its actual address in the child's FIZZY_API_BASE_URL. The child verifies that address and consumes the same listener. There is no release/rebind window, environment mutation, port search or retry. The mutation checker uses the parent handoff for message cases and accepts a caller-owned target directory; its test thread count is eight.

The Rails oracle now binds port zero. The recorded logical URL, Host header and metadata are independent of its physical dial target. Regenerating opengraph_expected.json with the previous port occupied and the four Fizzy form branches from pinned Rails d7c7de9264c63015be398001d7a1094e7695a6db left both files byte identical. Comparisons, vectors, masks, production code, timing thresholds and test concurrency are unchanged.

The WS13b log at .claude/delegation/rust-port/wave4/sol/ws13b-ws17.log:18344 identifies integrations::opengraph::document::tests::parses_pathological_pages_quickly. It charged scheduler delays to parser work using Instant, failing at 1.087637689s on the many-meta-tags page. PR #169 already fixed precisely this test with CLOCK_THREAD_CPUTIME_ID on main. This branch retains that fix and its one-second budget; it does not claim a new parser fix.

## Reproduction and loaded verification

Native builds used mise exec rust@1.98.1, --locked and -j2 with the global compiler wrapper enabled. Test commands always used --test-threads=8. Load came from sixteen CPU-only Python worker processes; the initial light parser attempt used eight and did not reproduce in ten attempts. Full loaded checks used sixteen.

- Port: one pre-fix matrix attempt with an owned listener holding 51598 failed (AddrInUse); its exact form test passed. The Rails oracle also failed with EADDRINUSE with that port held inside its own container. After: three of three loaded matrix/form runs passed while the port remained occupied, covering all seventeen child cases per matrix run. The oracle then passed with byte-identical output under the same collision condition.
- Parser: temporarily restoring only the original Instant assertion reproduced one failure in ten eight-thread OpenGraph suite runs during concurrent builds (1.661885458s on the same page, 55 passed / 1 failed). All temporary test changes were restored. The unchanged CPU-clock version passed ten of ten identically loaded OpenGraph suite runs (56 tests each) during concurrent builds. Initial light-load non-reproductions are recorded separately, not omitted.
- The adapted write mutation harness detected both mutants: bypassed sudo protection and the locked-thread guard. The latter failed the unchanged no-upstream-requests assertion, not socket setup. Production sources were restored afterward.
- Pinned default and first_run seeds validated against Rails: 29 and 4 checks passed, zero failed.
- Fresh clone created using git clone --no-hardlinks --single-branch --branch rust/deflake-2, with only the validated seeds and a private Cargo cache copied in. CI's cargo.sh used ws15e-media-toolchain:latest for pinned media-byte checks. Each Docker build held one slot of the unchanged machine-wide rustc throttle and used --build-jobs 1/-j1. Tests used eight threads and sixteen CPU load workers. No pixel work.

## Raw results

```text
port-before:
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 802 filtered out; finished in 1.26s
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 801 filtered out; finished in 1.31s
port-after:
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 801 filtered out; finished in 40.10s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 801 filtered out; finished in 53.30s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 801 filtered out; finished in 51.83s
parser-before:
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: FAILED. 55 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
parser-after:
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.00s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.01s
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 747 filtered out; finished in 5.00s
Native mutation harness:
1 fizzy_connections.rs sudo: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 802 filtered out; finished in 0.99s
2 fizzy_message_cards.rs locked: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 802 filtered out; finished in 2.47s
WS15e Fizzy write mutation checks: 2 detected, 0 survived
Fresh clone:
build:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5m 20s
app-1:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 32s
     Summary [ 152.166s] 800 tests run: 800 passed, 3 skipped
app-2:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 32s
     Summary [ 146.927s] 800 tests run: 800 passed, 3 skipped
workspace:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 36s
     Summary [ 244.565s] 2089 tests run: 2089 passed, 10 skipped
docs:
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.26s
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
clippy:
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 46s
TOML check: 15 manifests parsed; 76 workspace dependency keys; no duplicate keys
Rails oracle with port 51598 held: byte-identical output
WS15e Fizzy message form Rails oracle: 4 form branches
```

## Reproduction commands

Logs, temporary reproduction drivers and result JSON are retained in the ignored rust/target/deflake/deflake-2 directory. The native port probe holds a stdlib Python TCP socket on 51598 while invoking the campfire test binary with controllers::fizzy_message_cards::tests:: --test-threads=8 --nocapture. The parser probe uses integrations::opengraph:: --test-threads=8 --nocapture. The old assertion changes only cpu_time to Instant::now/elapsed; its source was restored before the implementation commit.

Rails fixture checks:

```sh
PARITY_IMAGE=ws19b-ci-reference PARITY_NAMESPACE=deflake2 PARITY_OWNER=deflake2 \
  rust/parity/bin/reference runner rust/target/deflake/deflake-2/oracle-port-owner.rb
PARITY_IMAGE=ws19b-ci-reference PARITY_NAMESPACE=deflake2 PARITY_OWNER=deflake2 \
  rust/parity/bin/reference runner --seed default \
  rust/reference-tools/embeds/fizzy_message_form.rb /work/vectors/ws15e_fizzy_message_form.json
rust/parity/bin/ci-seed validate
git diff --exit-code -- rust/crates/campfire/src/integrations/testdata/opengraph_expected.json \
  rust/vectors/ws15e_fizzy_message_form.json
```

## Fresh-clone commands and scope

The private clone tested implementation commit `95f275c3`. The later report commit changes only this report and the mutation tool's target-directory override, which was exercised by the two native mutation checks. Rust source, fixtures and manifests remain identical to the fresh-clone inputs.

The CI adapter was invoked through a held machine-wide compiler slot. Its cargo arguments were:

```sh
cargo test --locked -j1 --workspace --exclude html5ever --no-run
# Twice, with CI=true and sixteen CPU workers:
cargo nextest run --locked --build-jobs 1 --workspace --exclude html5ever \
  --profile ci --no-fail-fast --test-threads 8 -E 'package(campfire)'
# Third full app run plus all other crates, under the same load:
cargo nextest run --locked --build-jobs 1 --workspace --exclude html5ever \
  --profile ci --no-fail-fast --test-threads 8
cargo test --locked -j1 --workspace --exclude html5ever --doc --no-fail-fast -- --test-threads=8
cargo clippy --locked -j1 --workspace --exclude html5ever --all-targets -- -D warnings
```

The workspace JUnit confirms 800 app tests passed, for three consecutive full app passes in total. There are ten existing ignored workspace tests, including the three app ignores: `channels::tests::golden::record_reference`, `controllers::presenters::accounts::tests::manages_bots`, and `jobs::tests::push_latency`. No ignores were added. All eleven doctest targets completed without failures, retaining two existing ignored examples. The pinned media pipeline test passed rather than skipping byte comparisons.

`cargo metadata --locked --format-version 1` returned zero. The manifests parsed without duplicate keys. The clone's tracked diff was empty after verification. Its Cargo target, Cargo cache, runner temporary files, and the entire owned `.scratch` directory were removed after results were retained. No test processes or containers were left running.
