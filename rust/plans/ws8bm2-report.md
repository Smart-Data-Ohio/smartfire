# WS8b-m2 H: #229 review fixes

Both requested P2s are fixed. Source/fixtures tested: `b7bf9c579bd504c42ac8069ddca5a3ba928ea0ff`.
Fresh clone: `.scratch/ws8bm2-h-review229/fresh`, with all three seeds rebuilt
from pinned Rails. The later report and producer-control script add no runtime
or fixture changes.

Send-now carries the request's viewer zone through the scoped writer together
with its origin. Fresh Rails captures compare real HTTP responses, full stored
scheduled/message rows and ordered publications plus the exact receiver
multiset in UTC, New York and Kolkata. The year-10000 draft is sent at frozen
`2026-03-02T16:00:00Z`; New York's actual frame is `11:00:00-05:00`.
Periodic jobs retain their existing default-zone behavior.

Saved batches messages, rooms, direct-room names, creators, rich-text bodies and
plain-text attachments/mentions. Scheduled batches rooms, names, threads, sent
message links and sendability. Dispatch retains its transactional access recheck.
The new sendability query and reused association loaders use JSON-bound ID
lists; bind counts are independent of visible-row counts, including direct rooms.
Original item ordering and status/access scopes are retained.

## Failing-first and integrity evidence

The unchanged `91cb61210` runtime fails both physical read-growth assertions.
All twelve full page sections match fresh Rails before the fix, so those failures
are specifically query growth. The separate send-now regression fails at the
actual New York receiver frame: Rust `2026-03-02T16:00:00Z`, Rails
`2026-03-02T11:00:00-05:00`. Full persisted rows are captured through raw SQL,
avoiding Ruby model-attribute type casts. Timestamp storage spelling alone is
normalized for row comparisons.

Commands actually rerun before changing the runtime:

```sh
bash .scratch/ws8bm2-h/fresh-native.sh test --locked -p campfire review_229_tests -- --test-threads=4 --nocapture
bash .scratch/ws8bm2-h/fresh-native.sh test --locked -p campfire send_now_preserves_viewer_zone_in_fresh_rails_frames_and_rows -- --test-threads=4 --nocapture
```

Logs: `failing-first-clean.log` (page growth) and `failing-first-zone.log`.
The first capture initially exposed a Ruby boolean-versus-raw-SQL fixture shape
mismatch; the raw-row generator was corrected before the failing zone witness.
No runtime change was needed for that capture correction.

`check_review_229_mutants.py` changes the real Saved and Scheduled room-label
producers, preserving every fixture and comparator. Both are rejected at their
whole-section byte assertions; send-now remains passing. Source bytes are
restored in `finally`. The full fresh-clone suite is then run after restoration,
with source mtimes advanced so no mutant compiler artifact can be reused.

```sh
python3 rust/reference-tools/messaging/check_review_229_mutants.py bash .scratch/ws8bm2-h/fresh-native.sh
```

```text
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 2755 filtered out; finished in 2.37s
WS8bm2 review229 producer mutant: rejected at real feature section differs from fresh Rails: /saved
WS8bm2 review229 producer mutant: rejected at real feature section differs from fresh Rails: /scheduled_messages
```

The three regression names are:

- `review_229_tests::send_now_preserves_viewer_zone_in_fresh_rails_frames_and_rows`
- `review_229_tests::saved_visible_row_growth_and_complete_sections_match_fresh_rails`
- `review_229_tests::scheduled_visible_row_growth_and_complete_sections_match_fresh_rails`

## Visible-row physical reads

These are actual production HTTP requests, including page construction, at
4 / 16 new visible rows plus the two seed rows. Mixed cases include done saves,
a thread, sent/dropped drafts and inaccessible-room drafts. Each complete
feature section remains identical. The existing per-request hidden authenticity
input handling is unchanged; Rails' forgery-disabled oracle omits that input.
No expected data constructs the observed HTML, response, stored row or frame.

| Page / kind | Before Rust 4 / 16 | Fixed Rust 4 / 16 | Rails 4 / 16 |
|---|---|---|---|
| Saved / ordinary | 33 / 69 | 19 / 19 | 19 / 31 |
| Saved / mixed | 33 / 69 | 19 / 19 | 19 / 31 |
| Saved / direct | 37 / 85 | 20 / 20 | 20 / 32 |
| Scheduled / ordinary | 35 / 83 | 16 / 16 | 12 / 12 |
| Scheduled / mixed | 33 / 81 | 18 / 18 | 17 / 17 |
| Scheduled / direct | 39 / 99 | 17 / 17 | 13 / 13 |

## Fresh-clone verification

