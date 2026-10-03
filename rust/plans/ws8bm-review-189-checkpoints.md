# WS8b-m: #189 re-review of 21e4a77d

Worker GPT-6.1 Sol; pinned Rails `d7c7de9264c63015be398001d7a1094e7695a6db` plus wave4/_common.md approved drift. Tools and docs only. No Rust product, application asset, golden, normalization mask, Selenium atom or assertion deadline changed. Existing compiler throttle and eight-thread limit retained. This review is merged into the accepted continuation without reclassifying its declarations.

## Corrections and audit

| Check / category | Old behaviour | New behaviour | Pinned original |
| --- | --- | --- | --- |
| Category tabs / attribution | Initial Recent-tab absence could credit a chooseTab no-op before any tab click | Requires the post-people-click `tab('people')` caller and a witness from the actual mutated handler on that tab | message_toolbar_test.rb:92-95 |
| Outside attach tap / attribution | Shared `expanded` helper credited the initial open assertion | Requires the post-outside-click `expanded(false)` caller and the mutated handler's room-header pointer event | composer_attach_menu_test.rb:107-111 |
| Lazy emoji data / attribution | Shared resource predicate matched initial and later assertions | Names only the initial no-fetch assertion. This mutation acts during connect, so a pre-open failure is intended | message_toolbar_test.rb:105-107; later :118-120 |
| Edited code / attribution | Shared keyword helper could credit initial `const` for the missing `def` variant | `const` variants name the initial TS/search caller; `def` names only the post-save Python caller | code_highlighting_test.rb:129,141; search :91,98 |
| Restored edit draft / attribution | Cancellation and success shared an identical field call | Same field assertions and two-second budget, with separate named after-cancel and after-success callers. The saved-draft variant cannot credit cancellation | message_interactions_test.rb:125,137 |
| Cleared thread draft / attribution | Nonempty restoration could credit the later clear variant | Names only `waitForVisibleProperty(reply,'value','')` after send/navigation | composer_test.rb:283; restored :271 |
| Older search text / attribution | Broad source text matched the initial negative assertion too | Names only the visible result after pagination | search_forward_edit_test.rb:33,35 |
| Open menu owner / attribution | Broad owner selector also matched the closed-menu absence assertion | Names the visible-owner wait, excluding the hidden-state wait | message_interactions_test.rb:118,130; message_toolbar_test.rb:48; system_test_helper.rb:134 |
| Every negative pair / retry accounting | Retrying INVALID discarded an ESCAPED peer | Every attempt's escape is recorded before retry; its case/variant can never regain rejection credit in that run. Duplicate failure rows are avoided | Harness contract; no positive assertion changes |
| React with thumbs up / literal attribute | Accessible name accepted missing aria-label when title remained | Visible `button[aria-label="React with thumbs up"]`, two seconds | message_toolbar_test.rb:16 |
| Add reaction / literal attribute | Same accessible-name substitution | Visible literal aria-label, two seconds | message_toolbar_test.rb:17 |
| Reply to message / literal attribute | Same accessible-name substitution | Visible literal aria-label, two seconds | message_toolbar_test.rb:18 |
| Open thread / literal attribute | Same accessible-name substitution | Visible literal aria-label, two seconds | message_toolbar_test.rb:19 |
| More message actions / literal attributes | Accessible name plus a separate attribute read | Original visible combined aria-label/aria-haspopup selector, one two-second assertion | message_toolbar_test.rb:20 |
| Picker options / literal attribute | Named buttons could substitute aria-labelledby for required aria-label | Shared `.emoji-picker__option[aria-label=...]` lookup for each original option assertion/click. Existing 2/10-second budgets retained | message_toolbar_test.rb:73,78-79,96-101,116,126-127,137,149-150,163-164,175,189,202,209 |
| Category count / literal role | Accessible-role lookup | Original `[role="tab"]` CSS scope, same visible count of 11 and two-second budget | message_toolbar_test.rb:91 |

[The full target ledger](ws8bm-review-189-target-audit.tsv) audits all **159** registered case/variant pairs across **97** named checks, including the original 153 and six added literal-attribute variants. The sweep distinguishes failures of an affected initial render/visible lookup from failures in setup before the mutated action. Shared focus helpers already name post-action callers; release-click requires its actual click/guard/menu witness; CSS and scripted variants retain their state witnesses. The unit checks cover every target's existence and reject unrelated earlier assertions. A syntax check also validates the executable served tab-handler witness.

