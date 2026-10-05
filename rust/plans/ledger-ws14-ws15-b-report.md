# WS14 / WS15 acceptance continuation

Stacked on PR #239 head `ca92288e839e9dd8e97f83dc236ae8f889ddd3c4`; branch `rust/ledger-ws14-ws15-b`. No commits from this continuation are pushed to the parent branch. Batches follow the remaining ledger's Rails file/declaration order. Each new named test reproduces the original setup and discriminating assertions through the real Rust model/HTTP/job path. Duplicate declarations in the two inventories share the same named test and are counted explicitly as ledger records.

## Batch 1: test/models/event_test.rb

Twelve open declarations are closed by twelve named tests in `crates/db/src/tests/calendar_event_test/cutover_event_test.rs`. Setup uses the real User, Membership, Room, CalendarEvent and ActivityItem APIs, the Rails fixtures and a FrozenClock. Cases distinguish early/equal/no end, bot/outsider organizers, a deleted venue, each notification involvement, going/maybe/declined responses, unread state, duplicates, title-only edits, cancellation, membership accessibility and complete room deletion through its registered domain consumer. All pass without a production change.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_event_test::)' --no-fail-fast
```

```text
Summary [   0.755s] 12 tests run: 12 passed, 1384 skipped
```

The original unmodified Rails file also runs in the reference container with network disabled and four processes:

```text
18 runs, 74 assertions, 0 failures, 0 errors, 0 skips
```

The image tag `ws14e-reference:d7c7de92` has a later revision environment label, so it is not trusted by tag alone. The actual `app/models/event.rb` and `test/models/event_test.rb` SHA-256 hashes match `git show d7c7de92:<path>` byte-for-byte; shared model sources are checked separately. Rails execution receipts are tied to the verified source bytes. Native checks use the canonical Rust 1.98.1/libvips toolchain image, the existing machine-wide compiler slots, build jobs 2 and nextest workers 4. No real Google HTTP or scanner-shaped tokens.

## Batch 2: Calendar entry and channel timeline lifecycles

Seven new named tests close ten ledger rows (the three entry declarations appear in both WS14e and WS14g). Added the missing direct `google_entry::destroy` model API using the existing `RemoteDeleteJob` type, with captured arguments published after commit. Added the callback-free association delete_all API and made the real disconnect producer consume it. Reconciliation's existing `delete` continues to skip remote work. Timeline tests assert exact singleton announcement count, creator, title/relative URL, reference rows, no edit/cancel posts, no message inbox items, and deletion of references without deleting the message.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_entry_test::) | test(cutover_timeline_test::)' --no-fail-fast
```

```text
Summary [   0.303s] 7 tests run: 7 passed, 1396 skipped
```

Original Rails files: `10 runs, 40 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **22 ledger records closed; 199 remain**.

## Batch 3: recurrence guards, ordering, uniqueness and local edits

Nine original recurrence declarations are closed. Eight model tests cover nil singleton series/neighbors, head update range limits and rollback, unchanged recurrence form values, the active-series-slot unique index (including a real rejected SQLite write), equal-time cancelled ordering, over-cap rule rollback, a retimed head-only series and a local description edit. The ninth runs a guard-flag injection through real HTTP (422, weekly unchanged); a compile-fail EventChanges doctest additionally proves the unknown attribute cannot be supplied to the typed model. Rust cannot dynamically assign unknown model fields as ActiveRecord does; this checks both its typed boundary and the observable request boundary.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_recurrence_test::)' --no-fail-fast
cargo nextest run -p campfire -j 4 -E 'test(cutover_recurrence_guard_flag_injection)' --no-fail-fast
```

```text
Summary [   0.325s] 8 tests run: 8 passed, 1403 skipped
Summary [   0.896s] 1 test run: 1 passed, 2853 skipped
```

