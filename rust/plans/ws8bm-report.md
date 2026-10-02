# WS8bm PR #189 visible-assertion correction — review-ready partial slice

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus approved drift in wave4/_common.md. Behaviour and response parity only; no pixel checks.

Audited every assertion in all thirty reviewed checks against the pinned originals. Fixed the reported forwarded Ruby/composer gaps and the additional field, menu-owner, delivery-flag, link and explicit visible-lookup gaps. The shared helper now requires visibility and an expected value/attribute on the same node in one polling observation. Explicit visible lookups and their actions share the original deadline. The pinned Selenium 4.35.0 atom is unchanged, including ancestor opacity. **No Rust product or Rails changes.** No deadline, native concurrency setting or queue-observation race was changed.

The received inventory remains **102 passed / 27 deferred / 6 WS12-blocked** out of 135 exact system declarations, and **146/156 controller declarations**, with ten WS12 deferrals. This correction earns no declaration credit; full system sign-off remains partial. The release-click declaration remains honestly deferred despite any later passing diagnostic.

**Duplicate-delivery limitation:** the check injects Turbo markup in the browser on the actual mounted row/render queue and verifies the browser guard and object identity. It **does not verify server-originated redelivery**; the original Rails test calls `message.broadcast_create`.

## Audit table

Each row lists a changed assertion/lookup, its pinned Rails line and its own served opacity probe. The [complete audit](ws8bm-visibility-audit.md) separately inventories every assertion in all thirty checks, including unchanged raw reads already guarded on the same element, JavaScript geometry/focus/clipboard/identity observations, link attributes and explicit `visible: false`/`:all` exceptions. It also describes the supplementary search/forward and Markdown checks.

| ID | Assertion changed | Original Rails line and reason | Served mutant |
| --- | --- | --- | --- |
| V1 | Editing source field value | message_interactions:122, default-visible `assert_field` | transparent-edit-field |
| V2 | Draft restored after cancel | message_interactions:125, default-visible `assert_field` | transparent-cancelled-draft |
| V3 | Draft restored after successful save | message_interactions:137, default-visible `assert_field` | transparent-saved-draft |
| V4 | Editing source after duplicate delivery | message_interactions:169, default-visible `assert_field` | transparent-redelivery-field |
| V5 | Newer draft after asynchronous save | message_interactions:197, default-visible `assert_field`, wait 10 | transparent-newer-draft |
| V6 | Empty field after failed highlighter load | code_highlighting:123, default-visible `assert_field` | transparent-cleared-code-field |
| V7 | Checked Notify author field | message_interactions:234, default-visible `assert_field(checked: true)` | transparent-notify-field |
| V8 | Message owning the open shared menu | helper:134, default-visible `assert_selector`; used by menu-open calls | transparent-menu-owner |
| V9 | Open message row after More | message_toolbar:48, default-visible `assert_selector`; preserves the prior empty-value check | transparent-more-owner |
| V10 | Duplicate-delivery rendered HTML flag | message_interactions:166, default-visible `assert_selector`, replaces attached-only wait | transparent-redelivery-flag |
| V11 | Initial Ruby source before forwarding | search_forward_edit:46, visible text assertion; a visible row alone does not show its code | transparent-ruby-code |
| V12 | Forwarded Ruby source bytes | search_forward_edit:60, default-visible `assert_selector`; table visibility does not show the code | transparent-forwarded-code |
| V13 | Back to Designers link lookup | code_highlighting:95, default-visible `click_link` query; opacity is omitted by Playwright actionability | transparent-search-back-link |
| V14 | Project notes href | workspace_markdown:336, default-visible `assert_link` must match both visible link and href | transparent-project-link |
| V15 | Empty-search query field value | search_forward_edit:26, default-visible `assert_field` | transparent-search-field |
| V16 | Forward destination lookup | message_interactions:279, default-visible `find`, wait 10 | transparent-forward-destination |
| V17 | First forward checkbox lookup | message_interactions:298, default-visible `find`, wait 10 | transparent-forward-checkbox |
| V18 | Quick Thumb reaction lookup | message_interactions:318/331, default-visible `find` | transparent-quick-thumb |
| V19 | Emoji search field keyboard lookup | message_toolbar:177/191, default-visible `find_field` | transparent-picker-search |
| V20 | Flags tab lookup after selecting People | message_toolbar:99, default-visible `find`; the earlier 11-tab count no longer guards this later state | transparent-flags-lookup |
| V21 | Context-menu message body lookup | helper:129, default-visible `find` used by the reviewed menu-opening helper | transparent-context-body |
| V22 | Toolbar-hover message body lookup | message_toolbar:220, default-visible `find`; a visible toolbar does not show a separately transparent body | transparent-hover-body |
| V23 | Room-header dismissal lookup | message_actions_mobile:44, default-visible `find` | transparent-room-header |

## Failing-first evidence

