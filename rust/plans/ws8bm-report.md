# WS8bm PR #189 asserted-element visibility correction — partial port

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus approved drift in wave4/_common.md. Behaviour and response parity only; no pixel checks.

Fixed Astra's two supplementary Markdown gaps by checking the asserted `pre code` and initial `h2` themselves. The sweep of every ported assertion also corrected message bodies, thread bodies/conversations, reply previews, fields and text elements that previously borrowed visibility from another node. The unchanged pinned Selenium 4.35.0 atom supplies every visibility check, including effective ancestor opacity. Shared message text now checks the original `.message__body` wrapper while retaining its existing exact presentation text/count oracle in the same observation. No Rust product, Rails source, goldens, masks, network-error exceptions or concurrency settings changed. No wait was widened; repaired assertions use their original two-/ten-/twenty-second budgets, and URL cards still use fifteen seconds.

The received inventory remains **102 passed / 27 deferred / 6 WS12-blocked** out of 135 exact system declarations, and **146/156 controller declarations**, with ten WS12 deferrals. This correction earns no declaration credit. The release-click and attachment flows remain deferred; full system sign-off remains partial.

**Duplicate-delivery limitation:** its check injects Turbo markup in the browser on the mounted row/render queue and verifies the browser guard/object identity. It **does not verify server-originated redelivery**; the original Rails case calls `message.broadcast_create`.

## Audit table

Each row records the corrected assertion, its pinned original Rails line and its own served probe. `helper` means `test/test_helpers/system_test_helper.rb`; other filenames mean `test/system/<name>_test.rb`. The [complete audit](ws8bm-visibility-audit.md) retains the earlier V1–V23 findings and the inventory of every assertion in the thirty reviewed checks, plus the follow-up sweep. Explicit hidden contexts, More's `visible: false`, security `visible: :all`, original JavaScript observations and selectors that correctly assert the child itself remain unchanged.

| ID | Assertion corrected | Pinned Rails line and visibility requirement | Served probe |
| --- | --- | --- | --- |
| E1 | Markdown code source: check `pre code` itself before reading bytes | workspace_markdown:334; the separate visible keyword assertion at 340 does not prove the code element visible | hidden-code-visible-token |
| E2 | Initial Design review heading on author and peer: check `h2` itself | workspace_markdown:55,326 (called at 57/59); a visible message row does not prove its heading visible | transparent-initial-heading |
| E3 | Shared message-text count: select visible `.message__body`, retain presentation text/count oracle | helper:114–115; a descendant with overridden visibility cannot substitute for the selected wrapper | hidden-body-visible-presentation |
| E4 | Thread delivery and initial-message checks: select `.thread-panel__thread-content .message__body` | threads:577 (calls at 19/37/59/114); a visible row does not prove its body visible | transparent-thread-body |
| E5 | Safety check delivery: check its `.message__body` | workspace_markdown:134 → helper:115; visible code is a separate assertion at 137 | hidden-safety-body-visible-code |
| E6 | Markdown reply source: check its `.message__body` | workspace_markdown:145 → helper:115; a visible message row does not prove its body visible | hidden-reply-body-visible-presentation |
| E7 | Delivered attachment reply: check `.message__reply-preview` itself | workspace_markdown:165, wait 10; the containing row and download link are separate nodes | transparent-attachment-reply-preview |
| E8 | Combobox semantics: visibly locate the editor before reading its attributes | composer:74–99; `find_field` is visible by default; the popup does not prove its editor visible | transparent-combobox-lookup |
| E9 | Restored thread draft: visible field and value together | composer:271, default-visible `assert_field` | transparent-restored-thread-draft |
| E10 | Cleared thread draft after send: visible field and value together | composer:283, default-visible `assert_field`; earlier restored-field visibility cannot guard this later state | transparent-cleared-thread-draft |
| E11 | Boost draft after message replacement: visible field and value together | boosting_messages:56 → private helper:92, default-visible `has_field?` | transparent-boost-draft-after-edit |
| E12 | Boost draft after another user's boost: visible field and value together | boosting_messages:81 → private helper:92, default-visible `has_field?` | transparent-boost-draft-after-delivery |
| E13 | Thread creation: check conversation itself, then its title, each at original wait 10 | threads:572–573; a child may override the hidden conversation's visibility | hidden-conversation-visible-children |
| E14 | Submitted Markdown delivery: check body text, retaining exact rendered text | workspace_markdown:232 → helper:115; the visible `strong` child alone is insufficient | hidden-submitted-body-visible-strong |
| E15 | Older search text: check the actual text element | search_forward_edit:35, visible `assert_text`, wait 10; message-row visibility does not show a transparent paragraph | transparent-older-search-text |
| E16 | Initial URL-edit text: check the actual text element | search_forward_edit:69, visible `assert_text`, wait 10; message-row visibility does not show a transparent paragraph | transparent-initial-url-text |
| E17 | Draft-thread creation: check conversation itself before its title | composer:255, visible conversation assertion, wait 10 | hidden-initial-composer-conversation |