Original Rails recurrence file (source bytes checked against d7c7de92): `63 runs, 360 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **31 records closed; 190 remain**.

## Batch 4: references, reminder occurrences and venues

Eighteen named model tests close the next eighteen declarations. References check missing/no links, exact event identity on edit, and message deletion preserving the event. Reminders use the real due-ID scan and per-event transactional claim, with frozen midnight and consecutive occurrence clocks. Venues cover optional/voice/stage, rejected text/DM/nonmember rooms and exact error, self-room venue, retained links after leaving, real room deletion, all/local propagation, clearing, and no inbox mutation for venue-only edits. No production divergence found.

```sh
cargo nextest run -p campfire_db -j 4 -E 'test(cutover_reference_test::) | test(cutover_reminder_test::) | test(cutover_venue_test::)' --no-fail-fast
```

```text
Summary [   0.726s] 18 tests run: 18 passed, 1411 skipped
```

Three original Rails files (source bytes matched d7c7de92): `35 runs, 117 assertions, 0 failures, 0 errors, 0 skips`. Total continuation: **49 records closed; 172 remain**.

## Batch 5: remaining Event HTTP, attendance, interaction and card assertions

Thirty-two named HTTP tests close the next thirty-two WS14e declarations: 24 event controller cases, two attendance propagation cases, three scheduling/inbox/card interaction cases and three card integration cases. Setup uses the original Rails fixtures, fresh real sessions, the real CSRF/router path and a frozen September 22 clock. Non-fixture seed rows are cleared before fixture loading; a seed-issued cookie is not reused after replacing its sessions.

The two whole-request index query guards exposed missing preloads. Before the fix the occurrence-count case reads **47 then 57** statements, and the shared-venue case reads **38 then 78**. Both original Rails guards pass. The index now loads events and all presentation associations in batches (organizers, attendance/counts, venues, venue membership/live streams, calendar copies and series neighbors), keeping the show reader intact. Both guards now pass without count growth; the room/event-link guard already passes against the owner renderer. No JS/CSS or asset fingerprints changed.

The interaction ports exercise the server assertions behind the original system flows: links and form fields, schedule redirects and heading/description/current response, series list labels, invitation identity/count, inbox open redirect, persisted responses, lazy card attendance frame and an in-frame response without navigation. A pinned Rails HTTP probe runs those same assertions: `reference-tools/cutover/events-interactions.rb`. It uses real Rails controllers/models and fixture users, not generated expectations from Rust. It passes **3 runs, 114 assertions, 0 failures, 0 errors, 0 skips**. No browser/pixel run is claimed.

The three original controller/attendance/card files also run unmodified in Rails: **79 runs, 498 assertions, 0 failures, 0 errors, 0 skips**. Their installed source bytes match d7c7de92. Targeted Rust rerun also includes the prior attendance-shape and event-page regressions:

```sh
cargo nextest run -p campfire -j 4 -E 'test(tests::cutover::) | test(pr174_) | test(event_pages)' --no-fail-fast
```

```text
Summary [  28.141s] 36 tests run: 36 passed, 2850 skipped
```

Total continuation at the original starting head: **81 ledger records closed by 78 named native tests; 140 remained**. The exact open declarations remain in the gate and are listed in `ledger-ws14-ws15-remaining.md`.

## Parent review reconciliation

While final verification was running, the parent PR advanced to `ec35cd92c74a07addc2f91e9f67207098c8b7d64`. Merge commit `553eb2196fe265346d75dca688f57112e921964a` retains its strengthened tests and receipt corrections, together with all 81 closures above. No continuation commits were pushed to the parent branch.

The parent review reopened four earlier receipt claims, which are preserved as open assertions rather than folded into this slice:

- **WS14e-051**: execute the registered reminder-push job with the Lounge venue and assert the queued push suffix.
- **WS14e-057**: execute the registered reminder-push job for a recently started but already ended event, with a still-running control.
- **WS15g-056**: execute the registered GitHub fetch-PR job for a mapped PR and assert the thread card/header broadcast title and file.
- **WS15g-057**: execute that job with neither references nor mappings and assert zero broadcasts plus the committed fetch.

Current disposition: **81 assertions closed in this slice; 144 remain (2 WS14e, 97 WS14g, 45 WS15g)**. This is the requested coherent stopping point at about 80 closures. The ledger retains the precise Rails declarations, missing assertions and current row locations for all remaining records.


## Final validation

Verified code: `553eb2196fe265346d75dca688f57112e921964a`, in a fresh local clone created with `git clone --no-hardlinks`, then advanced to the merged head. Only this report changes after that code head. Commands below run inside the canonical `campfire-toolchain-ci-rust-speedups:latest` image with Rust 1.98.1, canonical libvips, `CI=true`, build jobs 2, the existing machine-wide rustc slot throttle and nextest workers 4. The shared `rust/target` cache is retained; checkout inputs come exclusively from the fresh clone. No test timing threshold or concurrency setting changed.

All three current-schema seeds were rebuilt and validated by Rails before being copied into the fresh clone:

```sh
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed prepare
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed check-image
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed build
PARITY_IMAGE=ws11ui-cutover-reference:current-schema rust/parity/bin/ci-seed validate
```

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

Final full-workspace run:

```sh
cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast
```

```text
     Summary [1876.631s] 5034 tests run: 5034 passed (7 slow), 20 skipped
