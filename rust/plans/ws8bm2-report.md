# WS8bm2 report — partial message-features delivery

Status: **PARTIAL**. This branch now ports poll/pin HTTP and broadcasts plus saved-item CRUD/list/reminder HTTP and the verified reminder claim/rearm path, plus scheduled-message HTTP, rows and broadcast delivery. It does not complete the full WS8bm2 brief, browser/pixel parity, or root-message cache integration. Do not treat it as cutover-ready.

Base: `9c8efaef6a4fd11e7b290db77dd524ed2ab0e5f9` (WS8b-m), including main `21a7332f2d3c324f0862cdf448baf17a84395aa0`. Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2`, branch `rust/ws8bm2-message-features`. Reference pin: `d7c7de9264c63015be398001d7a1094e7695a6db`.

Implementation commit: a036f6e6dcb696ba36d4ff64a9a4806bc9c65e3c. Final pushed report commit: read branch HEAD; the report cannot contain its own commit hash. No PR or deployment.

## Changes by file

Paths below are relative to `rust/`.

| File | Change |
| --- | --- |
| `crates/campfire/src/controllers/rooms/polls.rs` | New create/show/vote controllers; room membership and active-human gates; permitted builder fields, scalar/array ballots, HTML/JSON/Turbo formats and validation responses. Message, poll, options and durable bot jobs share a transaction. Uses WS8b-m root-message broadcast/release entry points after success. |
| `crates/campfire/src/controllers/messages/pins.rs` | New reachable-message pin/unpin controllers; idempotency, cap errors, HTML flash/back redirects, JSON counts and Turbo empty/303 responses. |
| `crates/campfire/src/controllers/rooms/pins.rs` | New pins index and plain list adapter; newest first, 200-character excerpts, pinner/author names, thread jump links and actual room STI DOM identity. |
| `crates/campfire/src/controllers/message_features.rs` | Shared authorization/error/boolean/date helpers and poll view-model adapter. Date input support is bounded; see limitations below. |
| `crates/campfire/src/controllers/message_features/tests.rs` | Fifteen seed-required request, byte-oracle, transaction, authorization, timezone and real-WebSocket tests. |
| `crates/campfire/src/channels/message_features.rs` | Ordered commit-thread rendering for poll, pin badge/count/list and the quiet pin note; request-origin RAII guard; one test for successful/error/panicking writer restoration. Uses a minimal badge model without loading the message body. |
| `crates/campfire/src/controllers.rs`, `controllers/rooms.rs`, `controllers/messages.rs` | Six route handlers and module declarations; exposes the existing bot-webhook enqueue helper to this sibling controller. |
| `crates/campfire/src/channels.rs`, `channels/sink.rs` | Small dispatch seam for owned rendered descriptions; preserves the existing fallback for other message/thread broadcasts. |
| `crates/db/src/database.rs` | Generic `write_scoped` wrapper holds a caller-supplied guard through synchronous after-commit callbacks, then drops it before resolving the write. Existing `write` delegates with a unit guard. No rendering logic in the DB. |
| `crates/db/src/slash_commands/time_parser.rs` | Exposes the existing WS8a Rails-compatible local datetime resolver; no change to its algorithm. |
| `crates/views/src/pins.rs`, `crates/views/src/lib.rs` | Plain list/count/badge/index models and Askama wrappers. |
| `crates/views/templates/rooms/pins/{_list,_count,index}.html` | Ports Rails pin list, empty state, count and frame bytes, including final newline behavior. Reuses the existing pin badge template. |
| `crates/views/src/messages/parts.rs`, `templates/polls/_poll.html` | Poll close time uses the view/request timezone rather than UTC. |
| `reference-tools/messaging/features.rb`, `features-reference-check.py`, `features-discriminate.py`, `vectors/messaging/features.json` | Actual pinned Rails HTTP/partial/date oracles, bounded image-source check and reproducible compiled mutation checks. |
| `plans/ws8bm2-report.md` | Tracked copy of this report. |

## Design and parity evidence

Controllers authorize and invoke WS8a domain methods. Templates consume plain models; they perform no DB reads. HTML remains in campfire/views. Pin cap/idempotency/quiet-note effects and poll replacement/retraction validation stay in the existing domain layer. Poll creation rolls back messages, poll/options and durable jobs when a real SQLite trigger rejects job insertion.

Broadcasts render synchronously on the writer's after-commit callback so ordering is preserved. Detached contexts have no viewer/session/CSRF/nonce values. Ordinary pin-list request forms retain request CSRF tokens. Poll forms continue using the existing token-free shared partial. The origin must survive the whole callback chain, which motivated `write_scoped`; a guard created only inside the write closure was already gone when frames rendered.

Golden vectors contain exact JSON strings and exact rendered HTML, without whitespace normalization or masks. The Rails generator uses frozen time and test-only forgery disabling for its HTTP oracle; security checks run separately against the real Rust CSRF middleware. Every request copies the Rails IntegrationSession headers because JSON GET can mutate the passed hash with a method override. Generator status assertions prevent recording accidental 404s as goldens.

The reference image `ws8bm2-reference:d7c7de92` was tagged from the installed `ws19b-ci-reference:latest`. The source checker verifies exactly the 15 controller/model/helper/template files this slice consumes against `git show d7c7de92:<file>`, and rejects injected digest/file-set drift. This is bounded source verification, not a proof that every image file equals the pin. Fresh default and first_run seeds were built from this image before the reported application tests. All 16 new tests require the seed with `expect`; none silently skip.

New York fold/gap, Hawaii, UTC date/time-only/blank, Lord Howe half-hour gap and Apia missing-day probes come from `Time.zone.parse` in our Rails container. Lord Howe exposed the difference between Rails' hourly gap advancement and Jiff's generic compatible disambiguation; the reused WS8a resolver matches these probes.

## Tests shown failing

The initial membership/reachability tests failed against the unimplemented routes before code was added:

```text
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 332 filtered out; finished in 0.41s
```

An expanded DST probe then failed before using the existing WS8a resolver (Lord Howe expected 16:15 UTC; generic gap handling returned 15:45 UTC):

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.01s
```