The explicit-attribute sweep covers every executable system module, including supplementary checks. Profile buttons and destination menu/focus scopes already use literal selectors; composer/list ARIA observations already compare literal attributes. `click_button`/`fill_in` actions retain their original named locator semantics. Scripted active-element observations remain attribute reads. The More-button hidden-allowed attribute query and all three hidden-scope diagnostic variants remain non-visible DOM scopes. The previous claim in ws8bm-review-189-labels.md (and its historical report copy) that toolbar/picker selectors were already literal was incorrect and is corrected.

## Failing-first evidence

Own evidence directory: `.scratch/ws8bm-review-tabs/`. Reviewer evidence was read only. A fresh no-hardlink clone started at `21e4a77d`; its target directory was absent. The exact reviewer served modules were copied into that clone, their import paths relocated, and the unchanged driver pointed to those diagnostic modules. No expectation, app response oracle, product source or deadline was replaced.

- `picker-before.log`: registered chooseTab no-op plus unrelated initial Recent selection; both apps fail the initial Recent assertion, each with `tabClicks: 0`, yet the old runner credits both and exits zero.
- `picker-after.log`: the same served module and initial assertion fail, but the new target denies credit on both apps; zero tab clicks and no mutated-handler witness. Expected exit one, not a passing negative receipt.
- `mixed-before.log`: the reviewer's exact driver-AST reproduction reports two attempts, `final_passed: 1`, empty failures and an erroneous exit zero. New tests execute the driver's actual retry/count AST with synthetic `CompletedProcess` results for both Rails ESCAPED/Rust INVALID and the mirror. Both tests fail first with `1 != 0`; strengthened tests executing the actual terminal exit block also fail against the baseline with `0 != 1` (`retry-exit-before.log`). After the fix both produce passed zero, a retained failure and actual driver exit one. Rejection/INVALID-only sequences still retry successfully in both directions.
- `aria-before.log`: removing only React's aria-label while retaining title is accepted on both apps. `aria-after.log` fails on that exact literal selector on both apps at 2,000 ms.
- `missing-all-toolbar-aria-labels-before.log`: removing all five toolbar labels while retaining titles is accepted on both apps. Five separately registered final variants each require the corresponding literal assertion and observe its actual label-less button.
- `missing-option-aria-label-before.log`: an actual picker option uses aria-labelledby to preserve its accessible name while dropping aria-label. The old named-option lookup accepts both apps. The final variant requires the original literal Grinning-face assertion and witnesses the changed option.
- `attribution-before.log`: four newly added setup/phase regressions fail on the old mappings. `extra-attribution-before.log`: three more reject-early/negative-scope regressions fail before the second narrowing. Synthetic attribution probes are unit evidence, not additional live Rails/Rust pairs.

These diagnostic runs and their expected failing summaries are retained below. The combined setup failure remains INVALID; it is not mislabeled REJECTED. A genuine registered chooseTab-only mutation must separately fail its post-click assertion with its witnessed handler.

## Verification boundaries and remaining scope

The after-probe setup-attribution run used `f7f2dfa52`; `e0bd321a4` then corrected and unit-checked only the served tab-handler witness syntax. The full helper/positive/negative runs use the fresh clone sealed at `e0bd321a4`. The final `df705466b` follow-up strengthens the synthetic tests to execute the driver exit and confines failure deduplication to negative runs, preserving each failed positive repetition; no negative predicate, action witness, assertion or deadline changed. Its fresh-clone helper/Python reruns are recorded below. The normal binary was built cold from the baseline's identical Rust source; the current-source tool host is rebuilt by the driver. All cases use fresh fixture copies; only documented read-only cases share servers with new browser contexts. Ordinary job behaviour and the deterministic URL/PR held-job boundaries are unchanged. There are no new Rust workspace, clippy or release-build claims for these tools/docs-only changes.

The frozen review inventory remains its historical **102 passed / 27 deferred / 6 blocked**. The continuation retains **156/156 controller declarations; 118 passed / 17 deferred / 0 blocked systems**. This review adds no declaration credit. The continuation's exact seventeen reasons remain in its inventory/report; release-click stays deferred. The duplicate-delivery check injects Turbo markup in the browser and does not verify server-originated redelivery. Queue-observation races remain inherited. Message-list/composer entry points and ws8bm-integration.md are unchanged.

## Current raw receipts

