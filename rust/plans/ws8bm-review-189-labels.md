# WS8bm PR #189: visible labels, intended-assertion discrimination and held URL jobs

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`; review base `2921b282`. Pinned Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, with only the approved drift in wave4/_common.md. These changes are tools/docs only: no tracked Rust crate, application asset, golden, normalization mask or product changes. No assertion budget, 700 ms hold, test concurrency or compiler throttle was enlarged. The direct forward identity query uses the original SQLite adapter's five-second busy limit (config/database.yml:10); it does not add a selector wait.

The five credited text-filtered label assertions now use the shared pinned Selenium visible-text predicate on the selected element. An accessible name alone cannot satisfy a Rails `assert_selector ..., text:` filter. Every remaining name/label assertion and action in the executable modules was swept against the pinned original.

| Changed check | Category | Old behaviour | New behaviour | Pinned Rails line |
| --- | --- | --- | --- | --- |
| Profile: Ban Kevin | Visible label text | Visible button could match an opacity-zero child through its accessible name | `filterVisibleText(browser.locator('button'), 'Ban Kevin')`, original 2 s | message_list_a11y_test.rb:564 |
| Attach: From this device | Visible label text | Accessible menu-item name accepted hidden text | Selected menu item must have visible text, 2 s | composer_attach_menu_test.rb:17 |
| Attach: From Google Drive | Visible label text | Accessible menu-item name accepted hidden text | Selected menu item must have visible text, 2 s | composer_attach_menu_test.rb:18 |
| Phone attach: From Google Drive | Visible label text | Same gap before menu geometry | Selected menu item must have visible text, 2 s; geometry bounds unchanged | composer_attach_menu_test.rb:121 |
| Boost: Delete this boost prompt | Visible label text | Accessible button name accepted hidden span | Selected button must have visible text, original 5 s | boosting_messages_test.rb:26 |
| Attachment filename (deferred flow) | Visible label text | Named download link substituted for the visible filename assertion | Message body must contain visible filename text, 2 s; download control remains a supplementary check | workspace_markdown_test.rb:166; system_test_helper.rb:115 |
| Forwarded Markdown context | Assertion scope | Forwarded row was selected by its code's visible text; hiding code caused an earlier table failure | After the original status assertion, query the actual fixture DB for the copied message ID, exactly as Rails does; table and code independently check their own visibility | search_forward_edit_test.rb:57-60 |
| Redelivery opacity target | Attribution | Target registration incorrectly named the later Edit action | Names the explicit original HTML-marker visibility assertion; a later action cannot earn this variant credit. No positive assertion changed | message_interactions_test.rb:166,168 |
| URL edit job execution | Job boundary | Production workers could replace Loading post before its original 15 s assertion | Rails TestAdapter with `perform_enqueued_jobs=nil`; real Rust TestApp without its job runner; original 15 s loading and 2 s edited-marker waits unchanged | test_helper.rb:13; search_forward_edit_test.rb:74-84 |
| All served-negative checks | Attribution | Any assertion/timeout after an applied mutation earned rejection credit | Registered assertion source/stack location, actual served mutation and state witnesses; unrelated failures INVALID, fresh-fixture retry up to three attempts, never credited | Tool discrimination contract; positive Rails assertion scopes unchanged |

Unchanged name/label scopes after the sweep:

| Group | Original scope retained | Pinned Rails lines |
| --- | --- | --- |
| Profile message button | Explicit `[aria-label]` CSS was retained | message_list_a11y_test.rb:557,563 |
| Toolbar labels; picker ARIA controls | Correction: at `21e4a77d`, these used accessible-name lookups and did **not** retain the original literal selectors. The follow-up [checkpoint report](ws8bm-review-189-checkpoints.md) replaces them with literal CSS attributes | message_toolbar_test.rb:16-20,73,78,96,101,149 |
| Copy-code and duplicate Edit button assertions | `assert_button` selects a named visible button; this is a locator filter, not `assert_selector ..., text:` on rendered descendant text | code_highlighting_test.rb:53,107,119,146; message_interactions_test.rb:147 |
| Project notes and attachment download links | Named link / href lookup with the selected link's visibility; no additional visible-text assertion is invented | workspace_markdown_test.rb:336; attachment download is supplementary, while the filename now follows :166 / system_test_helper.rb:115 |
| Named actions/fields, labelled thread fields | Original `click_on`, `fill_in`, `find_field` locator semantics, then the shared visible lookup on the selected element; scripted focus/label observations stay scripted | system_test_helper.rb:92,96-105,129; threads_test.rb:555-556,565-566 |
| Explicit hidden/all queries | DOM-based scope preserved; no visible-text/visible-count filter added | message_list_a11y_test.rb:313; message_toolbar_test.rb:49; existing security `:all` checks |

The pinned Capybara button/link selectors inspect locator text/attributes; their selected-node visibility is independent of the `text:` filter used by the changed text assertions. The existing unmodified Selenium visibility/text helpers remain the oracle. No helper atom, expected bytes or response mask changed.

Discrimination requires an existing, case/variant-specific source anchor in an actual failing stack frame. Method receivers and statement columns are retained, so an earlier call on the same line cannot borrow a later assertion's target. Startup, network, unapplied, escaped and non-assertion failures receive zero credit. CSS/script opacity probes also record the mutated DOM state; release-click additionally requires the broken guard and an actual click into the mounted menu. A geometry miss remains invalid. Global field-opacity variants identify the causal original visible fill action if it fails before the later field-value assertion; they do not claim that later assertion ran. The failed-delete boost variant can be observed at recipient-visible absence even when its sender optimistically removes its copy.

The URL and PR hosts preserve both real applications, routes, CSRF, writes and Cable. An opt-in reference-image initializer supplies Rails' nonperforming test adapter. A tool copies current tracked Rust test inputs and appends the ignored browser host only to that generated copy; it calls the existing `TestApp::without_job_runner()`. Ordinary checks keep their ordinary job path. Each run rebuilds its own seeds/binary/fixtures; no pre-existing scratch data or Cargo target is required. Rust's URL fetch row remains `ready`, attempts zero. This corrects the production-worker harness mismatch; it is not a demonstrated flake in the pinned Ruby original.

**Duplicate-delivery limit:** the existing check injects Turbo markup into the browser and checks its mounted-row guard. It does not verify server-originated redelivery; the pinned Rails original calls `message.broadcast_create`.

## Failing-first evidence

Logs live under `.scratch/ws8bm-review-labels/` in this worktree. They are evidence only; no check reads a prior log/fixture/target as an oracle.

The visible-label probes were served against the unchanged `2921b282` assertions before the fixes. `ban-before.log` contains the exact review CSS; `labels-before.log` covers the four further credited label assertions. Both applications accepted each broken text state:

```text
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 4 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