The affected suite is the complete `controllers::message_features` module plus
scheduled-message model tests. It includes Saved/Scheduled HTTP, wide-time and
zone comparisons, periodic tasks, callbacks, jobs and the new regressions.
One Cargo command runs at a time, with two build jobs and the configured rustc
throttle. Four test threads reserve shared machine capacity; no concurrency or
timing threshold was weakened. Owned Rails generators run one at a time.

```sh
bash rust/parity/bin/seed build default first_run agents_ui
WS8BM2_ORACLE_SCRATCH=.scratch/ws8bm2-h-review229/logs/oracle-replays python3 rust/reference-tools/messaging/verify_oracles.py
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
(cd rust && bash ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins)
bash rust/ci/cargo.sh test --locked -p campfire_db scheduled_message_test -j2 -- --test-threads=4 --nocapture
bash rust/ci/cargo.sh test --locked -p campfire --bin campfire controllers::message_features -j2 -- --test-threads=4 --nocapture
```

The wrappers are `.scratch/ws8bm2-h-review229/reference.sh` and `gates.sh`.
Oracle scratch is the absolute log directory in the actual replay invocation.
Raw summaries (ANSI colors removed only):

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1343 filtered out; finished in 1.18s
test result: ok. 254 passed; 0 failed; 0 ignored; 0 measured; 2504 filtered out; finished in 165.43s
WS8bm2 oracle replay: 58/58 independently replayed fixtures byte-identical
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 58.34s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 04s
cargo metadata --locked: exit 0
```

`origin/main` was fetched again at `b06d191142e555fa533de884ca925a2b8e4613a1`.
`git merge-tree --write-tree HEAD origin/main` succeeds cleanly on the fixed
source (tree `d856c54d38761fdee1c3ba347ac1c9319d6732d1`); the conditional main
merge is unnecessary. There is no history rewrite or stash.
Logs are under `.scratch/ws8bm2-h-review229/logs/`. No scratch target directory
was created; the pre-existing compiler cache remains. Owned runners/containers
are finished before handoff. No browsers, deployment or model-server operation.

## Remaining and owners

Both #229 P2s and the former visible-row batching flag are closed. Broader scope
is unchanged: lead's cutover browser rerun; WS8 / WS11-UI values outside the
existing I512 time representation; WS12's broader board/work parity. No
WS12-owned file changed. Atomic durable enqueue rollback remains the approved
decision-2 difference, not an owner block.

---

# Earlier H merge checkpoint (historical at 91cb61210)

The following receipts and remaining lists are historical. The verification and
remaining list above supersede them, including the now-closed batching flag.


Both requested merge commits are complete, without rebase or stash:
- `cebbd64f8`: G `b31d3d65e216fb1419589062613f99146be49f4c` into H.
- `bfb55dc43`: main `7e35a5fd6bb64de2e14c3f0e11725f0d1bd0a22a` into H.

## Conflict resolutions

`controllers/saved_items.rs` and `views/src/saved_items.rs` retain H's wide
presentation strings and viewer-zone rendering. They cover G's extended-year
reminders as well as H's wide created/reminded fields; G's redundant closure
adapter is unnecessary. G's microsecond arithmetic in `db/src/time.rs` and
`slash_commands/time_parser.rs` is unchanged. All G overflow regressions and
fresh Rails vectors are retained.

`message_features/container_input_tests.rs` retains both the warm four-zone
fragment comparison and the actual Saved-page item comparison, including G's
new event/reminder tests. `verify_oracles.py` retains all H oracles plus both G
overflow oracles, the named generator mode and the optional subset selector.
Main had no textual conflict. Its newly added jobless browser fixture host
needed G's lazy publication-capture field initialized; the compiler rejected
the merged constructor first, then the added default preserves both paths.

## Lossless overflow fixture

The new overflow consumer vector uses the same recorded-baseline/changed-row
encoding as H's relative and periodic vectors. Candidate state is still read
in full. Expansion proves equality for all ten HTTP envelopes, complete stored
rows, Saved-page items, read counts and ordered publication/wire bytes.
Independent fresh Rails regeneration is byte-identical. G's existing
per-request token handling is retained. No masks or allowlists were widened,
and no deadlines or expected-output substitutions changed.

The merged overflow HTTP reads remain flat at 4 / 16 old references:
`/event` Rails 5 / 5, Rust 11 / 11; `/remind` Rails 26 / 26, Rust 110 / 110.
Visible-row counts remain fixed, as in the original H proof.

## Current verification

Tested source: `f96f70e4e72e5f285ad3694e0a6adcd92b4be6ec`, from fresh clone
`.scratch/ws8bm2-h-merge/fresh`. All three seeds (`default`, `first_run`,
`agents_ui`) were rebuilt from pinned Rails. `CI=1` makes missing seeds fail.
One Cargo build runs at a time, with two build jobs and the existing rustc
throttle. The four-thread test invocations reserve shared capacity; no test
concurrency or timing threshold was changed. The workspace suite ran once.

Exact invocations (the native wrapper supplies the CI image and throttle):

```sh
bash rust/parity/bin/seed build default first_run agents_ui
WS8BM2_ORACLE_SCRATCH=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-h-merge/logs/oracle-replays python3 rust/reference-tools/messaging/verify_oracles.py
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 --no-run
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
(cd rust && bash ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins)
bash rust/ci/cargo.sh test --locked -p campfire_db relative_overflow -j2 -- --test-threads=4 --nocapture
```

The five focused `campfire` invocations were:

```sh
for selector in controllers::message_features::container_input_tests controllers::message_features::wide_html_tests controllers::message_features::periodic_delivery_tests controllers::message_features::saved_tests controllers::message_features::relative_split_input_tests; do
  bash rust/ci/cargo.sh test --locked -p campfire --bin campfire "$selector" -j2 -- --test-threads=4 --nocapture
