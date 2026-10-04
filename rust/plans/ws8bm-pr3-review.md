# #230 ledger follow-up

Release-click is deferred again, alongside scroll preservation and reopen focus. Current http-3 system counts: **127 passed / 8 deferred / 0 owner-blocked**. These are ledger corrections only; no browser or Rust suite was rerun. [Verifier regression, exact reasons and raw output](ws8bm-review230-ledger.md). Older receipts below are historical, and disputed repetitions supply no closure credit.

# Review #230 correction

Current totals are **127 passed / 8 deferred / 0 owner-blocked systems**; controllers remain 156/156. The former release-click/scroll/reopen closures are withdrawn. The attachment flow now closes after main/#231. Workspace uses its specific persisted row and original 2 s budget; native motion uses HQ and native negatives. See [the correction and raw receipts](ws8bm-review230.md). The original checkpoint below is historical and its 129/6 headline and motion/workspace claims are superseded.

# Historical WS8bm -3 checkpoint

Branch `rust/ws8bm-messages-http-3`, created from merged #221/main `b6444b7a7e3e24974a68c8383f4798f5a1356f3d`. Rails stays pinned to `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift. This is a partial continuation: eleven additional declarations are covered, giving **156/156 controllers; 129 passed / 6 deferred / 0 owner-blocked systems**. Six concrete own follow-ups remain below. Nothing here changes Rust product code, Rails client code, response goldens, masks, ignores, retry limits or machine throttling. Deadlines are retained or restored to their pinned values, never widened. The list/composer seam remains [ws8bm-integration.md](ws8bm-integration.md).

The eight motion cases retain the original geometry, focus and transition predicates. Their real browser actions exercise the served controllers; the drawer-animation positive uses the literal pinned Capybara/Selenium body and helpers (alongside the existing native phone control), while its served negatives retain the matching translated assertion; Rails creates the original 15 Scroll rooms / far room / 12 Sticky User fixtures, then identical database copies boot both apps. These originals do not create messages through their UI actions, so the supplementary row checks require unchanged message/thread rows and unchanged user/room identities. They deliberately permit ordinary presence/touch timestamps to change. The workspace case creates the actual Markdown message and actual mobile draft, checks coarse Enter did not persist a message, then requires the exact two saved rows and global message delta. The final POST must finish before closing the browser; optimistic HTML alone is insufficient. The enhanced picker uses the **pinned** `DriveShareMocks` SDK/scenario and real attach-menu/dialog controllers, not a replacement app response. Its original only opens the dialog; it must leave message/blob rows unchanged and make no upload/message POST.

## Original assertion and action scopes

Shared visibility uses the vendored Selenium 4.35.0 atom and pinned visible-text predicate, including element/ancestor opacity. Default queries/actions use Capybara's two seconds; explicit original waits remain explicit. Geometry and computed-style reads are raw only where the original uses `evaluate_script`. All `visible: :all` queries retain DOM presence, including hidden nodes. Screenshot lines are omitted under the behaviour-only scope.

