# WS8bm2 report — partial message-features delivery

**PARTIAL.** Polls/pins, saved-item HTTP plus reminder claim/rearm, scheduled-message HTTP/rows/sends, and search HTTP/chips/sections/tuple paging/date operators are implemented. Populated provider cards, composer/panel mounting, remaining feature controllers, broad coercion/auth gaps and browser/pixel parity remain. This branch is not cutover-ready.

Worktree: `/home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm2`, branch `rust/ws8bm2-message-features`. Base `9c8efaef6a4fd11e7b290db77dd524ed2ab0e5f9`, including main `21a7332f2d3c324f0862cdf448baf17a84395aa0`. Rails pin `d7c7de9264c63015be398001d7a1094e7695a6db`. Pushed slices: polls/pins `a036f6e6dcb696ba36d4ff64a9a4806bc9c65e3c`; saved/reminders `90bb304a3092fad869469ed7f8f729980c4abdc1`; scheduled `f39afa8bd28c368bf4c9f80c317ea037b2047539`; search is the branch HEAD accompanying this report (the report cannot contain its own SHA). No PR or deployment.

## Changes by file

Paths are relative to `rust/`; abbreviated controller/channel/job paths are under `crates/campfire/src/`, and template paths are under `crates/views/`.

| File | Change |
| --- | --- |
| `crates/campfire/src/controllers/rooms/polls.rs` | Poll create/show/vote, human/member gates, ballots/formats/errors, transactional poll/message/job creation. |
| `controllers/messages/pins.rs`, `controllers/rooms/pins.rs` | Reachable pin/unpin/list, cap/idempotency/quiet note, newest-first lists and STI identity. |
| `controllers/message_features.rs` | Shared authorization/date/error/poll adapters; numeric JSON message IDs supported for saved writes. Date grammar remains bounded. |
| `controllers/message_features/{tests,saved_tests,scheduled_tests}.rs` | Seed-required owned feature ports, byte oracles, transactions, real writer races and real sockets. |
| `controllers/saved_items.rs` | Saved CRUD/status/reminder/list, visibility, HTTP JSON/cache headers and redirects. |
| `controllers/scheduled_messages.rs` | Scheduled CRUD/send-now/list; ownership/pending scope; re-read under writer transaction; busy-before-parameter precedence; zone-offset JSON. |
| `controllers/searches.rs` | Parsed operators; create/clear avoid message lookup; unsupported clear formats refuse before deletion; initial HTML and older HTML/Turbo; capped history; display query preserved; search room icons on fragment misses. |
| `controllers/searches/ports.rs` | 43 seed-required tests: 36 named Rails behavior ports and seven additional parser/byte/HTTP/security/cursor/icon checks. |
| `controllers/presenters/view_context.rs` | Small WS6/WS8b-m chrome seam: ten ordered recents on signed-in pages; query prefilled only on searches. |
| `controllers.rs`, `controllers/rooms.rs`, `controllers/messages.rs` | Route/module declarations and existing bot-webhook helper exposed to sibling features. |
| `channels/message_features.rs`, `channels.rs`, `channels/sink.rs` | Owned poll/pin/quiet-note/scheduled-message commit-thread rendering, origin RAII and small sink seam. General root/thread/quote rendering remains WS8b-m. |
| `crates/db/src/database.rs` | `write_scoped` holds origin through commit callbacks; `queued_writes` observation synchronizes real HTTP lock races. WS2 seams. |
| `crates/db/src/models/search_query.rs`, `models/search_query/word_ranges.rs` | ID window plus probe, at most 40 instantiated rows; Rails-local date resolution including missing-day `on:`; capped access-scoped side sections with joined room labels; pinned Ruby Unicode word classifier. Pure domain reads. |
| `crates/db/src/models/search.rs` | SQL-capped ten-recents query, independent of legacy untrimmed history. WS8a read seam; record/trim algorithms unchanged. |
| `crates/db/src/slash_commands/time_parser.rs` | Existing WS8a local-time resolver exposed; algorithm unchanged. |
| `crates/campfire/src/integrations.rs`, removed `integrations/search.rs` and its `word_ranges.rs` | Removes unused stock non-word replacement sanitizer. Its verified classifier data now belongs to the active domain parser. |
| `crates/campfire/src/jobs/periodic.rs` | Existing reminder entry exposed as `pub(crate)` for actual runtime checks; behavior unchanged (WS3 seam). |
| `crates/campfire/src/jobs/tests.rs` | Small WS3 test seam: quote-refresh completion checks that class rather than requiring unrelated periodic jobs to disappear. Failed/pending quote jobs still fail. |
| `crates/views/src/{pins,saved_items,scheduled_messages,searches}.rs`, `views/src/lib.rs` | Plain view models and Askama partial/page wrappers, including room/thread schedule control and search recents/chips/sections/older streams. |
| `views/src/helpers/forms.rs` | Ordered hidden form-parameter helper; old callers delegate without parameters (WS6 seam). |
| `views/src/messages/parts.rs`, `templates/polls/_poll.html` | Request-zone poll closing time. No root message partial was edited. |
| `templates/rooms/pins/{index,_count,_list}.html`, `templates/saved_items/{index,_item}.html` | Owned list/count/frame/status/reminder bytes. |
| `templates/scheduled_messages/{index,_item,_past_item,_composer_button}.html` | Pending/stranded/sent/dropped rows, page and reusable schedule control. |
| `templates/searches/{index,_filters,_sections,_load_older,_page_recents}.html`, `{index,clear}.turbo_stream.html` | Rails result page, removable chips, access-scoped sections, pagination and clear-history streams. Existing header dropdown partial reused. |
| `reference-tools/messaging/{features,saved,scheduled,search}.rb`, `{features-reference-check,features-discriminate}.py`, `vectors/messaging/{features,saved,scheduled,search}.json` | Actual pinned Rails oracles, consumed-source checks and compiled regressions; exact bytes without masks/normalization. |
| `plans/ws8bm2-report.md` | Tracked copy of the external worker report. |

