# Board DELETE and Twitter operator backfill

Branch: `rust/board-delete-twitter-backfill`, started from `origin/main` at
`78b9b1546bdab4c6c1c9b8ddb94512f661289112`.

`DELETE /rooms/boards/:id(.:format)` now dispatches to a board-scoped handler.
It uses the existing room lookup, creator/administrator guard and destroy
lifecycle: soft deletion, membership removal, durable `Room::DestroyJob`,
`room.destroy` audit, global board-row removal broadcast, HTML/Turbo HTML
fallback redirect/flash, and JSON response. It does not duplicate cleanup.
The registered durable worker removes board-specific rows and purges the
message attachment and its file; the regression starts a new runner after
the request and checks the completed cleanup and unchanged FK findings.
The seed deliberately contains one preexisting message with a deleted author.
The established atomic-queue difference remains: queue rejection rolls back
the source write in Rust, whereas Rails separately retries its enqueue.

The route audit covers all 431 defined/implicit table entries (378 distinct
endpoints). No such entry now resolves to `not_yet_ported`. Google Calendar
notifications already had a dedicated unparsed-body live-router binding;
the table now names that same existing action too. Its existing HTTP security
and atomic-enqueue tests remain in the affected checks.

## Rails reference defect: board-route parity remains flagged

Fresh execution of the **unchanged** Rails board DELETE route produces
`NoMethodError: undefined method 'name' for nil`, raised by
`app/controllers/rooms_controller.rb:29` at `room_label = @room.name`.
`Rooms::BoardsController` re-declares the inherited callback names with scopes
that exclude `destroy`: `set_room` at line 2 and `ensure_can_administer` at
line 3. Rails replaces the inherited callback registrations. Seven allowed,
forbidden, inaccessible, missing and wrong-type probes all return 500 before
any deletion, membership change, audit, broadcast or destroy job.

The Rust handler implements the requested inherited destroy behavior. Its
six byte/state/broadcast/job comparisons execute Rails' working **base**
`/rooms/:id` route on actual board records, through real sessions and CSRF.
The vector records `reference_path` separately from Rust's board `path`,
retains the original `board_route_status: 500`, and explains the difference.
It does **not** claim the original Rails board route succeeds or that its
500 is byte-identical to Rust. Missing/deleted/wrong-type board scoping,
membership-before-role checks, session/CSRF, forbidden no-writes, and queue
rollback also have real Rust HTTP regressions.

Exact board-route parity is not closed: it needs the Rails controller owner
or lead to include `destroy` in both callback scopes (and regenerate the
board-route evidence), or expressly accept this reference-crash difference.
Rails source was left unchanged, following `rust/AGENTS.md`'s reference rule.
No status/body assertion is masked.

## Twitter maintenance operation

```sh
campfire twitter-backfill-references /path/to/storage/db/production.sqlite3
```

The offline command accepts only an existing, compatible database. It does
not create/migrate a database, boot HTTP, load application credentials,
start job workers, or perform network fetches. It uses the real Twitter
reference-sync service and durable job sink. Selection preloads Markdown and
rich text in primary-key batches of 1,000, including negative and zero IDs;
later batches use the primary-key range. Each matching message commits
independently, preserving earlier progress if a subsequent sync fails.
Its reference changes, fetch claim and fetch job commit together under the
approved atomic-queue rule. Re-running preserves rows, claims and jobs and
prints the same count. Singular/zero output, missing/unknown schemas and
queue rejection/retry are covered.

`reference-tools/twitter_backfill.rb` runs the actual Rails rake task twice
on 1,008 added rows spanning a batch boundary. It covers negative/zero IDs,
legacy link HTML, missing rich text, code-only URLs, forward-note-only
selection exclusion, duplicate posts and the four-post extraction limit.
Output is `Backfilled 8 messages\n`; all nine reference pairs, nine post
identities/URLs/fetch timestamps and seven new job targets match Rust.
Both reference generators verify their relevant Rails source hashes first.
`docs/x-posts.md` documents the Rust invocation alongside the rake task.

## Verification

