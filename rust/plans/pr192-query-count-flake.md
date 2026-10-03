# PR192 writer completion fix and query-count flake

The database writer now restores an even generation before notifying its caller.
The flatness assertion and failure-only SQL diagnostics remain unchanged, as do
the warmup, query capture boundary, permission calls, and expected counts.

The initial brief required a stop on a product finding. The diagnostic phase
below did that; the follow-up explicitly authorized this writer fix.

## Observed without a product-code change

On 2026-10-03, both private-work tests ran 30 times in nextest's `ci` profile with
four test slots, restricted to CPU 0 using Docker's `--cpuset-cpus=0`. The restored
Rails parity seed was the same for every iteration, not a randomized test seed.

```text
Summary [  43.494s] 30/30 stress run iterations: 28 passed, 2 failed
```

MCP failed at iteration 15 with `[22, 23]`; REST failed at iteration 20 with
`[28, 21]`. Both failures passed the preceding response-byte equality, row count,
private-link payload, and per-occurrence permission-service assertions. The SQL
multiset differences attribute the extra executions exclusively to:

```sql
SELECT c.disconnected_reason FROM agents a
JOIN github_connected_accounts c ON c.user_id=a.owner_id
WHERE a.id=? AND a.owner_id=? AND c.id=?
```

MCP executed that statement two versus three times. REST executed it nine versus
two times. The other changed SQL texts only have different numbers of `IN` bind
placeholders for the two list sizes; each executes once in both measurements.

## Completion-order race

Before the fix, `crates/db/src/database.rs` sent a write's oneshot reply inside
the job (`write_scoped`, baseline line 736). The writer incremented
`writer_generation` back to even only after that job returned (baseline line 662).
The caller could resume after
the transaction and its callbacks have completed while the generation remains odd.

Every bearer request writes its activity stamps (`concerns.rs:512`). MCP's
`list_work` also awaits a preflight write immediately before its reader operation
(`controllers/agents/pending.rs:78-99`). That preflight only reads the agent and
returns success; its `list_work` branch registers no callbacks or incidental writes.

`IdentityGuard::current` (`integrations/agent_repositories.rs:229-248`) reuses a
snapshot only if the generation is unchanged and even. It checks before and after
each permission call. If the committing thread is descheduled after sending its
reply, those immediately-ready permission calls cause repeated actual identity
SELECTs until the writer marks completion. Frozen time and equal warmups cannot
remove a completion race created again inside every measured request.

The capture belongs to each database and traces actual executions at reader
checkout. Test setup uses raw SQL, credential creation emits no broadcast, and
the app's job workers are stopped and joined. Model broadcasts are synchronous.
Connections are opened before measurement, and statement-cache preparation is
not counted as an executed SELECT. Nextest launches one test per process, so
in-process test ordering is not involved.

This established a product finding: request SQL was sensitive to writer scheduling.
The diagnostic-only commit preserved product behavior under the original stop
condition. The authorized fix below closes that completion window.

## Controlled scheduling probe

A temporary per-database condition-variable gate paused the second writer job
after the warm response (authentication is first, MCP preflight second). It paused
**after the job sent its reply and returned**, immediately before the writer's
existing even-generation increment. It changed no transaction, permission result,
query, returned payload, capture filtering, or count assertion.

The first N `IdentityGuard` read closures sampled an odd generation. At the start
of the next snapshot, the probe released the writer and waited for acknowledgment
of the even increment **before** sampling the snapshot. This deterministically
extends the existing scheduling window without permitting an off-by-one race.

| Odd snapshots held | SELECTs, size 5 / 50 | Identity SQL occurrences | Result |
| --- | --- | --- | --- |
| 2 | 24 / 22 | 4 / 2 | Strict assertion fails |
| 4 | 26 / 22 | 6 / 2 | Strict assertion fails |
| 0 (control) | 22 / 22 | 2 / 2 | Passes |

```text
Summary [   0.719s] 1 test run: 0 passed, 1 failed, 2658 skipped
Summary [   0.700s] 1 test run: 0 passed, 1 failed, 2658 skipped
Summary [   0.847s] 1 test run: 1 passed, 2658 skipped
```

These reproduce the two historical CI count signatures through the completion
race. The historical logs lacked SQL, so they cannot prove those past failures
had this cause independently. The natural local failures and controlled probe
prove that the current request path has it.

