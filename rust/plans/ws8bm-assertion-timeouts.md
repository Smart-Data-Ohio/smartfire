# PR #189 assertion timeout audit

The audit covers all 30 checks credited by 6427c785, compared with the four
original system files at Rails d7c7de9264c63015be398001d7a1094e7695a6db.
The pinned application's installed Capybara reports `default_max_wait_time = 2`;
`test/test_helpers/system_test_helper.rb` delegates `assert_focused` to that
value. Its menu-open helper allows 10 seconds and cable setup allows 15 seconds.
`ApplicationSystemTestCase::HIGHLIGHT_WAIT` is 20 seconds. Long press holds for
700 ms and cancellation moves 25 px; neither value changes.

The four groups set Playwright's action/assertion default to 2 seconds before
running their assertions, identically in positive and mutant runs. Explicit
waits below use the original budget. Plain Ruby `assert_equal`/JavaScript
observations remain immediate. Page navigation has a separate navigation
budget, not an assertion deadline. Startup requires the actual connected cable
source within its original 15-second helper budget. Additional peer checks use
the corresponding original delivery/highlight budget; they do not defer the
author's active-reaction assertion. Geometry settling is bounded by the 2-second
default, never an unbounded animation promise.

| Pinned file / exact declaration | Explicit waits; all other assertions use 2 seconds or are immediate |
| --- | --- |
| message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press | Menu 10 s; eight **visible** reactions 2 s; focus return 2 s; hold 700 ms. |
| message_interactions: a release click landing on the just-opened menu does not activate it | Menu 10 s; hidden context 2 s. |
| message_interactions: shows the message action menu as a bottom sheet on phones | Menu and metadata-dependent Edit action 10 s; geometry immediate after bounded settling. |
| message_interactions: edits through the normal composer and restores the saved draft on cancel and success | Menu 10 s; first Editing Message label 10 s; saved body 10 s; draft and hidden context 2 s. |
| message_interactions: a duplicate delivery does not replace the message while its actions are open | Menu, Edit availability and rendered flag 10 s; identity immediate; editor 2 s. Browser-injected Turbo markup only; no server redelivery claim. |
| message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode | Menu and first edit label 10 s; newer draft and failure feedback 10 s; fetch-gate readiness/context visibility 2 s. |
| message_interactions: replies with notify off and renders a tombstone when the target is deleted | Menu 10 s; reply context, preview and tombstone 10 s; extra real-write body delivery 10 s; target removal 2 s. |
| message_interactions: copies message text and link and forwards to a server-provided thread destination | Menu, dialog, destination lookup and forward status 10 s; clipboard immediate; fields/buttons 2 s. |
| message_interactions: forwarding twice in a row submits only once | Menu, dialog, destination checkbox, forward status and dialog dismissal 10 s; disabled submit 2 s; request count immediate. |
| message_interactions: groups emoji reactions, updates the live count, and highlights the current user | Menu and each delivered count 10 s; each active/inactive state 2 s. |
| message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones | Menu and Edit metadata 10 s; dismissal 2 s. |
| message_actions_mobile: message action menu stays a floating popover on desktop | Menu 10 s; geometry immediate after bounded settling; dismissal 2 s. |
| message_toolbar: the toolbar stays hidden until hover or focus and labels every action | Hovered toolbar 10 s; hidden/visible buttons 2 s. |
| message_toolbar: quick-react creates a boost from the toolbar | Toolbar and count 10 s; active class 2 s. |
| message_toolbar: reply and thread buttons drive the composer and the thread panel | Toolbar, reply context and thread-create pane 10 s. |
| message_toolbar: the more button opens the shared menu for its message | Toolbar and menu 10 s; open/expanded state and dismissal 2 s. |
| message_toolbar: keyboard users reach the toolbar from a focused message | Focused button immediate; delivered count 10 s; extra active class 2 s. |
| message_toolbar: the emoji picker searches and reacts | Toolbar, picker, Grinning face, Fire and boost count 10 s; selected tab/picker dismissal 2 s; focus immediate. |
| message_toolbar: the picker shows category tabs and switches between them | Toolbar, picker and Chequered flag 10 s; other tab/option assertions 2 s. |
| message_toolbar: the picker loads its emoji data only on first open | Toolbar and Grinning face 10 s; resource observations immediate; extra reopen uses same option budget. No extra picker prewait. |
| message_toolbar: the picker remembers recent reactions | Toolbar, initial Grinning face, boost count and second picker 10 s; recent tab/option 2 s. |
| message_toolbar: the picker Custom tab reacts with a workspace icon | Toolbar, picker, Acme image and boost count 10 s; selected tab/dismissal 2 s. |
| message_toolbar: the picker reacts with a brand icon shortcode | Toolbar, picker, OpenAI image and boost count 10 s; dismissal 2 s. |
| message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus | Toolbar, first picker, Grinning face and boost count 10 s; dismissal 2 s; focus immediate. Reopen has no extra picker prewait. |
| message_toolbar: picker tabs move with arrow keys and switch the grid | Toolbar and Grinning face 10 s; selected tabs/Waving hand 2 s; focused tab immediate. No extra picker prewait. |
| code_highlighting: language fences highlight common code without changing its text | Initial count 2 s; highlighted marker 20 s; generic token 2 s; copy result 2 s. |
| code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal | Unlabelled token and literal text marker 20 s; remaining selectors/copy count 2 s. |
| code_highlighting: search results highlight code on initial load and after returning to the channel | Highlighted **const** token in a single 20 s wait at each location; path and copy result 2 s. |
| code_highlighting: code and copying remain available when the highlighter cannot load | Literal source, copy result, absent highlight and cleared field 2 s. |
| code_highlighting: editing a code block replaces its language colors and copied source | Highlighted **const** before edit and **def** after edit, each one 20 s wait; menu 10 s; source/copy/old language 2 s. |

The served review mutants remove only const/def token classes, hide only the
Clapping quick reaction, or defer the actual boost POST for 11 seconds before
forwarding it. The original 30 mutants remain. Discrimination runs require
valid startup, an applied mutation, no actual network failures, and an assertion
failure on **both** applications. The diagnostic `--mutant` without `--negative`
records escape probes and cannot earn parity credit.