The final mutation run below separately proves compiled failures for removed room/message reachability, bypassed CSRF, anonymous voter disclosure, HTML byte drift, wrong broadcast origin, wrong DST gap step, cap drift, JSON key-order drift, missing quiet pin note, bypassed durable job enqueue, wrong Turbo action, wrong STI DOM identity, wrong request zone, wrong pin ordering and missing origin restoration. Each injected source is restored in `finally`. This includes temporary changes to WS8a models/jobs solely for the probe; they are not committed.

## Verification commands and raw summaries

Commands below were re-run for this delivery. Cargo commands run from `rust/`, with:

```sh
export TMPDIR="$PWD/../.scratch/tmp"
export CARGO_TARGET_DIR="$PWD/target"
export CI=1
export CABLE_TEST_PORT_RANGE=52500-52549
export MAIL_TEST_PORT_RANGE=52550-52599
```

From worktree root:

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/features-source-final.log 2>&1
```

```text
WS8bm2 reference source check: 15 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

From `rust/`:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/seed-final.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

From worktree root:

```sh
mkdir -p .scratch/oracle-delivery
cp -a rust/parity/.seed/default/. .scratch/oracle-delivery/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/oracle-delivery" rust/reference-tools/messaging/features.rb /rails/storage/db/features.json > .scratch/oracle-delivery.log 2>&1
cmp .scratch/oracle-delivery/db/features.json rust/vectors/messaging/features.json
```

```text
WS8bm2 Rails oracle: 8 poll reads/ballots; 6 poll creates; 4 pin writes; 11 partials; 10 zone/date probes
```

`cmp` exited 0 with no output: the regenerated vector equals the committed vector. Use a fresh private directory when repeating; the oracle writes to its private seed.

From worktree root:

```sh
python3 rust/reference-tools/messaging/features-discriminate.py > .scratch/discrimination-final.log 2>&1
```

