# Deflake round 7

Base: `c6c37fb8fd0e798aedc8b8fd37f85407ea7f9cb5`, branch `deflake/round-7`.
Only tests and their shared helper change; application code is unchanged.

## Reproduced causes and fixes

- **Stage join event** (`stage_test.rb:288`): the lazily imported
  `huddle-launcher` controller can remain unavailable after all Cable sources
  connect. Clicking the rendered button then does nothing. A Rack probe held
  the actual controller-module response until after the click and reproduced
  the original Timeout::Error. The test now waits for that controller before
  clicking. Condition waits use Capybara synchronization; the original event
  deadline and exact event payload/count assertion remain intact.
- **Service-worker readiness** (`service_worker_test.rb:10`): registration
  readiness and document control are different events. The old helper checked
  the controller twice and retained `unclaimed` forever if claiming happened
  later. A probe deferred real registration until the final room navigation,
  held the worker's actual claim call, then released it after both old checks.
  The old helper timed out after release; the revised helper observed control
  and passed every original cache assertion. It checks the live controller on
  every poll, with the same 15-second bound and no timer.
- **Thread title** (`threads_test.rb:158`, assertion at `:573`): `beginCreate`
  displays the form before its animation-frame callback focuses First message.
  That callback can interrupt native typing in Thread name. Holding the real
  callback and releasing it on the `Work hand` input event reproduced the
  exact truncated CI title. The shared title-entry helper waits for First
  message to receive focus, then types normally and checks the complete value.
  All six title-entry sites in threads, composer, and agent-work-assignment
  tests use it; the existing rendered-title assertions remain.
- **Group-DM rename/add/leave** (`people_group_dms_test.rb:310`): the name field
  already contains Weekend Plans before Save's response replaces the form.
  Selecting the next form's member during that replacement loses the selection.
  A probe held the actual rename request, then released it after JZ was checked;
  the original Add assertion failed. The test now waits for the server-rendered
  rename notice before selecting JZ. Separately, checking that the browser is
  not on the room URL
  succeeds immediately while it is still on the edit URL, before Leave reaches
  the server. Holding the real Turbo leave request reproduced the original
  membership assertion. The test now also waits to leave the edit URL before
  retaining both original room-path and membership assertions.

All five delayed-operation probes failed before and passed after their fixes.
Probe scripts and unabridged logs are retained under
`/home/riels/.cache/rust-port/deflake7/`.

## Separate investigations

The sign-in investigation logs signed-cookie presence, session-row identity,
expiry, browser reset boundaries, and monotonic sign-in duration. Cookie values
are never logged. Twenty reused-browser cases under network isolation passed
(20 runs, 40 assertions, 0 failures, 0 errors, 0 skips). This does not establish
why the earlier sidebar-200/member-401 sequence occurred. No speculative
sign-in change has been made. Across the warm probe and full-file diagnostics,
the trace recorded 1,100 successful sign-ins, zero sign-in assertion failures,
zero authentication-cookie removals, and zero room-member authentication misses.
Reset tracing covered 935 completed browser resets (the specialization wrapper
was corrected after the first two browser-file batches). No application bug
has been established; this candidate remains unresolved.

The reported Rust private-MCP failure was verified in the original run's
**attempt 1** full log; its default attempt 2 was cancelled. The test already
stopped and joined its job runner at both the failing revision and current
main. Query capture is per database, so other apps' SQL cannot simply enter
this capture. Current main passed 30 predetermined executions, with exactly
22 SELECTs at both 5 and 50 rows every time. The older log does not contain the
SQL statements, and no reproduction has established the source of its extra
four queries. Rust remains unchanged; this candidate remains unresolved.

## Verification

All repetitions used the exact final test files in a fresh clone at
`/home/riels/.cache/rust-port/deflake7/fresh`. The seven changed Ruby files were
compared byte for byte with the working tree. Browser cohorts ran serially,
with `PARALLEL_WORKERS=1`, native Chromium, and an isolated network.

| Files / candidate | Final-source repetitions | Result per execution |
| --- | --- | --- |
| `stage_test.rb`, including the reported join case | 20/20, seeds 15–34 | 15 runs, 157 assertions, no failures/errors/skips |
| `threads_test.rb`, `service_worker_test.rb`, `people_group_dms_test.rb`, `composer_test.rb`, `agent_work_assignment_test.rb` | 20/20 full-file cohorts, seeds 1–20 | 48 runs, 459 assertions, no failures/errors/skips |
| Rust private MCP query-count candidate, unchanged | 30/30, seeds restored and `CI=1` | 1 passed; 22 SELECTs at both sizes |

The reported Rails cases each passed all 20 full-file executions. In total,
the final-source Rails cohorts ran 1,260 cases and 12,320 assertions. Earlier
stage runs and the controlled probes are extra evidence, not part of that
total. The stage file exercised the pinned LiveKit server and real gateway,
with zero skips. A loopback-only private network could not establish WebRTC
media; an isolated dummy interface fixed the media smoke test. Invalid
environment/startup/probe runs are archived separately and excluded from the
successful repetition counts. The namespace-root browser needs `--no-sandbox`,
set only in the local launcher, never in repository tests.

The full Rust workspace completed successfully from the fresh clone:

```sh
cargo test --workspace --exclude html5ever --locked -- --test-threads=2
```

It used the pinned 1.98.1 toolchain, `CI=1`, two test threads, one build job,
the unchanged machine rustc-slot throttle, and the three Rails-validated,
input-matched parity seeds: `default`, `first_run`, and `agents_ui`. The 59
harnesses totalled **4,647 passed, zero failed, 16 ignored**. Rust source did
not change, so no new clippy run was required. Both scratch target directories
were deleted and verified absent. The protected model-server process is
untouched; no stash was used.

## Raw summaries

These Rails lines occurred in every one of the respective 20 full-file runs:

```text
15 runs, 157 assertions, 0 failures, 0 errors, 0 skips
48 runs, 459 assertions, 0 failures, 0 errors, 0 skips
Rails browser files: 20 executions, 0 failed
Rust private MCP: 30 executions, 0 failed
```

The main Rust app harness reported:

```text
test result: ok. 2616 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out; finished in 1730.34s
```

All per-run Rails and Rust repetition lines and all 59 workspace harness
summary lines are preserved in [deflake-round-7-summary.txt](deflake-round-7-summary.txt).
The final aggregate lines in that file are computed from the preceding raw
summaries. Full logs, trace data, and failing-before/passing-after probe scripts
remain in `/home/riels/.cache/rust-port/deflake7/`.