Commands below ran sequentially from the fresh clone `.scratch/ws8bm-review-tabs/fresh`, with `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=8`, and `CARGO_TARGET_DIR` / `CAMPFIRE_REFERENCE` pointing into that clone. The existing machine-wide rustc throttle was unchanged. The clone started without a target, and its first normal build completed in 3m 01s. Logs are under `.scratch/ws8bm-review-tabs/`; they are evidence, never prerequisites for tracked tests.

The broad positive run retains two failures: Rails missed the original 10 s attachment reply-preview assertion in the already-deferred Markdown attachment flow, and Rails timed out at the unchanged 15 s Stimulus startup gate in `viewer`, before the reply-preview fallback assertions ran. Rust was attempted independently. The thirty reviewed declarations pass separately. No broad-run failure is erased by an isolated rerun.

The full discrimination set ran all 159 registered pairs. Six invalid pair attempts were retained: three attachment-filename attempts failed the earlier attachment preview instead, and three other attempts failed at Stimulus startup/network gates. Fresh-fixture retries recovered the latter three. The filename variant therefore has no paired rejection credit and remains the sole final invalid variant. There were zero escaped application attempts in this run. All six added literal-ARIA variants and the actual post-click category default were rejected at their registered assertions on both apps.

`picker-before.log`:

```sh
WS8BM_REVIEW_PROBE=picker-unrelated WS8BM_DISCRIMINATION_RETRIES=1 python3 .scratch/review-probes/driver.py message_toolbar --case 'the picker shows category tabs and switches between them' --mutant default --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 0; bounded fresh-fixture retries only
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`picker-after.log` (expected failing proof):

```sh
WS8BM_REVIEW_PROBE=picker-unrelated WS8BM_DISCRIMINATION_RETRIES=1 python3 .scratch/review-probes/driver.py message_toolbar --case 'the picker shows category tabs and switches between them' --mutant default --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 1; bounded fresh-fixture retries only
WS8bm discrimination check: 0 served mutants rejected on Rails and Rust across 0 named checks; 1 invalid or escaped
```

`aria-before.log`:

```sh
WS8BM_REVIEW_PROBE=toolbar python3 .scratch/review-probes/driver.py message_toolbar --case 'the toolbar stays hidden until hover or focus and labels every action' --mutant missing-toolbar-aria-label --keep-going
```

```text
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`aria-after.log` (expected failing proof):

```sh
WS8BM_REVIEW_PROBE=toolbar python3 .scratch/review-probes/driver.py message_toolbar --case 'the toolbar stays hidden until hover or focus and labels every action' --mutant missing-toolbar-aria-label --keep-going
```

```text
WS8bm review escape check: 0 served mutants accepted on Rails and Rust; 1 failed probes; no parity credit
```

`retry-exit-before.log` (expected failing proof):

```sh
python3 -m unittest discover -s .scratch/ws8bm-review-tabs/retry-baseline -p '*_test.py'
```

```text
Ran 4 tests in 0.024s
FAILED (failures=2)
```

`final-helpers.log`:

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs rust/reference-tools/messaging/behavior-discrimination.test.mjs
```

```text
ℹ tests 25
ℹ pass 25
ℹ fail 0
ℹ skipped 0
```

`final-python.log`:

```sh
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
```

```text
Ran 11 tests in 0.035s
OK
```

`fresh-all-paired.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
```

```text
WS8bm behaviour check: 103 named cases passed on Rails and Rust; 2 failed; no pixel checks
```

`fresh-thirty.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions message_actions_mobile message_toolbar code_highlighting --keep-going
```

```text
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-all-mutants.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
```

```text
WS8bm invalid discrimination attempts: 6; bounded fresh-fixture retries only
WS8bm discrimination check: 158 served mutants rejected on Rails and Rust across 97 named checks; 1 invalid or escaped
```

`fresh-hidden.log`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
```

```text
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`fresh-inventory.log`:

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

```text
WS8bm system inventory: 135 named declarations; 102 mapped behaviour passes; 27 deferred; 6 WS12 blocked; no pixel checks
```

`fallback-isolated.log` (final tool source `df705466b`):

```sh
python3 rust/reference-tools/messaging/behavior-check.py composer --case 'clicking a reply preview falls back to the permalink when the target is not loaded' --keep-going
```

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

This exact isolated fresh-fixture control passes on both apps; the broad-run Rails 15 s Stimulus startup timeout remains recorded, with no deadline change and no additional declaration credit.