## Failing-first evidence at 70d325c5

Before changing assertions, verified fourteen assertion/helper modules byte-for-byte against `70d325c598162b1568bb77295d4b80e4475e936f`. Only the served-mutant registry/runner changed in scaffolding commits `cf970aee3`, `f00aa855` and `acee33b6`. Assertion corrections are in `8b0c5e628dff8e67e79ac36da658fc9dcd3d085d`. A preserved baseline clone at `.scratch/ws8bm-element-scope-baseline` contains the final scaffolding with unchanged 70d325c5 assertion source.

The initial thirteen-probe batch accepted twelve probes on both apps. Its attachment-source probe failed later on Rust's missing attachment reply preview (already deferred); the isolated repeat accepted that same served probe on both apps. The first submitted-source probe was too broad and hid the later plain message as well; it was not counted. Scoped it to `.markdown-body strong` in `acee33b6` and repeated the unchanged baseline successfully. The four final added probes (submitted body, older search text, initial URL text and initial composer conversation) also escaped on both apps. Every accepted diagnostic requires valid startup, an applied served mutation and no actual network failures. No diagnostic earns parity credit.

Baseline harness invocations (logs in `.scratch/ws8bm-element-scope-review/`):

```sh
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown sending_messages threads composer boosting_messages --mutant-set element-scopes --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'Markdown replies and file attachments remain usable' --mutant hidden-reply-body-visible-presentation --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown --case 'sending preserves the submitted source and a newer draft' --mutant hidden-submitted-body-visible-strong --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'search tolerates operators, shows an empty state and pages older results' --mutant transparent-older-search-text --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --mutant transparent-initial-url-text --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer --case 'thread drafts persist per thread without touching the channel draft' --mutant hidden-initial-composer-conversation --keep-going
```

The first command ran the initial thirteen-probe registry; the final registry now contains seventeen. The submitted-source command ran before and after narrowing the served probe. Raw summary lines (`before-scopes.log`, `before-reply-body-isolated.log`, `before-submitted-body.log`, `before-submitted-body-corrected.log`, `before-search-text.log`, `before-url-text.log`, `before-composer-conversation.log`, `before-proof.log`):

```text
WS8bm review escape check: 12 served mutants accepted on Rails and Rust; 1 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 0 served mutants accepted on Rails and Rust; 1 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm element-scope failing-first: 17 served probes ACCEPTED on Rails and Rust at 70d325c5; 14 assertion/helper modules byte-identical; 0 missing escape proofs
```

The final registry has one new served probe per E1–E17 row. Its negative set also runs every existing registered variant for those twelve named checks; absent default mutations are not invented. The startup/applied-mutation/network/error-kind rejection gate is unchanged.

## URL-card isolation and inherited failures

Repeated the unchanged URL-card case three times in isolation before changing assertions. The first two runs passed both apps; the third timed out on **Rails** at the unchanged fifteen-second Loading post assertion, before Rust ran. This reproduces reference-side intermittency; no wait was widened. Logs: `url-isolated-1.log`, `url-isolated-2.log`, `url-isolated-3.log`.

```sh
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --case 'editing to add a URL renders its card live and the edited marker on load' --keep-going
```

Raw summaries, in run order:

```text
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm behaviour check: 1 named cases passed on Rails and Rust; 0 failed; no pixel checks
WS8bm failed application: http://127.0.0.1:52020 editing to add a URL renders its card live and the edited marker on load
WS8bm browser flow FAILED: search_forward_edit: editing to add a URL renders its card live and the edited marker on load: TimeoutError: Selenium visibility visible timed out after 15000ms: locator('.message[data-message-id="935962058"] .x-post-card').filter({ hasText: 'Loading post' }); last visible count: 0
WS8bm behaviour check: 0 named cases passed on Rails and Rust; 1 failed; no pixel checks
```

The current fresh thirty-check run also reproduced the already-deferred release-click reference failure, `DIV` versus `menu` at behavior-actions.mjs:92, before Rust ran. Its original assertion and 700 ms hold remain unchanged. The supplementary run reproduced the already-deferred Rust attachment-preview gap at the original ten-second preview wait; the old 70d325c5 baseline also missed that preview after thirty seconds. No product fix, retry rule, timing relaxation or declaration credit was made for these failures. The GitHub live-worker declaration remains deferred and was explicitly excluded from the supplementary batch; its earlier diagnostics are retained separately in the inventory.

## Fresh-source verification

Fresh clone: `.scratch/ws8bm-element-scope-fresh`, source `8b0c5e628dff8e67e79ac36da658fc9dcd3d085d`, with no target, seeds or node_modules initially. The harness generated default/first_run seeds, built its own binary, installed declared Node/browser inputs, verified the pinned Rails image and exact atom, and exercised real HTTP/Cable with persisted-row checks. The configured rustc throttle and existing `-j2` are unchanged. Rust workspace tests/clippy were not rerun for this reference-tools-only request, per the earlier explicit waiver; the harness's fresh binary build is reported below. The clone was then advanced to metadata-only `c215815f8`; the reference-tool files remain byte-identical to the tested 8b0c5e62 source. Reran the strict inventory verifier there. Inventory/report changes after the tested source are metadata only.