| New named declaration | Pinned original | Scope / deadline retained | Served fault and intended assertion |
| --- | --- | --- | --- |
| Mobile drawer animation and focus return | motion_test.rb:26-70, helpers:330-369,392-398 | Literal pinned native opener/Escape and original helpers; visible open/closed state; original 30 s token override, transitionrun listeners, initial/mid-travel/live transform, Web Animations finish, 5 s polling at 50 ms | Surface transition-duration 0; `motion: drawer starts off-canvas` |
| Member selection leaves geometry unchanged | motion_test.rb:73-100 | Visible Show members text, optional 5 s; visible minimum-two members 10 s; visible-only checkbox collections; Control-click Jason; exact avatar-left/content-height arrays at all three phases, visible Message (2) | Selected avatar translated 10 px; `motion: member-select positions` |
| Directory selection leaves rows unchanged | motion_test.rb:103-116 | Visible minimum-two rows and first/all checkboxes, visible bar, negative bar wait; exact before/selected/cleared top arrays | Selected rows translated 10 px; `motion: directory-select positions` |
| Directory bar remains sticky when scrolling | motion_test.rb:119-146 | Original 12 rows, 1400×400 viewport, genuine overflow, exact scrollTop=100 with original 5 s polling, bar-bottom ≤ scrollport-bottom+1 | Bar position static; `motion: sticky bar inside scrollport` |
| Room menu full-scale edge clamp | motion_test.rb:149-174 | Visible first room; original synthetic contextmenu coordinates through real controller; visible menu and 5 s transform-none polling; original edge bound; Escape hidden-all DOM query | Producer measures scaled `getBoundingClientRect().width` instead of `offsetWidth`; `motion: menu clamped inside viewport` |
| Drawer preserves scroll on close/reopen | motion_test.rb:177-238 | Original 15 rooms, focus/blur, real ≥400 px overflow, exact 400 px scroll, current room outside viewport, exact closed/reopened offsets and fully-in-view focus; original 5 s polling | Closed scroller display:none; `motion: closed drawer keeps offset` |
| First drawer open reveals far current room | motion_test.rb:241-257 | Original far-room fixture; hidden-all current-link DOM lookup; initial zero/out-of-view, actual opener, focused current link with default deadline and 5 s full reveal | First-open focus directed to another link; `firstOpenCurrentFocus` call only |
| Drawer reopens on already visible current room | motion_test.rb:260-292 | Original 15 rooms; first focused current link, original 5 s scroll-settling predicate, exact reopen offset and current-link focus | Reopen focuses first link; `reopenedCurrentFocus` call only, never initial open |
| Workspace theme/mobile navigation | workspace_markdown_test.rb:242-290,295-323 | Visible heading/fields/buttons/text; compact form geometry; hidden-all file input and absence of close button; online JZ 5 s; actual theme/viewport/Tab/link/Escape/coarse-Enter/click; visible delivery 10 s | Profile bar margin-left 20 px; distinct `workspace: profile inside navigation` assertion frame |
| Enhanced Google Drive share flow | composer_attach_menu_test.rb:46-63; drive_share_mocks.rb | Original picker config and SDK scenario; visible attach/menu actions and Q3 Planning dialog filename, default 2 s; app requests untouched | Dialog filename opacity 0; its visible filename assertion |
| Release click does not activate just-opened menu | message_interactions_test.rb:55-86; system_test_helper.rb:200-210 | Original 700 ms touch press, instantaneous hit at the message centre, actual compatibility click, visible menu and hidden-all composer context; saved message/thread/reaction rows unchanged | Disabled release guard; specific post-release menu assertion with releaseClick/brokenGuard/menuVisible observed |

`SystemTestHelper#join_room` waits for **all** currently mounted cable streams, at least three (`system_test_helper.rb:71-84`, 15 s; selectors explicitly `visible: false`). The shared browser viewer follows that original gate. The layout's two room-list subscriptions live outside the lazy sidebar, alongside the room-message subscription; their connection does not promise that the sidebar surface is already present. A translated listener check can still fail before its affected animation state; it is not credited as a rejection. No extra diagnostic focus/geometry wait supplies rejection credit. The test-only motion input is set once per new document, then the motion-on cases can remove it exactly as Rails does. Supplying this input **does not** credit the separate server-emission declaration.

## Readback dispatch regression