All original probe hooks were removed before the diagnostic-phase checks.
`probe.py`, `probe.patch`, and the three `probe-*.log` files remain with the local
evidence. At that stage the zero-snapshot control was a scheduling comparison;
no product fix had yet been authorized.

## Environment and evidence

- CI toolchain image `ws8br-ci-toolchain:b908-mold` (`e509367582e7`): Rust 1.98.1,
  nextest 0.9.146, CI linker/debug settings, one Cargo build job.
- The configured machine-wide four-slot rustc throttle and locks were reused.
- `default`, `first_run`, and `agents_ui` were restored from the earlier local
  parity-seed cache for Rails pin `d7c7de9264c63015be398001d7a1094e7695a6db`.
  `parity/bin/ci-seed prepare` and `validate` ran against the pinned Rails image:
  respectively 29, 4, and 40 validator checks passed, zero failed.
- Initial unrestricted smoke: both private-work tests passed once.
- Contended command (through the toolchain wrapper):
  `nextest run --locked -p campfire --profile ci --test-threads 4 --no-fail-fast --stress-count 30 --success-output immediate -E 'test(pr192_r2_private)'`.
- CI run 37110580565 was being rerun, so the completed failure log was obtained
  with `gh run view 37110580565 --attempt 1 --log`. It confirms MCP `[24, 22]`
  and restored parity caches; it contains no captured SQL to attribute that
  specific historical failure directly.
- Local logs and temporary probe script are retained in
  `/home/riels/.cache/rust-port/pr192-flake/`; build targets are deleted after checks.

Failure diagnostics use SQLite's unexpanded statement text and include the full
ordered SELECT list for both sizes plus an occurrence-count diff. The observed
failure logs contain none of the test credential, repository token, private title,
branch name, or inserted user-content strings.

## Diagnostic-phase verification

The independent reviewer approved the diagnostics-only change with no blocking
findings and confirmed the SQL attribution and probe acknowledgment ordering.

All Campfire tests ran after removing the probe hooks, using the CI toolchain,
restored validated seeds, and four nextest test slots:
`nextest run --locked --workspace --exclude html5ever --profile ci --test-threads 4 --no-fail-fast -E 'package(campfire)'`.
Both private-work tests passed in this unrestricted full-suite run.

```text
Summary [1014.970s] 2652 tests run: 2652 passed (1 slow), 7 skipped
```

Including the initial smoke and full-suite run, each interface had 32 ordinary
test executions: 31 passed and one failed under single-CPU contention. MCP also
had the three controlled probe executions listed above.

Strict Clippy ran with the CI toolchain wrapper:
`clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings`.

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5m 53s
```

The production-input build ran through `ci/with-release-inputs.sh`:
`build --locked --workspace --bins`, with only the Docker builder's source inputs.

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 16s
```

The 6.1 GB task build target and this worktree's generated `rust/target` and
`.scratch` output were removed after verification. Logs, probe source/patch, and
the final JUnit report remain in the task evidence directory.


## Authorized writer fix and invariant audit

A private `WriterJob` separates execution from completion. `WriteJob` retains
both its oneshot sender and its result outside the operation closure. The writer
catches execution panics, rolls back any remaining transaction, advances the
generation to even, then completes the job. Success and ordinary errors still
send their original `Result`; a panic closes the retained sender and still
returns `WriterGone`. Completion has a separate unwind catch, because disposing
of a result after receiver cancellation can itself panic.

Every reader was checked with `rg 'writer_generation|write_generation' crates`:

- The only production reader is `IdentityGuard::current` in
  `crates/campfire/src/integrations/agent_repositories.rs` (lines 229, 239, 241).
  Reuse requires the same identity, unchanged generation, and an even value;
  fresh SQL is cached only when before/after generations match and are even.
  This is the handle's writer version, not a version of external DB writers.
- The existing database generation test checks one odd value throughout the
  transaction and callbacks, with two increments per job including rollback.
  It now also checks nested same-connection transactions and their callbacks.
- New notification and cancellation tests read the atomic at the exact wake or
  destructor, assert explicit even parity, and check the expected increment.
  Tests check scope construction/cleanup, transaction error/panic, callback
  error/panic, persisted versus rolled-back rows, cancelled results, a panicking
  result destructor, and subsequent writer liveness. A locally constructed
  `Rc` scope guard also preserves the existing ability to use non-Send guards.