```text
membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.84s
pin-reachability: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.48s
csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.41s
anonymous-voters: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.09s
pin-list-bytes: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.10s
broadcast-origin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.61s
dst-gap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.00s
pin-cap: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 2.22s
poll-json-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.47s
pin-note: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.44s
atomic-job: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.44s
turbo-vote: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.43s
sti-pin-list: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.48s
request-zone: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.51s
pin-order: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.42s
origin-restoration: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 347 filtered out; finished in 0.09s
WS8bm2 discrimination: 16 compiled regressions detected; sources restored
```

From `rust/`:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire message_features -- --nocapture > ../.scratch/features-final-verified.log 2>&1
```

```text
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 332 filtered out; finished in 2.41s
```

From `rust/`:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -- --test-threads=4 > ../.scratch/campfire-final.log 2>&1
```

```text
test result: ok. 345 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 33.50s
```

From `rust/`:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_db -p campfire_views -- --test-threads=4 > ../.scratch/db-views-final.log 2>&1
```

```text
test result: ok. 399 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 49.15s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

In order: DB unit tests, view unit tests, view core integration tests, DB doctests, view doctests. The two doctest targets contain no tests.

From `rust/`:

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/clippy-final.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.32s
```


The full application suite has three inherited `#[ignore]` tests: `channels::tests::golden::record_reference` (reference recorder), `controllers::presenters::accounts::tests::manages_bots` (WS11), and `jobs::tests::push_latency` (measurement). The DB suite also has three inherited ignores: `tests::differential_test::scenario_matches_ruby` (requires Rails scenario DB), `tests::fixtures_test::export_database_for_rails` (explicit DB export), and `tests::fixtures_test::fixtures_match_ruby_row_for_row` (requires Rails fixtures DB). This delivery did not run those separate Rails/model differential workflows. New tests: 16 ran, zero ignored, zero seed skips. Filtered tests in the focused run are not claimed as executed there. This is the application + DB/views test coverage listed above, not a full workspace test run, Docker Rust build, browser matrix or scheduler end-to-end run. Clippy covers the workspace's non-vendored targets.

## Rails test mapping and ownership

Covered controller behaviors are ported as grouped Rust tests and pinned HTTP/partial oracles; the Rails test suites themselves were not run here.

- `test/controllers/rooms/polls_controller_test.rb`: fourteen non-room-page cases covered: creates a poll; multiple anonymous close time; too few options; board rejection; replacement/broadcast; retraction; closed rejection; foreign options; non-member create/vote; bot create; viewer ballot; anonymous viewer ballot; outside-room show; bot show. Its final two room-page cases (anonymous cards carry no voter IDs; regular cards carry IDs) have exact partial assertions here, but full room-page/cache integration remains **deferred to WS8b-m**, with WS8bm2 responsible for poll endpoint/partial behavior.
- `test/controllers/messages/pins_controller_test.rb`: all six behaviors covered: creation plus four frames/note, idempotency, cap, unpin plus three frames, absent unpin and non-member rejection.
- `test/controllers/rooms/pins_controller_test.rb`: all three behaviors covered: newest-first jump/unpin list, empty list and non-member rejection.
- `test/system/polls_test.rb`: all four deferred to **WS8bm2**, coordinating composer/shell with WS8b-r and root cache with WS8b-m: builder/create/change/retract, anonymous reload markers, second-session live results, closed form absence.
- `test/system/pins_saved_test.rb`: all seven deferred to **WS8bm2**: menu/panel/live badge, same-tab note jump, compact note/no menu, panel unpin, save/remind/status/remove, tomorrow-9am fall-back and custom reminder. First four also require WS8b-r panel/shell wiring. Socket/partial tests here cover part of those behaviors but are not browser ports.

Every test in these files is deferred to **WS8bm2** (none implemented by this slice):