Adding a declaration exposed three Markdown persisted-row branches that selected their case by array position; this slice initially shifted those positions. The same audit found three older thread branches already unreachable after main's earlier insertion of continuation cases. All six now dispatch by the exact original case name. The regression walks the real driver's conditional chain and requires each original case to reach its own saved-source/thread-identity query. It fails for all six before the fix and passes afterwards; full paired controls exercise the actual queries. This strengthens existing checks rather than dropping readbacks to make the new case pass.

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p 'behavior_case_readbacks_test.py'
```

Before (`7d975b4df`; `readbacks-before.log`, expected failure) and after (`a9e080608`; `readbacks-after.log`):

```text
Ran 1 test in 0.023s
FAILED (failures=6)
Ran 1 test in 0.008s
OK
```

The new attribution helper tests also reject synthetic failures in the initial-open focus, later viewport predicate, baseline geometry snapshots and startup phases. Only the specific affected assertion may earn rejection credit. Shared teardown from merged #221 remains unconditional, including SQL/browser diagnostic failures.

## Async-script deadline and job boundary

The workspace helper at `workspace_markdown_test.rb:352-359` calls `evaluate_async_script`. Pinned Capybara 3.40.0's Selenium driver sets that script timeout to `Capybara.default_max_wait_time`, which is two seconds. The initial port used an unlimited `page.evaluate` promise. A served four-second message animation escapes on both apps at `510f4712d`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case "workspace follows the system theme and mobile navigation remains reachable" --mutant slow-workspace-animation --keep-going
```

```text
WS8bm review escape: workspace_markdown: workspace follows the system theme and mobile navigation remains reachable: slow-workspace-animation: Rails ACCEPTED
WS8bm review escape: workspace_markdown: workspace follows the system theme and mobile navigation remains reachable: slow-workspace-animation: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`b8cc7b584` restores the two-second bound with a monotonic deadline and preserves original script exceptions. It neither extends a wait nor supplies a diagnostic readiness condition. Two helper tests cover the missing callback and primary exception paths. The named fault may receive rejection credit only at the original `await settled()` call, not an earlier layout, presence or network failure.

| Changed check | Old behaviour | Pinned behaviour restored | Original line |
| --- | --- | --- | --- |
| Workspace transition helper | Finite animation promises could wait indefinitely | Fail at the Selenium async-script two-second budget; cancellation still resolves like the original | workspace_markdown_test.rb:250,256,269,352-359; Capybara 3.40.0 `selenium/driver.rb#evaluate_async_script` |

The workspace scenario now holds jobs on both hosts: pinned Rails uses `ActiveJob::TestAdapter` with `perform_enqueued_jobs=nil`, and Rust uses `TestApp::without_job_runner`. Its Markdown contains a URL; running real link jobs could replace the asserted element and cancel the animation. The existing URL/reply/PR held-job scenarios retain their same boundary. No browser write is mocked, and the mobile POST and exact saved rows remain mandatory.

## Native sequence and attachment identity

The translated animation control can reach `track_transition_runs` while the lazy sidebar surface is missing (`attached=1`), before the mutant's off-canvas assertion. Those failures remain recorded, including the final positive failure on Rust and final invalid negative on Rails. The original helper raises at exactly this setup point (`motion_test.rb:330-348`). The positive now executes the **literal pinned** body at :26-70 and every original helper through pinned Capybara 3.40.0 / Selenium 4.35.0. Extraction tests reconstruct every byte of the body and helpers. No readiness wait is added, and no assertion or deadline is removed. Three consecutive fresh pairs pass, eight native assertions per app/pair, followed by exact message/thread and user/room identity checks. The existing native phone control also passes after sharing the runner metadata; its body/helpers and sequence are unchanged.

The attachment port had omitted `Message.find_by!` and the `within_message` identity lookup (:146-149). It now reads the persisted parent immediately, without SQL polling, and uses its `client_message_id`, matching `Message#to_key`, to scope the visible menu action. This is not a response drain or extra readiness wait. Three fresh controls pass afterwards, but the filename variant still fails at the earlier reply-preview check. It remains INVALID and the declaration stays deferred; the report does not claim that this identity fix resolves all attachment timing.

| Changed check | Old behaviour | Pinned behaviour restored | Original line |
| --- | --- | --- | --- |
| Animation control | Faster translated commands sometimes attach one listener before the surface arrives | Execute every original Capybara/Selenium command and helper, including the same immediate two-listener guard | motion_test.rb:26-70,330-369,392-398 |
| Attachment parent scope | Select a text-matched row before proving the parent exists in storage | Immediate saved-parent lookup and DOM key from client_message_id, then original visible action deadline | workspace_markdown_test.rb:146-149; message.rb:402-404; system_test_helper.rb:118-120 |