## Design and parity evidence

Domain methods retain validations, dependent destruction, atomic callback/index/unread/job effects and dispatch policies. Controllers authorize, select permitted parameters, call domain operations and render plain models. Rendering stays outside the DB models. Detached broadcasts contain no viewer/session/CSRF/nonce data; ordinary request forms retain their tokens. The committing writer keeps origin through the callback chain and restores it after success, error or panic.

Saved-item idempotency, status, reachability and reminder rearm are request-tested. The real periodic reminder function is invoked after advancing a frozen clock. A SQLite trigger rejecting its durable job insert rolls back claim and inbox rows; dropping the trigger fires once, and rearm refreshes the same inbox activity. Actual push delivery still awaits WS17's `SavedItem::ReminderPushJob` handler/policy.

Scheduled update/cancel re-read under the writer lock. Both send-between-lookup-and-lock races use actual HTTP requests and SQLite, synchronized by the queued writer observation. Busy checks precede malformed/missing update parameters. Dispatch delegates to WS8a and produces one append/unread chain; job-insert rejection rolls back claim, message and history. One real WebSocket receives one token-free append with the request origin. Scheduled JSON retains the user's offset, matching UTC, Hawaii and New York gap oracles. Eight row-state partials, two composer controls and an empty page match exact bytes. `ComposerButton` is available but not mounted into WS8b-r's inherited stock Lexxy shell.

Search preserves human-readable operators for history/display/redirects; quotes every remaining Ruby word token for FTS; binds escaped operator values. The newest-first `(created_at,id)` ID query is bounded to 41, then only 40 rows load and display oldest-first. A reachable cursor is required for nonblank queries; blank queries do not resolve it. Older windows omit side sections. Three fixed, capped section reads join room labels; their plain views render without a DB gateway, including neutral direct-room labels. Header history is capped in SQL even for untrimmed old rows. Shared message fragment hits remain unchanged; search supplies room icons only on misses, matching the shared Rails cache behavior.

Date selections cover UTC, Hawaii, New York 23/25-hour days, São Paulo midnight gap and Apia's missing day. `on:` derives the day range after local conversion, so Apia's 2011-12-30 resolves to the valid following day as Rails does. Generic Jiff disambiguation alone does not match Rails' hourly gap advancement. A newly accepted Unicode character exposed Regex/Ruby word-class drift; all 771 word ranges were regenerated/verified against the pinned Ruby runtime and the active parser now uses them.