done
```

The single workspace execution was
`bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture`.
The lossless expansion command was
`python3 .scratch/ws8bm2-h-merge/verify-compaction.py`.

Raw summaries (ANSI color removed only):

```text
focused-db:
PR223 overflow Rust: 10^142 hours parse/render matched fresh Rails
PR223 overflow Rust: 13 render/since/ago boundaries matched fresh Rails
PR223 overflow Rust: 39 matched; 0 differed
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1364 filtered out; finished in 0.02s
focused-container_input_tests:
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 2751 filtered out; finished in 58.02s
focused-wide_html_tests:
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2753 filtered out; finished in 22.43s
focused-periodic_delivery_tests:
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2754 filtered out; finished in 10.21s
focused-saved_tests:
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 2737 filtered out; finished in 2.68s
focused-relative_split_input_tests:
PR223 overflow Rust: 13 parsed JSON renders matched fresh Rails
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 2753 filtered out; finished in 0.07s
WS8bm2 merged workspace aggregate: 4851 passed; 0 failed; 15 ignored; 61 result groups
clippy:
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 12s
release:
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 57.23s
WS8bm2 oracle replay: 57/57 independently replayed fixtures byte-identical
cargo metadata --locked: exit 0
WS8bm2 lossless relative_overflow_consumers: 2493769 -> 976831 bytes; all 10 envelopes, complete rows, read counts, Saved-page items and frame bytes identical; expanded SHA256 a55c7d1fbf132ad9802264463958e711364cd10e4af649e7c6a7b3df6f637151
```

Full workspace raw result groups:

```text
test result: ok. 2749 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 1074.60s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.06s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1364 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 162.43s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.50s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.07s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.03s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.34s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.50s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.53s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.89s
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
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.46s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Logs: `.scratch/ws8bm2-h-merge/logs/`. The clean-clone target and any
release-input scratch targets are removed after verification; the pre-existing
worktree compiler cache remains. No owned test process or container remains at
handoff. No browser suite or deployment was run.

## Remaining and owners

The requested merge checkpoint is complete. Existing broader scope remains:
- **Lead:** the final paired browser rerun at cutover (historical 45/45 each).
- **WS8 / WS11-UI:** values beyond the fixed-width I512 representation, outside
  the pinned corpus. G now covers the sampled huge relative values and the
  representable arithmetic boundary, including sub-microsecond offsets.
- **WS8b-m2:** visible Saved/scheduled-list batching, whose scaling is still
  unproven when visible-row or due-job counts grow. The current date matrices
  hold those counts fixed and vary old references.
- **WS12:** broader board/work parity and its owned query work.
Atomic durable enqueue rollback remains an approved difference under decision 2,
not an owner block. G's requested review fixes are merged, so that former
follow-up flag is closed. This checkpoint adds no new owner-blocked seam.

---

# Previous H report (historical at 65c15089)

The following receipts describe the pre-merge H checkpoint. Its older source
SHAs, test counts and G-follow-up flag are historical; the verification and
remaining list above supersede them for this handoff.

# WS8b-m2 slice H checkpoint (partial)

Stacked branch `rust/ws8bm2-message-features-h` starts at G `c4cdef6c7`.
G was left unchanged; no pushed history was rewritten. No browser run or deploy.

## Completed

The #220 P3 is fixed: both Rails and Rust capture total message counts before
and after each real built-in slash dispatch. The comparison checks those actual
SQL counts separately from the returned message. A real extra post after a
rejected `/remind` preserves the error and passes the former comparator. The
new assertion rejects `[185,186]` against Rails `[185,185]`. All 35 named cases
remain, including their actual payload, state, attachment, webhook and frame
checks. `check_slash_count_mutant.py` reproduces both the old escape and rejection.