The new four-second animation witness initially tried to query the `@keyframes` rule as a DOM selector. It correctly refused credit even though the two-second timeout occurred. It now observes the actual message and requires the named `ws8bm-slow-settle` animation, duration 4000 ms, in `running` state at the failure. Missing, unrelated, finished or wrong-duration states and an earlier profile assertion fail the helper tests. The live probe is valid on both apps at `await settled()`; no discrimination guard is removed.

## Exact six deferrals

These are partial own follow-ups, **not owner-blocked**. No invalid discrimination attempt is credited as a rejection. Deferrals are unfinished own scope, not claims that headless execution is impossible. This checkpoint closes eleven of the seventeen declarations and leaves the exact six below.

| Declaration | Pinned original | Exact remaining requirement / observed reason |
| --- | --- | --- |
| Markdown replies and file attachments remain usable | workspace_markdown_test.rb:143-173 | The immediate persisted-parent lookup and its `client_message_id` DOM scope are restored (:146-149; Message#to_key at message.rb:402-404). Three fresh paired controls then pass, including attachment rows and blob bytes. The filename variant still fails early at the unchanged 10 s reply-preview assertion on both apps, so that proof is INVALID. Keep this flow deferred pending the actual response/broadcast/context-sequence trace; no longer wait or successful declaration credit. |
| Legacy Google Drive picker | composer_attach_menu_test.rb:34-44 | Needs this harness's pinned server-side WebMock-equivalent Drive list transport and connected-account fixture; then actual picker route and visible Q3 Planning at the original deadline. Enhanced SDK mocking is a different boundary and cannot stand in for it. |
| Attach two Drive files, textless send and edit removal | drive_attachments_test.rb:10-76 | Implement pinned external list/file transport, two real selections/chip geometry and removal, textless persisted file-ID set, actual edit-frame replacement and final empty attachment rows. No owner dependency. |
| Edit root message and remove one of two Drive files | drive_attachments_test.rb:79-113 | Same unported external server transport; real composer edit/removal, remaining visible link and exact saved FILE_ID, plus original hidden-context DOM query. No owner dependency. |
| Attach Drive file through thread composer | drive_attachments_test.rb:116-140,151-164 | Same transport plus real joined-thread fixture, original 10 s settled drawer/picker/link predicates and saved thread-message attachment ID. No owner dependency. |
| Motion off by default in test environment | motion_test.rb:19-23; layouts/application.html.erb:2 | Both live hosts boot the production layout contract, which omits Rails.env.test?'s attribute. Needs a real test-environment host/layout contract proving server-emitted `data-test-motion=off` and all three 0ms tokens. Injecting the attribute for other cases is not this proof. |

Duplicate-delivery coverage remains browser-injected Turbo markup; it does not establish server-originated redelivery. The previously documented fast-Escape-before-menu-focus behaviour is shared by the pinned Rails/Rust client and remains a possible Rails client bug. There is no client modification here.

## Verification receipts

Verification uses the independent no-hardlinks fresh clone under `.scratch/ws8bm-pr3/fresh`, initially without a target or seeds; all source inputs, three seed families and browser dependencies are recreated from tracked source. Compiler jobs remain two and tests at most eight. Each discrimination invocation sets `WS8BM_DISCRIMINATION_RETRIES=1`; invalid attempts are retained and never turn into rejection credit. Full runs use **no case exclusions**. Raw logs live under `.scratch/ws8bm-pr3/`.

Private canonical media libraries/binaries are extracted from `triage-reference-d7c7de92`; no host libraries or model server are changed. `CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER=.scratch/ws8bm-pr3/canonical-runner.sh` supplies private libvips/FFmpeg to test processes only.

```text
WS8bm canonical libvips: 8.16.1
ffmpeg version 7.1.5-0+deb13u1 Copyright (c) 2000-2026 the FFmpeg developers
```

Rust tests, strict clippy and release-input build use fresh source `a9e080608`, whose Rust product/build inputs are byte-identical to final tools source `901dae47c`. The complete positive run uses `b8cc7b584`; subsequent changes only replace the positive animation control with its literal pin, restore the attachment identity lookup, and capture the delayed-animation witness. Those affected checks, native phone and all helper tests are replayed at `901dae47c`. The new two-second bound itself is already present during the full positive run. No product input changed after the workspace verification.

The first broad positive attempt aborted on an occupied ephemeral-range target port (`Address already in use (os error 98)`); it did not reach a summary and receives no full-run credit. Optional `WS8BM_BROWSER_PORT_BASE` retains the original 52020 default and occupied-port refusal; final runs explicitly use 22020/22021/22022 outside the host's 32768–60999 client range. No waiting threshold, retry limit or application listener semantics change.

The full positive run reports 135/3: the translated listener setup failure and attachment preview failure are retained, along with an unrelated composer startup failure after many `ERR_NETWORK_CHANGED` requests at the original 15-second Stimulus gate. The literal motion control passes all three fresh pairs afterward. The attachment controls also pass three pairs after their scope fix, but the filename negative still fails before its target and keeps that declaration deferred. These subset receipts do not rewrite the failed broad summary.

The complete 195-variant discrimination run reports 190 valid paired rejections and five invalid results, with zero escapes. The invalid variants are reopen-focus (earlier initial focus), header geometry (network), owner refusal (startup/not served/network), attachment filename (earlier preview), and initial URL text (network). The later affected motion set validates reopen-focus but retains one invalid listener setup; workspace validates both the profile geometry and the newly registered 196th slow-animation variant. The enhanced picker is valid. No invalid attempt is retried automatically (`WS8BM_DISCRIMINATION_RETRIES=1`), masked or given rejection credit. The declared rows' mutation proofs are in the original-scope table; motion animation's valid paired proof remains in the complete run at the unchanged translated assertion.

Commands below are direct tracked entry points, invoked in the fresh clone. Browser environment uses `PARITY_CPUS=2`, pinned `PARITY_IMAGE=triage-reference-d7c7de92`, the configured compiler throttle, test limit eight and `WS8BM_DISCRIMINATION_RETRIES=1`. No case exclusions are applied. A `--repeat 3` invocation records every fresh pair, including any failure; it is not a retry that discards failures. The private media runner applies only to child processes. Its binaries/libraries are recreated with tracked `reference-tools/users/media_runtime.sh`; tests never import the runner, an old seed, or an old target as source.


### Full positive set (retained failures)

`final-controls.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
```

```text
WS8bm behaviour check: 135 named cases passed on Rails and Rust; 3 failed; no pixel checks
```

### Full registered served set (195 variants)

`all-mutants.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 5; bounded fresh-fixture retries only
WS8bm discrimination check: 190 served mutants rejected on Rails and Rust across 128 named checks; 5 invalid or escaped
```

### Affected motion negatives

`final-motion-mutants.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py motion --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 1; bounded fresh-fixture retries only
WS8bm discrimination check: 7 served mutants rejected on Rails and Rust across 7 named checks; 1 invalid or escaped
```

### Literal native animation controls

`native-motion-final-controls.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py motion --case "mobile drawer animates in, lands in place, and returns focus with motion on" --repeat 3 --keep-going
```

```text
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
1 runs, 8 assertions, 0 failures, 0 errors, 0 skips
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
```

### Release-click controls

`release-control3.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case "a release click landing on the just-opened menu does not activate it" --repeat 3 --keep-going
```

```text
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
```

### Attachment controls after the identity fix

`attachment-final-controls.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case "Markdown replies and file attachments remain usable" --repeat 3 --keep-going
```

```text
WS8bm behaviour repetition: 3 paired attempts; 1 named declaration; 0 failed
```

### Attachment negatives (invalid filename proof retained)

`attachment-final-mutants.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case "Markdown replies and file attachments remain usable" --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 1; bounded fresh-fixture retries only
WS8bm discrimination check: 3 served mutants rejected on Rails and Rust across 1 named checks; 1 invalid or escaped
```

### Bounded workspace / specific animation witness

`workspace-deadline-final-mutants.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case "workspace follows the system theme and mobile navigation remains reachable" --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 2 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

### Enhanced picker negative

`final-enhanced-mutant.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py composer_attach_menu --case "From Google Drive starts the enhanced share flow when sharing is configured" --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

### Native phone regression

`native-phone-final-control.log`

```sh
python3 rust/reference-tools/messaging/behavior-check.py threads --case "keeps the thread drawer usable on a phone and preserves the channel" --keep-going
```

```text
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
1 runs, 25 assertions, 0 failures, 0 errors, 0 skips
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

### Browser helper tests

`latest-helpers.log`

```sh
node --test --test-concurrency=8 rust/reference-tools/messaging/behavior-*.test.mjs
```

```text
ℹ tests 54
ℹ pass 54
ℹ fail 0
ℹ duration_ms 3988.326775
```

### Python tool tests

`attachment-final-python.log`

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

```text
Ran 29 tests in 2.405s
OK
```

### Controller attribution

`controller-inventory.log`

```sh
python3 rust/reference-tools/messaging/controller-case-inventory.py
```

```text
WS8bm controller inventory: 156 named declarations; 156 scoped attributions; 0 owner-blocked
```

### System attribution

`system-inventory.log`

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

```text
WS8bm system inventory: 135 named declarations; 129 mapped behaviour passes; 6 deferred; 0 WS12 blocked; no pixel checks
```

### All three fresh seed families

`seeds.log`

```sh
bash rust/parity/bin/seed build default first_run agents_ui
```

```text
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
seed: building agents_ui
seed: agents_ui -> parity/.seed/agents_ui (6.1M)
```

### Fresh-clone workspace with canonical media

`workspace.log`

```sh
mise exec rust@1.98.1 -- cargo test --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --no-fail-fast -- --test-threads=8
```

```text
test result: ok. 2718 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 603.77s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.75s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1359 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 108.13s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.72s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.98s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.18s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.70s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.32s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.81s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.06s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.99s
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
```

### Strict workspace clippy

`clippy.log`

```sh
mise exec rust@1.98.1 -- cargo clippy --manifest-path rust/Cargo.toml --locked --workspace --exclude html5ever --all-targets -- -D warnings
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 17s
```

### Release-input-only binaries

`release.log`

```sh
bash rust/ci/with-release-inputs.sh mise exec rust@1.98.1 -- cargo build --locked --bins
```

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 22s
```

All six formerly mismatching media cases pass with canonical libraries/binaries. Every workspace target passes: the raw target summaries above total 4,806 passed, zero failed and 15 unchanged ignores across 59 targets. `html5ever` is the existing shared/CI vendor exclusion; this slice adds no exclusion or ignore.

Locked metadata is rerun with `cargo metadata --locked --format-version 1 >/dev/null`; every tracked Rust Cargo.toml is parsed with Python's TOML parser, which rejects duplicate keys. The branch has no Rust product/build-input changes and therefore adds no non-test include outside crates/:

```text
WS8bm merge metadata: locked metadata PASS; duplicate TOML keys absent
WS8bm product/release-input audit: 0 Rust product/build-input changes; no new non-test external includes
WS8bm fresh workspace totals: 4806 passed; 0 failed; 15 existing ignores; 59 test targets
```

The sole own scratch target is removed with the exact target directory supplied to Cargo; unrelated worktrees, shared caches and services remain untouched. Active app/forwarder/test/browser processes, native Chromium/ChromeDriver, all owned container mounts/names, both listener ranges and scratch target directories are audited afterward:

```text
     Removed 23024 files, 30.8GiB total
WS8bm final owned resources: {"processes": [], "listeners": [], "containers": [], "scratch_targets": []}
```

This is a partial continuation checkpoint: eleven declarations close, six own follow-ups remain, and none is owner-blocked. No final browser failure or invalid mutation result is hidden by the aggregate inventory. Work stops after pushing and rechecking that this resource inventory remains empty.
