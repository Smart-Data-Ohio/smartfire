# Deflake round 8

Branch: `deflake/round-8`.
Base after the requested fetch/switch:
`82a6ac15994b819d805ae0d1002408528de41696` (main at fetch, including #213).
Test source changes are limited to `global_search_test.rb` and
`search_files_test.rb`; application code, existing assertions, and existing
wait budgets remain unchanged.
The new readiness assertions use Capybara's existing default wait.

## Evidence and reproduced causes

Both cited CI failures occurred in **attempt 1**. The default run views showed
a successful rerun of 37099484161 and an active rerun of 37094280247, so their
failed logs were retrieved explicitly with `--attempt 1 --log-failed`.
GitHub's artifacts API listed no artifacts for 37099484161; its printed
screenshot path was not an available download.

### Back from search results

CI 37099484161 failed at `global_search_test.rb:206`, unable to find the visible
`#global-search-panel`. Browser history changes the location immediately, while
Turbo's restoration request and body replacement are still pending. The old
results page and the destination room both contain `#global-search-input`, so
the URL assertion lets the test click the old input. Restoration then replaces
its open panel with the room's closed panel.

The probe pauses the real room restoration request through
`turbo:before-fetch-request`. The original test clicks the old input; the
probe releases the actual request and observes the body replacement. The
original assertion then produces the exact CI error at line 206.

The fix waits for the destination room's exact header name before clicking.
The original Back navigation, URL assertion, empty-recents assertion, and
absence-of-options assertion remain intact. This also preserves stale-cache
coverage: a restored room with stale recents still fails those assertions.

### Files filters

CI 37094280247 failed at `search_files_test.rb:100`, expecting
`No uploads match.` while `system-cover.png` remained visible. That row is
already present before filename filtering finishes, and the filename-only
page already has one upload before Images finishes. Neither old assertion
proves the preceding navigation has rendered. A following native click can
therefore target content that is about to be replaced.

The probe lets the actual filename response finish, then pauses the real
Images request. The original one-row assertion passes against the old All
page. One native Videos click is split into its pointer-down and pointer-up
events, with the Images response deliberately landing between them. Replacing
the target loses the click: the trace records **zero accepted Videos visits**
and Images remains active. The unchanged empty-state assertion then fails at
line 100 with the same cover row as CI. A hover prefetch is recorded separately
and is not counted as a navigation.

The fix waits for the server-rendered `aria-current="page"` link with the
expected filename and type after Search, Images, and Videos. These attributes
come from the applied request; typing into the input cannot satisfy them.
Every original row, Drive-link, count, and empty-state assertion remains.

Both controlled probes fail before and pass after the fixes. On the fixed
test, each pending request is released when its readiness assertion is reached,
before the next native click. No sleeps, deadline extensions, or test retries
were added.

## Verification

Controlled probe summaries (original assertion failures are intentional):

| Probe | Before fix | After fix |
| --- | --- | --- |
| Back restoration | `1 runs, 5 assertions, 0 failures, 1 errors, 0 skips` | `1 runs, 8 assertions, 0 failures, 0 errors, 0 skips` |
| Files replacement during click | `1 runs, 7 assertions, 1 failures, 0 errors, 0 skips` | `1 runs, 10 assertions, 0 failures, 0 errors, 0 skips` |

Verification runs from a fresh clone at
`/home/riels/.cache/rust-port/deflake8/fresh`, with the exact two changed test
files copied in. One browser cohort runs at a time, `PARALLEL_WORKERS=1`, using
native Chromium and an isolated network. Capacity is checked between cohorts.
An orchestration pause leaves any running test to finish normally and resumes
at the next unused seed; no assertion timing is paused or failed test retried.
The admission monitor initially counted pending wrapper commands as worker
reservations. From unused seed 112 onward it counts running test harnesses
and compiler processes; test sources and execution timing remain unchanged.

The fixed tests each pass **20/20 controlled repetitions**, with the real Back
restoration and Images requests held on every repetition. Each combined run
reports `2 runs, 18 assertions, 0 failures, 0 errors, 0 skips`. The probe adds
one filename-response assertion; it does not replace an original assertion.
Both complete files also pass **20/20 full-file runs**, using seeds 101–120.
Each run reports `15 runs, 132 assertions, 0 failures, 0 errors, 0 skips`.
Each fixed test therefore passes 20 controlled repetitions plus 20 executions
as part of its complete file. Across the repetition campaigns: **340 runs,
3,000 assertions, 0 failures, 0 errors, 0 skips** (excluding one-off probes).
The exact summary line from every run is preserved in
[the raw summary file](deflake-round-8-summary.txt).

```text
2 runs, 18 assertions, 0 failures, 0 errors, 0 skips
15 runs, 132 assertions, 0 failures, 0 errors, 0 skips
```

Each line above occurred 20 times. Both Ruby files pass syntax checks, their
tested source bytes match the working tree, and `git diff --check` passes.

The reproduction scripts are `probe-global-search.rb` and
`probe-search-files.rb` in the evidence directory. Their `*-before.rb` source
copies come from the base commit; the probes accept those paths through
`DEFLAKE8_GLOBAL_FILE` and `DEFLAKE8_FILES_FILE`. The runner loads the probes as
test helpers and selects the original two test names. Full-file runs load only the passive
sign-in diagnostics and both complete system-test files:

```sh
cache=/home/riels/.cache/rust-port/deflake8
run_rails() {
  bwrap --die-with-parent --unshare-net --bind / / --proc /proc \
    --dev-bind /dev /dev --chdir "$cache/fresh" \
    env SE_OFFLINE=true RUBYOPT="-r$cache/selenium-native.rb" \
    BUNDLE_PATH=/home/riels/.cache/campfire-bundle PARALLEL_WORKERS=1 \
    bin/rails test "$@"
}
run_rails "$cache/sign-in-trace.rb" "$cache/probe-global-search.rb" \
  "$cache/probe-search-files.rb" \
  -n '/clearing_from_the_results_page_drops|the_Files_tab_lists_uploads/' --seed 1
run_rails "$cache/sign-in-trace.rb" test/system/global_search_test.rb \
  test/system/search_files_test.rb --seed 101
```

## Round 7 carryovers

The new CI logs and changes since round 7 supplied no new evidence for the
sign-in 401 or the Rust `[26,22]` private-MCP count. The relevant authentication,
test-helper, private-MCP test, and repository-access implementation are unchanged
between the round 7 merge and this base. Neither candidate received a
speculative change or a new Rust repetition campaign.

The existing sign-in diagnostics ran passively alongside this round's Rails
verification, recording session/cookie presence and browser reset boundaries
without cookie values: **360 successful sign-ins, 0 helper failures, 0 capture
errors**. This supplies no new reproduction of the 401 and is not a claimed
sign-in fix. Rust source is unchanged and Rust workspace tests/clippy are not
part of this Rails-only round.

Logs and probe scripts are retained under
`/home/riels/.cache/rust-port/deflake8/`; controlled-failure screenshots are in
its `screenshots/` directory. The cargo throttle configuration is
unchanged, no stash is used, and the model server is untouched. No scratch
Rust target is created by this work.