Search now batches the facts consumed by this branch's shared presenter: bodies (including reply sources), creators/boosters/mentioned users and avatar presence, rooms/direct names/icons, boosts, uploaded blobs, pins, thread counts, Drive IDs, steps, polls/options/voter names and the Markdown icon catalog. Domain DTO reads are in `models/message_rendering.rs`; `searches/preloads.rs` adapts them through small flagged presenter/storage/richtext seams. Rendering four and sixteen mixed Markdown/mention/boost/poll messages has constant SQLite SELECT authorization events; the old path failed with `(55, 199)`. Six complete populated message fragments match pinned Rails and the lazy presenter exactly, using committed SQL-row fixtures, with no local-state reads. Compiled lazy-query and missing-pin mutations fail. **Full search parity remains partial:** populated GitHub/Fizzy/X/LinkedIn/event/embed/quote providers are absent from the inherited root composition (WS8b-m/WS14/WS15), and browser/pixel coverage remains unproven.

The image `ws8bm2-reference:d7c7de92` was tagged from the installed `ws19b-ci-reference:latest`; 41 consumed app source files match the pin, with injected digest/file-set differences rejected. This is bounded source verification, not a whole-image source claim. Oracles use private copies of the fresh default seed, real Rails renderers/IntegrationSession, frozen time and test-only forgery disabling. Mutations/security tests exercise the real Rust middleware. JSON and HTML are compared directly, with no normalization/masks.

## Tests shown failing and runtime fix

Before the owned implementations, poll/pin gates failed 0/2, saved routes failed 0/12, and scheduled routes failed 0/17. The initial stock-search request run and the Unicode parser probe failed as follows:

```text
test result: FAILED. 7 passed; 25 failed; 0 ignored; 0 measured; 389 filtered out; finished in 1.34s
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.08s
```

Byte tests also failed before correcting schedule-control final newline, search time-target, load-older parameter order and older-stream final newline. The first tuple mutation showed an insufficient stream assertion: it checked included rows but did not count/reject the cursor row. The test now asserts the exact five-row window and rejects `<=`.

One full application run exposed a quote-runtime assertion race: the quote job had completed while `Retention::PruneJob` was still running. The test previously waited for quote completion then incorrectly asserted the whole queue empty. The narrow assertion now rejects any pending/running/failed quote-refresh row while allowing independent periodic jobs. Production job behavior did not change. The observed failed run was:

```text
test result: FAILED. 424 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 40.62s
```

The named real-runner check and complete application rerun pass below. No failure was ignored or classified as inherited to bypass it. Four obsolete stock-search tests (two controller, two sanitizer) were replaced by the named Rails query behavior/Unicode oracles; no new ignores/skips were added.

## Verification commands and raw summaries

These commands were executed for the current continuation. Cargo runs from `rust/`, with:

```sh
export TMPDIR="$PWD/../.scratch/tmp"
export CARGO_TARGET_DIR="$PWD/target"
export CI=1
export CABLE_TEST_PORT_RANGE=52500-52549
export MAIL_TEST_PORT_RANGE=52550-52599
```

From `rust/`:

```sh
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 parity/bin/seed build default first_run > ../.scratch/continuation-seed.log 2>&1
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
```

From worktree root:

```sh
PARITY_IMAGE=ws8bm2-reference:d7c7de92 python3 rust/reference-tools/messaging/features-reference-check.py > .scratch/search-source.log 2>&1
```