Both `SeqCst` increments remain in the writer loop. The odd interval still
covers scope lifetime, the whole transaction, persistence callbacks, commit
finalizers, ordered after-commit work, and panic rollback. Retaining the sender
outside the operation prevents panic unwinding from waking the caller early.
Accepted cancelled writes still run and commit; their results are disposed of
only after even. A subsequent queued job can legitimately make the generation
odd again before a notified caller resumes; the contract is ordering of each
job's notification, not global quiescence after every await.

`run_write` is unchanged: job persistence precedes COMMIT; resource finalizers
precede fallible callbacks; synchronous broadcasts and callbacks retain their
order. Ordinary callback errors preserve committed rows and drain remaining
committed job wakes while skipping later model hooks. Panic handling still
rolls back only an active transaction; it does not introduce ordinary error
wake-draining on a panic. Same-connection nested `run_write` calls from agent,
calendar, tag, and attachment callbacks remain in the outer odd interval. No
production code uses generation to require an early reply. Recursive blocking
submission to the same writer would already deadlock; those callbacks use the
connection directly. Asynchronous job enqueues retain their existing order.

Notification still precedes WAL maintenance, including the existing occasional
RESTART checkpoint. The change adds no queue, lock, callback wait, or checkpoint
wait, and retains one boxed job per submission.

## Controlled before/after evidence

The shared temporary probe was rerun on a saved unfixed binary and on the fix.
It pauses the second writer job before the even increment, as before. The
original snapshot-released gate would deadlock with the fix: the waiting caller
cannot supply snapshots until notification. The shared probe therefore also
records whether an actual reply was sent. If withheld at the gate, it asserts
zero identity snapshot attempts and releases the writer; otherwise it holds
exactly the requested two or four attempts and waits for even acknowledgment.
This observes the completion order without changing SQL, results, permission
calls, captured counts, or the strict assertion. The same logic runs on both
versions.

| Requested odd snapshots | Before counts / result | After counts / result |
| --- | --- | --- |
| 2 | 24 / 22, strict failure | 22 / 22, pass; reply withheld, 0 odd attempts |
| 4 | 26 / 22, strict failure | 22 / 22, pass; reply withheld, 0 odd attempts |

```text
Summary [   0.846s] 1 test run: 0 passed, 1 failed, 2658 skipped
Summary [   0.724s] 1 test run: 0 passed, 1 failed, 2658 skipped
Summary [   0.795s] 1 test run: 1 passed, 2658 skipped
Summary [   0.708s] 1 test run: 1 passed, 2658 skipped
```

The new completion/cancellation regressions also failed on the baseline,
recording odd generation 3 instead of completed generation 4. All seven
completion cases and both cancellation cases passed after the fix; explicit
parity assertions were then added following independent review.

```text
Summary [   0.421s] 2 tests run: 0 passed, 2 failed, 1321 skipped
Summary [   0.122s] 3 tests run: 3 passed, 1320 skipped
```

Probe source is `probe-fix.py`; per-phase logs are in the evidence directory's
`logs/fix/`. All probe hooks and the temporary benchmark example are removed
before clean-source stress runs and final checks.

## Database message-write latency

`writer_latency.rs` calls real `Message::create` writes against a fresh copy of
the restored default parity seed, using `Env::default()` and its `NullSink`.
It exercises model/rich-text/FTS/membership writes and callbacks, but does not
measure full application broadcasts, job processing, or network latency.

Before and after binaries were built without probe instrumentation using the
same CI toolchain, debug/linker settings, and one Cargo job. Each CPU-0 trial
warms up 500 writes, measures 3,000 awaited writes, and verifies all 3,500 rows
committed. Six trials per version alternate ABBA order (18,000 measured writes
per version). Background machine activity was shared by the paired trials.

```text
PR192_WRITE_LATENCY_AGGREGATE phase=before runs=6 measured_writes=18000 mean_us=670.567 mean_run_p50_us=551.173 mean_run_p95_us=907.060 mean_run_p99_us=2100.309
PR192_WRITE_LATENCY_AGGREGATE phase=after runs=6 measured_writes=18000 mean_us=675.030 mean_run_p50_us=550.858 mean_run_p95_us=918.213 mean_run_p99_us=2118.402
PR192_WRITE_LATENCY_CHANGE mean_pct=0.666 paired_min_pct=-3.084 paired_max_pct=3.580
```

