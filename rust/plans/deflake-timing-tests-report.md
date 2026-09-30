# Campfire timing-test hardening

Base: `a6f10a25`. Implementation: `1b6c48ec816305ed237d26367cccf44078ccf97e`.
All changes and scratch artifacts are inside the assigned `rust-deflake` worktree.
No Rails changes, parity masks, shared stashes, or production behavior changes.

## Causes and changes

- `message_broadcasts` waited for both subscriptions, then assumed the Turbo message arrived
  before the unread message. The connection multiplexes independent subscription streams;
  publication sequence is merged on disconnect, but ordinary delivery has no cross-stream
  ordering contract. An unread object in the first frame caused the reported string panic.
  Compare both complete frames, including identifiers, exact payloads and duplicate counts,
  under one deadline. Keep ordered assertions for later deliveries and the silence assertion.
  The original test did not fail in three full baseline runs or 100 focused loaded runs.
  The new real-socket reversed-arrival regression failed with an order-sensitive comparison
  (`0 passed; 1 failed`) before the comparison fix and passed after it. This demonstrates the
  failure mechanism; it does not claim a spontaneous reproduction of the original broadcast flake.
- `ws8_quote_refresh_jobs_execute_in_the_real_app_runner` returned when the quote job vanished,
  then asserted the entire queue was empty. Boot's periodic loop also enqueues retention work.
  All three baseline failures showed a running `Retention::PruneJob`. Wait for the asserted
  whole-queue condition, failing immediately on failed jobs and retaining full-row diagnostics.
- `until_closed_bounds_a_socket_that_keeps_pinging` deliberately caught the expected panic.
  Its assertions were sound, but the panic hook still logged that panic and the outer 11-second
  budget gave the inner 10-second deadline little scheduling slack. The same close-wait core
  now returns a diagnostic error; the public helper still panics on timeout. Exercise it with
  frequent real WebSocket pings, checking a bounded error without catching a panic.

The wider scan replaced short poll-count budgets with named 30-second overall deadlines,
including channel-state convergence, purge row/file completion, job execution, variant-start
and completion, mail routing, WebSocket upgrades/commands and golden replay readiness.
Shutdown tasks now wait behind registered release gates rather than racing a 250 ms sleep;
periodic shutdown waits for its sender to close rather than sleeping 100 ms. The multipart
regression observes actual body consumption followed by a pending read for the HTTP epilogue,
instead of sleeping 100 ms. Its status, session-count and cookie assertions remain intact.
HTTP fixtures use the assigned range; a binding lock protects the front server's two reserved
ports until its startup acknowledgement, removing its release-and-rebind race within the suite.

The loaded baseline also exposed `parses_pathological_pages_quickly` (3/3) and
`stops_reading_a_gzip_bomb_at_the_limit` (1/3). Their one- and two-second wall-clock CPU-work
budgets included scheduler starvation. On Linux, measure calling-thread CPU time and retain
those same numeric budgets; retain the ten-second inflater CPU budget and output-size assertion.
Other hosts retain wall-clock measurements. Trickling-response tests still exercise the real
500 ms/one-second operation deadlines and exact results, with a generous outer completion bound.
Remaining short sleeps pace synthetic slow/trickling peers; the 250 ms silence observation and
20/100 ms abort grace periods intentionally exercise negative/cancellation behavior.

## Verification

Seeds were built afresh with `rust/parity/bin/seed build default first_run` under
`PARITY_NAMESPACE=deflake PARITY_OWNER=deflake`. The private `deflake-reference` image was tagged
from the installed reference after verifying `GIT_REVISION` exactly matched
`rust/parity/reference.sha`: `d7c7de9264c63015be398001d7a1094e7695a6db`.
The resulting seed directories were 6.1 MB and 1.5 MB. No existing worker containers were modified.