```text
WS8bm2 reference source check: 41 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

Pinned oracles (each uses a fresh private seed directory):

```sh
mkdir -p .scratch/saved-oracle-final
cp -a rust/parity/.seed/default/. .scratch/saved-oracle-final/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/saved-oracle-final" rust/reference-tools/messaging/saved.rb /rails/storage/db/saved.json > .scratch/saved-oracle-final.log 2>&1
```

```text
WS8bm2 saved Rails oracle: 11 HTTP responses; 6 item partials; 1 empty page
```

```sh
mkdir -p .scratch/scheduled-oracle-final
cp -a rust/parity/.seed/default/. .scratch/scheduled-oracle-final/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/scheduled-oracle-final" rust/reference-tools/messaging/scheduled.rb /rails/storage/db/scheduled.json > .scratch/scheduled-oracle-final.log 2>&1
```

```text
WS8bm2 scheduled Rails oracle: 12 HTTP responses; 8 row partials; 1 empty page; 2 composer controls
```

```sh
mkdir -p .scratch/search-oracle6
cp -a rust/parity/.seed/default/. .scratch/search-oracle6/
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 rust/parity/bin/reference runner --storage "$PWD/.scratch/search-oracle6" rust/reference-tools/messaging/search.rb /rails/storage/db/search.json > .scratch/search-oracle6.log 2>&1
```

```text
WS8bm2 search Rails oracle: 15 parsed queries; 15 chip partials; 1 empty page; 1 clear stream; 18 zone/date selections; 1 populated sections partial; 1 load-older control; 1 empty older stream; 4 HTTP responses; 771 Unicode word ranges
```

```sh
cmp .scratch/saved-oracle-final/db/saved.json rust/vectors/messaging/saved.json
cmp .scratch/scheduled-oracle-final/db/scheduled.json rust/vectors/messaging/scheduled.json
cmp .scratch/search-oracle6/db/search.json rust/vectors/messaging/search.json
```

All three `cmp` commands exited 0 with no output. The re-generated search vector is byte-identical, including all 771 Unicode ranges. The saved/scheduled vectors were generated from the private runs above; earlier poll/pin oracle evidence remains in the corresponding committed vector/tool.

Current search compiled mutations (membership, CSRF, inclusive cursor, wrong zone, missing-day range, chip bytes, history cap, unquoted FTS and time-target bytes):

```sh
python3 rust/reference-tools/messaging/features-discriminate.py search > .scratch/search-discriminate.log 2>&1
```

```text
search-membership: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.49s
search-csrf: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.38s
search-tuple: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.53s
search-zone: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.50s
search-missing-day: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.62s
search-chips: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.09s
search-header-limit: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.45s
search-fts: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.43s
search-section-time: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.05s
WS8bm2 discrimination: 9 compiled regressions detected; sources restored
```

The prior slices also rejected 16 poll/pin, four saved and eight scheduled compiled mutations; their reproducible anchors remain in this script. These historical groups are not claimed as re-run by the current search-filtered command.

From `rust/` with the environment above:

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire saved_tests -- --test-threads=4 > ../.scratch/saved-green.log 2>&1
```

```text
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 412 filtered out; finished in 0.89s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire scheduled_tests -- --test-threads=4 > ../.scratch/scheduled-green.log 2>&1
```

```text
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 403 filtered out; finished in 1.15s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire searches::ports -- --test-threads=4 > ../.scratch/search-green.log 2>&1
```

```text
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 385 filtered out; finished in 2.48s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire jobs::tests::ws8_quote_refresh_jobs_execute_in_the_real_app_runner -- --exact > ../.scratch/search-quote-runtime.log 2>&1
```

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 427 filtered out; finished in 0.18s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire -- --test-threads=4 > ../.scratch/search-app.log 2>&1
```

```text
test result: ok. 425 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 33.31s
```

```sh
mise exec rust@1.98.1 -- cargo test --locked -j 4 -p campfire_db -p campfire_views -- --test-threads=4 > ../.scratch/search-db-views.log 2>&1
```

```text
test result: ok. 400 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 48.92s
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```sh
mise exec rust@1.98.1 -- cargo clippy --locked -j 4 --workspace --exclude html5ever --all-targets -- -D warnings > ../.scratch/search-clippy.log 2>&1
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.20s
```

DB/views lines are DB unit tests, view unit tests, view core tests, and two empty doctest targets, respectively. App's three existing ignores: reference cable recorder, WS11 accounts bot management, push-latency measurement. DB's three existing ignores: scenario differential, explicit export and Rails fixture comparison workflows. All owned new tests require the seed; none silently skip. 100 owned application tests and the new DB Unicode-data test ran through these suites. This is not a full workspace test run, Rust Docker build or browser matrix; clippy covers the non-vendored workspace/all targets.

## Rails test mapping and ownership

Covered controller behaviors are ported as grouped Rust tests and pinned HTTP/partial oracles; the Rails test suites themselves were not run here.

