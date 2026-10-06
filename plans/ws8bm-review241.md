# WS8bm PR #241 scope correction

Review baseline: `ca205d8fb77492dc3b226c0ef53433b2afb92916`. Work stays on `rust/ws8bm-deferrals`.

The extracted native release assertion now retains main's exact active-composer scope: `#composer [data-composer-target="context"][hidden]`, with `visible: false` and Capybara's unchanged two-second default. The original body is unchanged apart from naming the extracted test method and preserving this pre-existing scope. A source comparison test checks every other byte, including the original source line 83. The pinned 700 ms long press, ten-second menu wait, message-center hit test and `menu` equality remain intact.

The causal race correction remains the reviewed Selenium outer-window resize and W3C touch sequence. The old translation treated 390×844 as the content viewport; its cold menu started below the original press point. Native resize gives the original content viewport and menu geometry. The tighter composer scope adds no readiness sleep, retry or replacement assertion.

The served `unrelated-hidden-context` fault keeps the real release-click guard, unhides the real composer's context, and puts a hidden clone in a separate idle form. Rejection credit requires the original post-click line-83 scoped assertion, a witnessed menu hit/click, the actual visible composer context and the hidden idle sibling. Earlier menu/geometry/startup failures cannot earn credit. Default release suppression attribution remains unchanged.

## Failing-first evidence

The scope-only source regression fails against the old extractor (`scope-unit-before.log`): **2 passed / 1 failed**. The reviewer's served sibling-context fault passes on both old extractions (`scope-before-valid.log`), with diagnostics showing `composer.hidden=false`, an unrelated form's context hidden, and the original point hitting the menu. This is diagnostic escape evidence and earns no parity credit. The first baseline-loader launch stopped at a missing export before the browser case (`scope-before.log`); it earns no credit. The corrected loader exposes only the new registry metadata while continuing to execute `ca205d8fb`'s original extractor and guard. No automatic retries run.

Receipts for this correction are under `.scratch/ws8bm-review241-fix/`. Each CLI invocation uses `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=4`, `CI=1`, the worktree's `rust/target`, `WS8BM_BROWSER_PORT_BASE=22020` and `WS8BM_DISCRIMINATION_RETRIES=1` (one attempt).

## Browser replays

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --repeat 10 --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --negative --keep-going
WS8BM_SETUP_DELAY=250 WS8BM_OPEN_FOCUS_DELAY=400 python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --negative --keep-going
```

Scoped stock release positives (`release-ten.log`): **10/10 Rails and 10/10 Rust**, each with persisted-row comparison. Default suppression and hidden-sibling faults (`release-negative.log`) reject on both hosts at the intended assertion. Raw lines:

```text
WS8bm behaviour repetition: 10 paired attempts; 1 named declaration; 0 failed
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 2 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

The label “bounded fresh-fixture retries only” is generic runner output; the attempt limit is one, and no retry ran.

The scroll negative (`scroll-negative.log`) is a new clean paired proof, superseding the network-invalid Rails review attempt. Both hosts reach `behavior-motion.mjs:161` (`motion: closed drawer keeps offset`); the actual mutated closed scroller has `display:none` and zero boxes. Both attribution records have `valid:true`, no reasons and no network failures. Raw lines:

```text
WS8bm discrimination: motion: mobile drawer keeps the room list scroll position across close and reopen: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: motion: mobile drawer keeps the room list scroll position across close and reopen: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

The additional real-metadata-hold control (`release-metadata-delay.log`) keeps the restored `#composer` scope and passes on both hosts with unchanged persisted rows:

```sh
WS8BM_SETUP_DELAY=250 python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --keep-going
```

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

Thus there are eleven current scoped positive pairs: ten independent stock attempts and one controlled metadata-delay scenario. The ten repetitions are not retries.