- **Now covered in the continuation below:** `test/controllers/saved_items_controller_test.rb` (12 cases; 12 named Rust request tests pass, plus four additional checks): index lists saved messages with status filters; index hides items whose room access was lost; create saves with a reminder; create without a reminder leaves remind_at blank; create again updates the reminder instead of duplicating; create again after a fired reminder re-arms it; create rejects past and unparseable reminders; create is 404 for a message the user cannot see; update marks done and reopens; update rejects an invalid status; update and destroy are 404 for hidden or foreign items; destroy removes the item.
- **Now covered in continuation slice 2: 19/19 individually named Rust request ports passing.** `test/controllers/scheduled_messages_controller_test.rb` (19 tests): index lists upcoming and past rows; index hides other people's rows; index shows stranded rows so they can be cancelled; creates a scheduled message; create rejects past times; create is 404 outside membership; updates text and time; update during an active claim is refused; update during an active claim redirects with a notice in HTML; update after the claim goes stale is allowed; update is 404 for sent rows and other people's rows; destroy cancels pending rows only; a send that lands between the lookup and the lock refuses the cancel; a send that lands between the lookup and the lock refuses the edit; destroy during an active claim is refused; send_now posts immediately; send_now drops rows without access; send_now drops rows the model rejects with the reason; bots are forbidden.
- `test/controllers/searches_controller_test.rb` (36 tests): index initial view; finding reachable messages; unreachable messages are not found; operator words are searched literally instead of raising; a leading operator returns 200; a boolean-looking query does not exclude terms; a quote character returns 200 with sensible results; create does not run the search; clear does not run the search; clear answers Turbo with streams that empty the header and page recents in place; clear leaves recents alone when it can't answer the requested format; the header renders at most ten recents even when older rows exceed the trim; clear without Turbo returns to the page it came from; index renders the page for a Turbo Stream request without an older-results cursor; the header search field renders on every signed-in page, empty outside search; the search page without a query lists recents instead of a watermark; a query with no results shows an empty state; results page through Load older results; an older window renders as a page without JavaScript; create saves the search term; create with no searchable words redirects back with a notice and records nothing; clear search history; from: narrows results to that author; in: narrows results to that room; in: a room the user is not in returns nothing; sections exclude soft-deleted rooms; has:pin narrows results to pinned messages; has:link narrows results to messages carrying a link; on: narrows results to that day; is:thread narrows results to thread messages; filter-only queries list without text and show chips; chips link back without their operator; boards, work threads, and events render as sections scoped to access; search sections cost the same queries regardless of section size; search sections label direct rooms neutrally; operator values cannot inject SQL or FTS syntax.
- `test/controllers/rooms/slash_commands_controller_test.rb` (10 tests): shrug posts through the dispatcher; unknown commands answer an error without posting; status answers ephemeral confirmation; event answers an open_url; bare event opens the blank form; poll answers open_poll; agent commands invoke through the room; thread commands dispatch with the thread; non-members get 404; bots are forbidden.
- `test/controllers/autocompletable/icons_controller_test.rb` (6 tests): requires authentication like the users endpoint; returns mixed brand and emoji matches with their payloads; orders prefix matches first and limits the results; returns no matches for blank or unknown queries; returns workspace icons with their stable image URL; lists every workspace icon for the picker Custom tab.
- `test/controllers/autocompletable/slash_commands_controller_test.rb` (7 tests): lists built-ins with metadata; exposes takes_arguments for immediate and argument commands; includes the room's agent commands; agent commands registered without arguments run immediately; thread conversations hide root-only commands; filters by query; non-members get 404.
- `test/controllers/autocompletable/users_controller_test.rb` (5 tests): search returns matching users; search results escape HTML in names; room search returns matching users; room search omits the Markdown token for duplicate display names; room search is scoped by membership.
- `test/controllers/rooms/message_links_controller_test.rb` (12 tests): a member of the source room sees the quote card; a quote of a soft-deleted source room shows only the private chip; a non-member of the source room sees only the private chip; a non-member of the quoting room gets nothing; a reference from another room gets nothing; a same-room quote renders inline in the room; a cross-room quote renders a lazy frame in the room; editing the source enqueues a job that refreshes quoting cards over the stream; deleting the source clears quoting cards and busts their cache; two viewers of a cached direct-room quote see the same neutral label; quote cards cost the same queries regardless of card count; editing a message to add a permalink replaces its own card container.
- `test/controllers/rooms/files_controller_test.rb` (8 tests): lists uploads newest first with jump links; lists Drive attachments as generic picker-only rows; type filters narrow uploads; filename search matches substrings and escapes wildcards; uploads page cumulatively; only the room's own files are listed; non-members get nothing; rendering costs the same queries for 4 files as for 16.
- `test/system/slash_commands_test.rb` (26 tests): typing slash opens the command picker with combobox semantics; picking poll by Enter runs it immediately; picking poll by click runs it immediately; picking event by Enter opens the form immediately; picking huddle by Enter runs it immediately; suggestion rows hint argument placeholders only; the close button dismisses the picker without sending; the close button does not overlap the first row's text; Escape closes the picker without sending; mentions and emoji pickers have no close button and still commit; the picker lists registered agent commands; agent commands that take arguments insert and wait; agent commands without arguments run immediately when picked; shrug posts through the picker; arguments close the slash picker and Enter posts; Enter submits while the picker's deactivating update is still pending; a submit queued during the live command check still runs the command; unknown slash words post as normal messages; double slash escapes a known command; a command registered after page load still runs; me renders as an action line; poll opens the poll builder; event navigates to the prefilled form; huddle reports when unconfigured; agent commands respond ephemerally until the agent replies; status sets the custom status.
- `test/system/search_files_test.rb` (4 tests): filter chips show parsed operators and remove them; a pasted permalink renders a quote card with a working jump link; a cross-room permalink loads its quote frame for members and outsiders; the Files tab lists uploads and Drive rows with working filters.
- `test/system/scheduled_messages_test.rb` (4 tests): schedules from the composer and lists in the Scheduled view; schedule send requires a draft; edits, sends now, and cancels from the Scheduled view; the sidebar links the Scheduled view.