- `test/controllers/rooms/polls_controller_test.rb`: fourteen non-room-page cases covered: creates a poll; multiple anonymous close time; too few options; board rejection; replacement/broadcast; retraction; closed rejection; foreign options; non-member create/vote; bot create; viewer ballot; anonymous viewer ballot; outside-room show; bot show. Its final two room-page cases (anonymous cards carry no voter IDs; regular cards carry IDs) have exact partial assertions here, but full room-page/cache integration remains **deferred to WS8b-m**, with WS8bm2 responsible for poll endpoint/partial behavior.
- `test/controllers/messages/pins_controller_test.rb`: all six behaviors covered: creation plus four frames/note, idempotency, cap, unpin plus three frames, absent unpin and non-member rejection.
- `test/controllers/rooms/pins_controller_test.rb`: all three behaviors covered: newest-first jump/unpin list, empty list and non-member rejection.
- `test/system/polls_test.rb`: all four deferred to **WS8bm2**, coordinating composer/shell with WS8b-r and root cache with WS8b-m: builder/create/change/retract, anonymous reload markers, second-session live results, closed form absence.
- `test/system/pins_saved_test.rb`: all seven deferred to **WS8bm2**: menu/panel/live badge, same-tab note jump, compact note/no menu, panel unpin, save/remind/status/remove, tomorrow-9am fall-back and custom reminder. First four also require WS8b-r panel/shell wiring. Socket/partial tests here cover part of those behaviors but are not browser ports.

The following file mapping distinguishes completed request ports from deferred controller/system work. Deferred cases remain with **WS8bm2**, with the integration owners identified below:

- **Covered: 12/12 named Rails behavior ports pass.** `test/controllers/saved_items_controller_test.rb` (12 cases; 12 named Rust request tests pass, plus four additional checks): index lists saved messages with status filters; index hides items whose room access was lost; create saves with a reminder; create without a reminder leaves remind_at blank; create again updates the reminder instead of duplicating; create again after a fired reminder re-arms it; create rejects past and unparseable reminders; create is 404 for a message the user cannot see; update marks done and reopens; update rejects an invalid status; update and destroy are 404 for hidden or foreign items; destroy removes the item.
- **Covered: 19/19 individually named Rust request ports pass.** `test/controllers/scheduled_messages_controller_test.rb` (19 tests): index lists upcoming and past rows; index hides other people's rows; index shows stranded rows so they can be cancelled; creates a scheduled message; create rejects past times; create is 404 outside membership; updates text and time; update during an active claim is refused; update during an active claim redirects with a notice in HTML; update after the claim goes stale is allowed; update is 404 for sent rows and other people's rows; destroy cancels pending rows only; a send that lands between the lookup and the lock refuses the cancel; a send that lands between the lookup and the lock refuses the edit; destroy during an active claim is refused; send_now posts immediately; send_now drops rows without access; send_now drops rows the model rejects with the reason; bots are forbidden.
- **36/36 named Rust behavior ports pass; complete message preloads and populated whole-page/browser parity remain partial.** `test/controllers/searches_controller_test.rb` (36 tests): index initial view; finding reachable messages; unreachable messages are not found; operator words are searched literally instead of raising; a leading operator returns 200; a boolean-looking query does not exclude terms; a quote character returns 200 with sensible results; create does not run the search; clear does not run the search; clear answers Turbo with streams that empty the header and page recents in place; clear leaves recents alone when it can't answer the requested format; the header renders at most ten recents even when older rows exceed the trim; clear without Turbo returns to the page it came from; index renders the page for a Turbo Stream request without an older-results cursor; the header search field renders on every signed-in page, empty outside search; the search page without a query lists recents instead of a watermark; a query with no results shows an empty state; results page through Load older results; an older window renders as a page without JavaScript; create saves the search term; create with no searchable words redirects back with a notice and records nothing; clear search history; from: narrows results to that author; in: narrows results to that room; in: a room the user is not in returns nothing; sections exclude soft-deleted rooms; has:pin narrows results to pinned messages; has:link narrows results to messages carrying a link; on: narrows results to that day; is:thread narrows results to thread messages; filter-only queries list without text and show chips; chips link back without their operator; boards, work threads, and events render as sections scoped to access; search sections cost the same queries regardless of section size; search sections label direct rooms neutrally; operator values cannot inject SQL or FTS syntax.
- **0/10 ported; all deferred.** `test/controllers/rooms/slash_commands_controller_test.rb` (10 tests): shrug posts through the dispatcher; unknown commands answer an error without posting; status answers ephemeral confirmation; event answers an open_url; bare event opens the blank form; poll answers open_poll; agent commands invoke through the room; thread commands dispatch with the thread; non-members get 404; bots are forbidden.
- **0/6 ported; all deferred.** `test/controllers/autocompletable/icons_controller_test.rb` (6 tests): requires authentication like the users endpoint; returns mixed brand and emoji matches with their payloads; orders prefix matches first and limits the results; returns no matches for blank or unknown queries; returns workspace icons with their stable image URL; lists every workspace icon for the picker Custom tab.
- **0/7 ported; all deferred.** `test/controllers/autocompletable/slash_commands_controller_test.rb` (7 tests): lists built-ins with metadata; exposes takes_arguments for immediate and argument commands; includes the room's agent commands; agent commands registered without arguments run immediately; thread conversations hide root-only commands; filters by query; non-members get 404.
- **0/5 ported; all deferred.** `test/controllers/autocompletable/users_controller_test.rb` (5 tests): search returns matching users; search results escape HTML in names; room search returns matching users; room search omits the Markdown token for duplicate display names; room search is scoped by membership.
- **0/12 ported; all deferred.** `test/controllers/rooms/message_links_controller_test.rb` (12 tests): a member of the source room sees the quote card; a quote of a soft-deleted source room shows only the private chip; a non-member of the source room sees only the private chip; a non-member of the quoting room gets nothing; a reference from another room gets nothing; a same-room quote renders inline in the room; a cross-room quote renders a lazy frame in the room; editing the source enqueues a job that refreshes quoting cards over the stream; deleting the source clears quoting cards and busts their cache; two viewers of a cached direct-room quote see the same neutral label; quote cards cost the same queries regardless of card count; editing a message to add a permalink replaces its own card container.
- **0/8 ported; all deferred.** `test/controllers/rooms/files_controller_test.rb` (8 tests): lists uploads newest first with jump links; lists Drive attachments as generic picker-only rows; type filters narrow uploads; filename search matches substrings and escapes wildcards; uploads page cumulatively; only the room's own files are listed; non-members get nothing; rendering costs the same queries for 4 files as for 16.
- **0/26 browser ports; all deferred.** `test/system/slash_commands_test.rb` (26 tests): typing slash opens the command picker with combobox semantics; picking poll by Enter runs it immediately; picking poll by click runs it immediately; picking event by Enter opens the form immediately; picking huddle by Enter runs it immediately; suggestion rows hint argument placeholders only; the close button dismisses the picker without sending; the close button does not overlap the first row's text; Escape closes the picker without sending; mentions and emoji pickers have no close button and still commit; the picker lists registered agent commands; agent commands that take arguments insert and wait; agent commands without arguments run immediately when picked; shrug posts through the picker; arguments close the slash picker and Enter posts; Enter submits while the picker's deactivating update is still pending; a submit queued during the live command check still runs the command; unknown slash words post as normal messages; double slash escapes a known command; a command registered after page load still runs; me renders as an action line; poll opens the poll builder; event navigates to the prefilled form; huddle reports when unconfigured; agent commands respond ephemerally until the agent replies; status sets the custom status.
- **0/4 browser ports; all deferred.** `test/system/search_files_test.rb` (4 tests): filter chips show parsed operators and remove them; a pasted permalink renders a quote card with a working jump link; a cross-room permalink loads its quote frame for members and outsiders; the Files tab lists uploads and Drive rows with working filters.
- **0/4 browser ports; all deferred.** `test/system/scheduled_messages_test.rb` (4 tests): schedules from the composer and lists in the Scheduled view; schedule send requires a draft; edits, sends now, and cancels from the Scheduled view; the sidebar links the Scheduled view.