All loaded runs use `CI=true`, `CABLE_TEST_PORT_RANGE=53300-53349`,
`MAIL_TEST_PORT_RANGE=53300-53349` and a disk-backed temporary directory inside this worktree.
`stress-ng` was unavailable, so 16 private Python CPU workers supplied continuous artificial
load on the 20-core machine, with cleanup on exit. Other agents supplied varying additional load.
The before loop used `mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -- --nocapture`.
After building the fixed harness with that toolchain, the after loop runs an immutable copy of
that full harness with `--nocapture` (same complete test selection and default parallelism),
avoiding repeated asset recompilation. The driver caps each complete run at 300 seconds.

Raw logs, driver, immutable before/after binaries and JSON summaries are in
`rust/target/deflake/` (ignored build artifacts). The original before driver was intentionally
stopped after three completed runs; its unstarted fourth run is excluded.

No integration test was skipped for missing seed data. One passed helper test intentionally
prints a local-missing-seed skip message using its own empty temporary directory.
Three pre-existing ignored tests remain ignored (bot presenter, latency measurement, golden recorder).

### Results

Before: **3/3 completed full runs failed**, with seven test failures across 1,194 executed tests
(1,187 passed; nine ignored across the three runs).
After: **0/10 completed loaded full runs failed**, with 3,990 passed test invocations and
30 ignored invocations. An additional fresh-clone full run passed all 399 executed tests
(three ignored). No seeded integration tests skipped.

One additional after attempt (original run 5) exited with **SIGKILL (-9)** after 41.28 seconds,
with no test summary or preceding assertion failure. It is recorded as incomplete, excluded
from completed-run failure rates, and replaced by a successful additional loaded full run.
Its cause remains unconfirmed: this application's cgroup reported zero OOM/oom_kill events;
kernel-buffer access was unavailable. The original after driver therefore reports one failed
process exit among ten attempts; the extra driver supplies the tenth completed suite.

Both phases used the same 16 artificial CPU workers. One-minute load at run starts ranged
38.73–83.29 before and 7.81–67.46 after; shared-host load varied, and rose during fresh compilation.
This is a loaded regression check, not a controlled estimate of the natural flake probability.

| Test | Before full-suite failures | After completed loaded failures |
|---|---:|---:|
| Quote refresh runner | 3/3 | 0/10 |
| Pathological OpenGraph pages | 3/3 | 0/10 |
| OpenGraph gzip bomb | 1/3 | 0/10 |
| Message broadcasts | 0/3 (also 0/100 focused; separate reversed-arrival discriminator failed) | 0/10 |
| Pinging socket close bound | 0/3, expected panic logged | 0/10, no caught-panic log |

Raw baseline summary lines:

```text
test result: FAILED. 395 passed; 3 failed; 3 ignored; 0 measured; 0 filtered out; finished in 88.78s
test result: FAILED. 396 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 97.89s
test result: FAILED. 396 passed; 2 failed; 3 ignored; 0 measured; 0 filtered out; finished in 81.75s
```

Raw completed after summary lines (original 1–4, 6–10, then replacement):

```text
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 46.21s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 47.18s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 65.69s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 82.47s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 72.42s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 62.91s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 65.03s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 76.69s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 58.40s
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 55.90s
```

Fresh-clone command, using its own target directory and copies of the same built seeds:

```sh
CABLE_TEST_PORT_RANGE=53300-53349 MAIL_TEST_PORT_RANGE=53300-53349 CI=true TMPDIR="$PWD/rust/target/deflake/tmp" mise exec rust@1.98.1 -- cargo test --manifest-path rust/target/deflake/fresh/rust/Cargo.toml --locked -j 4 -p campfire -- --nocapture
```

```text
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 80.66s
```

Final clippy (exit 0), from `rust/`:

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.04s
```

No production-code race was identified. The broadcast mechanism was demonstrated by an
adversarial real-socket arrival order rather than a spontaneous baseline failure. The incomplete
SIGKILL attempt remains unexplained; no claim is made that its cause was repaired.