```

The earlier fresh-clone run before merging the parent review corrections also passed: `Summary [1748.683s] 5030 tests run: 5030 passed (7 slow), 20 skipped`. The final PASS log contains **78/78** unique test names from the 81 updated records; none of this slice's tests is skipped. These tests run under the existing blocking db/app CI package selectors in `.github/workflows/rust.yml`. The canonical `avatar_bot_logo_uploads_match_pinned_rails` comparison passes. No inherited flake appeared.

Strict lint check on the merged code:

```sh
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized] target(s) in 43.01s
```

Exit status 0. Two test-only helper lints encountered earlier were fixed by moving the regex outside its loop and returning the query count directly; no lint suppression was added. The affected HTTP tests were rerun:

```sh
cargo nextest run -p campfire -j 4 -E 'test(tests::cutover::)' --no-fail-fast
```

```text
     Summary [  20.911s] 33 tests run: 33 passed, 2853 skipped
```

Workspace doctests (the recurrence guard compile-fail case passes; the two existing kit examples remain ignored):

```sh
cargo test --workspace --exclude html5ever --doc --no-fail-fast -- --test-threads=4
```

```text
test crates/db/src/models/calendar_event/changes.rs - models::calendar_event::changes::EventChanges (line 20) - compile fail ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Ledger validation:

```sh
python3 rust/reference-tools/cutover/check-ws14-ws15.py
```

```text
Acceptance ledger: 445 records checked; 181 baseline-CI passed; 119 new assertions; 1 test-only outside gate; 144 explicitly open
```

Production-input binary build, excluding test vectors/reference tools and Rails source outside the explicit asset inputs:

```sh
bash ci/with-release-inputs.sh cargo build --workspace --bins
```

```text
    Finished `dev` profile [unoptimized] target(s) in 1m 20s
```

Exit status 0. Own fresh-clone/Rails storage/scratch output is removed after the PR is opened; the shared target cache and inherited worktree output are preserved.

## PR #243 per-assertion audit

Merged parent review head ec35cd92 (merge 553eb219); its four reopened declarations remain explicitly open and belong to the next continuation. Parent audit fix239b has not landed as of this receipt.

All **81 continuation records / 271 original Rails assertion calls** now have an individual mapping in `ledger-ws14-ws15-b-assertions.json` and its rendered Markdown. The validator checks physical citations, complete original assertion enumeration and actual native PASS names; semantic pairing is reviewed manually and sampled with production mutations. Historical receipts remain in place.

Strengthened real-path assertions cover the organizer-specific error with an independently member bot, event-scoped title-only activity counts, deletion of captured attendance IDs, actual Event association readers, simultaneous title/time edits, parsed heading/card selectors, card-scoped organizer text, Turbo redirects and the existing-zone/nonmember-venue request setup. Validation errors are asserted directly before checking their messages. The recurrence flag reset is checked by its next-update validation consequence, with a pinned-Rails probe (`event-guard-reset.rb`).

Audit run (canonical Rust 1.98.1/libvips container, unchanged machine rustc slots, nextest workers 4):

```sh
cargo nextest run --workspace --exclude html5ever -j 4 -E 'test(cutover_)' --no-fail-fast
python3 rust/reference-tools/cutover/assertion-maps.py rust/plans/ledger-ws14-ws15-b-assertions.json --render --pass-log AUDIT_PASS_LOG
cargo clippy --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
     Summary [  16.257s] 78 tests run: 78 passed, 4976 skipped
Assertion maps: 81 records; 271 original Rails assertion calls; 0 unmapped
    Finished `dev` profile [unoptimized] target(s) in 1m 13s
```

Direct Result assertions added after this audit run replace the previous equivalent unwrap-error / error-variant panic guards; they change where the mutation fails, not the tested predicate. They were compiled by the mutation build and strict clippy. The full unmutated workspace run below executes these exact assertions, including the direct Result guards.

Nineteen distinct newly added tests have retained mutation failures, each at an assertion mapped to its Rails declaration. See `ledger-ws14-ws15-b-mutations.md` for every raw nextest summary and panic citation, and the JSON receipt for production source hashes. No compile/setup failure counts. The equality-end exploratory mutation also failed, but is not counted in the retained sample. Temporary production guards are absent from the committed tree.

