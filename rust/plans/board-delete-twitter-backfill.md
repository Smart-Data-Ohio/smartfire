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
not create/migrate a database, boot HTTP, start job workers, or perform
network fetches. Application credentials are not required. It uses the real Twitter
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

## PR #238 rendered-body backfill review fix

Astra's review at `b17487b1` found that raw stored rich text was an incorrect
prefilter. Rails matches `markdown_source` first, then `message.body.to_s`:
canonical Action Text content, rendered attachments, sanitization and its
layout. Rust now calls the existing full Action Text renderer through the
model's rich-text adapter. A missing renderer fails explicitly instead of
silently returning raw HTML. The offline command installs `AppRichText` with
the existing database attachable resolver; no renderer or model logic is
copied. Its temporary signed markup is neither returned nor persisted. It
uses configured `SECRET_KEY_BASE` when present, otherwise a process-private
ephemeral signing base; signatures and local signed paths contain no plaintext
post URL, so their key does not affect this selection predicate.

Batches still preload at most 1,000 rows and include negative and zero IDs.
Within each batch, rendering and independently committed syncs alternate in
primary-key order, as in Rails, instead of rendering the entire batch before
any write. Earlier progress survives a later failure. The approved atomic
queue rule, fetch claims, job registration and idempotence remain intact.

Extraction intentionally uses a different projection: Rails' `body.body.to_html`
canonicalizes attachment content without sanitizing/rendering it. Rust now
uses that same canonicalizer before the existing non-code text/href walker.
Markdown URL selection short-circuits body rendering, including for a body
that raises if rendered. Code-only bodies can be selected without creating a
reference; URLs in retained attributes can select a message and then extract
its forward note. Stripped attributes/comments must not select it, and entity
encoded visible and href links must select it after rendering decodes them.

`reference-tools/twitter_backfill_rendered.rb` executes unchanged Rails using
input-only `twitter_backfill_rendered_inputs.json`: all 11 reviewer edge forms,
14 additional boundary cases, and the exact single-row reproduction. Four
actual rake invocations cover single and combined datasets twice. The single
probe prints `Backfilled 2 messages\n`, with no new reference/job for its note;
the 25-case dataset prints `Backfilled 15 messages\n`. Full rendered and
canonical body bytes, non-code extraction text, references, post URLs, fetch
claim timestamps and fetch job targets are pinned without masks. The old
1,008-row batch differential and queue-rejection regression still pass.

Five added regressions cover the single probe, combined references/jobs and
idempotence, canonical extraction/Markdown short-circuit, complete rendered
body projections, and the real production operator implementation. Before
changing the producer, three fail against `b17487b1` at their intended assertions:
3 versus 2 messages; missing encoded post 88002; and an unwanted reference from
canonicalized attachment children. The production-command test additionally
checks complete repeated post/reference/job facts without timestamp changes;
its first run uses wall-clock time, while the frozen-clock differential checks
every expected claim timestamp against Rails.

```text
Summary [   0.878s] 3 tests run: 0 passed, 3 failed, 2864 skipped
```

Current verification (Rust 1.98.1 via mise, `CI=1`, all three seeded databases,
two build jobs, four nextest workers, one Cargo build at a time):

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 -p campfire --profile ci -E 'test(controllers::rooms::) | test(controllers::tests::every_) | test(admin::) | test(integrations::twitter::) | test(controllers::google_calendar::)'
cargo clippy --manifest-path rust/Cargo.toml --locked -j 2 --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
Rails Twitter rendered backfill: 25 edge cases + exact single-row review repro; four actual rake invocations; repeated state unchanged
Summary [ 183.279s] 414 tests run: 414 passed, 2455 skipped
Finished `dev` profile [unoptimized] target(s) in 1m 01s
```

Strict clippy exits zero with `-D warnings`. A second fresh generator execution
produces a byte-identical vector. All five new regressions, both route guards,
all room controllers, admin commands and Twitter integration tests pass.

The board-route handler/vector and exhaustive route guard are unchanged by this
fix. No Rails source, JS/CSS, CodeQL, timing threshold, rustc throttle or Python
model-server change was made. No new large scratch output outside `target/`
remains.

## PR #238 saved HTML content attachment follow-up

Merged `origin/main` with a merge commit (`52dbb1e72`), bringing in #236's
reference refresh. The plain Rails image `review236-reference:78b9b1546`
has `GIT_REVISION=78b9b1546bdab4c6c1c9b8ddb94512f661289112`, matching
`parity/reference.sha`. All three gate seeds were validated with that image:
default 29/29, first_run 4/4, agents_ui 40/40. Freshly regenerated board,
1,008-row Twitter backfill, and 25-edge rendered-backfill vectors change only
their reference identity; all response and state output remains identical.
Their producers now record the image's real `PARITY_REFERENCE_SHA`.

Fresh `reference-tools/twitter_backfill_attachment.rb` reproduces Astra's
encoded tweet link in a **normally saved** `ActionText::RichText`, not merely
raw SQL HTML. The precise failure is `ActionView::Template::Error: undefined
method 'include?' for nil`, caused by `NoMethodError` in ActionView's
`AbstractRenderer#merge_prefix_into_object_path` (`abstract_renderer.rb:93`).
`ContentAttachment#to_html` (`content_attachment.rb:25`) calls the nested
`Content#render` as an object. Outside Action Text's controller around_action,
that uses a new ApplicationController renderer with a nil context prefix.
The request's real MessagesController renderer has a prefix and renders the
same saved attachment successfully. Rust exposes that default-renderer mode
explicitly; it does not infer renderer identity from the optional request host.
The rich-text model adapter's rendered-body projection uses this mode.