## Helpers and inventory

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*test.py'
node --test --test-concurrency=4 rust/reference-tools/messaging/*.test.mjs
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

Raw summaries:

```text
Ran 41 tests in 9.724s
OK
ℹ tests 71
ℹ pass 71
ℹ fail 0
ℹ skipped 0
WS8bm system inventory: 135 named declarations; 135 mapped behaviour passes; 0 deferred; 0 WS12 blocked; no pixel checks
```

## Fresh-clone Rust verification

A clean shared-object clone at `.scratch/ws8bm-deferrals/fresh` checks out the source correction commit `f0eb81c25`. It builds its own `default`, `first_run` and `agents_ui` seeds. The pinned reference's required schema overlay matches the checkout (SHA256 `68f02627974758d3944e968fa61d4112986db229c938bc82fbfbedd6eaa7fc88`). The validators report:

```text
  "passed": 29,
  "failed": 0
  "passed": 4,
  "failed": 0
  "passed": 40,
  "failed": 0
```

Nextest 0.9.146 is extracted from the existing `campfire-toolchain-ci-rust-speedups` image into this task's `rust/target/ws8bm-tools`; it is not installed globally. Its `ci` profile has zero retries. The unchanged machine-wide compiler throttle remains active; Cargo build jobs are two, and nextest test workers are four. The existing task-local runner selects the canonical libvips 8.16.1 / FFmpeg 7.1.5 runtime in `rust/target/ws8bm-media`; its generation command and complete runner are recorded in [the cutover report](ws8bm-deferrals.md). This is execution configuration, not an untracked fixture dependency. Compiler output stays in the worktree cache. The clone's approximately 1 MB JSON/JUnit output target is removed with the clone.

Seeds and validators ran from the fresh clone's `rust/`; Cargo ran from the worktree root with the fresh manifest. `CI=1`, `RUST_TEST_THREADS=4` and `CARGO_BUILD_JOBS=2` apply. Exact Cargo invocations (the runner configuration is the same for every nextest child):

```sh
PARITY_IMAGE=ws8bm-deferrals-reference-d7c7de92 parity/bin/seed build default first_run agents_ui
PARITY_IMAGE=ws8bm-deferrals-reference-d7c7de92 parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb default
PARITY_IMAGE=ws8bm-deferrals-reference-d7c7de92 parity/bin/reference runner --seed first_run --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb first_run
PARITY_IMAGE=ws8bm-deferrals-reference-d7c7de92 parity/bin/reference runner --seed agents_ui --time 2026-03-02T16:00:00Z --freeze reference-tools/campfire/verify_parity_seed.rb agents_ui
env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1 CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/.scratch/ws8bm-deferrals/canonical-runner.sh" PATH="$PWD/rust/target/ws8bm-tools:$PATH" mise exec rust@1.98.1 -- cargo nextest run --manifest-path .scratch/ws8bm-deferrals/fresh/rust/Cargo.toml --locked --workspace --exclude html5ever --build-jobs 2 -j 4 --profile ci
env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1 CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" mise exec rust@1.98.1 -- cargo clippy --manifest-path .scratch/ws8bm-deferrals/fresh/rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -j 2 -- -D warnings
```

Full nextest exits 0 (`nextest.log`). Strict clippy exits 0 (`clippy.log`). Raw terminal lines:

```text
    Starting 4948 tests across 48 binaries (20 tests skipped)
     Summary [1869.880s] 4948 tests run: 4948 passed (8 slow), 20 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.94s
```

There are **4,948 passed / 0 failed / 20 existing skips**. Nextest does not run doctests; the two ignored doctests in the historical libtest receipt are not added to its count. No new filter, ignore or retry was added; the workspace keeps its standard `html5ever` exclusion.

The release-input-only binary build also exits 0 (`release-build.log`), from the worktree root:

```sh
env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1 CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" bash .scratch/ws8bm-deferrals/fresh/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 50s
```

## Cleanup

The verified task-owned 372 MB fresh clone is deleted, including its approximately 1 MB JSON/JUnit output target; the external compiler cache is preserved. Release-input copies and native Chrome profiles/proof directories are gone. Only small logs and controls remain in `.scratch/ws8bm-review241-fix/`. The final audit (`final-resources.log`) reports:

```text
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "created_big_outputs": []}
```

No test servers, listeners or containers started by this task remain. Protected model/provider services and the compiler throttle were untouched.