The full 80-request relative-consumer matrix now passes, including the formerly
flagged 24 non-UTC slash appends. UTC, New York, Lord Howe and Apia retain exact
HTTP envelopes, complete persisted tables and actual ordered publications plus
wire envelopes. Message day/permalink/system-note and edited datetime tags use the renderer zone;
avatar versions use the presenter's zone. The slash writer and its pooled
broadcast read carry the existing #179 zone guard. Detached message fragment
keys also include their zone; matching reads have an explicit zone API and the
retained UTC default. The same persisted message is warmed in UTC, New York,
Lord Howe and Apia, then revisited in UTC, with complete independently captured
Rails partial bytes and real cache hits. Ordinary scheduled JSON's viewer-zone branch is
unchanged. Background periodic broadcasts retain Rails' UTC renderer default.

The inherited root declaration fixture rendered a detached UTC meta after an
HTTP GET and never inspected that GET's meta. Fresh real Rails responses show
Pacific/Tokyo offsets in both the created and edited datetime attributes, with
the edited title still UTC. The generator now captures that complete actual
HTTP component, while every other response and persisted field stays unchanged.
The candidate comparison also extracts its actual HTTP component. Restoring UTC
edited datetime output is rejected at that assertion.

Wide saved/scheduled HTML uses plain presentation strings from WS11's shared
`rails_compat::datetime` renderer and calendar formatting, rather than narrowing
through Jiff. No second date parser was built. Thirty-two row/zone cases cover
9999, 10000, -10000 and 178956971 across all four zones, comparing all 96 whole
saved/upcoming/history partials byte-for-byte. Forty-eight real HTML POST/GET
pairs additionally compare raw complete time tags, datetime-local fields, flash
spans and complete saved/scheduled tables. These HTTP date projections do not
claim full-layout or random session-token parity; detached whole partials retain
all bytes. No existing mask, allowlist or token gate was loosened.

The periodic proof selects the real task objects from the production registry
and executes `Periodic::tick`: 8 scenarios, 24 ticks, exact task names, full
message/rich-text/saved/scheduled/activity tables, actual durable class/arguments,
ordered publication bytes and receiver silence. It covers due-now, normal
future, wide future and wide past; a same-time tick and the next interval prove
idempotence. It enqueues the real reminder/room push consumers; physical push
transport is not claimed by this matrix. Rails' year-10000 future draft is
selected lexically by SQLite, then its claim is released after a real time
comparison. Rust previously posted it. The new post-claim check matches Rails.

Failing-first runtime evidence: the real New York frame contained UTC timestamp
and avatar bytes; wide HTML panicked at `timestamp within Jiff calendar range`;
periodic wide-future state had 270 messages against Rails' 269. All are now
closed. Producer controls reject UTC timestamp/avatar restoration, corrupt HTML
dates, form values and notices, omitted reminder claims, altered durable job
arguments, skipped scheduled delivery, removal of the future recheck, changed
publication action and missing production task registration at their intended
output assertions. Controls are documented beside each new test. The cache-zone collapse is also
rejected by the warmed New York whole-fragment assertion.

## Approved difference

The lead ruled durable enqueue rollback consistent with decisions.md decision 2.
`ws8bm2-approved-differences.md` records it as **approved, not owner-blocked**.
The 24 actual Rails adapter cases remain pinned. Eight real Rust first/second
Generic/LinkedIn durable INSERT failures prove full metadata/claim/job rollback
and empty publications at 4/16 references. Swallowing the real error is rejected.
No adapter workaround or owner API change was installed.

## Lossless fixture reduction

`relative_consumers.json`: 13,417,007 → 1,266,737 bytes. The periodic fixture
also shrank from 8,362,748 → 3,768,430 bytes. Unchanged table rows are recorded
once; changed rows retain every column and deletions retain every ID. Only Rails
expectations are expanded. Candidate tables are still independently selected
and compared in full. Expansion exactly reproduces the original captures,
including all HTTP/frame bytes and reads. Fresh independent regeneration is
byte-identical, and the existing input producer controls still reject.

## Read counts

Both sizes are **4/16 old references**, with the same visible feature rows and
scheduled/reminder candidates. These proofs do not claim constant cost when the
number of due jobs or visible saved/scheduled rows changes. Reads include the
production callbacks/cache-key path; arrangements and observations are outside
the capture. No per-reference growth was introduced.