The WS8bm2 owner retains `/play` presentation parity and any matching play tests; room agent-command integration depends on WS11. Upload/message-root work remains with WS8b-m; room Files listing is WS8bm2. No test is handed back as completed merely because the underlying WS8a model exists.

## Remaining work / integration questions

1. **Unimplemented WS8bm2 scope:** search controllers/operator chips/preloads/tuple cursor/date coercion/timezone parity; slash/autocomplete icons/commands/users and `/play`; room message-links/quote visibility; Files upload/Drive listing/filtering/pagination.
2. **WS8b-m cache integration:** `Presenter::message_item` reads `cached_message_fragment(id, message.updated_at, origin)`; poll updates touch the poll, not the parent message. Wire the already available Rails composite message key (including poll stamp) through root-message cache reads/writes. Current standalone live poll card works; root-message reload/cache behavior is not proven and can be stale. No root message partial was edited here.
3. **Other callback origins:** own vote/pin/unpin writes use the scoped origin. WS8b-m message deletion and other owners' deletion/unpin operations must adopt the scope where their request-origin pin-list frames require it. Jobs correctly use detached `http://example.org`. General message/thread/quote broadcast rendering remains WS8b-m's adapter, not this slice's fallback.
4. **WS8b-r shell:** pins count/list/frame are available, but `rooms/pins/_panel`, header button/dialog and end-user opening flow are not wired by this slice. Confirm ownership with the room shell worker.
5. **Date/coercion partial:** the controller date parser handles the tested ISO/date/datetime-local/time-only inputs, not the entire Ruby `Date._parse` grammar. Natural-language input, civil overflow and malformed ISO exception behavior remain. Currently an unrecognized/malformed string can become no close time instead of Rails' exception. Broad Unicode option stripping/coercion inherited from WS8a also needs differential coverage; Rust `trim` is not Ruby ASCII `strip`.
6. **WS11 auth boundary:** real session bot denial, bot-key denial and unknown Bearer credential rejection are tested. Valid agent-token authorization and its endpoint 403 parity are not proven until WS11's authentication seam exists.
7. **Verification gaps:** no browser/pixel matrix or allowlist changes; no constant-query-count proof for poll/pin adapters (some name/body reads are per row); no actual periodic-close runtime end-to-end socket test; no full workspace tests or Rust Docker image build. Model/scheduler tests that ran in the application suite do not replace those missing checks.

