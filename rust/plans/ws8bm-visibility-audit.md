# PR #189 element-visibility assertion audit

Pinned originals: Rails d7c7de9264c63015be398001d7a1094e7695a6db.
All line numbers below refer to `test/system/<file>_test.rb` at that pin;
`helper` means `test/test_helpers/system_test_helper.rb` at the same pin.
This audits every assertion in the thirty reviewed declarations, plus the
reported forwarded-code declaration and the search-field/Markdown-link
assertions in the files audited with it. Navigation/status, persisted-row
checks, browser JavaScript observations and setup readiness are not Capybara
visible-element assertions. No screenshot assertions are credited.

The shared helper runs the unmodified pinned Selenium atom with
`ignoreOpacity=false`. Property assertions check visibility and the expected
property on the **same node in the same polling observation**, within one
original deadline. Ordinary fields use two seconds; the asynchronous draft
uses ten. Explicit `visible: false`/`:all` assertions keep DOM semantics.

## Changed assertions and one opacity probe for each

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

V1 also reproduces Astra's exact composer CSS. V11 reproduces its exact
`pre code.language-ruby { opacity: 0 !important; }` CSS; V12 hides only
forwarded code so the initial-source visibility assertion cannot reject it.
V2/V3/V5 modify the served composer restore implementation to make the field
transparent only after the corresponding restore. Conditional field/tab
probes disable the target's transition in the served mutant so opacity is zero
at the assertion, rather than fading through Selenium-visible positive
opacity. App transitions and assertion deadlines are unchanged. V4/V10 apply
only after Turbo sets the delivery flag. None replaces an oracle value or
write response. V16–V23 check the explicit Capybara element lookups, since
Playwright actionability also omits opacity.
Each lookup and its subsequent action share the original timeout budget.

## Follow-up audit: visibility on the asserted node

The follow-up sweep covers every ported `behavior*.mjs` assertion, including
supplementary Markdown, threads, composer, boost and search checks. Direct
selector assertions test their selected node with the pinned atom. A visible
child or ancestor cannot stand in for that node. Default-visible message text
uses the original `.message__body` wrapper; its existing exact presentation
text/count oracle is retained in the same observation. Two-second defaults,
ten-second explicit delivery/conversation waits and the twenty-second keyword
highlight wait come from the pinned originals. The URL-card broadcast wait
remains fifteen seconds. No timeout was widened.

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

E1/E2 are Astra's exact probes. E3/E5/E6/E14 distinguish an invisible message
body from a descendant whose visibility is explicitly restored. E4 hides only
the thread body while its row remains visible. E13/E17 hide the conversation
but restore its children's visibility; E17 applies only to the first creation,
so a later conversation visibility assertion cannot reject it for a different
state. E10 hides only the cleared-field state after a second message exists.
All seventeen probes escaped the unchanged 70d325c5 assertion/helper modules
on Rails and Rust before these corrections. The failing attachment attempt
and initial overly broad submitted-source probe are retained in the report,
with their valid isolated/corrected baseline evidence.

Unchanged scopes were checked against their original selector: the thirty's
keyword assertions select the `.code-token` itself (code_highlighting:91/98/
129/141), and the custom/brand picker assertions select the image itself
(message_toolbar:149/163). Neither is a proxy for an original parent assertion.
Direct row counts/selectors in message-list, standalone-message and unread
checks also retain their original row scope. Raw ids, browser focus/geometry,
clipboard, event-routing metadata, saved-row facts and additional diagnostic
observations are not default-visible selector assertions. Explicit hidden
contexts, More-button `visible: false`, and security `visible: :all` checks
remain unchanged. `assert_no_text` visibility/text filtering is distinct from
this selected-element sweep; the pre-pagination alpha is absent from the
initial fixture, and its original row absence check is retained.

## Every assertion in the thirty reviewed declarations

Shared setup: real page status/path and one composer DOM node are setup
invariants, not borrowed Capybara assertions; Stimulus/cable connection checks
are explicitly DOM/JavaScript readiness. Shared menu-open assertions map to
helper:133–134 (visible menu, then visible owning row). Menu dismissal maps to
default-visible `assert_no_selector`; the retained detached wait is stronger
than that negative assertion. Shared `text` checks now count the original atom-visible `.message__body`
wrappers (E3), together with the retained presentation text and delivery budget. Shared field checks
are V1–V6. All other selector/count waits below already use the atom.