| Path | Rails reads 4 / 16 | Rust reads 4 / 16 |
| --- | --- | --- |
| Slash reminder / posted message, all zones | 26 / 26 | 110 / 110 |
| Slash event prefill | 5 / 5 | 11 / 11 |
| Wide reminder create | 6 / 6 | 13 / 13 |
| Wide schedule create, JSON | 5 / 5 | 13 / 13 |
| 3 wide detached partials | 8 / 8 | 10 / 10 |
| Saved HTML POST + GET | 22 / 22 | 37 / 37 |
| Scheduled HTML POST + GET | 17 / 17 | 35 / 35 |
| Periodic due/wide-past first tick | 35 / 35 | 114 / 114 |
| Periodic wide-future visited tick | 4 / 4 | 10 / 10 |
| Periodic normal future / idempotent later tick | 2 / 2 | 2 / 2 |
| Periodic same-time tick | 0 / 0 | 0 / 0 |
| First durable sibling refusal | 7 / 7 | 15 / 15 |
| Second durable sibling refusal | 7 / 7 | 14 / 14 |

The additional legacy cache-integrity arrangement performs five renders of one
actual stored row, with 133 / 133 native reads across old-reference sizes. Those
arrangement reads are outside the production HTTP count and have no claimed
like-for-like Rails cost; their complete partial bytes match the Rails capture.

The fixed render costs above exceed Rails; inherited general reader internals
and WS12's board/work files were not changed.

## Remaining and owners

- **Lead:** final both-app cutover browser rerun; prior accepted inventory is
  45/45 each. No browser or pixel check this round.
- **WS8 / WS11-UI shared time representation:** inputs beyond the existing I512
  Timestamp/duration range remain outside the pinned corpus. The remaining
  boundary is `crates/db/src/time.rs:25` and `slash_commands/time_parser.rs:235`;
  this slice does not claim exhaustive arbitrary-size Ruby Integer parity.
- **WS8b-m2:** visible saved/scheduled-list batching is still unproven by these
  date matrices, which hold the visible list fixed. Existing per-row associations
  remain at `controllers/saved_items.rs:81` and `scheduled_messages.rs:130`;
  scheduled sendability also reads per row at `scheduled_messages.rs:95`.
  H adds no per-item query, and claims no visible-row scaling credit.
- **WS12:** broader board/work parity and owner N+1 work. No edits to its board,
  channel-thread board/work, board-post or work-thread files.
- G review follow-ups remain on **G / lead** until the lead requests a merge.
  Calendar retries, provider jobs, WS12 consumption and physical push/agent/huddle
  integrations retain their earlier real-owner implementations and suite coverage.

All three assigned H gaps are closed. This is a coherent PR-ready checkpoint,
not an owner-blocked-only declaration: the shared out-of-range corpus remains
unproven. Atomic enqueue failure is an approved difference, not remaining work.

## Changed files

| Files | Change |
| --- | --- |
| `controllers/rooms/slash_commands.rs`, `channels/message_features.rs`, `controllers/presenters.rs` | Carry the real request zone through the writer/pooled render; format actual avatars in the presenter zone. |
| `views/src/messages.rs`, three message templates | Renderer-zone timestamps, UTC edited title, shared zone-aware legacy cache writer/readers. |
| `controllers/saved_items.rs`, `controllers/scheduled_messages.rs`, corresponding view models and `views/src/time.rs` | Wide date presentation strings, real viewer-zone HTML fields and notices, no Jiff narrowing. |
| `rails_compat/src/datetime.rs` | Shared extended-year calendar formatting, reusing the existing wide renderer; no new parser. |
| `db/src/models/scheduled_message.rs` | Rails' post-claim future-time check and claim release. |
| `campfire/src/jobs/periodic.rs`, `jobs/Cargo.toml`, `jobs/src/periodic.rs`, campfire dev feature | Select and execute actual production-registered tasks; isolation API only under test support. |
| `slash_named_tests.rs`, `slash_named.rb`, vector | Independently captured before/after total message counts, retaining all 35 named comparisons. |
| `container_input_tests.rs`, `wide_html_tests.rs`, `periodic_delivery_tests.rs`, quote row loader | Complete real HTTP/state/frame comparisons, warm cache checks, actual periodic consumers and 4/16 read proofs. |
| `messages/declaration_tests.rs`, `root-declarations.rb`, vector | Actual HTTP meta on both sides, retaining every edit/no-op/tombstone/provider declaration. |
| `oracle-state.rb`, `comparison_support.rs`, relative/periodic generators and vectors | Lossless baseline-plus-row-change representation; candidates still select every complete persisted row. |
| Producer tools, reference-source guard, oracle replay tool | Restored-source negative controls, 55 independent replays and explicit approved-boundary diagnostic. |
| `ws8bm2-approved-differences.md` | Lead-approved atomic enqueue rollback ledger. |

## Verification

