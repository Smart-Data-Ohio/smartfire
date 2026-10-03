# WS8bm -2: continuation PR and phase-attribution fixes

Branch `rust/ws8bm-messages-http-2`. Main `2fd002490865e8f5ec04466cd4ea526e8eeb8f08` (#189) merged cleanly in `45ea50271` with a merge commit; no stash or test weakening. Implementation checkpoint `fd97891af` is verified in a cold, no-hardlinks clone under the authorized `.scratch/ws8bm-pr2-review/fresh`. That clone started at the merge with no target or seed; the generated target is removed after verification. Rails pin remains `d7c7de9264c63015be398001d7a1094e7695a6db` plus approved drift.

## What the PR adds beyond main

- Eight paired controller flows attribute the ten formerly WS12-blocked thread/work declarations: authenticated/unauthenticated reads, permissions, ownership, work status, validation, stale updates and preservation of the discussion. Direct Rails model differentials and two Rust database regressions cover omitted-owner untracking and distinct stale-instance changes/history.
- Sixteen additional system declarations cover work tracking/assignment/activity/guidance, phone thread navigation, anchored/read/unread thread history, live mobile chrome/overflow/navigation, coarse-input field sizing, and narrow/theme-aware thread code. These use original visibility predicates and deadlines, real browser writes and persisted-row checks.
- Held-job upload/provider fixtures use Rails' nonperforming TestAdapter and Rust's `TestApp::without_job_runner()`. The generated cold browser-host copy includes its release/public/provider callback inputs; it needs no pre-existing scratch files or target. Main's newly included agents-UI cast/replay input files are also copied; the cold build caught their omission and a failing-first copier regression covers all four. List/composer shell seams remain documented in `ws8bm-integration.md`.
- Initial-phase rejection attribution and pinned native phone execution close this review's four P3s. Reports retain the seventeen concrete deferrals, distinguish injected browser Turbo duplicate delivery from server-originated redelivery, and keep the release-click case deferred.

The new P3 changes touch reference tools only. The continuation's Rust delta consists of test support and database regression tests; no Rust product behavior, Rails JavaScript, message goldens, masks or deadline changes are included.

## Failing-first attribution and phone audit

The reviewer's actual asset-installer/synthetic-stack reproduction was rerun against the merged baseline. All seven stacks got credit, including four wrong-phase stacks. The committed regression tests fail against that baseline: **3 pass / 4 fail**. These are controlled stack inputs at real source call sites, not a claim of browser failures. The fixed tests retain those four wrong-phase stacks and add a distinct-message state-witness rejection.

| Finding | Before | After | Pinned assertion/phase |
| --- | --- | --- | --- |
| onContextMenu no-op | Shared menu wait accepted later Shift+F10 and long-press failures. | Only the initial `await openMenu(page)` call can earn credit; the unaffected handlers fail closed. | `message_interactions_test.rb:18` initial right-click; later keyboard/press checkpoints stay separate. |
| First bold message hidden | Shared text wait accepted failure on the newer plain draft, using an earlier bold body's hidden-state witness. | Target the first submitted-text call and require the hidden witness's normalized text to be `First message stays exact.`; the later draft or a different seeded body cannot supply credit. | `workspace_markdown_test.rb:232,237` first send versus second draft/send. |
| Initial conversation hidden once | Shared conversation locator accepted reopened-conversation failures after the sessionStorage one-shot guard stopped acting. | A named creation-only call wraps the same conversation assertion and original ten-second deadline; a real inline-hidden conversation witness is required. Reopen assertions stay separate. | `composer_test.rb:255,269,281` initial creation versus later room navigation/reopens. |
| Phone Escape focus race | Translated original phone control: 1 paired pass / 2 failed pairs, with Rust failing the two-second post-Escape panel assertion. Reviewer independently traced the same behavior on both apps. | Positive parity runs the pinned body and private/system helpers through pinned Capybara 3.40.0 / Selenium 4.35.0. Only the excluded screenshot is removed. Three consecutive fresh-fixture pairs pass, 25 assertions per app/pair; recorded Escape targets are inside the menu. No diagnostic focus wait earns credit. | `threads_test.rb:293-340`, especially menu assertions :317, Escape :328, panel :329; `system_test_helper.rb:128-135`. |

The native helper extracts both Ruby source files from the exact git pin at runtime. A regression reconstructs the original body byte-for-byte after restoring only its method header and screenshot, and verifies the exact private helper text. It refuses to reuse an occupied ChromeDriver port, owns and removes a short cache directory for Chrome sockets, and uses the same session, room and production-host test-motion input as the paired runner. Served negative phone controls still execute their translated initial creation checkpoint; they do not get native positive credit.

## Possible shared Rails client bug

`message_actions_controller.js:643` schedules menu focus on requestAnimationFrame. `thread_panel_controller.js:103` exempts Escape only if the event starts inside the menu/dialog/details. If Escape starts on the message before the focus frame arrives, it closes the drawer as well as the menu on **both** apps. Both client files exactly match the pinned Rails sources; no client fix is attempted here.

Reproduction: at a 390 × 844 phone viewport with pinned `data-test-motion=off`, open the thread drawer, create the second thread, right-click its first message, and deliver Escape immediately while the message still owns focus (`menuContainsActive=false`). Observe `#thread-panel[aria-hidden=true]`; the original post-Escape assertion expects false within two seconds. Delivering Escape after menu focus leaves the drawer open. Review evidence, read only: `/home/riels/.cache/rust-port/ws8bmbr/smartfire/.scratch/rereview-ad0fb6aa/phone-summary.json` and `phone-playwright-motion.log`. Own baseline and canonical-native logs are retained in `.scratch/ws8bm-pr2-review/phone-before.log` and `phone-after.log`. The native traces show menu focus preceding Escape on each of the six passing application runs. This is a possible Rails client bug for the lead, not a Rust-only defect or a new credited assertion.

## Commands and raw receipts

Pending final verification receipts.