Before changing any assertion module or the helper at `042d8e86253ed5938c81bb5b9a5830fbea244637`, added only served mutations and diagnostic runner scaffolding. Verified those six modules/helper were byte-identical to `git show 042d8e86:<file>`. All 23 probes escaped on **both** applications, with valid startup, applied served mutations and no actual network failures. Scaffolding is committed in `ddda6039f`; the correction is `ce56adb99`, with the empty-anchor value preserved in `1b11dc60b` and conditional probes made instantaneous in `c5c5d2253`.

First ran the fifteen assertion probes, then added and ran the eight explicit-lookup probes. Logs: `.scratch/ws8bm-visible-assertions-review/before.log` and `before-lookups.log`. The diagnostic mode does not earn parity credit. The final registry/runner can reproduce all 23 together against unchanged reviewed assertion source using `--mutant-set visible-assertions`. A separate baseline clone at `ddda6039f` overlaid only those two scaffolding files, again verified all assertion modules/helper against `042d8e86`, and reran the five instantaneous conditional probes.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_interactions code_highlighting search_forward_edit workspace_markdown --mutant-set visible-assertions --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions --mutant-set visible-lookups --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_interactions --mutant-set instantaneous-opacity --keep-going
```

```text
WS8bm review escape check: 15 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 8 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm failing-first: 23 served opacity mutants ACCEPTED on Rails and Rust at 042d8e86; assertion modules/helper byte-identical; 0 failed probes
WS8bm corrected-probe baseline: all assertion modules/helper are byte-identical to 042d8e86
WS8bm review escape check: 5 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

V1 reproduces Astra's exact textarea CSS; V11 reproduces its exact Ruby-code CSS. V12 hides only forwarded Ruby so the source visibility assertion cannot mask the forwarded-code defect. Draft probes target cancel, successful save and preserving newer typing separately. All mutations alter served app assets/implementations; none changes an oracle value. The five conditional probes disable the mutated target's transition, so opacity is zero at lookup time instead of fading through Selenium-visible positive opacity. This affects served mutants only; product transitions and assertion deadlines are unchanged. The repeat before/after evidence uses these corrected probes.

## Fresh-source verification