All builds use the CI image, two build jobs and the unchanged rustc throttle.
The workspace uses four test threads, with a capacity guard. Exact single-test
controls retain that flag and reserve their one actual worker when sharing the
machine. No deadlines, retries, concurrency flags or allowlists were relaxed.

Fresh clone: `.scratch/ws8bm2-h/fresh-final`, created with `git clone
--no-hardlinks --single-branch --branch rust/ws8bm2-message-features-h <this
worktree>`. It built its own default/first-run/agents-ui seeds; CI fails on missing
seeds. Only the existing owned compiler cache is shared, with source timestamps
forced newer than producer-control artifacts. No fixture state or generated test
outputs were copied into this clone.

The full workspace and 55 oracles ran at production code `941b4f123c63be9fe3578f64f865927714b328ef`.
Clippy then found the static regex inside the two-zone test loop. Commit
`e779c2c9632ca153182d1e65dc4485ef4323807b` only hoists that test regex; it changes
no assertion, production source or vector. This is the sole non-documentation
change after the full suite. After fast-forwarding the same clean clone, the
focused real HTTP comparison and its real UTC-output producer control were
rerun, followed by strict clippy and the release-input build. The final report
commit changes documentation only.

Commands actually run, from the fresh clone (CI wrapper/environment retained):

```sh
bash rust/parity/bin/seed build default first_run agents_ui
bash rust/ci/cargo.sh metadata --locked --format-version 1
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 --no-run
bash rust/ci/cargo.sh test --locked --workspace --no-fail-fast -j2 -- --test-threads=4 --nocapture
WS8BM2_ORACLE_SCRATCH=/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2/.scratch/ws8bm2-h/final-gates/oracle-replays python3 rust/reference-tools/messaging/verify_oracles.py
bash rust/ci/cargo.sh test --locked -p campfire --bin campfire controllers::messages::declaration_tests::root_edit_markers_match_rails_for_noops_attachments_formatting_reactions_fetches_tombstones_and_zones -j2 -- --exact --test-threads=4 --nocapture
bash rust/ci/cargo.sh clippy --locked --workspace --all-targets -j2 -- -D warnings
cd rust
bash ci/with-release-inputs.sh bash ./ci/cargo.sh build --locked --workspace --bins
```