Commands (run before changing these assertions):

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --case 'profile message and ban buttons have accessible names' --mutant transparent-ban-text --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer_attach_menu boosting_messages --mutant-set labels --keep-going
```

The supplementary attachment filename correction is in an already-deferred flow. It has an additional served opacity probe; an earlier preview failure cannot earn filename rejection credit. No new declaration or paired failing-first credit is claimed for that probe.

For attribution, `attribution-baseline.py` temporarily restored the original runner from `2921b282`, retained the corrected deterministic job boundary, and printed the actual failure. Both apps timed out at Loading, fifteen seconds before any marker assertion. The old runner nevertheless printed:

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

That is **the historical false credit**, not a valid rejection receipt. The same composed delayed-marker/hidden-loading probe with the new runner records the Loading frame, the marker target mismatch and no hidden marker state at failure, and prints:

```text
WS8bm invalid discrimination attempts: 1; bounded fresh-fixture retries only
WS8bm discrimination check: 0 served mutants rejected on Rails and Rust across 0 named checks; 1 invalid or escaped
```

The new-run command was:

```sh
WS8BM_UNRELATED_FAILURE_PROBE=1 WS8BM_DISCRIMINATION_RETRIES=1 python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --mutant delayed-url-edited-marker --negative --keep-going
```

The ordinary full run does not enable that composed probe. Each real variant has its own registered target. Unknown targets fail closed; helper tests check every registration and prove that unrelated failures, unapplied changes, network errors, same-line preceding calls and release-click geometry misses earn no credit.

## Fresh-clone verification

Source clone: `.scratch/ws8bm-label-fresh`, local clone without hardlinks, cold source `2f16bb893`, updated through `7c3f2bbda`, the final forwarded-identity correction `3a5eedcc6`, and the marker-target registration `cc74b2949`. Its Cargo target was absent before the build, npm/browser dependencies were installed there, and the driver rebuilt the Rails parity seed. The broad positive attempt exposed the owned forward lookup defect; its isolated rerun and final forward discrimination retry use the corrected persisted-ID lookup. The other assertion changes are identical throughout the full discrimination run; source/target-registration updates and failed attempts are retained, not represented as one clean, sealed-source run. Environment: `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, `CAMPFIRE_REFERENCE` set to the clone and `CARGO_TARGET_DIR` set to the clone's own `rust/target`. No outside pre-existing target or untracked fixture supplies a test input. No Rust workspace/clippy/release rerun is claimed for this tools/docs-only review.

Commands and raw summaries follow below. Retained failed attempts are distinguished from subsequent isolated reruns; they are not rewritten as a clean full run.

The full discrimination run initially refused credit for the redelivery-root variant: its registered target named the later Edit action. The pinned original explicitly asserts the HTML checkpoint's visibility at message_interactions_test.rb:166. Registration now names that assertion, with a unit regression refusing credit for the later action. No assertion or wait changed.