Restart at the search slice, retaining the poll/pin and saved-item commits and coordinating the cache, panel and auth seams above. No schema, dependencies/lockfile, Rails source, parity masks or allowlists were changed.


## Continuation slice 1: saved items and reminders

Saved-item controllers, forms and list are implemented. All 12 Rails controller cases have individually named request ports (12/12 passing); four extra tests cover exact HTTP/partial bytes, CSRF/Turbo redirects, and the periodic reminder path. Browser/system/pixel cases remain deferred; push delivery awaits WS17's missing `SavedItem::ReminderPushJob` handler and notification policy. The existing menu save dialog is already in the shared message-actions template; it was not modified here.

Files added: `crates/campfire/src/controllers/saved_items.rs`, `controllers/message_features/saved_tests.rs`, `crates/views/src/saved_items.rs`, `templates/saved_items/{index,_item}.html`, `reference-tools/messaging/saved.rb`, `vectors/messaging/saved.json`. Module/route seams in `controllers.rs`, `views/src/lib.rs`, and `message_features.rs`. Shared form helper `button_to_form_params` accepts pre-flattened ordered hidden parameters and preserves existing callers through delegation; this is the small WS6 seam for Rails' status button forms. The existing periodic reminder entry point was exposed as `pub(crate)` solely to exercise the actual runtime function from request tests (WS3 seam); its behavior was not changed.

WS8a still owns SavedItem validations (associated user/message, per-user uniqueness, status inclusion, changed-reminder future time), the `clear_fired_claim` callback, and dependent activity destruction. No domain algorithm was changed. The HTTP tests prove idempotency, rearm, invalid/past time rejection, status changes, reachable messages, hidden/foreign mutation denial, no-store/no-cache headers and deletion. A SQLite trigger refuses the reminder push-job insert after an HTTP-created reminder becomes due; the real periodic entry leaves both claim and inbox insert rolled back. Removing the trigger allows one firing; rearming refreshes the same inbox source. This is not proof that the pending WS17 push handler delivers.

Twelve controller tests failed before routes were added:

```text
test result: FAILED. 0 passed; 12 failed; 0 ignored; 0 measured; 348 filtered out; finished in 0.72s
```

Additional checks rejected four deliberate compiled regressions:

```sh
python3 rust/reference-tools/messaging/features-discriminate.py saved > .scratch/saved-discriminate.log 2>&1
```

```text
saved-json: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 363 filtered out; finished in 0.52s
saved-partials: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 363 filtered out; finished in 0.10s
saved-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 363 filtered out; finished in 0.38s
saved-reminder-job: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 363 filtered out; finished in 0.45s
WS8bm2 discrimination: 4 compiled regressions detected; sources restored
```

The Rails checker was extended to seven consumed saved-item controller/model/dispatcher/pusher/job/template files:

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/saved-source.log 2>&1
```

```text
WS8bm2 reference source check: 22 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Actual pinned Rails oracle (private seed, no normalization):

```sh
mkdir -p .scratch/saved-oracle2
cp -a rust/parity/.seed/default/. .scratch/saved-oracle2/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/saved-oracle2" rust/reference-tools/messaging/saved.rb /rails/storage/db/saved.json > .scratch/saved-oracle2.log 2>&1
```

```text
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
```

Cargo from `rust/` with the same private target/TMPDIR/CI/ports environment documented above:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire saved_tests -- --test-threads=4 > ../.scratch/saved-green.log 2>&1
```

```text
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 348 filtered out; finished in 0.90s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -- --test-threads=4 > ../.scratch/saved-app.log 2>&1
```

```text
test result: ok. 361 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 27.85s
```

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/saved-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.95s
```

No new ignores or seed skips. The three application ignores remain those listed above. Date parsing still has the bounded grammar caveat; browser tomorrow-9am/custom-reminder flows and WS17 push runtime remain partial.

## Continuation slice 2: scheduled messages

All 19 cases in `test/controllers/scheduled_messages_controller_test.rb` have individually named request ports (19/19 passing), with six additional tests for exact JSON/time zones, partials/empty page, composer controls, CSRF/busy parameter precedence, transactional job failure, and real WebSocket delivery. Total: 25 new tests, zero ignores/skips. The four scheduled system cases remain deferred to WS8bm2 with the WS8b-r composer/sidebar shell seam.