```text
Mutation summary: 19 run; 19 killed; production restored=True
```

Pinned Rails guard probe (`d7c7de92`, network disabled):

```text
1 runs, 4 assertions, 0 failures, 0 errors, 0 skips
```

```sh
python3 rust/reference-tools/cutover/check-ws14-ws15.py
```

```text
Acceptance ledger: 445 records checked; 181 baseline-CI passed; 119 new assertions; 1 test-only outside gate; 144 explicitly open
```

Final PR #243 audit validation (compiled audit code ec172c88a; no temporary mutations):

```sh
cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast
python3 rust/reference-tools/cutover/assertion-maps.py rust/plans/ledger-ws14-ws15-b-assertions.json --pass-log WORKSPACE_PASS_LOG
```

```text
     Summary [2036.636s] 5034 tests run: 5034 passed (8 slow), 20 skipped
Assertion maps: 81 records; 271 original Rails assertion calls; 0 unmapped
```

All 78 named native tests used by the 81 closures ran and passed. The canonical `avatar_bot_logo_uploads_match_pinned_rails` comparison passes. No inherited timing flake appeared. The earlier fresh-clone receipt is historical; this audit validation runs in the working checkout against the newly compiled, restored audit code. The new stacked continuation has separate receipts.

## Full parent audit reconciliation (6880979a)

The fix239b parent audit landed while continuation C was being verified. It is merged into #243, including its reviewed reference-pin refresh and strengthened code/assertions. All 81 B closures and their 271 assertion mappings are retained. The incoming parent audit metadata and all 32 newly reopened contracts are retained separately; none of those reopens is replaced by an unrelated B receipt. The combined ledger now has **176 open records** before C's closures, rather than the earlier 144. These are precise missing assertions, not owner waivers.

The ledger verifier now checks both the original parent audit schema and the continuation assertion maps. It preserves the historical parent full-run hash/counts; new continuation execution is recorded separately. Native test identifiers in B receipts now use nextest's actual `campfire::bin/campfire` package identifier. The earlier full-workspace receipts above remain tied to their recorded code heads; validation of this integration is recorded in the continuation C report.

```text
Assertion maps: 81 records; 271 original Rails assertion calls; 0 unmapped
Acceptance ledger: 445 records checked; 86 baseline-CI passed; 182 current implementation receipts; 1 test-only outside gate; 176 explicitly open
Full closure audit: 220 records; 555 Rails assertions/predicates; 487 mapped; 78 strengthened closures; 32 reopened
```


## PR #243 rendered interaction correction

Reviewed head `31b76775752d541e3e5d9d9cd3a7312a2761ca47`. This correction uses the separately checked-out `rust-ws14g-b` worktree and changes only Rust inputs. The earlier system-closure claim above is historical: the earlier interaction HTTP probe also fabricated requests and is not a current browser receipt.

- **WS14e-099 — option (a):** read the rendered room Events, New event and All events links; submit the actual Schedule, inbox Open and Going forms using their action, method, hidden fields (including CSRF and `_method`), selected option values and successful submitter. Follow the server's returned Location rather than requesting a separately known page. Check all nine original Rails assertions through the real router and persisted model state.
- **WS14e-100 — option (a):** the same real markup driver selects the rendered Weekly option and fills its recurrence-until control. All eleven original Rails assertions retain their exact scope, invitation count and three propagated responses.
- **WS14e-101 — option (b):** reopen `test/system/events_test.rb:136` (actual browser current path) and `:137` (announcement retained in the same browser DOM after the click). A server response or a fresh room GET cannot establish those predicates. The partial HTTP regression now reads the actual lazy frame src/id, attendance form action/method/hidden fields and effective rendered submitter/form/frame target; it does not force a frame header when the markup says `_top`. Its response and persisted RSVP checks are retained as partial evidence, with the missing browser assertions clearly marked.

The test-only markup driver follows the vendored Turbo target precedence and sends successful HTML controls through `Browser::send`; it does not add a separately synthesized CSRF header. No production template/controller change or browser/pixel run is needed. The system Rails test file is checked byte-for-byte against pin `78b9b1546`. Every changed assertion has a refreshed physical file/line mapping, including the real redirect-following guard that now rejects a wrong form action. The ledger verifier separately checks closed and explicitly reopened continuation records, so a partial map cannot count as a closure.

Current B disposition: **80 continuation records closed; 177 explicitly open**. The 81-record/271-call map retains the historical record and now has exactly **2 unmapped browser assertions in one reopened record**. The structured receipt, original inventory and remaining list agree. Original receipts are preserved rather than rewritten as current proof.