The successful URL repetition was also run before the fresh-clone repetition. `url-five.log` and `fresh-url-five.log` each contain five consecutive paired passes, independent fixtures and the held-row observation `WS8bm held URL jobs: Rust 1 ready; 0 attempts` for every attempt. The fresh repetition's raw final line appears below; it is not five declarations of credit.

`fresh-build.log`:

```sh
mise exec rust@1.98.1 -- cargo build --locked -j2 --manifest-path rust/Cargo.toml -p campfire --bin campfire
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 58s
```

`fresh-helpers-final.log`:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs rust/reference-tools/messaging/behavior-discrimination.test.mjs
```

```text
ℹ tests 15
ℹ pass 15
ℹ fail 0
ℹ skipped 0
```

`fresh-planner-final.log`:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p "*_test.py"
```

```text
Ran 6 tests in 0.000s
OK
```

`fresh-all-paired.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.67s
WS8bm behaviour check: 101 named cases passed on Rails and Rust; 4 failed; no pixel checks
```

`fresh-thirty.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions message_actions_mobile message_toolbar code_highlighting --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 24s
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-hidden.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.22s
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`fresh-url-five.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --repeat 5 --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
WS8bm behaviour repetition: 5 paired attempts; 1 named declaration; 0 failed
```

`fresh-all-mutants.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 37.35s
WS8bm invalid discrimination attempts: 6; bounded fresh-fixture retries only
WS8bm discrimination check: 153 served mutants rejected on Rails and Rust across 97 named checks; 0 invalid or escaped
```

`fresh-sending-final.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.33s
WS8bm behaviour check: 2 named cases passed on Rails and Rust; 1 failed; no pixel checks
```

`fresh-delete-isolated.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'deleting messages' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-browse-final.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'browses active and closed threads and can join or leave a closed one' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-forward-final.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'forwarded Markdown keeps tables and code blocks' --keep-going
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.24s
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-inventory.log`:

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

```text
WS8bm system inventory: 135 named declarations; 102 mapped behaviour passes; 27 deferred; 6 WS12 blocked; no pixel checks
```

## Retained failures and attribution limits

- The broad positive attempt recorded four failures. The attachment reply preview failed on Rust and remains deferred. Rails timed out at startup before the closed-thread browsing check; its unchanged isolated rerun passed on both apps. The forwarded Markdown check exposed an owned tool response-listener/body-lookup defect; the final direct model-ID lookup matches Rails :57, and the isolated control plus intended code-opacity probe pass on both apps. Rails' picker-arrow panel wait timed out at its unchanged 10 s budget; the same case passed in the separate thirty-check run.
- The final sending-file rerun recorded an additional Rust startup asset `net::ERR_NETWORK_CHANGED` / 15 s Stimulus timeout in deleting messages. Its exact isolated rerun passed on both apps. This infrastructure attempt remains in the report; it is not a product or queue-race fix.
- The full stricter run's six invalid attempts comprise two message-list/navigation startup/network batches, two owned forward-lookup attempts, one redelivery target-registration/startup attempt and one Rails missing-def startup/network attempt. All were retried with fresh fixtures; only the intended assertion failures earned the final 153 receipts. No escaped variant, invalid final variant or widened wait is hidden by those retries.
- The extra filename probe's unchanged-assertion diagnostic was attempted three times (`filename-before-1.log` through `-3.log`), restoring only the original lack of a visible-body filename assertion. Rust accepted it each time. Rails failed at the earlier 10 s attachment preview each time. Each raw summary is `WS8bm review escape check: 0 served mutants accepted on Rails and Rust; 1 failed probes; no parity credit`. This is **not paired failing-first evidence**; it receives no such credit. The final full discrimination run did reach the corrected filename assertion and reject the opacity state on both apps. The broader flow remains deferred.
- PR Discuss now has the previously missing paired control/persisted-row proof in the broad run, plus its intended header-stripping rejection on both apps. This review leaves the frozen branch's historical attribution JSON unchanged; the accepted continuation already credits the PR flow. It does not reclassify other deferred declarations.
- No queue-observation race was fixed. No Rust workspace, clippy or release-input test line is quoted as a new run. The only compilation used current-source normal/test hosts for behaviour verification.

The old review branch's inventory remains the historical **102 / 27 / 6**, including its unmerged WS12 production dependencies. The continuation branch already has **156/156 controllers; 118 / 17 / 0 systems**. These six frozen-branch statuses are not a claim that WS12 is still blocked on main. Merging this review into the continuation must retain its credited cases, hidden-allowed role lookups, test-only motion input and upload job boundary.

The public message list/composer contract is unchanged: `Presenter::messages`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. [ws8bm-integration.md](ws8bm-integration.md) remains the shell's contract for verified context, selected records/divider, room/thread facts, ordered commands, Drive flow, real schedule child and request-owned pending template.