| Exact reviewed declaration | Complete assertion inventory and pinned original lines |
| --- | --- |
| opens message actions from context menu and keyboard, and cancels a moving long press | Menu helper; eight visible reactions 19; closed rows 22/41; focus 36 (helper:31–41 is JavaScript, not visibility); viewport geometry 45 and private 409+ is JavaScript; visible Delete and row 48–49. Raw row id is only the focus-check selector, not a separate element assertion. |
| a release click landing on the just-opened menu does not activate it | Menu helper 60/82; hit target JavaScript equality 80; hidden context 83 explicitly `visible: false`, retained attached. Deferral remains; no extra credit. |
| shows the message action menu as a bottom sheet on phones | Menu helper 93/103; closed rows 97/107; private 359 waits for visible Edit, private 360–405 measures rectangles in JavaScript and asserts bounds/44px sizes; its `getClientRects` filter is the original's own geometry, not a Capybara visibility query. |
| edits through the normal composer and restores the saved draft on cancel and success | Visible edit label 121; V1/V2/V3 at 122/125/137; visible saved body 135; hidden context 136 explicitly `visible: false`. Extra recipient body uses corresponding delivery budget. |
| a duplicate delivery does not replace the message while its actions are open | Menu helper; visible Edit 147; V10 at 166; connected-node identity JavaScript 167; V4 at 169. Stream action/target attributes are event-routing metadata, not visible DOM assertions. Browser-injected Turbo only: does not verify server-originated redelivery. |
| keeps newer typing through an asynchronous edit and leaves failures in edit mode | Menu helper; visible edit label 178; V5 at 197; hidden context 198 explicitly `visible: false`; visible feedback/context 219–220. Fetch-resolver readiness is JavaScript and retains default deadline. |
| replies with notify off and renders a tombstone when the target is deleted | Menu helper; visible reply label 233; V7 at 234; visible preview/tombstone 239/246; persisted reply target/notify facts 240–242 checked in the database verifier. Extra author/peer delivery and actual target removal do not replace those facts. |
| copies message text and link and forwards to a server-provided thread destination | Menu helper; clipboard string/inclusion JavaScript 264/271; visible dialog 278 and V16 destination lookup 279; visible status 285; saved forward/thread projection 286 checked in database. Clipboard permalink is a string, not an element link-attribute assertion. |
| forwarding twice in a row submits only once | Menu helper; visible dialog 296; V17 checkbox lookup 298; forward count 301–305 checked in database; visible status 303; visible disabled submit selector 304 already scopes the following raw `isDisabled` read; hidden dialog 306. Network POST count is an additional non-DOM fact. |
| groups emoji reactions, updates the live count, and highlights the current user | Menu helper; V18 quick reaction lookup 318/331; visible counts 320/324/333/337/342/346; active/inactive states 321/325/334/338/343/347. Combined browser predicate already includes atom visibility of the chip, count and active state; preceding count assertion checks visible count text. Persisted boost ownership is additionally verified. |
| message action menu is a bottom sheet with touch-sized targets on phones | Menu helper 15; visible Edit 19; geometry existence/bounds/target sizes/rows 21–41 use private JavaScript 68–88 and original rectangle filters; V23 visible header lookup 44; closed row 45. |
| message action menu stays a floating popover on desktop | Menu helper 54; geometry existence/width 57–60 use same original JavaScript; closed row 63. |
| the toolbar stays hidden until hover or focus and labels every action | Zero visible toolbars 11; visible toolbar 15; visible named buttons 16–20. More's raw `aria-haspopup` is read on the exact button just checked visible, not an independently unguarded element; visibility wait plus immediate property equality preserves this scope. |
| quick-react creates a boost from the toolbar | V22 visible hover-body lookup helper:220; visible toolbar helper:221; visible delivered count and active chip 28–29; extra peer count uses same 10-second budget. |
| reply and thread buttons drive the composer and the thread panel | V22 visible hover-body lookup helper:220; visible toolbar helper:221; visible reply label 35 and thread-create pane 40. |
| the more button opens the shared menu for its message | Menu helper; V9 at 48; expanded button attribute 49 explicitly `visible: false`, retained raw read; closed row 52. |
| keyboard users reach the toolbar from a focused message | Active element aria-label is original immediate JavaScript 60–61, not a visible-selector assertion; delivered count 64 visible. Additional active chip/peer count visible. |
| the emoji picker searches and reacts | Visible panel/tab/options 71–73/78; active element label JavaScript 74–75; zero visible panel 81; visible count 82; database boost 83. |
| the picker shows category tabs and switches between them | Visible panel 89; 11 visible tabs 91; recent-tab attribute 92 is read on one of those same 11 atom-visible tabs; visible selected People/Waving hand 95–96; zero visible Grinning face 97; V20 Flags lookup 99; selected Flags/Chequered flag 100–101 visible. |
| the picker loads its emoji data only on first open | Resource request observations 105–120 are original JavaScript/non-DOM; Grinning face 116 visible. Additional reopened option is visible and resource count verifies cache. |
| the picker remembers recent reactions | Visible initial option/count/panel/selected recent tab/option 126/128/133/136–137; database boost 129. |
| the picker Custom tab reacts with a workspace icon | Visible panel/tab/image 145/148–149; zero visible panel 152; visible count 153; database boost 154. |
| the picker reacts with a brand icon shortcode | Visible panel/image 160/163; zero visible panel 166; visible count 167; database boost 168. |
| picker arrows move through options, Enter selects, and Escape returns focus | Visible panel/options 174–175/189; V19 visible search-field lookups 177/191; focus-label JavaScript 178/181/185; zero visible panel 184/194; visible count 195; database boost 196. |
| picker tabs move with arrow keys and switch the grid | Visible initial option 202; visible selected People/Waving hand/Smiley 207/209/212; active element id 208/213 is original JavaScript. |
| language fences highlight common code without changing its text | Visible count 32; visible highlighted code find 37, tokens 38 then immediate source-byte read 39 on that same guarded code; security DOM count 41 explicitly `visible: :all`; execution flag JavaScript 42; c#/c++ attributes 43–44 are on code already individually atom-checked in the sample loop; visible copied button 53; clipboard JavaScript 55. Additional recipient code marker/token visible. |
| unlabelled code is detected while text unknown languages and inline code stay literal | Visible token/text code 76–77; source read on same just-guarded code; zero visible literal/inline spans 78; visible unknown-language source 79 then immediate bytes on same guarded node; security DOM count 80 explicitly `visible: :all`; execution JavaScript 81; visible copy-button count 82. |
| search results highlight code on initial load and after returning to the channel | Visible const tokens/copy counts 91–92/98–99; V13 link query 95; URL facts 96/103 not element assertions; visible copied button 107; clipboard JavaScript 109. |
| code and copying remain available when the highlighter cannot load | Visible literal source 116 then immediate bytes from same guarded code; visible copied button 119; clipboard/worker flags JavaScript 120–121; zero visible highlighted code/tokens 122; V6 at 123. |
| editing a code block replaces its language colors and copied source | Visible const/def keywords 129/141; menu helper; zero visible old language 142; visible copy count/result 143/146; clipboard JavaScript 148; exact saved source 149 checked in database. Message-id read is metadata for scoping replacement/recipient checks; visible keyword on its descendant already guards the row. |

The supplementary forwarded case also retains visible table (line 59), dialog,
status and forwarding destination checks, and checks saved forward rows. The
supplementary Markdown content checks retain visible text, table, checkboxes,
const token and copy count (lines 325–341); E1 guards the code element
itself before the byte read, and E2 guards the initial heading itself. V14 now checks the link and href together. The paging
search's additional final `inputValue` observation has no original Rails
counterpart and is kept as an extra query-preservation fact; V15 covers the
original visibility-scoped field assertion at line 26. IDs/deduplication,
HTTP status and URL observations are non-visible metadata checks.
