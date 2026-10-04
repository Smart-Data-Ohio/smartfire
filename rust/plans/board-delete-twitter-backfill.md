# Board DELETE and Twitter operator backfill

Branch: `rust/board-delete-twitter-backfill`, started from `origin/main` at
`78b9b1546bdab4c6c1c9b8ddb94512f661289112`.

`DELETE /rooms/boards/:id(.:format)` dispatches to a board-scoped handler that
reproduces the unchanged Rails reference error. The subclass callback scopes
exclude `destroy`, so inherited `RoomsController#destroy` raises
`NoMethodError: undefined method 'name' for nil` at
`app/controllers/rooms_controller.rb:29` (`room_label = @room.name`). The lead
explicitly chose reference parity, as for `Rooms::DirectsController#show`.
Rust runs the ordinary application authentication and CSRF checks, then returns
the same public 500 response without room lookup, room authorization or writes.
It does not add a working second board-deletion path with different permissions.
Nothing remains flagged for this route.

The regenerated `vectors/board_destroy.json` executes **only the unchanged
Rails `/rooms/boards/:id` route**, through real session and CSRF middleware.
Seven individually executed comparisons cover administrator JSON, creator HTML,
administrator Turbo, forbidden member, inaccessible administrator, missing ID
and wrong room type. All seven Rails responses are 500; the JSON body is 46 bytes
and HTML/Turbo bodies are 4,887 bytes. Rust compares status, complete body bytes,
Content-Type, Location and Cache-Control (including absent headers), and flash.
Each fresh Rails request raises the same nil-room `NoMethodError`. There is no
base-route substitute, body/status mask or Rails source edit in these vectors.

Both apps retain the complete pre-request rows of rooms, memberships, messages,
threads, audits, durable jobs, all four board-specific tables, activity items,
boosts and all three Active Storage tables (15 tables total). Rails also records
zero enqueued jobs and broadcasts; Rust stops its job runner before inspecting
the complete queue and subscribes to the global room stream before deletion,
asserting silence. No deletion, membership change, audit, broadcast or job occurs.
The ordinary authentication redirect and missing-CSRF 422 still precede the
inherited action, and leave the same domain/queue snapshot unchanged.

The UI's existing **base `/rooms/:id` route** continues deleting boards. Only the
unique board-specific durable-cleanup regression is retained: after that base
request, a new real worker drains the committed destroy/purge jobs. It removes
messages, threads, memberships and all four board tables, deletes the attachment
and blob rows and purges the stored file. The full before/after FK results match
(the seed intentionally contains a preexisting message with a deleted author).
The redundant board-scoped success, permission, redirect and broadcast tests
were removed, along with the board handler's successful destroy implementation.

The route audit still covers all 431 defined/implicit table entries (378 distinct
endpoints). No such entry resolves to `not_yet_ported`. Google Calendar
notifications keep the table alias for their existing unparsed-body live action.
The Twitter CLI implementation and its existing tests are unchanged by the
board-route follow-up.

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

## Verification of the lead follow-up

Fresh Rails (unchanged controller source hashes checked by the generator):

```sh
PARITY_OWNER=ws11api-board-cutover PARITY_NAMESPACE=ws11api-board-cutover PARITY_IMAGE=ws11api-reference:d7c7de92 rust/parity/bin/reference runner --seed default rust/reference-tools/rooms/board_destroy.rb
```

```text
Rails board DELETE: 7 unchanged-route HTTP/state/broadcast comparisons; 7 reference 500s; zero domain writes, jobs or broadcasts
```

All seven new comparisons were run first against `2a3124d24348c83647e3e2e55880d5a847ca85bb`'s
successful board handler. Each failed at its intended status assertion: JSON
allowed 200, creator HTML and administrator Turbo 302, forbidden member 403,
inaccessible/missing/wrong-type 302, each instead of Rails' 500.

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 -p campfire --profile ci -E 'test(/controllers::rooms::board_destroy_tests::(admin_json|creator_html|admin_turbo|forbidden_member|inaccessible_admin|missing|wrong_type)$/)'
```

```text
Summary [   1.864s] 7 tests run: 0 passed, 7 failed, 2857 skipped
```

After changing the handler, verification covers all room-controller tests,
both route guards, the Twitter operator and reference-service tests, and strict
workspace clippy. Commands run from the worktree root with Rust 1.98.1 via mise,
`CI=1`, all three Rails-migrated seeds present, two build jobs and four test
workers. One Cargo build runs at a time; the configured rustc throttle is intact.

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 -p campfire --profile ci -E 'test(controllers::rooms::) | test(controllers::tests::every_) | test(admin::twitter_tests::) | test(integrations::twitter::references::tests::) | test(controllers::google_calendar::)'
cargo clippy --manifest-path rust/Cargo.toml --locked -j 2 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
Summary [ 157.645s] 396 tests run: 396 passed, 2468 skipped
Finished `dev` profile [unoptimized] target(s) in 20.01s
```

Strict clippy exits zero with `-D warnings`; all 396 selected tests pass.

The second fresh Rails execution produces a byte-identical vector.

The prior checkpoint (`2a3124d24`) also completed the workspace, release-input
build, doctests and normal production-CLI smoke checks. These are **prior-run**
results, not a claim that the workspace/release build was repeated for the
follow-up. The seven native media-byte failures passed unchanged in the pinned
libvips/ffmpeg runtime; no masks or timing changes were introduced.

```text
Summary [1295.924s] 4960 tests run: 4953 passed (3 slow), 7 failed, 20 skipped
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 2855 filtered out; finished in 2.93s
Finished `dev` profile [unoptimized] target(s) in 1m 57s
Production CLI smoke: 4 invocations passed; 0 failed; repeated default run retains one durable fetch job
```

No CodeQL, JS/CSS, rustc-throttle or model-server changes were made. New scratch
logs are small; no new large output outside `target/` is retained.
