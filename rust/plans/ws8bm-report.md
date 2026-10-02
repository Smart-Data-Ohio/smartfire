# WS8bm PR #189 action, visible-text and deadline corrections — partial port

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus the approved drift in wave4/_common.md. Behaviour and byte-response parity only; no pixel checks.

Fixed the three URL findings and their categories throughout the thirty reviewed declarations and every supplementary executable module (105 names in thirteen files). Element actions use the shared Selenium-visible lookup within the original action budget. Text matching uses Selenium 4.35.0's visible-text routines on the selected element, excluding hidden descendants. The existing vendored isDisplayed atom is unchanged; the new text routines are unmodified upstream functions, with Closure dependencies supplied by the tool. Explicit hidden/all queries, scripted actions, literal code-source reads and geometry observations preserve their original scopes.

Every assertion uses Capybara's pinned two-second default or its original explicit wait. The URL loading assertion stays fifteen seconds; the reloaded edited marker is two seconds. Negative visible-selector/text checks retry while forbidden matches remain, and explicit all-node security checks retry without filtering hidden nodes. Timeout zero is a single snapshot. Successful absence returns immediately, matching Capybara's synchronize semantics; these checks do not claim that a forbidden node can never appear later. Navigation and application/Cable readiness remain separate from assertion waits.

| Category / principal changed check | Old behaviour | New behaviour | Pinned Rails line |
| --- | --- | --- | --- |
| Text: URL loading card | Visible card accepted hidden descendant Loading post text | Card's Selenium visible text must contain Loading post, within 15 s | search_forward_edit_test.rb:78 |
| Action: URL editor fill | Playwright filled an opacity-zero editor | Shared visible field lookup and fill, within 2 s | search_forward_edit_test.rb:74 |
| Deadline: reloaded edited marker | Supplementary default allowed 30 s | Visible marker/text within the original 2 s | search_forward_edit_test.rb:84 |
| Action: picker search fill | Playwright filled an opacity-zero search field | Shared visible field lookup and fill, within 2 s | message_toolbar_test.rb:77 |
| Negative/deadline: focused-message removal | DOM detach accepted a 4 s removal under the supplementary 30 s default | Original visible absence retries within 2 s; surviving focus within its original 10 s | message_list_a11y_test.rb:127–128 |

The complete per-call action/text table and additional wait/negative corrections follow below. Raw property/attribute, count, focus, scripted browser and explicit hidden/all exceptions were swept as part of the same audit. All changes are reference tools or documentation; no Rust product, Rails application, golden, normalization mask, test concurrency or rustc throttle changes. No assertion wait was widened to obtain a pass.

Inventory remains **102 passed / 27 deferred / 6 WS12-blocked** out of 135 named system declarations, and **146/156 controller declarations**, with ten WS12 deferrals. This review earns no new declaration credit. Release-click, attachment preview and PR Discuss remain deferred even if a diagnostic attempt passes. Activity/work/board remain flagged WS12 seams. Exact remaining names are listed at the end.

**Duplicate-delivery limitation:** the case injects Turbo markup into the browser and checks the mounted-row/render-queue guard. It does **not verify server-originated redelivery**; the pinned original calls `message.broadcast_create`.

## Failing-first served evidence at 89de2839

Before changing any assertions, only added the five served category probes and diagnostic registry/CLI selection. Both apps accepted all five against the unchanged 89de2839 assertions. These are deliberate application asset/response mutations, not replacement oracle values: hidden loading text, opacity-zero URL editor, a CSS edited-marker reveal after four seconds, an opacity-zero picker field, and an actual Turbo remove render delayed four seconds. Ordinary hidden-allowed probes remain separate positive acceptance checks.

Commands at the unchanged assertion revision:

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit message_toolbar --mutant-set categories --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y --mutant-set categories --keep-going
```

Raw summaries (`before-categories.log`, `before-negative-removal.log` under `.scratch/ws8bm-category-review/`):

```text
WS8bm review escape check: 4 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

Each log includes a Rails ACCEPTED and Rust ACCEPTED line for every named variant. Post-fix fresh-clone paired rejection evidence is reported below, including all five.

## Pinned text and wait semantics

A remote Selenium probe uses the reference image's actual Capybara 3.40.0 and Selenium 4.35.0 gems with host Chromium/ChromeDriver 153. It confirms that visibility-hidden and opacity-zero descendant text does not match, and preformatted whitespace agrees with the shared text helper. This is a helper semantic check, **not execution of the original Ruby system files**. The broader mapped checks still drive both live apps and inspect their persisted rows.

Raw lines from `text-oracle.log`:

```text
WS8bm Capybara oracle: version 3.40.0; default 2; retry 0.01
WS8bm Capybara text: card: "X"
WS8bm Capybara text: opacity: "visible"
WS8bm Capybara text: code: "one\n  two "
WS8bm Capybara match: card: false
WS8bm Capybara match: opacity: false
```

Source/provenance and checksums are documented in `reference-tools/messaging/selenium/UPSTREAM.md`. The pinned visibility atom still hashes to `ae26018c01cd27448b250f8e55a094cbfcd2e2cbbe171c78aaa906e1b5c3ed7c`; every live harness invocation validates it against the reference image. The helper tests cover hidden descendant text, whitespace, element visibility, six action types, negative retry/all-node scope and timeout-zero snapshots.

## Fresh-clone verification

Created `.scratch/ws8bm-category-fresh` with `git clone --no-hardlinks --no-checkout .` and checked out the tool commits. Its first cold build generated its own seeds, dependencies and Cargo target; no test required a pre-existing scratch fixture or target. The initial broad diagnostic loaded several consecutive tool revisions while the audit finished; its failures are retained below. The final 105-name suite, separate thirty-name suite, full-registry discrimination pass, helper/planner checks and hidden-scope probes use the sealed source revision `629d98dd2f88ae2a206e477d03c3c2b6a44b4c61`. Documentation/evidence-only follow-ups do not change those source files. Every browser invocation validates the pinned source and Selenium atom and runs `cargo build --locked -j2`; the rustc throttle remains configured.

The required review discrimination set is **95 unique variants across 45 names**: the previous 89, five new category probes, and the existing default variant for the newly selected focused-removal name. The expanded run also attempts all **147 registered variants across 97 names**. It proves 146; the remaining `threads / discusses a pull request from its card / default` never applies its thread-header mutation before the Rails card loses its title. Its isolated retry also remains invalid. It is the existing deferred PR flow and gets no rejection or declaration credit. The three permitted hidden-scope CSS variants are positive acceptance proofs, excluded from negative scheduling. All five new probes have explicit Rails and Rust REJECTED markers after the fix.

Run the following from the fresh clone, with `CARGO_TARGET_DIR` set to that clone's `rust/target` (the drivers did this). Every command below was executed in this review; the isolation commands ran three times each. The broad positive command ran first diagnostically and then against the final sealed source.