These commands ran in the fresh clone with `CARGO_TARGET_DIR="$PWD/rust/target"`:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --keep-going
node --test rust/reference-tools/messaging/behavior-visibility.test.mjs
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown sending_messages threads composer boosting_messages search_forward_edit --exclude-case 'discusses a pull request from its card' --keep-going
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py search_forward_edit --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py workspace_markdown sending_messages threads composer boosting_messages search_forward_edit --mutant-set element-scopes --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py composer boosting_messages search_forward_edit --mutant-set element-scopes --negative --keep-going
python3 rust/reference-tools/messaging/behavior-check.py code_highlighting --case 'search results highlight code on initial load and after returning to the channel' --negative --mutant default --keep-going
python3 rust/reference-tools/messaging/behavior-check.py code_highlighting --case 'code and copying remain available when the highlighter cannot load' --negative --mutant default --keep-going
```

Raw summary lines (`fresh-reviewed-paired.log`, `fresh-helper-tests.log`, `fresh-supplementary-paired.log`, `fresh-reviewed-mutants-rerun.log`, `fresh-search-mutants.log`, `fresh-scope-tail-mutants.log`, the two `fresh-code-*-default-recheck.log` files and `final-proof.log`):

```text
seed: default -> parity/.seed/default (6.1M)
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5m 23s
WS8bm behaviour check: 29 named cases passed on Rails and Rust; 1 failed; no pixel checks
WS8bm behaviour check: 32 named cases passed on Rails and Rust; 2 failed; no pixel checks
ℹ tests 4
ℹ suites 0
ℹ pass 4
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 2278.146691
WS8bm discrimination check: 54 served mutants rejected on Rails and Rust across 30 named checks; 2 invalid or escaped
WS8bm discrimination check: 8 served mutants rejected on Rails and Rust across 3 named checks; 0 invalid or escaped
WS8bm discrimination check: 15 served mutants rejected on Rails and Rust across 6 named checks; 0 invalid or escaped
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
WS8bm discrimination check: 1 served mutants rejected on Rails and Rust across 1 named checks; 0 invalid or escaped
WS8bm asserted-element proof: 17 served probes accepted at 70d325c5 and rejected after the fix on Rails and Rust; 0 missing proofs
WS8bm full discrimination evidence: 88 expected unique served variants rejected on Rails and Rust across 43 named checks; 0 unresolved; 2 invalid network attempts and 2 startup-aborted runs retained
WS8bm reviewed discrimination evidence: 56/56 expected variants rejected on Rails and Rust across 30 named checks; 0 unresolved
WS8bm scope discrimination evidence: 29/29 expected variants rejected on Rails and Rust across 12 named checks; 0 unresolved
```

All thirty requested positives were attempted: twenty-nine passed, and release-click failed on Rails before Rust. All thirty-four supplementary positives were attempted: thirty-two passed, with the deferred Rust attachment preview and intermittent Rails URL-card failures retained. No positive failure is hidden by the negative counts, and mutation rejections alone do not close a deferred positive flow.

The first reviewed negative run aborted after thirteen paired variants, and the first scope run after fourteen, when the Rails reference missed its unchanged sixty-second startup deadline. Neither abort produced a successful gate summary. Retained them in `fresh-reviewed-mutants.log`/`fresh-scope-mutants.log` and the owned server log. Repeated the reviewed set with the same configuration and completed its fifty-six variants, except two code defaults whose browser setup reported `ERR_NETWORK_CHANGED` and `ready: false`. The gate correctly rejected those attempts as invalid, not as mutant rejections. Repeated only those two defaults in isolation and obtained one valid paired rejection each, with zero invalid or escaped. The scope continuation ran its fifteen remaining variants; fourteen preserved prefix proofs plus fifteen continuation proofs cover all twenty-nine. The search set's eight variants completed in one invocation. No automatic retry, network allowlist change, test exclusion, readiness extension or resource override was introduced.

```text
reference: instance on port 52020 didn't come up within 60s
reference: instance on port 52020 didn't come up within 60s
WS8bm browser flow FAILED: code_highlighting: search results highlight code on initial load and after returning to the channel: AssertionError [ERR_ASSERTION]: mutant must reach the actual named case, not fail startup
WS8bm browser flow FAILED: code_highlighting: code and copying remain available when the highlighter cannot load: AssertionError [ERR_ASSERTION]: mutant must reach the actual named case, not fail startup
```

Reconciled exact file/case/variant/application rejection markers against the registry, retaining duplicate attempts without double counting. The full requested/affected set has eighty-eight distinct variants across forty-three named checks: fifty-six reviewed, eight search, and twenty-nine scope, with five shared search variants counted once. Each E1–E17 pair has unchanged-70d325c5 acceptance proof and corrected-source rejection proof on **both** apps. The four helper tests also demonstrate that neither a visible token/child nor a visible row can substitute for a hidden code/body/heading node.

E15/E16's original visible-text semantics were additionally verified using the pinned Capybara/Selenium gems with a local diagnostic ChromeDriver: an opacity-zero paragraph yields no visible page text and neither text nor visible-selector match. This is a semantic diagnostic, not a declaration pass:

```text
WS8bm pinned Capybara text scope: text=""; assert_text=false; visible paragraph=false
```


## Attribution and cleanup

The unchanged inventory verifier initially rejected two existing deferred entries with nonempty pass-evidence fields. In `d024dae1`, moved their uncredited histories to `diagnostic` and left pass evidence empty; statuses/names/source hashes and the verifier's assertions are unchanged.

```sh
python3 rust/reference-tools/messaging/deferred-system-inventory.py
```

```text
WS8bm system inventory: 135 named declarations; 102 mapped behaviour passes; 27 deferred; 6 WS12 blocked; no pixel checks
```

Removed only the owned fresh clone's 4.4G `rust/target` after verifying no process used it, all three assigned ports were free and no WS8bm-owned container remained. Preserved baseline/source clones, generated browser inputs and raw logs. No scratch Cargo target remains.

```text
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

The requested asserted-element corrections are complete; stopping here as requested. The whole WS8bm port remains partial, with exact remaining declarations below.

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — known reference failures; this fresh run again fails on Rails (`DIV` versus `menu`) before Rust; original assertion/deadlines and the reliability deferral remain, with no new declaration credit

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
