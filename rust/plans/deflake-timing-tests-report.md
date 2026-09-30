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
budgets included scheduler starvation. The initial fix used calling-thread CPU time on Linux;
other hosts retain wall-clock measurements. Review of `71549224` found two blind spots in that
fix: the async gzip CPU measurement could accept a stalled fetch, and the trickling test's outer
completion bound did not prove that its supplied 500 ms deadline was honored. The follow-up
below corrects both; the initial loaded results alone did not prove those properties.
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

## Follow-up to Astra's review of `71549224`

First merged current `origin/main`, `2e20b24c3f2be9db8a646a1352c159b4afacad0e`,
with merge commit `f881f7736d6398f25b286e4d304ea592b08a3870`.
`mise exec rust@1.98.1 -- cargo metadata --locked --format-version 1` succeeded after the merge.
Test-only implementation: `45f594f71077c17436eed1c843e02c6e92d7ebf3`.

- `integrations/opengraph/tests.rs`: the trickling response test drives the real TCP fetch
  on a paused current-thread Tokio runtime. It waits for the fixture's first response byte
  while preventing automatic clock advancement, verifies that the fetch remains pending at
  499 ms, then requires exactly `Ok(NoContent)` at 500 ms. A separate OS thread and
  `recv_timeout(30 seconds)` supply an independent wall-clock watchdog and propagate any
  assertion panic to the test harness.
- `integrations/test_support.rs`: the trickling fixture can acknowledge its first response
  byte. Existing fixture users keep their behavior. `Cargo.toml` enables Tokio's `test-util`
  feature only as a dev dependency; there is no lockfile change.
- `integrations/opengraph/tests.rs`: the gzip fixture verifies that its compressed body fits
  below the size limit, retains the successful small-page unfurl and large-page `NoContent`
  assertions, and also calls `fetch_document` directly. For this valid 200 HTML fixture,
  `Ok(None)` establishes inflated-size rejection; a transport timeout remains an error.
  Each async operation has a named 30-second wall-clock watchdog.
- `integrations/net/http.rs`, inside `cfg(test)`: measure only synchronous inflater work,
  after constructing the compressed fixture, under the original gzip test's two-second CPU
  budget. Retain the inflated-output bound. The synchronous parser's one-second CPU checks
  and the CPU clock are unchanged.

No production implementation changed. Broadcast/queue assertions, the remaining 250/100 ms
socket windows, and database checkpoint polling are unchanged by this follow-up.

### Mutation checks

Each temporary mutant was applied separately and restored in `finally` before real-code
validation. Commands ran from `rust/` with the assigned port ranges and `CI=true`:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire integrations::opengraph::tests::gives_up_on_a_trickling_page -- --exact --nocapture
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire integrations::opengraph::tests::stops_reading_a_gzip_bomb_at_the_limit -- --exact --nocapture
```

1. Replace the supplied deadline in `unfurl_within` with `UNFURL_DEADLINE` (ten seconds).
   The controlled-time assertion fails at 500 ms, without waiting ten wall-clock seconds:

   ```text
   assertion `left == right` failed: trickling fetch must expire at its 500 ms deadline
     left: None
    right: Some(Ok(NoContent))
   test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 401 filtered out; finished in 0.00s
   ```

2. Make the fake server send the large gzip response headers and then await a permanently
   pending future instead of sending its body (GET `/`, compressed body over 512 KiB).
   The new fetch-outcome assertion rejects the stalled transport rather than accepting its
   eventual unfurl `NoContent`:

   ```text
   expected inflated-size rejection, got Err(Http(ReadTimeout))
   test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 401 filtered out; finished in 5.32s
   ```

Both commands exited 101. Neither mutant is committed. Restored real code passed all
integration tests, including the synchronous parser and inflater checks:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire integrations:: -- --nocapture
```

```text
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 361 filtered out; finished in 7.08s
```

Raw mutation logs, mutation driver, real-code logs and fresh-clone verification artifacts are
under `rust/target/deflake/review-fixes/` inside this worktree.

### Fresh-clone loaded suites

Cloned the committed branch with `git clone --no-hardlinks --single-branch --branch
rust/deflake-timing-tests . rust/target/deflake/review-fixes/fresh`. The clone's tested source
SHA was `45f594f71077c17436eed1c843e02c6e92d7ebf3`; it used its own Cargo target directory and
copies of both freshly built pinned seeds. `parity/reference.sha` remains
`d7c7de9264c63015be398001d7a1094e7695a6db` after the merge.

Each full suite ran twice with 16 continuous artificial CPU workers (the same forked Python
CPU loop as the original reproduction), `CI=true`, ports 53300–53349 and disk-backed scratch.
The app's combined Cargo command stops on its failure, so database tests were subsequently
run twice with their own command. Both commands below ran from the fresh clone's `rust/`:

```sh
CI=true CABLE_TEST_PORT_RANGE=53300-53349 MAIL_TEST_PORT_RANGE=53300-53349 TMPDIR="$PWD/target/deflake-tmp" mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -p campfire_db -- --nocapture
CI=true CABLE_TEST_PORT_RANGE=53300-53349 MAIL_TEST_PORT_RANGE=53300-53349 TMPDIR="$PWD/target/deflake-tmp" mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_db -- --nocapture
```

**App: 2/2 runs failed on one inherited golden test; timing checks failed 0/2.** Each run
passed 398 tests, failed one and ignored three. No seeded integration test skipped; one
passed helper deliberately exercises the local missing-seed message.

Raw app lines:

```text
test result: FAILED. 398 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 61.83s
test result: FAILED. 398 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 53.44s
```

**Database: 0/2 runs failed.** Each ran 445 tests and ignored four. Database doctests also
ran both times, discovering zero doctests. No database tests skipped for missing fixtures.

Raw database unit/doctest lines, in run order:

```text
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 68.65s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 445 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 59.16s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

One-minute load at the start/end of each app run was 12.10/41.56 and 41.56/40.13;
for the database runs it was 18.78/29.29 and 29.29/30.19. The 16 artificial workers
remained active throughout each suite; no watchdog expired and no process was killed.

The only app failure was
`app::full_page_tests::complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks`.
Checked out exact `origin/main` SHA `2e20b24c` inside the fresh clone, kept the same seeds,
ran that test with the same toolchain and `--exact --nocapture`, then restored the branch:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire app::full_page_tests::complete_auth_templates_match_fifteen_seeded_rails_pages_without_masks -- --exact --nocapture
```

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.18s
```

All 15 golden pages differ only in asset fingerprints. Example: the goldens contain
`/assets/people-b8926caa.css`, while merged main builds `/assets/people-8adb2aea.css`.
The changed Rails assets came from main's status-popup merge. Recorded full actual/expected
pages and digest-only comparison results in the ignored verification directory. No golden
was edited, normalized in its test, or masked. The full app gate is still red because of this
confirmed main failure; this report does not claim the branch is entirely green.

### Fresh-clone clippy

After restoring the fresh clone to the tested branch SHA, workspace clippy exited zero.
Exact command from the assigned worktree root:

```sh
CI=true CABLE_TEST_PORT_RANGE=53300-53349 MAIL_TEST_PORT_RANGE=53300-53349 TMPDIR="$PWD/rust/target/deflake/review-fixes/fresh/rust/target/deflake-tmp" mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/target/deflake/review-fixes/fresh/rust/Cargo.toml --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 02s
```

The remaining issue is main's asset-fingerprint golden drift, not a timing failure. This
follow-up makes no production changes and does not alter the negative socket windows or
checkpoint polling. Its mutation failures and real-code successes establish the two
requested regression checks; the full app failure is separately disclosed above.