```sh
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs
python3 -m unittest discover -s rust/reference-tools/messaging -p '*_test.py'
python3 rust/reference-tools/messaging/behavior-check.py --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_interactions message_actions_mobile message_toolbar code_highlighting --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_list_a11y message_toolbar --mutant-set hidden-scopes --keep-going
python3 rust/reference-tools/messaging/behavior-check.py --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'sending messages between two users' --mutant hidden-body-visible-presentation --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py threads --case 'discusses a pull request from its card' --mutant default --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --mutant hidden-url-loading-text --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'desktop keyboard composition keeps line breaks and sends once after composition ends' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer --case 'clicking a reply preview scrolls to the loaded message instead of navigating' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py sending_messages --case 'editing messages' --keep-going
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

Retained failures and follow-ups:

- The first 105-name diagnostic reports 102 passes and three failures. The keyboard assertion combined two lines into one raw-newline expectation after switching to visible text; the original workspace_markdown:107–109 asserts First line and Second line separately. Corrected the port to those two visible assertions while retaining the exact single two-line database row. It passes in the final broad run and isolation. The other failures are the existing deferred PR Discuss flow and a Rails Send Message click timeout in the loaded-reply flow at 1,992 ms. The final broad reply run and all three reply isolates pass at the original two-second action budget. This is evidence of an intermittent mapped Rails click failure, not proof that the original Ruby case fails.
- The full negative run retains three invalid attempts: sending-message startup closed before readiness, an unapplied deferred PR header mutation, and network-change errors in the hidden URL loading probe. Only the missing tuples were retried. Sending and hidden loading now reject on both apps; the PR mutation stays unapplied and is not counted. No network, startup or job-observation fix was made.
- The final 105-name run reports 103 passes and two failures: Rails Stimulus startup for root editing, before any case assertion, and the deferred PR card/title lookup at its original ten-second wait. Root editing passes in its isolated rerun on both apps, including persisted rows. The verifier requires current paired proof for every one of the 102 previously credited declarations.
- **URL timing:** both broad attempts pass the author-only Loading post sequence. The first two final URL isolates pass on both apps; the third times out on Rails at exactly the unchanged fifteen-second loading assertion, before Rust runs. The selected card, visible-text predicate and sequence match search_forward_edit:74–84; the pinned gem probe above confirms hidden text semantics. Astra's read-only 89de2839 evidence measures the loading state lasting only 63–74 ms before replacement by the error card. This is a retained intermittent Rails-side failure of the mapped loading assertion, suitable for the lead's deflake list. The original Ruby system test was not executed, so no claim of a reproduced Ruby-original flake is made. No wait is widened and no loading assertion is removed.

Raw summaries below are copied verbatim from `.scratch/ws8bm-category-review/`; invalid attempts remain alongside successful follow-ups.

`fresh-helper.log`

```text
ℹ tests 8
ℹ pass 8
ℹ fail 0
ℹ skipped 0
```

`fresh-planner.log`

```text
Ran 6 tests in 0.000s
OK
```

`fresh-all-paired.log`

```text
WS8bm behaviour check: 102 named cases passed on Rails and Rust; 3 failed; no pixel checks
```

`fresh-final-all-paired.log`

```text
WS8bm behaviour check: 103 named cases passed on Rails and Rust; 2 failed; no pixel checks
```

`fresh-final-thirty.log`

```text
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-hidden-scopes.log`

```text
WS8bm review escape check: 3 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

`fresh-all-mutants.log`

```text
WS8bm discrimination check: 144 served mutants rejected on Rails and Rust across 95 named checks; 3 invalid or escaped
```

`fresh-mutant-retry-1.log`

```text
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`fresh-mutant-retry-2.log`

```text
WS8bm discrimination check: 0 served mutants rejected on Rails and Rust across 0 named checks; 1 invalid or escaped
```

`fresh-mutant-retry-3.log`

```text
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
```

`fresh-mutant-evidence.log`

```text
WS8bm full discrimination evidence: 146/147 unique served mutants rejected on Rails and Rust; 1 missing paired proofs
```

`fresh-keyboard-isolation.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-reply-isolation-1.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-reply-isolation-2.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-reply-isolation-3.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolation-1.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolation-2.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`fresh-url-isolation-3.log`

```text
WS8bm behaviour check: 0 named cases passed on Rails and Rust; 1 failed; no pixel checks
```

`fresh-positive-retry-1.log`

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

`verified-evidence.log`

```text
WS8bm required discrimination evidence: 95/95 unique served variants rejected on Rails and Rust across 45 named checks; 0 unresolved
WS8bm hidden-scope evidence: 3/3 permitted served CSS variants accepted on Rails and Rust; excluded from negative scheduling; 0 missing
WS8bm reviewed positive evidence: 30/30 paired passes; 0 failed attempts retained; 0 missing declarations
WS8bm current credited evidence: 102/102 paired behaviour and persisted-row passes; 0 missing current proofs; deferred credit unchanged
```

`fresh-final-inventory.log`

```text
WS8bm system inventory: 135 named declarations; 102 mapped behaviour passes; 27 deferred; 6 WS12 blocked; no pixel checks
```

Raw URL failure, retained:

```text
WS8bm failed application: http://127.0.0.1:52020 editing to add a URL renders its card live and the edited marker on load
WS8bm browser flow FAILED: search_forward_edit: editing to add a URL renders its card live and the edited marker on load: TimeoutError: Selenium visibility visible timed out after 15000ms: locator('.message[data-message-id="935962058"]').locator('.x-post-card').locator('capybara-text={"mode":"filter","value":"Loading post","exact":false}'); last visible count: 0
```

Fresh cold binary build (the harness invocation above):

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 03s
```

Rust workspace tests and strict clippy were not rerun: this review changes reference tools and documentation only, as authorized for this scope. No old test/clippy summary is presented as a current run.

## Cleanup and final scope

After all test processes exited, removed only this review's regenerable fresh-clone Cargo target. Its listeners on 52020–52022 and its Rust/browser processes are gone. Verified no `.scratch` target directories remain. Kept raw receipts for review. The final declaration names/statuses/source hashes and the pinned visibility atom are unchanged.

From the fresh clone:

```sh
mise exec rust@1.98.1 -- cargo clean --manifest-path rust/Cargo.toml --target-dir /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-category-fresh/rust/target
```

Raw cleanup and scope lines:

```text
     Removed 6852 files, 5.2GiB total
WS8bm scope verification: tools/docs only; 0 Rust product, golden or mask file changes; declaration names/statuses/source hashes unchanged; 0 scratch targets remain
```

## Per-check audit

Pinned source: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`. This covers every implemented check in the 30 reviewed declarations and all supplementary modules (105 executable names in 13 files). It is an audit of mapped behaviour checks, not a claim that the original Ruby system files ran. Pixel assertions receive no credit.

Each table row identifies a changed call in the reviewed `89de2839` source. Repeated/shared checks list the corresponding pinned query/action lines in their declarations. The shared helper supplies the same scope to every call; scripted browser observations, raw code-source reads and explicit hidden/all queries are treated separately below.

### Changed actions and visible-text lookups