Rails does **not** transact the whole backfill, wrap a whole batch, or wrap
the entire per-message sync. `PostReferenceBackfill.call` and
`PostReferenceSync.call` contain no enclosing transaction; individual Active
Record model writes use their normal save transactions. At the rendering
abort there are zero open transactions. Earlier messages' synced references,
post claims and fetch jobs remain committed; the failing message has no new
reference/post/job, and the later message has not been visited. Rust's
existing per-message sync transactions preserve the same partial progress
under the approved atomic-queue rule. The regression compares the complete
projected reference/post/claim/job state twice for both the raw review input
and the normally saved content, including absence of forward-note post 99113
and later post 99114. Repeating the abort changes nothing.

Direct `Twitter::PostReferenceSync.call` does **not** render: it reads canonical
`Content#to_html`. It succeeds and creates the forward-note reference/job
99113, then is idempotent. Rust's same sync service matches this behavior;
there is no error added to canonicalization. A Markdown URL also short-circuits
rendering in both backfills and allows the later message to be processed.
The vector includes exact request-rendered bytes, saved canonical bytes,
sync/repeat state, and the successful Markdown short-circuit count/state.

The real `bin/rails twitter:backfill_references` subprocess exits **1**, has
empty stdout (no `Backfilled` summary), and prints its rendering exception,
cause, source excerpt and filtered trace to stderr. Rust returns the same
exit status and full stderr bytes for this particular reference exception.
The displayed Ruby frames identify the emulated Rails failure, not a Rust
backtrace. Ordinary schema/argument/queue errors retain the existing command
contract. The fresh vector captures stderr without a projection or mask;
the operator regression compares it in full. The real-binary checker also
compares exit status, both output streams, exact frozen-clock partial state,
and complete unchanged rows on repeat.

Three regressions failed first against `82f49253`'s implementation retained
in the main merge, before changing either producer. The backfill and operator
returned success; the default renderer returned rendered attachment HTML.

```text
Summary [   0.684s] 3 tests run: 0 passed, 3 failed, 2869 skipped
```

The new reference vector has four actual rake aborts (three in-process and
one real CLI), two direct sync calls, request rendering, and a successful
Markdown short-circuit invocation. A second fresh execution reproduces every
response, error, CLI diagnostic and state byte. No Rails source substitution,
reference-output injection, broad masking or timing adjustment is involved.

Current follow-up verification (Rust 1.98.1 through mise, CI=1, validated gate
seeds, two build jobs, four nextest workers, configured rustc throttle and one
Cargo build at a time). The first suite covers every database/rich-text test
and the affected room/admin/Twitter/route-guard/Calendar app tests. After a final
error-evaluation-order correction (Ruby canonicalizes the nested content before
entering the failing object renderer), the complete rich-text/operator subset
was rerun. Strict clippy and the normal binary build use the final source.
The CLI wrapper executes that binary in the plain pinned runtime with a frozen
wall clock, no network and no server/job runner. Both real binary invocations
match Rails' exit status, empty stdout, full stderr, exact partial rows/claims/jobs
and complete unchanged source rows; the repeat keeps all five inspected tables
byte-identical as rows.

```sh
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 -p campfire -p campfire_db -p campfire_richtext --profile ci -E 'package(campfire_db) | package(campfire_richtext) | test(controllers::rooms::) | test(controllers::tests::every_) | test(admin::) | test(integrations::twitter::) | test(controllers::google_calendar::)'
cargo nextest run --manifest-path rust/Cargo.toml --locked --build-jobs 2 -j 4 -p campfire -p campfire_richtext --profile ci -E 'package(campfire_richtext) | test(admin::twitter_tests::)'
cargo clippy --manifest-path rust/Cargo.toml --locked -j 2 --workspace --exclude html5ever --all-targets -- -D warnings
cargo build --manifest-path rust/Cargo.toml --locked -j 2 -p campfire --bin campfire
PARITY_IMAGE=review236-reference:78b9b1546 rust/reference-tools/check_twitter_attachment_cli.sh
```

```text
Summary [ 418.125s] 1877 tests run: 1877 passed (1 slow), 2459 skipped
Summary [  40.079s] 92 tests run: 92 passed, 2861 skipped
Finished `dev` profile [unoptimized] target(s) in 1m 03s
Finished `dev` profile [unoptimized] target(s) in 1m 23s
Production Twitter attachment CLI: 2 aborts matched Rails exit/status, stdout/stderr bytes, exact partial state and unchanged repeat rows; 0 differences
```

Strict workspace clippy exits zero with `-D warnings`. There are no remaining
flagged board-route or Twitter-backfill differences. No Rails source, JS/CSS,
CodeQL configuration, timing thresholds, rustc throttle, or model server was
changed. The retained new scratch files are only small logs/reference outputs;
private reference and production-CLI databases have been removed. No extra
Cargo target directory was created.