### Before/after rendering mutations

[Exact substitutions](../reference-tools/cutover/review243-mutations.json) and the [compact receipt](ledger-ws14-ws15-b-review243-mutations.json) retain the reviewed head, source hashes, activation messages, raw summaries and assertion locations. Each of the three reviewed tests passes with the fault against the old assertion bodies; each fails after the markup driver correction. Both mutated production files are restored byte-for-byte and match the reviewed head. No compile/setup failure counts as a kill.

```sh
python3 rust/reference-tools/cutover/mutation-check.py rust/reference-tools/cutover/review243-mutations.json rust/target/review243-fixes/before -- rust/target/review243-fixes/mutation-cargo.sh
python3 rust/reference-tools/cutover/mutation-check.py rust/reference-tools/cutover/review243-mutations.json rust/target/review243-fixes/after -- rust/target/review243-fixes/mutation-cargo.sh
python3 rust/reference-tools/cutover/mutation-check.py rust/reference-tools/cutover/review243-mutations.json rust/target/review243-fixes/after --recheck
```

The executable prefix invokes `cargo nextest run --workspace --exclude html5ever -j 4 --no-fail-fast -E 'test(<name>)' --success-output immediate` in the canonical Rust/libvips environment with the unchanged shared rustc throttle. Before the aggregate command exits 1 because the three tests survive (all actual nextest processes exit 0); after it exits 0 because each actual nextest process rejects its fault at the intended assertion.

```text
BEFORE WS14e-099-action:
     Summary [   0.905s] 1 test run: 1 passed, 5057 skipped
BEFORE WS14e-100-action:
     Summary [   0.872s] 1 test run: 1 passed, 5057 skipped
BEFORE WS14e-101-frame:
     Summary [   0.918s] 1 test run: 1 passed, 5057 skipped
Mutation summary: 3 run; 0 killed; 3 not accepted; production restored=True
AFTER WS14e-099-action:
     Summary [   0.888s] 1 test run: 0 passed, 1 failed, 5057 skipped
AFTER WS14e-100-action:
     Summary [   0.846s] 1 test run: 0 passed, 1 failed, 5057 skipped
AFTER WS14e-101-frame:
     Summary [   0.971s] 1 test run: 0 passed, 1 failed, 5057 skipped
Mutation summary: 3 run; 3 killed; 0 not accepted; production restored=True
```

The two scheduling faults fail at `cutover/rendered.rs:204` when following the real submission: the rendered wrong action returns 404 instead of the required redirect. That helper assertion is mapped alongside the original post-Schedule heading assertion. The `_top` button fault fails at `cutover/interactions.rs:162`: submitting the rendered top-level target returns 302 rather than the frame's 200. The assertion is mapped to the partial server Going-response predicate; the explicit browser-current-path claim remains open even though this wiring fault is now caught.

### Corrected B checkout validation

Verified native source `ea09026ba201fdfdc8bacbfa0893ec21a5b790f7` in the separate B worktree, with fresh pinned default/first_run/agents_ui seed validation (29 + 4 + 40 = 73 passed, zero failed). Canonical Rust 1.98.1/libvips, `CI=true`, two build jobs, unchanged shared compiler throttle and nextest workers 4. All 78 named native tests in the 81-record map ran; the 20 skips are the existing ignored tests. The canonical logo/PNG comparison passes. No timing failure appeared. Only ledger/report metadata follows this native source.

```sh
cargo nextest run --locked --workspace --exclude html5ever -j 4 --no-fail-fast
cargo clippy --locked --workspace --exclude html5ever --all-targets -- -D warnings
python3 rust/reference-tools/cutover/assertion-maps.py rust/plans/ledger-ws14-ws15-b-assertions.json --pass-log rust/target/review243-fixes/nextest.log
python3 rust/reference-tools/cutover/check-ws14-ws15.py --nextest-log rust/target/review243-fixes/nextest.log
```

```text
     Summary [1217.504s] 5038 tests run: 5038 passed (1 slow), 20 skipped
    Finished `dev` profile [unoptimized] target(s) in 54.84s
Assertion maps: 81 records; 271 original Rails assertion calls; 2 unmapped
```

Both Cargo commands exit 0. Compact raw logs and mutation receipts are archived in the C worktree under `rust/target/review243-fixes/` before this B worktree is removed. Generated seeds, reference inputs and test runtime output are removed with the disposable B worktree. The C integration has a separate validation receipt.