Added `controllers/scheduled_messages.rs`, `controllers/message_features/scheduled_tests.rs`, `views/src/scheduled_messages.rs`, `templates/scheduled_messages/{index,_item,_past_item,_composer_button}.html`, `reference-tools/messaging/scheduled.rb`, and `vectors/messaging/scheduled.json`. Route/module declarations are the small seams in `controllers.rs`, `message_features.rs`, and `views/src/lib.rs`. `channels/message_features.rs` now consumes only the owned scheduled-send message description as well as quiet pin notes. General root/thread broadcasts remain WS8b-m's responsibility. `db/src/database.rs::queued_writes` is a small queue-depth observation seam used to synchronize real writer races, not a mocked lock.

Update and cancellation re-read the row under the actual SQLite writer transaction after the initial ownership/pending lookup; a concurrent send refuses either with 409. Busy checks precede invalid/missing update parameters. `send_now` delegates to WS8a dispatch with the request origin kept through commit callbacks: no second unread or append chain. A trigger rejecting durable job insertion proves claim/post/history rollback. One retry emits one token-free message over a real socket. List partitions pending/sendable, pending/stranded and past rows, retaining cancellation for lost-access drafts. No-store/Pragma and bot/member/ownership protections run through the request pipeline.

Rails JSON retains the user's zone offset for scheduled timestamps (unlike the saved endpoint's explicit UTC timestamps). UTC, Hawaii and New York spring-gap HTTP bytes match. Eight UTC/Hawaii row-state partials, an empty page, and two room/thread composer controls match byte-for-byte. The control is available as `scheduled_messages::ComposerButton`; it is **not mounted** into the inherited stock Lexxy room composer, which is WS8b-r's shell. This slice did not replace that other worker's composer. Browser scheduling, sidebar reachability, populated whole-page byte/pixel parity and full Ruby date grammar remain partial.

Before adding routes:

```text
test result: FAILED. 0 passed; 17 failed; 0 ignored; 0 measured; 364 filtered out; finished in 0.89s
```

Eight deliberate compiled mutations fail the named tests and restore sources. The final golden control first failed on the missing final newline before it was fixed (24 passing, one failing).

```sh
python3 rust/reference-tools/messaging/features-discriminate.py scheduled > .scratch/scheduled-discriminate.log 2>&1
```

```text
scheduled-race-edit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.43s
scheduled-race-cancel: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.41s
scheduled-zone: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.43s
scheduled-partials: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.07s
scheduled-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.35s
scheduled-job: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.42s
scheduled-socket: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 5.42s
scheduled-composer: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 388 filtered out; finished in 0.06s
WS8bm2 discrimination: 8 compiled regressions detected; sources restored
```

From worktree root (fresh private seed):

```sh
mkdir -p .scratch/scheduled-oracle2
cp -a rust/parity/.seed/default/. .scratch/scheduled-oracle2/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/scheduled-oracle2" rust/reference-tools/messaging/scheduled.rb /rails/storage/db/scheduled.json > .scratch/scheduled-oracle2.log 2>&1
```

```text
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
```

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/scheduled-source.log 2>&1
```

```text
WS8bm2 reference source check: 29 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

From `rust/`, with the private target/TMPDIR/CI/port environment above:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire scheduled_tests -- --test-threads=4 > ../.scratch/scheduled-green.log 2>&1
```

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 364 filtered out; finished in 1.13s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -- --test-threads=4 > ../.scratch/scheduled-app.log 2>&1
```

```text
test result: ok. 386 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 27.44s
```

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/scheduled-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.10s
```

The application suite retains the same three inherited ignores listed above. Clippy passes with warnings denied. No schema, dependencies, lockfile, Rails source, masks or allowlists changed. Saved push delivery still awaits WS17. Search, slash/autocomplete/play, message links/files, root cache/panel/auth/date integration, and the remaining file-grouped Rails cases follow in the requested order.