One Cargo build at a time, build jobs two, test workers four. The configured
rustc slot throttle was preserved. Rails executed its actual new migration
on each seed, then validated default (29/0), first_run (4/0), and agents_ui
(40/0). No fixtures were fabricated to bypass schema checks.

Failing-first controls against the original dispatcher/command:

```text
Summary [   0.363s] 3 tests run: 0 passed, 3 failed, 2850 skipped
```

The route guard lists board DELETE and the missing table alias for the already
live Google webhook. The new operator test rejects the previously unknown
command. The seeded board test in that first run hit the missing migration;
after Rails migrated the seed, the original table bindings were restored
temporarily and both relevant controls failed at their intended assertions:

```text
Summary [   0.972s] 2 tests run: 0 passed, 2 failed, 2859 skipped
```

The board response is 501 instead of 200; the exhaustive route guard reports
the two missing table bindings. The final bindings were then restored.

Final commands (from the worktree root, Rust 1.98.1 via mise; profile debug info
was disabled locally; nextest 0.9.146 was checksum-verified from Dockerfile's pin):

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 --workspace --exclude html5ever --profile ci
cargo clippy --manifest-path rust/Cargo.toml --locked -j 2 --workspace --exclude html5ever --all-targets -- -D warnings
rust/ci/with-release-inputs.sh cargo build --locked -j 2 -p campfire
```

The workspace uses `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER` pointing to
`reference-tools/rooms/pinned_media_runner.py` for the storage vector executable,
with `PARITY_IMAGE=ws11api-reference:d7c7de92`. App media tests first run natively.
All default/first_run/agents_ui seeds are present; `CI=1` makes missing seeds fail.

Raw completed summary lines:

```text
Summary [1295.924s] 4960 tests run: 4953 passed (3 slow), 7 failed, 20 skipped
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 2850 filtered out; finished in 3.31s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2855 filtered out; finished in 2.93s
Finished `dev` profile [unoptimized] target(s) in 1m 55s
```

All 12 newly added tests pass. The seven native failures are existing complete
PNG/WebP/metadata byte comparisons: account logos, fresh video, four missing
video-file representation cases, and the attachment-processing differential.
The **same seven compiled tests, with every assertion enabled**, pass in the
pinned libvips/ffmpeg image. The full storage-vector executable also ran pinned
in the workspace check. Thus all 4,960 executed tests have a passing result in
their appropriate runtime; 20 ignored tests remain ignored, not claimed closed.

The targeted commands use the workspace's compiled campfire test executable
with `--test-threads=4 --exact` followed by the selected full test names (12 new
regressions natively; seven media failures in `docker run --rm --cpus 2` with
`ws11api-reference:d7c7de92`). Both commands are preserved in their raw logs;
no version guard, timing threshold, body, size, checksum or header mask changed.

The release-input build succeeded without fixtures, vectors, parity files or
reference tools in its source input tree. Normal (non-test) binary smoke checks
unset application secret variables and run the real command twice on both an
empty seed and a default seed copy; counts and durable-job idempotence pass.

```text
Finished `dev` profile [unoptimized] target(s) in 1m 57s
Backfilled 0 messages
Backfilled 0 messages
Backfilled 2 messages
Backfilled 2 messages
Production CLI smoke: 4 invocations passed; 0 failed; repeated default run retains one durable fetch job
```

`python3 rust/reference-tools/agents/check-case-ports.py` also passes (51/51
broader named API assertions; zero pending; all 26 pinned source files match).
A final fetch confirmed `origin/main` still at the starting SHA, so no merge was
needed. The downloaded nextest archive was deleted and its executable moved
inside `target/`; new big scratch output outside `target/` was removed. The
release-input script removes its temporary source copy automatically. No
CodeQL, JS/CSS, rustc-throttle or model-server change was made.

Doctests also ran with the same four-thread cap:

```sh
cargo test --manifest-path rust/Cargo.toml --locked -j 2 --workspace --exclude html5ever --doc -- --test-threads=4
```

All 11 crate doctest runners exit successfully; there are no enabled doctests
and two preexisting ignored examples. The only nonzero raw doctest summary is:

```text
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