The WS8bm2 owner retains `/play` presentation parity and any matching play tests; room agent-command integration depends on WS11. Upload/message-root work remains with WS8b-m; room Files listing is WS8bm2. No test is handed back as completed merely because the underlying WS8a model exists.


## Remaining work in the requested order

1. **Finish search:** batch the inherited message rendering/mention/resolver details through a coordinated WS8b-m presenter seam; prove constant total queries and populated page/older-window bytes. First-page section preloads and zero-query plain section rendering are implemented, but not the complete message preload path.
2. **Slash/autocomplete/play:** room slash HTTP (10), icons (6), command metadata (7), room/global user autocomplete (5), `/play` presentation and 26 slash browser cases. Registered agent commands/auth depend on WS11.
3. **Message links/files:** quote visibility, lazy frames, edits/deletes/cache refresh and query-count cases (12); upload/Drive rows, filters/wildcards/cumulative pagination and constant-query rendering (8). Root upload/delete work remains WS8b-m; Drive domain facts come from WS14.
4. **Cache/panel/auth/date integration:** root cache still reads `cached_message_fragment(id,message.updated_at,origin)` rather than the available Rails composite key; poll-only updates can leave cached root cards stale. Own origin guards cover vote/pin/unpin/scheduled sends, while other owners' deletion/unpin writes need the scope. WS8b-r must mount pin panel/header/dialog and schedule composer/sidebar flows. Valid agent-token endpoint 403 parity awaits WS11. Full Ruby `Date._parse`, natural-language/civil overflow/malformed-ISO behavior and Ruby option stripping/coercion remain partial. Reminder push delivery awaits WS17.
5. **Deferred Rails/system verification:** file-grouped cases/counts are listed above; browser/pixel matrix, populated cards, constant query proof for poll/pin lists, periodic-close runtime socket test, full workspace tests and Rust Docker image build remain. All deferred feature cases remain WS8bm2's responsibility, coordinating the named integration owners.