| Check at 89de2839 | Category | Old behavior | New behavior | Pinned Rails line |
| --- | --- | --- | --- | --- |
| behavior-actions.mjs:56 `row.scrollIntoViewIfNeeded()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:39,43,59 (visible message find before long press); system_test_helper.rb:202–210 (scripted touch and 700 ms hold) |
| behavior-actions.mjs:75 `row.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:16,25,47 |
| behavior-actions.mjs:82 `page.getByRole('menuitem',{name:'Delete message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:47 |
| behavior-actions.mjs:100 `editor.fill('A draft that must survive editing')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:113 |
| behavior-actions.mjs:101 `page.getByRole('button',{name:'Cancel message context',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:124 |
| behavior-actions.mjs:124 `page.getByRole('menuitem',{name:'Edit message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:168 |
| behavior-actions.mjs:126 `editor.fill('First edit request')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:179 |
| behavior-actions.mjs:133 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:191,218 |
| behavior-actions.mjs:135 `editor.fill('A newer draft typed while saving')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:192 |
| behavior-actions.mjs:142 `editor.fill('Failed edit')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:217 |
| behavior-actions.mjs:142 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:191,218 |
| behavior-actions.mjs:143 `page.locator('#composer [data-composer-target="feedback"]').filter({hasText:'The message could not be saved'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:219 |
| behavior-actions.mjs:145 `page.getByRole('button',{name:'Cancel message context',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:221 |
| behavior-actions.mjs:147 `page.getByRole('menuitem',{name:'Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:231 |
| behavior-actions.mjs:148 `page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:233,239 |
| behavior-actions.mjs:150 `page.getByLabel('Notify author',{exact:true}).uncheck()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:235 |
| behavior-actions.mjs:151 `browser.locator('.message__reply-preview').filter({hasText:'Replying to JZ'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:239 |
| behavior-actions.mjs:152 `page.getByRole('menuitem',{name:'Delete message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:228,231,237 |
| behavior-actions.mjs:154 `page.locator('.message__reply-preview').filter({hasText:'Replying to a deleted message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:246 |
| behavior-actions.mjs:157 `page.getByRole('menuitem',{name:'Copy text',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:263 |
| behavior-actions.mjs:159 `page.getByRole('menuitem',{name:'Copy link',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:260,263,267,270,274,277,279,282 |
| behavior-actions.mjs:161 `page.getByRole('menuitem',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:277,279,280,282 |
| behavior-actions.mjs:163 `dialog.locator('.message-forward-dialog__destination').filter({hasText:'Forward destination'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:279 |
| behavior-actions.mjs:164 `dialog.getByLabel('Add a note (optional)',{exact:true}).fill('Forwarded from the interaction test')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:280 |
| behavior-actions.mjs:165 `dialog.getByRole('button',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:277,279,280,282 |
| behavior-actions.mjs:166 `page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:'Forwarded to 1 destination'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:285 |
| behavior-actions.mjs:168 `page.getByRole('menuitem',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:295 |
| behavior-actions.mjs:171 `submit.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:292,295,302 |
| behavior-actions.mjs:172 `page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:'Forwarded to 1 destination'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:303 |
| behavior-actions.mjs:182 `chip.locator('.reaction-chip__count').filter({hasText:new RegExp(^${count}$)})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_interactions_test.rb:320,324,333,337,342,346 |
| behavior-actions.mjs:191 `row.locator('.reaction-chip[data-reaction="👍"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_interactions_test.rb:315,318,328,331,341 |
| behavior-attach-menu.mjs:21 `button.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:14 |
| behavior-attach-menu.mjs:25 `button.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:27,28 |
| behavior-attach-menu.mjs:25 `menu.getByRole('menuitem',{name:'From this device',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:28 |
| behavior-attach-menu.mjs:28 `button.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:72 |
| behavior-attach-menu.mjs:30 `button.press('ArrowDown')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:85,90,93,96 |
| behavior-attach-menu.mjs:36 `button.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:106,109 |
| behavior-attach-menu.mjs:36 `page.locator('.room-header__name').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:109 |
| behavior-attach-menu.mjs:38 `button.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:120 |
| behavior-attach-menu.mjs:41 `menu.getByRole('menuitem',{name:'From Google Drive',exact:true}).click({trial:true})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_attach_menu_test.rb:120 |
| behavior-attach-menu.mjs:58 `page.locator('#composer .composer__file').filter({hasText:'hello'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_attach_menu_test.rb:166 |
| behavior-attach-menu.mjs:63 `page.locator('#composer .composer__file').filter({hasText:'pasted'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_attach_menu_test.rb:174 |
| behavior-attach-menu.mjs:68 `page.locator('#composer .composer__file').filter({hasText:'dropped'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_attach_menu_test.rb:183 |
| behavior-boosts.mjs:7 `message(browser).getByRole('link',{name:'Add a boost',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | boosting_messages_test.rb:87 |
| behavior-boosts.mjs:8 `message(browser).locator('input[name="boost[content]"]').fill(value)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | boosting_messages_test.rb:13,37,49,63,71,86,88 |
| behavior-boosts.mjs:10 `message(browser).getByRole('button',{name:'Submit',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | boosting_messages_test.rb:14,72 |
| behavior-boosts.mjs:11 `browser.locator('[data-boost-delete-target="content"]').filter({hasText:value})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | boosting_messages_test.rb:26,30,36,51,55,62,96 |
| behavior-boosts.mjs:15 `david.locator('[data-boost-delete-target="content"]').filter({hasText:/^Hello$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | boosting_messages_test.rb:26,30 |
| behavior-boosts.mjs:16 `boost.locator('[data-boost-delete-target="content"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | boosting_messages_test.rb:25,27 |
| behavior-boosts.mjs:16 `boost.getByRole('button',{name:'Delete this boost',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | boosting_messages_test.rb:27 |
| behavior-boosts.mjs:18 `browser.locator('[data-boost-delete-target="content"]').filter({hasText:/^Hello$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | boosting_messages_test.rb:26,30 |
| behavior-code.mjs:12 `code.locator('.code-token').filter({hasText:keyword})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | code_highlighting_test.rb:38,76,91,98,122,129,141 |
| behavior-code.mjs:19 `page.getByRole('combobox',{name:'Write a message',exact:true}).fill(source)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | code_highlighting_test.rb:31,74,115,127 (`send_message`); system_test_helper.rb:96–105 (`find_field`, click, paste) |
| behavior-code.mjs:20 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | code_highlighting_test.rb:31,74,115,127 (`send_message`); system_test_helper.rb:92 (`click_on "Send Message"`) |
| behavior-code.mjs:27 `row.getByRole('button',{name:'Copy code',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | code_highlighting_test.rb:52,106,118,145 |
| behavior-code.mjs:49 `row.locator('code.language-unknown-language').filter({hasText:'<script>window.codeExecuted = true</script>'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | code_highlighting_test.rb:79 |
| behavior-code.mjs:65 `row.locator('pre code.language-ts').filter({hasText:'const value: string = "hello";'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | code_highlighting_test.rb:116 |
| behavior-code.mjs:71 `page.getByRole('combobox',{name:'Write a message',exact:true}).fill(code_replacement)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | code_highlighting_test.rb:138 |
| behavior-code.mjs:71 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | code_highlighting_test.rb:139 |
| behavior-composer.mjs:11 `message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by composer_test.rb |
| behavior-composer.mjs:15 `page.getByRole('menuitem',{name:'Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:273,290 |
| behavior-composer.mjs:16 `page.locator('[data-composer-target="contextLabel"]').filter({hasText:'Replying to'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:173,291 |
| behavior-composer.mjs:18 `page.locator('[data-reply-target="body"]').filter({hasText:body})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:16,19,25,45,80,85,86,90,108,117,133,144,161,170,173,174,200,209,215,237,251,255,269,274,281,291,294,300 |
| behavior-composer.mjs:19 `message.locator('.message__reply-preview').filter({hasText:"Third time's a charm."})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:130,154,173,174,294 |
| behavior-composer.mjs:24 `page.locator(a[href="/rooms/${id}"]).first().click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:21,130,154,168,169,236,250,254,268,273,280,289,290,293,299 |
| behavior-composer.mjs:30 `editor.fill(':thu')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:15 |
| behavior-composer.mjs:30 `page.locator('suggestion-option').filter({hasText:'thumbsup'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:16 |
| behavior-composer.mjs:32 `editor.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:21 |
| behavior-composer.mjs:32 `editor.fill('hello')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:22 |
| behavior-composer.mjs:32 `editor.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:22,23 |
| behavior-composer.mjs:43 `editor.fill(':zx')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:42 |
| behavior-composer.mjs:43 `editor.fill(':open')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:44 |
| behavior-composer.mjs:44 `page.locator('suggestion-option').filter({hasText:'OpenAI'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:45 |
| behavior-composer.mjs:45 `editor.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:47 |
| behavior-composer.mjs:49 `editor.fill('@kev+in')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:63 |
| behavior-composer.mjs:55 `editor.fill(':open')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:79 |
| behavior-composer.mjs:55 `page.locator('suggestion-option').filter({hasText:'OpenAI'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:80 |
| behavior-composer.mjs:62 `editor.press('ArrowDown')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:92,97 |
| behavior-composer.mjs:63 `editor.press('Escape')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:92,97 |
| behavior-composer.mjs:66 `editor.fill(':open')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:107 |
| behavior-composer.mjs:66 `page.locator('suggestion-option').filter({hasText:'OpenAI'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:108,117 |
| behavior-composer.mjs:68 `page.locator('suggestion-option').filter({hasText:'OpenAI'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:108,117 |
| behavior-composer.mjs:69 `page.locator('.message[data-message-id]').filter({hasText:/^:open$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:108,117 |
| behavior-composer.mjs:70 `editor.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:121 |
| behavior-composer.mjs:73 `message.locator('.message__reply-preview-link').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:130 |
| behavior-composer.mjs:84 `message.locator('.message__reply-preview-link').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:154 |
| behavior-composer.mjs:93 `page.getByRole('menuitem',{name:'Delete message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:169 |
| behavior-composer.mjs:95 `browser.locator('[data-reply-target="body"]').filter({hasText:'A reply whose source goes away'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:170,173,174 |
| behavior-composer.mjs:96 `delivered.locator('.message__reply-preview').filter({hasText:'Replying to a deleted message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:173 |
| behavior-composer.mjs:105 `david.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from david')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:194 |
| behavior-composer.mjs:106 `recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from kevin')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:197,203 |
| behavior-composer.mjs:107 `page.locator('[data-typing-notifications-target="author"]').filter({hasText:/^David, David$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:200,209,215 |
| behavior-composer.mjs:108 `recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from kevin')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:197,203 |
| behavior-composer.mjs:109 `david.getByRole('combobox',{name:'Write a message',exact:true}).fill('')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:194,197,203,206,212 |
| behavior-composer.mjs:110 `page.locator('[data-typing-notifications-target="author"]').filter({hasText:/^David$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:200,209,215 |
| behavior-composer.mjs:111 `recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:194,197,203,206,212 |
| behavior-composer.mjs:114 `editor.fill('Designers draft')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:222 |
| behavior-composer.mjs:116 `editor.fill('Pets draft')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:229 |
| behavior-composer.mjs:118 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:236 |
| behavior-composer.mjs:126 `panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item').filter({hasText:'Composer draft thread'}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:268,280 (visible thread-item find/click) |
| behavior-composer.mjs:126 `panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item').filter({hasText:'Composer draft thread'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:268,280 (find thread item with text: thread_name) |
| behavior-composer.mjs:130 `panel.getByRole('button',{name:'New thread',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:250 |
| behavior-composer.mjs:133 `panel.locator('[data-thread-panel-target="createName"]').fill('Composer draft thread')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:252 (fill_in "Thread name") |
| behavior-composer.mjs:134 `panel.locator('[data-thread-panel-target="createMessage"]').fill('The thread for draft persistence.')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:253 |
| behavior-composer.mjs:136 `panel.locator('[data-thread-panel-target="createSubmit"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:254 (visible createSubmit find/click) |
| behavior-composer.mjs:138 `panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:'Composer draft thread'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | Supplementary title consistency after composer_test.rb:255 (conversation pane); not an original title assertion |
| behavior-composer.mjs:139 `panel.getByRole('combobox',{name:'Write a thread reply',exact:true}).fill('Thread draft')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:258,272 |
| behavior-composer.mjs:139 `editor.fill('Channel draft')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:260 |
| behavior-composer.mjs:142 `reply.fill('Thread draft sent')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:272 |
| behavior-composer.mjs:142 `panel.getByRole('button',{name:'Send Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | composer_test.rb:273 |
| behavior-composer.mjs:143 `panel.locator('.message__body').filter({hasText:'Thread draft sent'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | composer_test.rb:274 |
| behavior-message-destinations.mjs:19 `message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by message_list_a11y_test.rb |
| behavior-message-destinations.mjs:29 `message.scrollIntoViewIfNeeded()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:434,489 (visible message find before long_press); system_test_helper.rb:202–210 |
| behavior-message-destinations.mjs:41 `page.locator('#search-results .message').filter({hasText:'A searchable menu result'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_list_a11y_test.rb:423 |
| behavior-message-destinations.mjs:67 `page.locator('.message').filter({hasText:"Third time's a charm."})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_list_a11y_test.rb:496 |
| behavior-message-destinations.mjs:84 `bio.fill(caseName===destinationCases[6]?'Reduced motion flash check':'Reduced motion dismiss check')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:598 |
| behavior-message-destinations.mjs:85 `bio.locator('xpath=ancestor::form').getByRole('button',{name:'Save changes',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:575,600 |
| behavior-message-destinations.mjs:92 `page.locator('.flash__dismiss').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:603 |
| behavior-message-list.mjs:35 `row(id).locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by message_list_a11y_test.rb |
| behavior-message-list.mjs:127 `editor.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:235 |
| behavior-message-list.mjs:127 `editor.press('ArrowUp')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:236 |
| behavior-message-list.mjs:128 `page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_list_a11y_test.rb:238 |
| behavior-message-list.mjs:136 `editor.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:252 |
| behavior-message-list.mjs:136 `editor.press('ArrowUp')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:253 |
| behavior-message-list.mjs:137 `page.locator('.flash--client[role="alert"]').filter({hasText:'temporarily unavailable'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_list_a11y_test.rb:255 |
| behavior-message-list.mjs:139 `page.getByRole('menuitem',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:267 |
| behavior-message-list.mjs:143 `page.getByRole('menuitem',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_list_a11y_test.rb:284 |
| behavior-message-list.mjs:156 `list.locator('.message').filter({hasText:/^History post 0$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_list_a11y_test.rb:332,353,354,370,371 |
| behavior-search-forward.mjs:7 `search.fill('nonsense zebra tuxedo xyzzy')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:24 |
| behavior-search-forward.mjs:7 `search.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:24 |
| behavior-search-forward.mjs:8 `page.getByText('No messages match',{exact:false})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:25 |
| behavior-search-forward.mjs:14 `page.locator('#search-results .message').filter({hasText:'system paging alpha'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:32,36 |
| behavior-search-forward.mjs:15 `page.getByRole('link',{name:'Load older results',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:34 |
| behavior-search-forward.mjs:16 `page.locator('#search-results').getByText('system paging alpha',{exact:true})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:33,35 |
| behavior-search-forward.mjs:22 `page.locator('pre code').filter({hasText:'puts :forwarded'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:60 |
| behavior-search-forward.mjs:23 `source.locator('pre code').filter({hasText:'puts :forwarded'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:60 |
| behavior-search-forward.mjs:24 `source.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by search_forward_edit_test.rb |
| behavior-search-forward.mjs:25 `page.getByRole('menuitem',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:49,53 |
| behavior-search-forward.mjs:27 `dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)').filter({hasText:'Designers'}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:51 |
| behavior-search-forward.mjs:27 `dialog.locator('.message-forward-dialog__destination:not(.message-forward-dialog__destination--thread)').filter({hasText:'Designers'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:51 |
| behavior-search-forward.mjs:28 `dialog.getByRole('button',{name:'Forward',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | search_forward_edit_test.rb:49,53 |
| behavior-search-forward.mjs:29 `page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:/Forwarded to 1 destination/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:46,50,55,59,60 |
| behavior-search-forward.mjs:31 `viewer.locator('pre code.language-ruby').filter({hasText:'puts :forwarded'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:60 |
| behavior-search-forward.mjs:38 `message.getByText('nothing linked yet',{exact:true})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:69 |
| behavior-search-forward.mjs:43 `message.locator('.x-post-card').filter({hasText:'Loading post'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:78 |
| behavior-search-forward.mjs:45 `message.locator('.message__edited').filter({hasText:'(edited)'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | search_forward_edit_test.rb:84 |
| behavior-toolbar.mjs:12 `row.getByRole('button',{name:'Add reaction',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:69,88,115,125,132,144,159,173,188,201 |
| behavior-toolbar.mjs:18 `row.locator(.reaction-chip[data-reaction="${content}"] .reaction-chip__count).filter({hasText:/^1$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_toolbar_test.rb:11,15,16,17,18,19,20,28,29,35,40,48,49,52,64,71,72,73,78,81,82,89,91,92,95,96,97,100,101,116,126,128,133,136,137,145,148,149,152,153,160,163,166,167,174,175,184,189,194,195,202,207,209,212,221 |
| behavior-toolbar.mjs:22 `originalMessage(recipient).locator(.reaction-chip[data-reaction="${content}"] .reaction-chip__count).filter({hasText:/^1$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_toolbar_test.rb:11,15,16,17,18,19,20,28,29,35,40,48,49,52,64,71,72,73,78,81,82,89,91,92,95,96,97,100,101,116,126,128,133,136,137,145,148,149,152,153,160,163,166,167,174,175,184,189,194,195,202,207,209,212,221 |
| behavior-toolbar.mjs:30 `row.getByRole('button',{name:'React with thumbs up',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:26 |
| behavior-toolbar.mjs:32 `row.getByRole('button',{name:'Reply to message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:34 |
| behavior-toolbar.mjs:33 `page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | message_toolbar_test.rb:35 |
| behavior-toolbar.mjs:34 `page.getByRole('button',{name:'Cancel message context',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:36 |
| behavior-toolbar.mjs:35 `row.getByRole('button',{name:'Open thread',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:39 |
| behavior-toolbar.mjs:37 `row.getByRole('button',{name:'More message actions',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:45 |
| behavior-toolbar.mjs:44 `row.focus()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:56,57 |
| behavior-toolbar.mjs:48 `search.fill('fire')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:77 |
| behavior-toolbar.mjs:48 `option('Fire').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:79 |
| behavior-toolbar.mjs:52 `page.locator('#emoji-picker-tab-people').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:93 |
| behavior-toolbar.mjs:62 `option('Grinning face').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:127 |
| behavior-toolbar.mjs:63 `page.locator('#emoji-picker-tab-recent').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:134 |
| behavior-toolbar.mjs:65 `page.locator('#emoji-picker-tab-custom').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:146 |
| behavior-toolbar.mjs:66 `option('Acme Corp').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:150 |
| behavior-toolbar.mjs:68 `search.fill('openai')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:162 |
| behavior-toolbar.mjs:69 `option('OpenAI').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:164 |
| behavior-toolbar.mjs:75 `page.locator('#emoji-picker-tab-smileys').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | message_toolbar_test.rb:204 |
| behavior-unread.mjs:12 `target.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by unread_divider_test.rb |
| behavior-unread.mjs:13 `page.getByRole('menuitem',{name:'Mark unread',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | unread_divider_test.rb:103 |
| behavior-unread.mjs:14 `page.locator('#sidebar .unread').filter({hasText:'Designers'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | unread_divider_test.rb:107 |
| behavior-unread.mjs:18 `list.locator('.message').filter({hasText:'First unread off page'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | unread_divider_test.rb:86 |
| behavior-unread.mjs:28 `pill.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | unread_divider_test.rb:93 |
| behavior-unread.mjs:29 `list.locator('.message').filter({hasText:'First unread off page'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | unread_divider_test.rb:86 |
| behavior-unread.mjs:45 `pill.click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | unread_divider_test.rb:66 |
| behavior.mjs:93 `page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:96–105 (`find_field`, click, paste); per-call `fill_in` in the named declaration |
| behavior.mjs:94 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:92 (`click_on`) |
| behavior.mjs:98 `page.getByRole('combobox',{name:'Write a message',exact:true}).fill(value)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:96–105 (`find_field`, click, paste); per-call `fill_in` in the named declaration |
| behavior.mjs:99 `page.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:92 (`click_on`) |
| behavior.mjs:102 `message.click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible `find(...).right_click`) |
| behavior.mjs:104 `page.getByRole('menuitem',{name:'Edit message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | sending_messages_test.rb:39 |
| behavior.mjs:105 `page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | sending_messages_test.rb:40 |
| behavior.mjs:106 `page.locator('#composer').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | sending_messages_test.rb:40 |
| behavior.mjs:153 `author.locator('.github-pr-card__title').filter({hasText:'Fix login'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:509,520 |
| behavior.mjs:154 `card().getByRole('button',{name:'Discuss',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:506,517 |
| behavior.mjs:155 `author.locator('.github-pr-thread-header .github-pr-card__title').filter({hasText:'Fix login'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:509,520 |
| behavior.mjs:156 `author.locator('.github-pr-files__heading').filter({hasText:'Files changed'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:510 |
| behavior.mjs:157 `author.locator('.github-pr-files__path').filter({hasText:'app/models/user.rb'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:511 |
| behavior.mjs:159 `card().getByRole('link',{name:'Discuss',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:506,517 |
| behavior.mjs:160 `author.locator('.github-pr-thread-header .github-pr-card__title').filter({hasText:'Fix login'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:509,520 |
| behavior.mjs:171 `root.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by threads_test.rb |
| behavior.mjs:172 `page.getByRole('menuitem',{name:'Create thread',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:552 |
| behavior.mjs:177 `panel.getByRole('button',{name:'New thread',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:101,563 |
| behavior.mjs:186 `nameField.fill(name)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:30,32,46,57,78,103,104,555,556,565,566 |
| behavior.mjs:189 `firstField.fill(first)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:30,32,46,57,78,103,104,555,556,565,566 |
| behavior.mjs:194 `panel.locator('[data-thread-panel-target="createSubmit"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:22,26,33,41,44,47,51,55,58,63,66,79,85,88,101,112,129,131,143,149,153,506,517,531,533,535,543,549,552,557,563,567 |
| behavior.mjs:197 `panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:name})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:18,45,48,54,56,60,67,74,75,81,89,92,93,102,130,132,137,138,139,142,145,146,147,150,151,154,155,379,380,384,392,393,509,510,511,520,539,544,554,564,572,573,577,583 |
| behavior.mjs:209 `author.locator('[data-reply-target="body"]').filter({hasText:value})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:18,45,48,54,56,60,67,74,75,81,89,92,93,102,130,132,137,138,139,142,145,146,147,150,151,154,155,379,380,384,392,393,509,510,511,520,539,544,554,564,572,573,577,583 |
| behavior.mjs:211 `panel.locator('.thread-panel__thread-content .message__body').filter({hasText:value})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:577 |
| behavior.mjs:214 `panel.getByRole('combobox',{name:'Write a thread reply',exact:true}).fill(value)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:32,46,57,78 |
| behavior.mjs:215 `panel.getByRole('button',{name:'Send Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:33,47,58,79 |
| behavior.mjs:227 `message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by threads_test.rb |
| behavior.mjs:231 `page.getByRole('button',{name:'Close threads',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:543 |
| behavior.mjs:241 `panel.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:'This thread link is invalid.'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:380 |
| behavior.mjs:258 `other.locator('[data-thread-panel-target="manage"] summary').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:129,131,143,149,153 |
| behavior.mjs:259 `other.locator('[data-thread-panel-target="closeThread"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:129,131,143,149,153 |
| behavior.mjs:260 `other.locator('[data-thread-panel-target="threadStatus"]').filter({hasText:/Closed thread/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:130,132,137,138,139,142,145,146,147,150,151,154,155 |
| behavior.mjs:265 `items.filter({hasText:'Active planning thread'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:130,132,137,138,139,142,145,146,147,150,151,154,155 |
| behavior.mjs:266 `items.filter({hasText:'Closed planning thread'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:130,132,137,138,139,142,145,146,147,150,151,154,155 |
| behavior.mjs:267 `panel.locator('[data-thread-panel-target="filter"]').selectOption('closed')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:141,143 |
| behavior.mjs:268 `items.filter({hasText:'Closed planning thread'}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:129,131,143,149,153 |
| behavior.mjs:268 `items.filter({hasText:'Closed planning thread'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:130,132,137,138,139,142,145,146,147,150,151,154,155 |
| behavior.mjs:272 `panel.getByRole('button',{name:'Join',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:149 |
| behavior.mjs:275 `panel.getByRole('button',{name:'Leave',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:153 |
| behavior.mjs:296 `author.getByRole('menuitem',{name:'Delete message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:88 |
| behavior.mjs:303 `panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:18,60 |
| behavior.mjs:305 `panel.locator('[data-thread-panel-target="preferences"] summary').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:22,26,33,41,44,47,51,55,58,63,66 |
| behavior.mjs:306 `panel.locator('[data-thread-panel-target="involvement"]').selectOption('nothing')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:23 |
| behavior.mjs:307 `panel.locator('[data-thread-panel-target="manage"] summary').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:22,26,33,41,44,47,51,55,58,63,66 |
| behavior.mjs:308 `panel.locator('[data-thread-panel-target="autoArchive"]').selectOption('1440')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:27 |
| behavior.mjs:309 `author.getByRole('combobox',{name:'Write a message',exact:true}).fill('A channel draft stays here.')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:30 |
| behavior.mjs:312 `author.getByRole('menuitem',{name:'Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:33,44,47,58 |
| behavior.mjs:313 `panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Replying to'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:45 |
| behavior.mjs:315 `panel.locator('.message__reply-preview').filter({hasText:'A reply from the thread drawer.'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:32,37 |
| behavior.mjs:316 `author.getByRole('menuitem',{name:'Edit message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:55 |
| behavior.mjs:317 `panel.locator('[data-composer-target="contextLabel"]').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:56 |
| behavior.mjs:319 `panel.locator('[data-thread-panel-target="parent"]').filter({hasText:"Third time's a charm."})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:18,60 |
| behavior.mjs:321 `author.locator('.message__quick-reaction[title="Thumbs up"]').click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | threads_test.rb:22,26,33,41,44,47,51,55,58,63,66 |
| behavior.mjs:322 `panel.locator('.boosts__reactions').filter({hasText:'👍'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | threads_test.rb:67 |
| behavior.mjs:335 `message.locator('h2').filter({hasText:/^Design review$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:337 `message.locator(selector).filter({hasText:value})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:340 `message.locator('pre code').filter({hasText:'const message = "<script>literal code</script>";'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:343 `message.locator('pre code.language-javascript[data-highlighted="yes"] .code-token').filter({hasText:'const'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:347 `author.locator('h2').filter({hasText:/^Design review$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:348 `message.locator('h2').filter({hasText:/^Design review$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:355 `page.locator(.message[data-message-id="${id}"] h2).filter({hasText:/^Review complete$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:55,67,72,78 |
| behavior.mjs:364 `editor.fill('First line')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:93 |
| behavior.mjs:364 `editor.press('Shift+Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:94,95,108,114 |
| behavior.mjs:364 `editor.pressSequentially('Second line')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:95 |
| behavior.mjs:369 `editor.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:94,95,108,114 |
| behavior.mjs:372 `editor.press('ArrowUp')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:94,95,108,114 |
| behavior.mjs:373 `author.locator('#composer').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:115 |
| behavior.mjs:379 `page.locator('p').filter({hasText:/^Safety check$/})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:134,137,138 |
| behavior.mjs:380 `message.locator('.message__body').filter({hasText:'Safety check'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:134 |
| behavior.mjs:381 `message.locator('pre code').filter({hasText:'<img onerror="literal code">'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:137 |
| behavior.mjs:392 `author.locator('strong').filter({hasText:'A useful point'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:145,153,165 |
| behavior.mjs:393 `parent.locator('.message__body').filter({hasText:'A useful point'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:145,153,165 |
| behavior.mjs:394 `parent.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by workspace_markdown_test.rb |
| behavior.mjs:395 `author.getByRole('menuitem',{name:'Reply',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:151 |
| behavior.mjs:396 `author.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:152 |
| behavior.mjs:397 `author.locator('#composer [data-composer-target="contextPreview"]').filter({hasText:'A useful point'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:145,153,165 |
| behavior.mjs:398 `author.getByLabel('Notify author',{exact:true}).uncheck()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:155 |
| behavior.mjs:400 `author.locator('#composer').filter({hasText:'markdown-workspace-attachment'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:160 |
| behavior.mjs:401 `author.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:161 |
| behavior.mjs:403 `page.locator('.message__reply-preview').filter({hasText:'A useful point'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:165 |
| behavior.mjs:404 `attachment.locator('.message__reply-preview').filter({hasText:'A useful point'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:165 |
| behavior.mjs:418 `editor.fill('@Kev')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:181 |
| behavior.mjs:418 `author.locator('suggestion-option').filter({hasText:'Kevin'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:182 |
| behavior.mjs:419 `editor.press('Enter')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:183 |
| behavior.mjs:423 `page.locator('strong').filter({hasText:'the layout'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:182,191,195,196,197 |
| behavior.mjs:424 `message.locator('.mention').filter({hasText:'Kevin'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:191,195,197 |
| behavior.mjs:436 `author.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:209,215 |
| behavior.mjs:440 `author.getByRole('combobox',{name:'Write a message',exact:true}).fill('')` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:214 |
| behavior.mjs:441 `author.getByRole('button',{name:'Restore draft',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:212 |
| behavior.mjs:443 `messages(page).locator('strong').filter({hasText:'Recovered'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:216 |
| behavior.mjs:446 `author.getByRole('combobox',{name:'Write a message',exact:true}).fill(first)` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:223 |
| behavior.mjs:454 `messages(recipient).filter({hasText:second})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | workspace_markdown_test.rb:232,237 |
| behavior.mjs:455 `author.getByRole('button',{name:'Send Message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | workspace_markdown_test.rb:236 |
| behavior.mjs:469 `original.click({button:'right'})` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | system_test_helper.rb:129 (visible body `find(...).right_click`), called by sending_messages_test.rb |
| behavior.mjs:472 `author.getByRole('menuitem',{name:'Edit message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | sending_messages_test.rb:39 |
| behavior.mjs:473 `author.locator('#composer').filter({hasText:'Editing Message'})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | sending_messages_test.rb:40 |
| behavior.mjs:481 `author.getByRole('menuitem',{name:'Delete message',exact:true}).click()` | Action | Raw Playwright action omitted Selenium opacity/visible lookup | Shared visible lookup + action inside the original budget | sending_messages_test.rb:65 |
| behavior.mjs:483 `messages(author).filter({hasText:"Third time's a charm."})` | Text | Raw descendant text could satisfy the selected node | Selenium visible descendant text on the selected node; hidden text excluded | sending_messages_test.rb:57,69 |

### Additional text/property, deadline and negative corrections

| Check | Category | Old behavior | New behavior | Pinned Rails line |
| --- | --- | --- | --- | --- |
| URL Loading post | Text | Visible card + hidden-inclusive `hasText` | Visible card + Selenium visible descendant Loading text, 15 s | search_forward_edit:78 |
| URL textarea fill (shared submit) | Action | Opacity-zero field filled by Playwright | Default-visible field lookup and fill, 2 s | search_forward_edit:74 |
| Reloaded edited marker | Deadline | Supplementary inherited 30 s | Visible marker/text at default 2 s | search_forward_edit:84 |
| Forwarded Ruby source | Text | Visible code node + raw textContent | Visible code node + visible text includes source | search_forward_edit:60 |
| Markdown raw code content assertion | Text | Code visibility then raw source string | Visible code text includes the literal source, independently of tokens | workspace_markdown:334 |
| Malicious thread title | Text | Raw title.textContent | Selected title visible text includes/equals the literal title | threads:392 |
| First thread reply count | Text/deadline | Raw textContent after 30 s visibility | Visible indicator text, broadcast deadline 15 s | threads:74 |
| Second reply indicator | Deadline | Supplementary 30 s default | Visible indicator, broadcast deadline 15 s | threads:81 |
| Empty reply indicator | Negative/deadline | Wait only for hidden attribute at inherited default | Retry visible absence 15 s, then retain all-node hidden-attribute check | threads:92–93 |
| Deleted thread replies | Negative/deadline | Required DOM detach, inherited 30 s | Retry visible absence 10 s | threads:89 |
| Thread join/leave controls | Negative/deadline | Instant false visibility reads, positive 30 s | Retry negative default 2 s; positive controls 10 s | threads:145–155 |
| Invalid external deep link pane | Deadline | 30 s | Visible pane, explicit 10 s | threads:379 |
| Thread create pane | Deadline | 30 s | Explicit visible pane 10 s | threads:102,554,564 |
| Conversation/title after create | Deadline/text | Conversation already 10 s, hidden-inclusive title text | Both visible pane and visible title text at 10 s | threads:572–573 |
| Thread parent preview | Deadline/text | 30 s/hidden descendant text | Visible preview text, 10 s | threads:18,59 |
| Thread reply context | Deadline/text | 30 s/hidden descendant text | Visible context text, 10 s | threads:45 |
| Thread edit context | Deadline/text | 30 s/hidden descendant text | Visible context text, 10 s | threads:54 |
| Drawer reply preview | Deadline/text | 30 s/hidden descendant text | Visible preview text, 10 s | threads:48 |
| PR card and header lookups (deferred flow) | Deadline/text | Inherited 30 s, then global 2 s | Separate original card scope 10 s and action find 2 s; visible header text 10 s | threads:505–506,509,516–517,520 |
| Drawer reaction | Deadline/text | 30 s/hidden descendant text | Visible reaction text, 10 s | threads:67 |
| Thread browser active/closed items | Deadline/text | 30 s lookup/wait | Explicit visible text 10 s, subsequent find/click default 2 s | threads:138,142–143 |
| Closing a thread/status | Deadline/text | Default action with no original explicit close-control assertion, then 30 s text | Close-control assertion 10 s, action 2 s, visible status text 10 s | threads:130–132 |
| Close drawer | Negative/deadline | 30 s function wait | Retry body class disappearance, 10 s | threads:544 |
| Toolbar popup attribute | Deadline | Instant attribute check after separate visibility | Atom + expected attribute in one 2 s query | message_toolbar:20 |
| Recent tab not selected | Deadline/visible state | Instant raw aria-selected inequality | Original visible non-selected tab selector retries for 2 s | message_toolbar:95 |
| Reaction active/inactive after count | Negative/deadline | Additional function mixed raw text and visibility with active state | Count text uses its 10 s visible-text assertion; active/inactive selector retries separately at 2 s | message_interactions:321,325,333,338,343,347 |
| Message/menu close and delete | Negative | Required DOM detach | Retry original default-visible absence at 2 s (dialog remains 10 s) | message_interactions:22,49,94,107; message_toolbar:52; message_actions_mobile:45,63; sending_messages:69; system_test_helper:115 |
| All-node code security counts | Negative | Instant raw count | Retry count without visibility filtering at default 2 s | code_highlighting:41,80 |
| All-node Markdown security count | Negative | Instant raw count | Retry raw all-node count at default 2 s | workspace_markdown:138 |
| Keyboard message text | Text | Combined raw-newline expectation | Separate visible First line / Second line assertions, both 2 s; exact single two-line database row retained | workspace_markdown:107–109 |
| Composer URL-encoding request | Deadline | Waited for HTTP response at 30 s, unlike request observation | Request-record observation synchronizes at default 2 s | composer:65–68 |
| Composer reply context/preview | Deadline/text | 30 s text waits | Visible context and preview text, 10 s | composer:291,294 |
| Autocomplete blur delivery | Deadline/text | Body assertion 2 s, 30 s element waits | Default 2 s fields/suggestions; delivered text explicit 10 s | composer:25 |
| Loaded reply target | Deadline | 30 s | Explicit visible target 10 s | composer:133 |
| Deleted reply target/tombstone | Negative/deadline/text | DOM detach and 30 s text | Visible absence and tombstone text at 10 s; link absence default 2 s | composer:170,173–174 |
| Saved room draft delivery | Deadline | 2 s added sender body wait | Original delivered text wait 10 s | composer:237 |
| Thread draft pane/body | Deadline/text | 30 s | Create/conversation/body explicit 10 s, draft fields 2 s | composer:251,255,269,274,281 |
| Boost delete button | Deadline | Raw click, no explicit wait-5 assertion | Visible button wait 5 s, visible lookup/click 2 s | boosting_messages:26–27 |
| Boost removal | Negative/text | Required DOM detach of selected boost | Retry visible boost/text absence 2 s | boosting_messages:30 |
| Message-list initial focus/tab stop | Deadline | 30 s | Both explicit 10 s | message_list_a11y:10,14 |
| Stream/direct replacement | Deadline | 30 s | Stream 10 s; direct swap default 2 s | message_list_a11y:78,90,112 |
| Deleted message and neighbor focus | Negative/deadline | DOM detach 30 s and inherited focus wait | Visible absence 2 s, neighbor focus 10 s | message_list_a11y:127–128,142–143 |
| Message-list edit/error contexts | Deadline/text | 30 s text wait | Visible text at explicit 10 s | message_list_a11y:238,255,387 |
| Forward dialog after metadata gate | Deadline | 30 s | Explicit 10 s | message_list_a11y:272 |
| Pending-action owner | Deadline | Instant raw owner attribute | Visible owner + expected attribute within 2 s | message_list_a11y:293 |
| Cached-page expanded attribute | Deadline/all scope | Instant raw attribute | Retry CSS attribute selector 2 s, including hidden rows | message_list_a11y:307, visible: false |
| History text/live-region return | Negative/deadline/text | 30 s text/function waits | Visible inserted text 10 s; region not busy restoration 10 s | message_list_a11y:353–354 |
| Live edit/own body and region | Negative/deadline/text | 30 s text/region waits | Visible delivered text and restoration at 10 s | message_list_a11y:391–392,411–412 |
| Standalone/search menu owner | Deadline | Instant raw attribute | Visible owner attribute query 2 s | system_test_helper:134 |
| Reduced-motion flash enter/leave | Negative/deadline | 30 s waits | Enter 10 s; automatic leave 10 s; manual dismiss leave 2 s | message_list_a11y:577,589,602,604 |
| Empty/search first-page results | Deadline/text | 30 s | No-matches/area/count-40 explicit 10 s; count-42 and negative visible text default 2 s | search_forward_edit:25,29,32–33,36 |
| Forward dialog/status/table | Deadline/text | 30 s | Explicit 10 s; visible text; Ruby code default 2 s | search_forward_edit:50–51,56,59–60 |
| Unread divider/pill text | Text/deadline | Raw textContent + inherited 30 s | Selected node visible text; initial divider 5 s, jump landing default 2 s | unread_divider:19,35,61,70–71,94,107 |
| Jump pill hiding / room unread | Negative/deadline | 30 s | Retry visible absence/unread state 5 s | unread_divider:64,70,104; system_test_helper:123 |
| Shared assertion defaults | Deadline | Only 30 reviewed files had 2 s; other positives 30 s and negatives 3 s | Every file uses Capybara default 2 s identically in positive/negative modes; navigation 30 s and readiness/Cable 15 s stay separate | Capybara 3.40.0 default_max_wait_time=2; application_system_test_case:23–25,37,48; system_test_helper:71–78 |
| Zero-wait snapshot helper | Deadline | timeout=0 could poll indefinitely | One observation succeeds/fails immediately; helper regression covers it | Capybara Node::Base#synchronize; explicit setup all-node wait:0 at system_test_helper:73–74 |

### Audited exceptions retained

- `page.keyboard`/mouse/CDP actions and `evaluate`/`dispatchEvent` are the original scripted/browser-level actions. They do not invent an element find. Scripted focus remains scripted in attach-menu and the message-list cases; only original visible `find` calls receive visible lookup.
- Code literal byte comparisons remain raw `textContent` where code_highlighting:39 explicitly uses `evaluate_script("this.textContent")`; code visibility and visible selector-text assertions are separate. Clipboard results, persisted rows and JavaScript/geometry observations remain literal/raw.
- Field/link values remain property/attribute queries on the same atom-visible node. The viewport meta, cached-page attributes, hidden composer context, thread controller-attribute and all-node security queries preserve explicit hidden/all semantics. The three permitted served CSS variants remain outside negative scheduling.
- Every negated selector/text query retries while forbidden visible matches remain, for the original wait. It may return immediately once absence is observed, as Capybara Node::Base#synchronize does; it does not wait a fixed minimum or claim future absence.
- Release-click stays deferred. Its hit-test and 700 ms hold are unchanged. Duplicate delivery still injects Turbo markup in the browser; it does not verify server-originated message.broadcast_create.

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — known metadata/hit-test reliability failure; successful mapped attempts do not remove its deferral; the original assertion, 700 ms hold and deadlines remain unchanged, with no new declaration credit

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