Final raw summaries (terminal colors removed; all workspace result lines retained):

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
cargo metadata --locked: exit 0
WS8bm2 full workspace aggregate: 4821 passed; 0 failed; 14 ignored; 61 result groups
test result: ok. 2726 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 876.48s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.45s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.81s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.09s
test result: ok. 1357 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 122.68s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.40s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.25s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.99s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.54s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.16s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.82s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.96s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.60s
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
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2730 filtered out; finished in 1.06s
WS8bm2 hoisted-root producer: rejected at actual viewer-zone meta
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 09s
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 03s
WS8bm2 reference source check: 120 controller, model, helper and template files match 955af4c3781bef07b97b7aefce12c376110a812c
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
WS8bm2 oracle replay: 55/55 independently replayed fixtures byte-identical
```

The first fresh suite caught the legacy reader/writer cache-key mismatch and the
inherited detached-UTC meta comparison. These were corrected before the final
clone; no failure was hidden or threshold widened. Raw earlier lines:

```text
test result: FAILED. 2725 passed; 1 failed; 5 ignored; 0 measured; 0 filtered out; finished in 1111.08s
test result: FAILED. 52 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.83s
```

Controlled regressions run this round, with all temporary producer bytes restored:

```sh
python3 rust/reference-tools/messaging/check_slash_count_mutant.py --expect-escape -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/check_slash_count_mutant.py -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/check_rendering_mutants.py -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/check_input_mutants.py -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/probe_adapter_rejections.py --output .scratch/ws8bm2-h/adapter-actual.json -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 rust/reference-tools/messaging/probe_adapter_rejections.py --control --output .scratch/ws8bm2-h/adapter-control-actual.json -- bash .scratch/ws8bm2-f-order216/run-test.sh
python3 .scratch/ws8bm2-h/verify-compaction.py
```

The old-comparator replay now requires both actual count increases, 185→186 and
186→187. The rendering control runner requires exactly one selected test; its
original substring filter selected 1 and 10–13 together. The final 13 receipts
below use exact names, each with one intended assertion failure. These tooling
errors were fixed, not treated as successful evidence.

```text
Old returned-message comparator observed actual counts: [185,186]
Old returned-message comparator observed actual counts: [186,187]
WS8bm2 slash count producer: escaped old comparator
WS8bm2 slash count producer: rejected at actual total-message counts
WS8bm2 rendering producer mutant 1: rejected at warm zone actual legacy fragment differs from Rails: America/New_York
WS8bm2 rendering producer mutant 2: rejected at warm zone actual legacy fragment differs from Rails: America/New_York
WS8bm2 rendering producer mutant 3: rejected at wide HTML actual saved partial differs from Rails
WS8bm2 rendering producer mutant 4: rejected at wide HTML actual scheduled partial differs from Rails
WS8bm2 rendering producer mutant 5: rejected at wide HTML actual HTTP notice differs from Rails
WS8bm2 rendering producer mutant 6: rejected at periodic actual persisted rows.reminded_at
WS8bm2 rendering producer mutant 7: rejected at periodic actual durable jobs differ from Rails
WS8bm2 rendering producer mutant 8: rejected at periodic actual persisted row count: messages
WS8bm2 rendering producer mutant 9: rejected at periodic actual persisted row count:
WS8bm2 rendering producer mutant 10: rejected at periodic actual publications
WS8bm2 rendering producer mutant 11: rejected at periodic actual registered ticks differ from Rails
WS8bm2 rendering producer mutant 12: rejected at warm zone actual legacy fragment differs from Rails: America/New_York
WS8bm2 rendering producer mutant 13: rejected at actual viewer-zone meta differs from Rails: Pacific Time (US & Canada)
WS8bm2 input producer mutant 1: rejected at relative/split actual output differs from Rails
WS8bm2 input producer mutant 2: rejected at relative/split actual output differs from Rails
WS8bm2 input producer mutant 3: rejected at container actual HTTP envelope differs from Rails
WS8bm2 input producer mutant 4: rejected at container persisted rows.status
WS8bm2 input producer mutant 5: rejected at container actual HTTP envelope differs from Rails
WS8bm2 input producer mutant 6: rejected at container actual publications UTC/slash_0
WS8bm2 input producer mutant 8: rejected at container actual HTTP envelope differs from Rails
WS8bm2 adapter approved-difference probe: 8/8 atomic refusal executions; complete actual rows and empty publications; flat reads; atomic rollback is an approved difference under decision 2
WS8bm2 adapter enqueue-error producer mutant: rejected at adapter actual durable refusal
WS8bm2 lossless relative_consumers: 13417007 -> 1266737 bytes; all envelopes, complete rows, read counts and frame bytes identical; expanded SHA256 61a3c706684d77b9ed010b3e924a06b44bf91eaa220b02aae09a9c1f54007580
WS8bm2 lossless periodic_delivery: 8362748 -> 3768430 bytes; all envelopes, complete rows, read counts and frame bytes identical; expanded SHA256 c8cc423b2cb74345ff142d58e6e4bea563c4f2d629024af55923862e38da0657
```

Failing-first original runtime summaries, with actual differing fields described
above; these expected failures precede the fixes:

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2728 filtered out; finished in 7.98s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2729 filtered out; finished in 0.44s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2729 filtered out; finished in 1.88s
```

Fresh positive coverage summaries:

```text
WS8bm2 warm legacy message size=4: Rust 133; complete independently captured Rails partials and actual cache hits in four zones
WS8bm2 warm legacy message size=16: Rust 133; complete independently captured Rails partials and actual cache hits in four zones
WS8bm2 container Rust: 80 requests; 0 envelope differences; complete persisted tables, ordered publications and flat reads
WS8bm2 container Rust: 84 requests; 0 envelope differences; complete persisted tables, ordered publications and flat reads
WS8bm2 periodic Rust: 24 actual registered ticks; complete rows, jobs and byte-identical ordered publications; flat reads
WS8bm2 wide HTML Rust: 32 cases; 96 byte-identical real partials; flat reads
WS8bm2 wide HTTP Rust: 48 actual POST/GET pairs; byte-identical date tags/fields and notices; full persisted rows; flat reads
```

Both fresh scratch `rust/target` output directories were deleted after checking.
The pre-existing owned compiler cache was retained. All owned H native/ Rails
coordinators and throwaway containers are stopped. No browser, deployment,
stash, other-worktree edit or model-server operation occurred.



## Retained named Rails ports and browser inventory

All 35 named slash comparisons and 140 named controller behaviors remain inventoried, and their Rust ports run in the final workspace suite. This is not a claim that all Rails test files ran as suites; evidence is the independently replayed Rails vectors and named Rust tests. Existing earlier command mutations are historical evidence, not claimed as rerun this round.

