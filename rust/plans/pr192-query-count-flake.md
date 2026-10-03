# PR192 query-count product finding

The flatness assertion remains strict. This branch adds failure-only SQL diagnostics;
it does not change the request path, warmup, capture boundary, or expected counts.

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

`crates/db/src/database.rs` sends a write's oneshot reply inside the job
(`write_scoped`, line 736). The writer increments `writer_generation` back to an
even value only after that job returns (line 662). The caller can resume after
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

This is a product finding: request SQL is sensitive to writer scheduling. Per the
task's stop condition, this branch leaves the product behavior and assertion intact.

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

All probe hooks were removed before final checks. `probe.py`, `probe.patch`, and
the three `probe-*.log` files are retained with the local evidence. No product fix
was attempted under the task's stop condition; the control is a scheduling
comparison, not a shipped fix.

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

## Verification

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