Fresh clone: `.scratch/ws8bm-visible-assertions-fresh`, initially at `ce56adb99`, then advanced to `c5c5d22538a6f152749e90b78e2285d388004919` for the final thirty-check and full discrimination runs; it started with no target, seeds or node_modules. The harness builds its own seeds/binary, installs declared Node/browser inputs, verifies the pinned Rails image and exact atom, and checks real HTTP/Cable and successful saved-row projections. Its own `rust/target` is the only added scratch Cargo target. The configured machine-wide rustc throttle and `-j2` remain in use. Rust workspace tests/clippy are not rerun for this reference-tools-only request, as authorized in the preceding request. Historical native results are retained in the [received report at 042d8e86](https://github.com/Smart-Data-Ohio/smartfire/blob/042d8e86253ed5938c81bb5b9a5830fbea244637/rust/plans/ws8bm-report.md); they are not claimed as rerun results.

Commands below ran in that fresh clone with `CARGO_TARGET_DIR="$PWD/rust/target"`. The supplementary two search/forward positives passed in the broader run recorded below; the separate Markdown positive and all seven supplementary mutants passed. Those supplementary assertion implementations and mutation entries are unchanged between `ce56adb99` and the final source revision.

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --exclude-case 'editing to add a URL renders its card live and the edited marker on load' --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'Markdown messages reach other users and editing preserves the original source' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'Markdown messages reach other users and editing preserves the original source' --negative --keep-going
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs
```

Raw summary lines (logs: `final-paired.log`, `final-mutants.log`, `fresh-search-mutants.log`, `fresh-markdown-paired.log`, `fresh-markdown-mutants.log` and `final-helper-tests.log` in `.scratch/ws8bm-visible-assertions-review/`):

```text
seed: default -> parity/.seed/default (6.1M)
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 19s
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm discrimination check: 56 served mutants rejected on Rails and Rust across 30 named checks; 0 invalid or escaped
WS8bm discrimination check: 5 served mutants rejected on Rails and Rust across 2 named checks; 0 invalid or escaped
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm discrimination check: 2 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
ℹ tests 3
ℹ suites 0
ℹ pass 3
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1933.626687
WS8bm opacity proof: 23 served mutants accepted at 042d8e86 and rejected after the fix on Rails and Rust; 0 missing probes
WS8bm full discrimination total: 63 served mutants rejected on Rails and Rust across 33 named checks; 0 invalid or escaped
WS8bm pinned system inventory: 102 passed / 27 remaining / 6 owner-blocked; 135 exact names and source hashes verified
```

Reconciled every V1–V23 variant by exact case/variant/application lines across the baseline and corrected logs. The five instantaneous conditional probes were repeated against the unchanged baseline; the full baseline clone's six assertion modules/helper are still byte-identical to `042d8e86`. All 23 unique new probes escaped that source and are rejected after the correction on both apps. The 63-variant total is 56 in the reviewed thirty, five in the two changed search/forward checks and two in the changed Markdown check. Inventory names and SHA256 hashes were separately rechecked against pinned Git source; statuses are unchanged. The helper tests additionally prove that a hidden correct-value/attribute node and a visible wrong-value sibling cannot jointly satisfy an assertion.

Removed only the owned fresh clone's 4.4G Cargo target after checking that no process was running a target executable, all three assigned ports were free and no WS8bm-owned container remained. Preserved source clones, baseline/probe logs and browser inputs. No scratch Cargo targets remain.

```text
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

The requested reference-tool correction is complete and PR-ready. Stopping here as requested; the remaining original port scope is listed below.

## Supplementary failures retained

The initial broader run also tried the unchanged third search/forward declaration, `editing to add a URL renders its card live and the edited marker on load`. It failed on **Rails** at the original 15-second Loading post wait (`behavior-search-forward.mjs:42`); the thirty reviewed declarations and the two changed search/forward declarations all passed. No product fix, timing relaxation, expectation change or new declaration credit was made for this supplementary failure. The subsequent requested thirty-check gate runs every one of those thirty; the separate search-mutant command selects only its two changed declarations and explicitly omits this unmodified extra. Log: `fresh-paired.log`.

The first discrimination attempt exposed four conditional-opacity probes that still faded through visible positive opacity, and one `net::ERR_NETWORK_CHANGED` startup failure. The gate classified all five as invalid/escaped, never as successful rejections. The conditional probes were corrected and rerun against unchanged reviewed source before the final full gate. No network-error allowlist, concurrency override or retry was added to the harness. Log: `fresh-reviewed-mutants.log`.

```text
WS8bm behaviour check: 32 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm failed application: http://127.0.0.1:52020 editing to add a URL renders its card live and the edited marker on load
WS8bm discrimination check: 51 served mutants rejected on Rails and Rust across 30 named checks; 5 invalid or escaped
```

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — two earlier reference failures; passed this fresh run with the pinned atom and original assertion/deadlines; received deferral retained pending reference reliability sign-off, with no new declaration credit

`test/system/code_highlighting_test.rb`

- thread code stays readable in both themes and scrolls within a narrow screen

`test/system/threads_test.rb`

- keeps the thread drawer usable on a phone and preserves the channel
- marks a joined thread read only while the conversation is visible
- opens a shared thread message link around an older post
- keeps an anchored older thread unread when a new reply arrives
- discusses a pull request from its card

`test/system/workspace_markdown_test.rb`

- Markdown replies and file attachments remain usable
- workspace follows the system theme and mobile navigation remains reachable

`test/system/composer_attach_menu_test.rb`

- From Google Drive starts the legacy picker flow
- From Google Drive starts the enhanced share flow when sharing is configured

`test/system/drive_attachments_test.rb`

- attach Drive files from the picker, send textless, and remove through edit
- edit a room message in the composer and remove one of two attachments
- attach a Drive file from the thread composer

`test/system/motion_test.rb`

- motion is off by default in the test environment
- mobile drawer animates in, lands in place, and returns focus with motion on
- member selection mode moves no rows and resizes nothing
- people directory bar shifts no rows when toggling
- people directory bar stays stuck while scrolling
- room menu measures at full scale when clamping to the viewport edge
- mobile drawer keeps the room list scroll position across close and reopen
- mobile drawer reveals a current room far down the list on first open
- mobile drawer reopens on the current room when it is already in view

`test/system/mobile_layout_test.rb`

- the profile page fits phone widths without scrolling sideways
- headers outside the workspace shell stay opaque over scrolled content
- headers outside the workspace shell never cover the page or its scrollbar
- pages outside the workspace shell show no drawer toggle that opens nothing

### Owner-blocked system declarations (6)

`test/system/threads_test.rb`

- tracks work, assigns an owner, completes and reopens it without losing the conversation
- shows work-thread guidance in the new-thread form and on the work page
- keeps the new-thread guidance usable on a phone
- shows work assignment activity to the owner and opens the exact thread

`test/system/message_list_a11y_test.rb`

- text fields stay at 16px on touch devices without changing the desktop look

`test/system/mobile_layout_test.rb`

- every drawer destination has one toggle that opens the drawer on itself

### Owner-blocked controller declarations (10)

All are in `test/controllers/channel_threads_controller_test.rb`, awaiting WS12 board/work writes and agent work services.

- converts a thread to work, assigns an eligible owner, and keeps an audit trail
- work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable
- assigned owner can change work status but cannot reassign it
- only a thread manager can remove work tracking
- the work model also protects conversion when the owner field is omitted
- work status updates from separate stale instances produce one event per real change
- a manager can assign an eligible agent and the agent is notified
- the owner picker lists eligible agents with profiles and excludes ineligible ones
- a member who cannot manage the thread cannot assign an agent
- ordinary thread fields remain separate from work tracking