| Rails name | Rust test name |
| --- | --- |
| registry holds every shipped command with metadata | `builtin_registry_holds_every_shipped_command_with_metadata` |
| command_text? matches slash commands but not escapes or play passthrough | `builtin_command_text_matches_slash_commands_but_not_escapes_or_play_passthrough` |
| huddle starts a call when configured | `builtin_huddle_starts_a_call_when_configured` |
| huddle errors when unconfigured | `builtin_huddle_errors_when_unconfigured` |
| event opens the prefilled form url | `builtin_event_opens_the_prefilled_form_url` |
| event without a time prefills the title only | `builtin_event_without_a_time_prefills_the_title_only` |
| event rejects past times | `builtin_event_rejects_past_times` |
| bare event opens the blank form | `builtin_bare_event_opens_the_blank_form` |
| poll opens the builder in channels but not threads | `builtin_poll_opens_the_builder_in_channels_but_not_threads` |
| remind posts and saves with a reminder | `builtin_remind_posts_and_saves_with_a_reminder` |
| remind rejects unusable input without posting | `builtin_remind_rejects_unusable_input_without_posting` |
| status sets emoji and text until end of day | `builtin_status_sets_emoji_and_text_until_end_of_day` |
| status rejects blank arguments | `builtin_status_rejects_blank_arguments` |
| dnd toggles, takes durations, and turns off | `builtin_dnd_toggles_takes_durations_and_turns_off` |
| dnd rejects garbage durations | `builtin_dnd_rejects_garbage_durations` |
| ooo sets an end with a note, and off clears it | `builtin_ooo_sets_an_end_with_a_note_and_off_clears_it` |
| ooo takes week durations, dates, and datetimes | `builtin_ooo_takes_week_durations_dates_and_datetimes` |
| ooo bare tomorrow and weekdays run to the end of the day | `builtin_ooo_bare_tomorrow_and_weekdays_run_to_the_end_of_the_day` |
| ooo bare dates run to the end of the day | `builtin_ooo_bare_dates_run_to_the_end_of_the_day` |
| ooo bare month dates roll to next year when this year's passed | `builtin_ooo_bare_month_dates_roll_to_next_year_when_this_year_s_passed` |
| ooo day durations stay exact | `builtin_ooo_day_durations_stay_exact` |
| ooo broadcasts the badge and the notice | `builtin_ooo_broadcasts_the_badge_and_the_notice` |
| ooo off while calendar OOO covers says the calendar still shows it | `builtin_ooo_off_while_calendar_ooo_covers_says_the_calendar_still_shows_it` |
| ooo rejects blank arguments, garbage, past times, and long notes | `builtin_ooo_rejects_blank_arguments_garbage_past_times_and_long_notes` |
| shrug posts with the shrug | `builtin_shrug_posts_with_the_shrug` |
| posting commands in a board answer an error without posting | `builtin_posting_commands_in_a_board_answer_an_error_without_posting` |
| posting commands in a board thread still post | `builtin_posting_commands_in_a_board_thread_still_post` |
| slash posts in threads skip the legacy webhook fanout | `builtin_slash_posts_in_threads_skip_the_legacy_webhook_fanout` |
| slash posts in channels fan out to legacy webhooks | `builtin_slash_posts_in_channels_fan_out_to_legacy_webhooks` |
| slash posts in threads process attachments once | `builtin_slash_posts_in_threads_process_attachments_once` |
| me posts an action line | `builtin_me_posts_an_action_line` |
| me requires an action | `builtin_me_requires_an_action` |
| play posts through the normal message path | `builtin_play_posts_through_the_normal_message_path` |
| slash posts never start a stream | `builtin_slash_posts_never_start_a_stream` |
| unknown commands error with the available list | `builtin_unknown_commands_error_with_the_available_list` |

| File under `test/controllers/` | Named behaviors covered / total |
| --- | --- |
| `rooms/polls_controller_test.rb` | 16/16 |
| `messages/pins_controller_test.rb` | 6/6 |
| `rooms/pins_controller_test.rb` | 3/3 |
| `saved_items_controller_test.rb` | 12/12 |
| `scheduled_messages_controller_test.rb` | 19/19 |
| `searches_controller_test.rb` | 36/36 |
| `rooms/slash_commands_controller_test.rb` | 10/10 |
| `autocompletable/icons_controller_test.rb` | 6/6 |
| `autocompletable/slash_commands_controller_test.rb` | 7/7 |
| `autocompletable/users_controller_test.rb` | 5/5 |
| `rooms/message_links_controller_test.rb` | 12/12 |
| `rooms/files_controller_test.rb` | 8/8 |

| Browser file under `test/system/` | Prior accepted Rails/Rust behavior results |
| --- | --- |
| `polls_test.rb` | 4/4 each |
| `pins_saved_test.rb` | 7/7 each |
| `slash_commands_test.rb` | 26/26 each |
| `search_files_test.rb` | 4/4 each |
| `scheduled_messages_test.rb` | 4/4 each |
| Total | 45/45 each; not rerun this checkpoint |