Mean latency differs by +0.67%, within the variation across paired trials;
these samples show no material regression, rather than proving zero overhead.
Percentile fields are averages of each trial's percentile, not pooled percentiles.
Raw per-trial summaries, the source, paired-run script, and aggregation script
are retained with the local evidence.

## Clean-source stress and regression checks

After removing every probe hook and adding the explicit parity assertions, the
completion/cancellation/nested-write checks passed again. Both private-work
interfaces then ran 30 and 100 times with Docker `--cpuset-cpus=0`, nextest's
`ci` profile, four configured slots, the unchanged rustc throttle, and the same
restored default Rails parity seed. There is no random test seed here.

```text
Summary [   0.131s] 3 tests run: 3 passed, 1320 skipped
Summary [  43.494s] 30/30 stress run iterations: 28 passed, 2 failed
Summary [  44.073s] 30/30 stress run iterations: 30 passed
Summary [ 166.128s] 100/100 stress run iterations: 100 passed
```

The first stress line is the unfixed baseline. Each fixed iteration executes
both MCP and REST: 130/130 passes per interface across the two stress runs,
260 executions total. Nextest's stress mode can return success despite failed
iterations, so the raw summaries were checked in addition to command exit status.
`stress-30.log` and `stress-100.log` retain every iteration's result.

## Final suites, lint, build, and review

Clean-source full suites used the same Rust 1.98.1 CI container and restored
validated `default`, `first_run`, and `agents_ui` seeds. Commands were:

```sh
nextest run --locked -p campfire -p campfire_db --profile ci --test-threads 4 --no-fail-fast -E 'package(campfire)'
nextest run --locked -p campfire -p campfire_db --profile ci --test-threads 4 --no-fail-fast -E 'package(campfire_db)'
test --locked -p campfire_db --doc --no-fail-fast -- --test-threads=4
clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
CARGO_TARGET_DIR=/task/target bash ci/with-release-inputs.sh <toolchain-wrapper> build --locked --workspace --bins
```

The wrapper retains one Cargo build job and the configured machine-wide rustc
throttle. A task-local budget monitor paused only this branch's nextest
coordinators when other test runs occupied more than four threads. The full-run
wall times and slow markers include those coordinator pauses. No assertion,
retry, timeout, warmup, or query-capture setting was changed; the CI profile has
zero retries. `test-budget.py` and its log record the scheduling pauses.

Raw summaries, in command order:

```text
Summary [1337.513s] 2652 tests run: 2652 passed (5 slow), 7 skipped
Summary [ 194.331s] 1319 tests run: 1319 passed (4 slow), 4 skipped
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 26s
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 30s
```

There are no database doctests; the doctest gate completed successfully. All
five `agent_review_r2_tests` tests passed in the full Campfire suite. Full-suite
JUnit reports and logs are retained under `logs/fix/`. These builds use dev
profiles, as the CI input guard does; the last command verifies the restricted
production source inputs, not an optimized release benchmark.

Independent Astra review approved the production change with no blocking
findings. Its two notes were addressed: explicit even-parity assertions at
notification/disposal, and precise labeling of the NullSink message-write
latency experiment. Final database source SHA-256:
`6d719e278c1862bc4d137e11676f2a619fdde6bcca1a62667d893d2931796ded`.
The diagnostics and identity-reader files remain byte-identical to the
diagnostic-only commit. Ordinary unwind panics are covered; Rust aborts and
double panics retain their existing process-abort semantics.

Task-owned build targets, generated test/release scratch directories, and the
saved experiment binaries are removed after validation. Source scripts, logs,
SQL failure evidence, latency summaries, and both final JUnit reports remain in
`/home/riels/.cache/rust-port/pr192-flake/`.

Final evidence review approved integration with no material gaps. Cleanup was
verified: the 5.2 GB task target, 248 KB worktree test output, 1.4 MB generated
scratch output, 1.9 MB temporary files, and saved 49/49/996 MB experiment binaries
were removed; an empty task TMPDIR remains for replaying the retained scripts.