Restart at search message preloads, retaining the pushed saved/scheduled/search slices. No schema, dependencies, lockfile, Rails source, parity masks or allowlists changed. No root message partial changed. External report is synchronized with this tracked copy.

## Preload continuation slice

New files: `crates/db/src/models/message_rendering.rs`, `controllers/searches/preloads.rs`, `reference-tools/messaging/preloads.rb`, `vectors/messaging/preloads.json`. The last fixture holds six complete Rails message fragments and nine row tables. Added two search checks, now 45. Source verification now covers 44 files. `searches/preloads.rs` owns the adapter; WS8b-m retains the root partial and ordinary rendering path. `DbResolver::mention_user` and `rich_text::icons` are exposed without changing their algorithms; Boost/Poll row decoders are crate-visible; Storage provides one bounded, ordered attachment/blob read. No schema, dependencies, masks or allowlists changed.

This slice's command results (final clean-clone verification will supersede these):

```text
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 385 filtered out; finished in 1.96s
preload-lazy: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 429 filtered out; finished in 0.32s
preload-pin: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 429 filtered out; finished in 0.15s
WS8bm2 discrimination: 2 compiled regressions detected; sources restored
WS8bm2 preload Rails oracle: 6 complete message fragments; 9 committed fixture tables
WS8bm2 reference source check: 44 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.03s
```

## Slash/autocomplete continuation slice

`controllers/rooms/slash_commands.rs` authorizes the live room, active human and optional same-room thread, dispatches WS8a inside the real writer transaction, and uses a scoped commit-thread renderer for one slash message append. Registered agent execution remains WS11's seam; metadata includes real room registrations. `controllers/autocompletable/{icons,slash_commands}.rs`, `db/{autocomplete_users,command_suggestions}.rs` and the users controller provide the Markdown picker payloads, canonical icon/alias rank, SQL page caps, and name uniqueness over the complete active room/global scope. Autocomplete avatar versions use the request zone. Huddle readiness reads the five settings and separated host/port policy; WS13 owns launching it. Root message partial is unchanged. Existing `/play` rendering matches all 56 Rails sounds plus five unknown/blank/case inputs.

36 owned tests cover 9/10 named slash controller cases (agent execution deferred), 6/6 icons, 7/7 slash metadata and 5/5 users, plus nine HTTP byte, CSRF/thread, atomic rollback, uniqueness, presentation and actual socket checks. One readiness test compares twelve Rails cases; the existing origin guard test now checks slash scope restoration on success/error/panic. Browser slash picker interaction remains deferred. The inherited WS6 autocomplete smoke test expected obsolete Lexxy HTML; pinned Rails proves HTML is 406, so its assertion was updated without skipping it.

Failing-first: the initial real request run rejected all thirteen new security/route checks (0 passed; 13 failed). Exact JSON comparison subsequently exposed the difference between explicit Rails render-json and Jbuilder HTML escaping, then avatar timestamp zones; both corrected. Six compiled mutations fail for CSRF, duplicate-name tokens, icon rank, root-only thread metadata, durable job rollback, and socket delivery, then restore the sources.

```text
test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 430 filtered out; finished in 1.65s
WS8bm2 discrimination: 6 compiled regressions detected; sources restored
WS8bm2 slash Rails oracle: 20 dispatch responses; 17 picker responses; 61 play presentation fragments; 3 format responses; 12 huddle readiness cases
WS8bm2 reference source check: 57 controller, model, helper and template files match d7c7de92
WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.95s
```

The final fresh-clone verification will replace historical command summaries above. Status remains partial; next are message links/files, integration gaps, reminder push and browser/deferred Rails cases.
