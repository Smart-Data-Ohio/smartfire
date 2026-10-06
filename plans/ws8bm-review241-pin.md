# PR #241: refreshed pin and two upload declarations

Main was merged with merge commit `f1b072217` (main `ea94edeaa`, including #236). Main's `rust/parity/reference.sha` plumbing and source hashes are retained. The reference is `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. `system-inventory-hashes.py --write` regenerates the sixteen hashes from the pinned Git bytes and refuses a changed declaration list; its tests also verify that closure fields are preserved. The three reviewed causal closures, including the literal `#composer` release scope, remain intact.

The two added declarations are actually in the new pin's `sending_messages_test.rb:29-55` (fresh thread video) and `workspace_markdown_test.rb:174-218` (late progress). Both execute the literal original test body through the pinned Capybara 3.40.0 / Selenium 4.35.0 stack on each app. Only the extracted test method's name changes. An exact-body unit test verifies every action, assertion, visibility scope and deadline. The original two-second default and fifteen-second `BROADCAST_WAIT` are retained. The native file input uses the original `visible: :all` scope and the actual pinned file bytes.

Both hosts hold ordinary jobs as the Rails test helper does (`TestAdapter.perform_enqueued_jobs=nil`; Rust `TestApp::without_job_runner()`). The video's original `perform_enqueued_jobs(only: Message::AttachmentProcessingJob)` dispatches to a tools-only endpoint in each test host, executing the real pending job through the real app implementation. Rails's disk-existence checks and preview/variant lookup then read the same mounted files, including files written by Rust. The Rust endpoint is added only to the generated test helper; no release route or Rust product source changes. Media uses the pinned image's libvips 8.16.1 and FFmpeg 7.1.5. Their paths and shared libraries are applied only to the Rust host, without replacing host tools/libraries.

After every successful pair, the runner checks that all original message rows are unchanged, exactly one browser-uploaded message exists, its room/author/thread or reply/notify identity is correct, its attachment name/size matches, and disk bytes equal the original Git fixture. The video also preserves the original thread and membership premise. Readback regressions cover wrong reply identity, changed history and missing/wrong disk bytes.

The video's served mutant removes posters only from `#thread-panel video[poster]`; rejection requires the original line-53 poster assertion and evidence that the real processing job ran and the mutant encountered its output. The progress mutant restores the old delivered-element overwrite; rejection requires the original line-215 body-equality assertion and evidence that late progress encountered a delivered message. Earlier failures, missing mutation delivery and network failures earn no rejection credit. Unit tests exercise these exclusions.

## Causal harness fixes and excluded attempts

The new native checks initially exposed harness/environment defects before their test bodies: a cold generated tree omitted the new tracked `parity/reference.sha`; exporting the canonical media libraries to the entire harness broke the host curl ABI; a native driver terminated during setup; and Rails asset requests were cancelled with `ERR_NETWORK_CHANGED` while Docker interfaces changed. None earns closure or mutation credit. A later interrupted process ended with `SIGKILL` after the Rails body passed; its cause is unproven and it also earns no credit. Logs retain these attempts under `.scratch/ws8bm-review241-pin/*aborted*.log` and the earlier `progress-first.log` / `progress-baseline.log`.

The corrected generator includes the tracked pin input. Media libraries are scoped to the actual media host. Drivers use a held ephemeral Selenium endpoint and cleanup handles children already ended by signals. Native Chrome runs in a private network/PID namespace, using the repo's existing `capture/forward.ts` boundary to forward HTTP, Cable and Selenium bytes unchanged. This prevents Docker veth churn from cancelling browser requests; ordinary teardown signals propagate to the driver and its descendants. Binary relay/active-stream teardown unit tests and a real driver readiness/teardown probe verify that boundary. No readiness assertion earns system closure credit.

An initial workspace invocation shared its target with the generated test package, which has the same Cargo identity. Relinking replaced the suite executable: the initial log reported 3,150 passes / 1,798 failures with missing-executable or wrong-host/seed errors. This run is invalid, not a product parity result. The generator now always uses a separate `target/ws8bm-browser-host` namespace, with a regression covering the collision. The valid fresh suite uses `target/ws8bm-fresh-suite`. An isolated rebuild was also interrupted by rustc `SIGKILL` before tests; its cause is unproven. Neither interrupted run earns a pass, and no test retry, deadline change or ignore was added. The final complete suite below is the eligible result.

## Eligible verification

Receipts are in `.scratch/ws8bm-review241-pin/`. Browser commands run from the worktree root with:

```sh
export PARITY_IMAGE=review236-reference:78b9b1546
export WS8BM_BROWSER_PORT_BASE=22020
export WS8BM_DISCRIMINATION_RETRIES=1
export CI=1
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'uploading a fresh video in the thread composer' --repeat 10
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'late upload progress preserves a delivered attachment and reply preview' --repeat 10
```

Each case has ten independent positive pairs, with exact persisted-row and byte checks; neither series has a failure or retry. Each video host reports seven original native assertions; each progress host reports thirteen. Raw summaries (`video-ten.log`, `progress-ten.log`):

```text
WS8bm behaviour repetition: 10 paired attempts; 1 named declaration; 0 failed
WS8bm behaviour repetition: 10 paired attempts; 1 named declaration; 0 failed
```

The fresh clone rebuilds and validates all three parity seeds using `parity/bin/seed build default first_run agents_ui`, `parity/bin/ci-seed prepare`, and `parity/bin/ci-seed validate`, with `PARITY_IMAGE=review236-reference:78b9b1546`. Raw validator summaries are 29/0, 4/0 and 40/0 (`fresh-seed-validation.log`):

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

The valid suite executes the fresh clone at `32a42eef8`, containing the merge and the upload helper. Product crates, vectors, Cargo inputs and test support are byte-identical to the subsequent tools-only commits (`git diff --exit-code 32a42eef8 HEAD -- rust/crates rust/vectors rust/Cargo.toml rust/Cargo.lock rust/test-support`). The clone was then fast-forwarded to `031732472` for final clippy and the release-input build. The unchanged machine-wide rustc throttle applies; Cargo jobs are two and nextest workers are four. Nextest's standard CI profile has zero retries. The task-local canonical runner selects the canonical libvips 8.16.1 / FFmpeg 7.1.5 runtime; its generation command and wrapper are recorded in [the cutover report](ws8bm-deferrals.md). Source and fixtures come from the fresh clone and regenerated/validated seeds, without a prior scratch target.

```sh
env PATH="$PWD/rust/target/ws8bm-tools:$PATH" CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=4 CI=1 CARGO_TARGET_DIR="$PWD/rust/target/ws8bm-fresh-suite" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="$PWD/.scratch/ws8bm-deferrals/canonical-runner.sh" mise exec rust@1.98.1 -- cargo nextest run --manifest-path "$PWD/.scratch/ws8bm-deferrals/fresh/rust/Cargo.toml" --locked --workspace --exclude html5ever --build-jobs 2 -j 4 --profile ci
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" mise exec rust@1.98.1 -- cargo clippy --manifest-path "$PWD/.scratch/ws8bm-deferrals/fresh/rust/Cargo.toml" --locked --workspace --exclude html5ever --all-targets -j2 -- -D warnings
env CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR="$PWD/rust/target" CAMPFIRE_REFERENCE="$PWD/.scratch/ws8bm-deferrals/fresh" bash .scratch/ws8bm-deferrals/fresh/rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --workspace --exclude html5ever -j2
```

Raw complete nextest and strict-clippy summaries (`nextest-isolated.log`, `clippy.log`):

```text
     Summary [1849.082s] 4948 tests run: 4948 passed (11 slow), 20 skipped
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 11s
```

The twenty skips already exist in the workspace; no filter, skip, mask, expectation or timing threshold was changed. No Rust product, JavaScript/CSS asset or golden changes are introduced by this continuation.

## Intended negatives and inventory

With the same environment and one-attempt limit:

```sh
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'uploading a fresh video in the thread composer' --negative
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'late upload progress preserves a delivered attachment and reply preview' --negative
python3 rust/reference-tools/messaging/system-inventory-hashes.py --write
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

Both new mutants are rejected on both hosts (`video-negative.log`, `progress-negative.log`). Each attribution is `valid:true` with no reasons, including the actual native assertion line and encountered mutated state. Each command reports:

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

The generic “bounded fresh-fixture retries only” label does not mean a retry ran: `WS8BM_DISCRIMINATION_RETRIES=1` allows one attempt, and both raw invalid counts are zero. Native Minitest failures are the deliberate broken builds, not positive failures.

The two closure entries carry reasoned notes and machine-checked ten-pair/zero-retry/intended-rejection proofs. Raw hash/inventory output:

```text
WS8bm system source hashes: 16 pinned files verified at 78b9b1546bdab4c6c1c9b8ddb94512f661289112
WS8bm system inventory: 137 named declarations; 137 mapped behaviour passes; 0 deferred; 0 WS12 blocked; no pixel checks
```

Final helper commands preserve the worker limit:

```sh
node --test --test-concurrency=4 rust/reference-tools/messaging/*.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

Raw summaries (`node.log`, `python.log`):

```text
ℹ tests 79
ℹ pass 79
ℹ fail 0
Ran 48 tests in 6.923s
OK
```

The release-input-only build exits zero (`release-inputs.log`):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 39s
```

## Retained closures at the refreshed pin

Each of the three reviewed causal closures also has a fresh paired control at the new pin, with persisted-row checks. All four existing served variants reject at their intended assertions on both apps, with zero invalid/escaped attempts. The previous ten-pair causal proofs remain unchanged. Commands use the same one-attempt environment above:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it'
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen'
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view'
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it' --negative
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer keeps the room list scroll position across close and reopen' --negative
python3 rust/reference-tools/messaging/behavior-check.py motion --case 'mobile drawer reopens on the current room when it is already in view' --negative
```

Raw controls (`{release,scroll,reopen}-pin-positive.log`), one identical summary per command:

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

Raw negatives (`{release,scroll,reopen}-pin-negative.log`):

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 2 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

## Cleanup

The owned 1.3 GB fresh clone (including its CI archive/seeds and output target), 252 MB generated source copy, two interrupted native profiles (125 MB and 8 MB), and their two small proof directories are deleted after checking for active processes/containers referencing them. Compiled caches and the canonical media runtime stay under `rust/target/`. Small raw receipts remain. Release-input copies were removed by the guard's trap.

The final audit checks both ordinary and nested generated-host binaries, native/forwarder processes, private Chrome profiles, app ports, containers mounting this worktree and remaining generated source/profile directories. It reports (`final-resources.log`):

```text
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "created_big_outputs": []}
```

No task-started test servers, listeners or containers remain. The protected Python/tensorfold services and machine-wide compiler throttle were untouched.
