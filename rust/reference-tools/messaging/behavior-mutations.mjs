// Deliberately broken served implementations, never replacement oracle values.
// Each named check must reach its case assertions and reject its specific mutant.
import assert from 'node:assert/strict';
import {workControllerCases} from './behavior-work-controllers.mjs';
import {mutationTarget} from './behavior-discrimination.mjs';
import {WORKSPACE_CASE} from './behavior-workspace.mjs';
import {motionCases} from './behavior-motion.mjs';
import {actionMutations} from './behavior-action-mutations.mjs';
const list='controllers/message_list_controller-';
const actions='controllers/message_actions_controller-';
const composer='controllers/composer_controller-';
const live='helpers/live_region_helpers-';
const mutations=new Map([
  ...actionMutations,
  ['From Google Drive starts the enhanced share flow when sharing is configured',['messages-','.message__quick-reaction {','.drive-share-dialog__file { opacity: 0 !important; }\n.message__quick-reaction {']],
  [WORKSPACE_CASE,['messages-','.message__quick-reaction {','#sidebar .sidebar__tools { margin-left: 20px !important; }\n.message__quick-reaction {']],
  [motionCases[0],['messages-','.message__quick-reaction {','#sidebar .sidebar__container { transition-duration: 0s !important; }\n.message__quick-reaction {']],
  [motionCases[1],['messages-','.message__quick-reaction {','#channel-members .is-selecting .member-panel__avatar { transform: translateX(10px) !important; }\n.message__quick-reaction {']],
  [motionCases[2],['messages-','.message__quick-reaction {','#main-content:has(.multi-select-bar:not([hidden])) .people-directory__row { transform: translateY(10px) !important; }\n.message__quick-reaction {']],
  [motionCases[3],['messages-','.message__quick-reaction {','.people-directory .multi-select-bar { position: static !important; }\n.message__quick-reaction {']],
  [motionCases[4],['messages-','.message__quick-reaction {','#room-menu:not([hidden]) { left: calc(100vw - 10px) !important; }\n.message__quick-reaction {']],
  [motionCases[5],['messages-','.message__quick-reaction {','@media (max-width: 63.999rem) { #sidebar:not(.open) .sidebar__scroll { display: none !important; } }\n.message__quick-reaction {']],
  [motionCases[6],['controllers/workspace_navigation_controller-','currentRoom.focus()','focusableFault(this.sidebarTarget); function focusableFault(sidebar) { sidebar.querySelector("a[href],button:not([disabled])")?.focus() }']],
  [motionCases[7],['controllers/workspace_navigation_controller-','target?.focus({ preventScroll: true })','(allowReveal ? target : focusable[0])?.focus({ preventScroll: true })']],
  ['thread code stays readable in both themes and scrolls within a narrow screen',['models/code_highlighter-','span.className = "code-token"','span.className = "missing-code-token"']],
  ...workControllerCases.map(name=>[name,['work-controller-json-response']]),
  ['the profile page fits phone widths without scrolling sideways',['messages-','.message__quick-reaction {','#main-content fieldset { min-width: 1000px !important; }\n.message__quick-reaction {']],
  ['headers outside the workspace shell stay opaque over scrolled content',['messages-','.message__quick-reaction {','#nav { background: transparent !important; }\n.message__quick-reaction {']],
  ['headers outside the workspace shell never cover the page or its scrollbar',['messages-','.message__quick-reaction {','#nav { position: absolute !important; height: 200px !important; }\n.message__quick-reaction {']],
  ['pages outside the workspace shell show no drawer toggle that opens nothing',['workspace-toggle-response']],
  ['every drawer destination has one toggle that opens the drawer on itself',['messages-','.message__quick-reaction {','#nav button[aria-label="Open workspace navigation"] { opacity: 0 !important; }\n.message__quick-reaction {']],
  ['text fields stay at 16px on touch devices without changing the desktop look',['messages-','.message__quick-reaction {','@media (pointer: coarse) { .board-post__form input[name="thread[tags]"] { font-size: 10px !important; } }\n.message__quick-reaction {']],
  ...[
    ['tracks work, assigns an owner, completes and reopens it without losing the conversation','[data-thread-panel-target="workStatusLabel"]'],
    ['shows work-thread guidance in the new-thread form and on the work page','details.thread-panel__guide summary'],
    ['keeps the new-thread guidance usable on a phone','details.thread-panel__guide summary'],
    ['shows work assignment activity to the owner and opens the exact thread','[data-thread-panel-target="workOwnerLabel"]'],
    ['keeps the thread drawer usable on a phone and preserves the channel','[data-thread-panel-target="conversationTitle"]'],
    ['opens a shared thread message link around an older post','.message-area__return-to-latest'],
  ].map(([name,selector])=>[name,['controllers/thread_panel_controller-','connect() {',`connect() { const style = document.createElement("style"); style.textContent = '${selector} { opacity: 0 !important; }'; document.head.append(style);`]]),
  ['marks a joined thread read only while the conversation is visible',['controllers/thread_panel_controller-','#markReadIfJoined(thread, { requireVisible = false, requireLatest = false } = {}) {','#markReadIfJoined(thread, { requireVisible = false, requireLatest = false } = {}) { return;']],
  ['keeps an anchored older thread unread when a new reply arrives',['controllers/thread_panel_controller-','if (requireLatest && this.contentTarget.dataset.threadContentAtLatest !== "true") return','if (false) return']],
  ['Markdown messages reach other users and editing preserves the original source',['models/code_highlighter-','code.dataset.highlighted = "yes"','code.dataset.highlighted = "no"']],
  ['the message list is a single tab stop with a roving tabindex',[list,'index === messages.length - 1 ? 0 : -1','index >= 0 ? 0 : -1']],
  ['arrow keys move between messages',[list,'next.focus()','message.focus()']],
  ...[
    'a stream replacing the focused message keeps focus and the tab stop on its replacement',
    'a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement',
    'a direct DOM swap of the focused message keeps focus and the tab stop on its replacement',
  ].map(name=>[name,[list,'replacement.tabIndex = 0','replacement.tabIndex = -1']]),
  ['deleting the focused message moves focus to the surviving tab stop',[list,'this.#tabbable.focus({ preventScroll: true })','document.body.focus()']],
  ['deleting an older focused message hands focus to its neighbour, not the newest',[list,'if (tabbableRemoved && !replaced && tabbableNeighbour?.isConnected','if (false && !replaced && tabbableNeighbour?.isConnected']],
  ["a focus move during a stream render survives Turbo's focus restore",['initializers/stream_focus-','move.target.focus()','document.getElementById(beforeId)?.focus()']],
  ['a no-change room refresh does not yank focus back to the composer',['refresh-content-type']],
  ['the ContextMenu key opens the shared menu and Escape returns focus',[list,'event.key === "ContextMenu"','event.key === "NeverContextMenu"']],
  ['a late composer autofocus does not steal focus from a message',[composer,'if (!this.#focusIsOnMessageOrField) this.markdownTarget?.focus()','if (true) this.markdownTarget?.focus()']],
  ['up arrow from an empty composer still edits my last message',[actions,'this.onEditLast = this.#onEditLast.bind(this)','this.onEditLast = () => {}']],
  ['up-arrow-to-edit shows an error when the actions endpoint fails',[actions,'this.#flashError("Message actions are temporarily unavailable")','void("Message actions are temporarily unavailable")']],
  ['forward reuses the menu-open metadata request instead of fetching again',[actions,'if (this.#metadataPromise && this.#metadataMessage === this.#message)','if (false && this.#metadataMessage === this.#message)']],
  ['a menu opened while an action waits does not redirect the pending action',[actions,'async forward(event) {\n    event.preventDefault()\n    const message = this.#message\n    await this.#ensureMetadata()\n    if (this.#message !== message || !message?.isConnected) return','async forward(event) {\n    event.preventDefault()\n    const message = this.#message\n    await this.#ensureMetadata()\n    if (!message?.isConnected) return']],
  ['the menu closes before Turbo caches the page',[actions,'this.onBeforeCache = this.#onBeforeCache.bind(this)','this.onBeforeCache = () => {}']],
  ['the main message list is a live log',['controllers/messages_controller-','connect() {','connect() { this.messagesTarget.setAttribute("aria-relevant", "removals");']],
  ...[
    'paginated history stays quiet past the insert, then the live region comes back',
    'an edit replacement is not announced as an addition',
    'an own message is not re-announced when its broadcast replaces the pending copy',
  ].map(name=>[name,[live,'region.setAttribute("aria-live", "off")','region.setAttribute("aria-live", "polite")']]),
  ['Markdown replies and file attachments remain usable',[composer,'new FileUploader(file, this.element.action, clientMessageId, this.#uploadProgress.bind(this), reply)','new FileUploader(file, this.element.action, clientMessageId, this.#uploadProgress.bind(this), null)']],
  ['mention suggestions select a room member without sending the unfinished message',['mention-response']],
  ['a rejected message can be recovered corrected and sent',[composer,'recover(event) {','recover(event) { return;']],
  ['sending preserves the submitted source and a newer draft',[composer,'if (this.markdownTarget.value === submission.content)','if (true)']],
  ['search tolerates operators, shows an empty state and pages older results',['search-page-response']],
  ['forwarded Markdown keeps tables and code blocks',['forward-response']],
  ['editing to add a URL renders its card live and the edited marker on load',['controllers/messages_controller-','connect() {','connect() { document.addEventListener("turbo:before-stream-render", event => { if (event.target.getAttribute("target")?.startsWith("twitter_cards_")) event.preventDefault(); });']],
  ['few unread render the divider above the first new message and keep the bottom scroll',['controllers/messages_controller-','connect() {','connect() { const divider = document.getElementById("unread-divider"); divider?.nextElementSibling?.after(divider);']],
  ['many unread scroll the room to the divider',['controllers/messages_controller-','this.#scrollToUnreadDivider(true)','this.messagesTarget.scrollTop = 0']],
  ['the jump pill shows while the divider is off-screen and returns to it',['controllers/messages_controller-','this.#scrollToUnreadDivider(false)','void(false)']],
  ['unread older than the last page keeps the last page and the pill links to the first unread',['controllers/messages_controller-','connect() {','connect() { document.getElementById("jump-to-unread")?.setAttribute("href", "/rooms/654632876?message_id=1");']],
  ['mark unread from the message menu points the divider at that message',[actions,'method: "DELETE",','method: "POST",']],
  ...[
    'search results keep their menus and focusability',
    'the standalone thread page keeps menus and focusability',
    'the standalone message page keeps its menu and focusability',
  ].map(name=>[name,[list,'message.setAttribute("aria-haspopup", "menu")','message.setAttribute("aria-haspopup", "dialog")']]),
  ['the message-list top padding does not apply to search results',['messages-','.messages:not(.searches__results)','.messages']],
  ['the viewport allows pinch zoom',['controllers/messages_controller-','connect() {','connect() { document.querySelector("meta[name=\\\"viewport\\\"]")?.setAttribute("content", "width=device-width, initial-scale=1, user-scalable=no");']],
  ['profile message and ban buttons have accessible names',['profile-button-response']],
  ['flash persists its 5-second minimum under reduced motion',['flash-','animation-duration: 5s !important;','animation-duration: 2s !important;']],
  ['flash dismisses on demand under reduced motion',['controllers/element_removal_controller-','this.element.remove()','void(this.element)']],
  ['blurring an open autocomplete does not leave a zombie that swallows Enter',['lib/autocomplete/base_autocomplete_handler-','this.suggestionController.destroy()','void(this.suggestionController)']],
  ['a stale icon response does not poison the suggestion commit',['lib/autocomplete/markdown_icons_autocomplete_handler-','if (requestId !== this.#requestId) return','if (false) return']],
  ['mention queries are URL-encoded',['lib/autocomplete/base_autocomplete_handler-','query=${encodeURIComponent(query)}','query=${query}']],
  ['composer autocomplete exposes combobox semantics over a polite listbox',['lib/autocomplete/suggestion_controller-','element.setAttribute("aria-expanded", String(expanded))','element.setAttribute("aria-expanded", "false")']],
  ['composing text does not commit a suggestion or send the message',['lib/autocomplete/suggestion_controller-','this.#committing || event.isComposing || event.keyCode === 229','this.#committing || event.keyCode === 229']],
  ['clicking a reply preview scrolls to the loaded message instead of navigating',['controllers/reply_controller-','target.classList.add("message--reply-target")','void(target)']],
  ['clicking a reply preview falls back to the permalink when the target is not loaded',['controllers/reply_controller-','if (!target) return','if (!target) { event.preventDefault(); return }']],
  ['deleting a replied-to message turns open reply previews into a tombstone',['reply-tombstone-response']],
  ['two typers with the same name do not merge',['models/typing_tracker-','this.currentlyTyping[id] = { name, timestamp: Date.now() }','this.currentlyTyping[name] = { name, timestamp: Date.now() }']],
  ['composer drafts persist per room and clear on send',[composer,'window.localStorage.setItem(this.#draftKey(), value)','void(value)']],
  ['thread drafts persist per thread without touching the channel draft',[composer,'const scope = this.threadIdValue ? `thread-${this.threadIdValue}` : "main"','const scope = "main"']],
  ['+ shows both attach options when Drive is available',['controllers/attach_menu_controller-','this.buttonTarget.setAttribute("aria-expanded", "true")','this.buttonTarget.setAttribute("aria-expanded", "false")']],
  ['From this device triggers the file input',['controllers/attach_menu_controller-','chooseDevice(event) {','chooseDevice(event) { return;']],
  ['+ opens the file picker directly without Drive',['controllers/attach_menu_controller-','if (!this.hasMenuTarget) {\n      this.fileInputTarget.click()','if (!this.hasMenuTarget) {\n      void(this.fileInputTarget)']],
  ['arrow keys move between items and Escape closes back onto +',['controllers/attach_menu_controller-','nextIndex = (currentIndex + 1) % items.length','nextIndex = currentIndex']],
  ['a tap outside closes the menu',['controllers/attach_menu_controller-','this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)','this.onDocumentPointerDown = event => { window.__ws8bmMutatedActions?.push({action: "outsideTap", target: event.target?.closest(".room-header__name") ? "room-header" : "other"}); }']],
  ['phone layout keeps the menu above the composer with no horizontal overflow',['controllers/attach_menu_controller-','const top = buttonRect.top - menuHeight - VIEWPORT_PADDING','const top = buttonRect.bottom + VIEWPORT_PADDING']],
  ['device files, paste, and drag-and-drop still preview uploads',[composer,'this.#files.push(...files)','this.#files.push()']],
  ['boosting a message',['boost-create-response']],
  ['a stray create re-entry does not wipe the half-filled thread name',['controllers/thread_panel_controller-','else if (this.createTarget.hidden || parentMessageId !== this.createParentIdTarget.value)','else if (true)']],
  ['deleting a boost',['boost-delete-response']],
  ...['message update preserves the input state','boost by another user preserves the input state'].map(name=>[name,['controllers/messages_controller-','connect() {','connect() { document.addEventListener("turbo:before-stream-render", () => { for (const input of document.querySelectorAll("input[name=\\\"boost[content]\\\"]")) input.value = ""; });']]),
]);

// Supplement the original one-per-case mutants with the review's escaped
// defects. Keep source text intact while removing only its keyword styling.
const missingKeyword=keyword=>['models/code_highlighter-','span.className = "code-token"',`span.className = token.content.trim() === "${keyword}" ? "missing-keyword-token" : "code-token"`];
const reviewMutations=new Map([
  ...['assigned owner can change work status but cannot reassign it','a member who cannot manage the thread cannot assign an agent'].map(name=>[name,new Map([['wrong-permission-status',['work-controller-permission-response']]])]),
  ['work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable',new Map([['wrong-validation-status',['work-controller-permission-response']]])],
  ['search results highlight code on initial load and after returning to the channel',new Map([['missing-const',missingKeyword('const')]])],
  ['editing a code block replaces its language colors and copied source',new Map([['missing-const',missingKeyword('const')],['missing-def',missingKeyword('def')]])],
  ['opens message actions from context menu and keyboard, and cancels a moving long press',new Map([
    ['hidden-clapping',['messages-','.message__quick-reaction {','.message__quick-reaction[title="Clapping"] { display: none !important; }\n.message__quick-reaction {']],
    ['transparent-clapping',['messages-','.message__quick-reaction {','.message__quick-reaction[title="Clapping"] { opacity: 0 !important; }\n.message__quick-reaction {']],
    ['transparent-reaction-ancestor',['messages-','.message__quick-reaction {','.message__quick-reactions { opacity: 0 !important; }\n.message__quick-reaction {']],
  ])],
  ['quick-react creates a boost from the toolbar',new Map([['delayed-boost-write',['boost-delay-write']]])],
]);
// One served-opacity probe per newly visibility-scoped assertion. These can
// also run diagnostically over the unchanged reviewed assertion modules.
const opacity=css=>['messages-','.message__quick-reaction {',`${css}\n.message__quick-reaction {`];
const editor='#composer textarea[name="message[markdown_source]"]';
const restoredEditor=condition=>[composer,'this.#setMarkdownValue(content)',`this.#setMarkdownValue(content); if (${condition}) { this.markdownTarget.style.setProperty("transition", "none", "important"); this.markdownTarget.style.setProperty("opacity", "0", "important") }`];
export const visibilityAssertionMutations=new Map([
  ['edits through the normal composer and restores the saved draft on cancel and success',new Map([
    ['transparent-edit-field',opacity(`${editor} { opacity: 0 !important; }`)],
    ['transparent-cancelled-draft',restoredEditor('submittedContent === undefined')],
    ['transparent-saved-draft',restoredEditor('submittedContent !== undefined && current === submittedContent')],
  ])],
  ['a duplicate delivery does not replace the message while its actions are open',new Map([
    ['transparent-redelivery-field',opacity(`html[data-duplicate-delivery-rendered] ${editor} { transition: none !important; opacity: 0 !important; }`)],
    ['transparent-redelivery-flag',opacity('html[data-duplicate-delivery-rendered] { opacity: 0 !important; }')],
  ])],
  ['keeps newer typing through an asynchronous edit and leaves failures in edit mode',new Map([
    ['transparent-newer-draft',restoredEditor('submittedContent !== undefined && current !== submittedContent')],
  ])],
  ['code and copying remain available when the highlighter cannot load',new Map([
    ['transparent-cleared-code-field',opacity(`${editor} { opacity: 0 !important; }`)],
  ])],
  ['replies with notify off and renders a tombstone when the target is deleted',new Map([
    ['transparent-notify-field',opacity('#composer input[name="message[reply_notify_author]"] { opacity: 0 !important; }')],
  ])],
  ['copies message text and link and forwards to a server-provided thread destination',new Map([
    ['transparent-menu-owner',opacity('.message[data-message-actions-open] { opacity: 0 !important; }')],
  ])],
  ['the more button opens the shared menu for its message',new Map([
    ['transparent-more-owner',opacity('.message[data-message-actions-open] { opacity: 0 !important; }')],
  ])],
  ['forwarded Markdown keeps tables and code blocks',new Map([
    ['transparent-ruby-code',opacity('pre code.language-ruby { opacity: 0 !important; }')],
    ['transparent-forwarded-code',opacity('.message:has(.message__forwarded-label) pre code.language-ruby { opacity: 0 !important; }')],
  ])],
  ['search results highlight code on initial load and after returning to the channel',new Map([
    ['transparent-search-back-link',opacity('a.searches__back { opacity: 0 !important; }')],
  ])],
  ['Markdown messages reach other users and editing preserves the original source',new Map([
    ['transparent-project-link',opacity('.message a[href="https://example.com/notes"] { opacity: 0 !important; }')],
  ])],
  ['search tolerates operators, shows an empty state and pages older results',new Map([
    ['transparent-search-field',opacity('#global-search-input { opacity: 0 !important; }')],
  ])],
]);
export const visibilityLookupMutations=new Map([
  ['copies message text and link and forwards to a server-provided thread destination',new Map([
    ['transparent-forward-destination',opacity('.message-forward-dialog__destination { opacity: 0 !important; }')],
    ['transparent-context-body',opacity('.message[data-message-id="607264868"] [data-reply-target="body"] { opacity: 0 !important; }')],
  ])],
  ['forwarding twice in a row submits only once',new Map([
    ['transparent-forward-checkbox',opacity('.message-forward-dialog__destination input { opacity: 0 !important; }')],
  ])],
  ['groups emoji reactions, updates the live count, and highlights the current user',new Map([
    ['transparent-quick-thumb',opacity('.message__quick-reaction[title="Thumbs up"] { opacity: 0 !important; }')],
  ])],
  ['picker arrows move through options, Enter selects, and Escape returns focus',new Map([
    ['transparent-picker-search',opacity('#emoji-picker-panel input[aria-label="Search emoji and icons"] { opacity: 0 !important; }')],
  ])],
  ['the picker shows category tabs and switches between them',new Map([
    ['transparent-flags-lookup',opacity('#emoji-picker-panel:has(#emoji-picker-tab-people[aria-selected="true"]) #emoji-picker-tab-flags { transition: none !important; opacity: 0 !important; }')],
  ])],
  ['quick-react creates a boost from the toolbar',new Map([
    ['transparent-hover-body',opacity('.message[data-message-id="607264868"] [data-reply-target="body"] { opacity: 0 !important; }')],
  ])],
  ['message action menu is a bottom sheet with touch-sized targets on phones',new Map([
    ['transparent-room-header',opacity('.room-header__name { opacity: 0 !important; }')],
  ])],
]);
// Conditional opacity must become zero at the lookup, not fade through
// positive opacity (which Selenium correctly considers visible).
export const instantaneousOpacityMutations=new Map([
  ['edits through the normal composer and restores the saved draft on cancel and success',new Map([...visibilityAssertionMutations.get('edits through the normal composer and restores the saved draft on cancel and success')].filter(([name])=>name!=='transparent-edit-field'))],
  ['a duplicate delivery does not replace the message while its actions are open',new Map([['transparent-redelivery-field',visibilityAssertionMutations.get('a duplicate delivery does not replace the message while its actions are open').get('transparent-redelivery-field')]])],
  ['keeps newer typing through an asynchronous edit and leaves failures in edit mode',visibilityAssertionMutations.get('keeps newer typing through an asynchronous edit and leaves failures in edit mode')],
  ['the picker shows category tabs and switches between them',visibilityLookupMutations.get('the picker shows category tabs and switches between them')],
]);
// Assert visibility on the Rails-selected element, not a visible child or row.
// These served probes run over unchanged assertions for failing-first evidence.
export const elementScopeMutations=new Map([
  ['Markdown messages reach other users and editing preserves the original source',new Map([
    ['hidden-code-visible-token',opacity('pre code { visibility: hidden !important; } pre code .code-token { visibility: visible !important; }')],
    ['transparent-initial-heading',opacity('.message:not(:has(.message__edited)) .markdown-body h2 { opacity: 0 !important; }')],
  ])],
  ['sending messages between two users',new Map([
    ['hidden-body-visible-presentation',opacity('.message__body { visibility: hidden !important; } .message__body [data-reply-target="body"] { visibility: visible !important; }')],
  ])],
  ['a stray create re-entry does not wipe the half-filled thread name',new Map([
    ['transparent-thread-body',opacity('#thread-panel .thread-panel__thread-content .message__body { opacity: 0 !important; }')],
    ['hidden-conversation-visible-children',opacity('#thread-panel [data-thread-panel-target="conversation"] { visibility: hidden !important; } #thread-panel [data-thread-panel-target="conversation"] > * { visibility: visible !important; }')],
  ])],
  ['untrusted markup stays inert in the delivered message',new Map([
    ['hidden-safety-body-visible-code',opacity('.message__body { visibility: hidden !important; } .message__body pre code { visibility: visible !important; }')],
  ])],
  ['Markdown replies and file attachments remain usable',new Map([
    ['hidden-reply-body-visible-presentation',opacity('.message:has(strong) .message__body { visibility: hidden !important; } .message:has(strong) [data-reply-target="body"] { visibility: visible !important; }')],
    ['transparent-attachment-reply-preview',opacity('.message__reply-preview { opacity: 0 !important; }')],
  ])],
  ['composer autocomplete exposes combobox semantics over a polite listbox',new Map([
    ['transparent-combobox-lookup',opacity(`${editor} { opacity: 0 !important; }`)],
  ])],
  ['thread drafts persist per thread without touching the channel draft',new Map([
    ['transparent-restored-thread-draft',opacity('#thread-panel textarea[name="message[markdown_source]"] { opacity: 0 !important; }')],
    ['transparent-cleared-thread-draft',opacity('#thread-panel:has(.thread-panel__thread-content .message[data-message-id] ~ .message[data-message-id]) textarea[name="message[markdown_source]"] { transition: none !important; opacity: 0 !important; }')],
  ])],
  ['message update preserves the input state',new Map([
    ['transparent-boost-draft-after-edit',opacity('input[name="boost[content]"] { transition: none !important; opacity: 0 !important; }')],
  ])],
  ['boost by another user preserves the input state',new Map([
    ['transparent-boost-draft-after-delivery',opacity('input[name="boost[content]"] { transition: none !important; opacity: 0 !important; }')],
  ])],
]);
elementScopeMutations.set('sending preserves the submitted source and a newer draft',new Map([
  ['hidden-submitted-body-visible-strong',opacity('.message:has(.markdown-body strong) .message__body { visibility: hidden !important; } .message:has(.markdown-body strong) .message__body strong { visibility: visible !important; }')],
]));
elementScopeMutations.set('search tolerates operators, shows an empty state and pages older results',new Map([
  ['transparent-older-search-text',opacity('#search-results .message .markdown-body p { opacity: 0 !important; }')],
]));
elementScopeMutations.set('editing to add a URL renders its card live and the edited marker on load',new Map([
  ['transparent-initial-url-text',opacity('.message:not(:has(.message__edited)) .markdown-body p { opacity: 0 !important; }')],
]));
elementScopeMutations.get('thread drafts persist per thread without touching the channel draft').set('hidden-initial-composer-conversation',[
  'controllers/thread_panel_controller-',
  'if (this.hasConversationTarget) this.conversationTarget.hidden = view !== "conversation"',
  'if (this.hasConversationTarget) { this.conversationTarget.hidden = view !== "conversation"; if (view === "conversation" && !sessionStorage.getItem("ws8bm-mutant-first-conversation")) { sessionStorage.setItem("ws8bm-mutant-first-conversation", "1"); this.conversationTarget.style.setProperty("visibility", "hidden", "important"); for (const child of this.conversationTarget.children) child.style.setProperty("visibility", "visible", "important") } }',
]);
for(const [name,variants] of elementScopeMutations) {
  if(!reviewMutations.has(name)) reviewMutations.set(name,new Map());
  for(const [variant,mutation] of variants) reviewMutations.get(name).set(variant,mutation);
}
for(const [name,variants] of visibilityLookupMutations) {
  if(!visibilityAssertionMutations.has(name)) visibilityAssertionMutations.set(name,new Map());
  for(const [variant,mutation] of variants) visibilityAssertionMutations.get(name).set(variant,mutation);
}
for(const [name,variants] of visibilityAssertionMutations) {
  if(!reviewMutations.has(name)) reviewMutations.set(name,new Map());
  for(const [variant,mutation] of variants) reviewMutations.get(name).set(variant,mutation);
}
// These served changes are allowed by explicit Rails visible: false scopes.
// They must PASS their named checks; never schedule them as negative mutants.
export const hiddenScopeProbes=new Map([
  ['the main message list is a live log',new Map([
    ['hidden-live-log-and-messages',opacity('.messages[role="log"] { visibility: hidden !important; }')],
    ['hidden-live-log-visible-messages',opacity('.messages[role="log"] { visibility: hidden !important; } .messages[role="log"] > .message { visibility: visible !important; }')],
  ])],
  ['the more button opens the shared menu for its message',new Map([
    ['hidden-open-message-toolbar',opacity('.message[data-message-actions-open] .message__toolbar { display: none !important; }')],
  ])],
]);
const hiddenAttachText=action=>['controllers/attach_menu_controller-','connect() {',`connect() { for (const button of this.element.querySelectorAll('[data-action*="${action}"]')) { const span = document.createElement("span"); span.style.opacity = "0"; span.textContent = button.textContent; button.replaceChildren(span); }`];
export const labelMutations=new Map([
  ['Markdown replies and file attachments remain usable',new Map([
    ['transparent-attachment-filename',opacity('.message__body [data-reply-target="body"] > .flex-inline span { opacity: 0 !important; }')],
  ])],
  ['+ shows both attach options when Drive is available',new Map([
    ['transparent-device-text',hiddenAttachText('chooseDevice')],
    ['transparent-drive-text',hiddenAttachText('chooseDrive')],
  ])],
  ['phone layout keeps the menu above the composer with no horizontal overflow',new Map([
    ['transparent-phone-drive-text',hiddenAttachText('chooseDrive')],
  ])],
  ['deleting a boost',new Map([
    ['transparent-boost-delete-text',opacity('.boost__delete span { opacity: 0 !important; }')],
  ])],
]);
for(const [name,variants] of labelMutations) {
  if(!reviewMutations.has(name)) reviewMutations.set(name,new Map());
  for(const [variant,mutation] of variants) reviewMutations.get(name).set(variant,mutation);
}
export const categoryMutations=new Map([
  ['profile message and ban buttons have accessible names',new Map([['transparent-ban-text',opacity('form[action="/users/712064548/ban"] button span { opacity: 0 !important; }')]])],
  ['deleting the focused message moves focus to the surviving tab stop',new Map([
    ['delayed-negative-removal',[list,'connect() {','connect() { document.addEventListener("turbo:before-stream-render", event => { if (event.target.getAttribute("action") === "remove") { const render = event.detail.render; event.detail.render = async stream => { await new Promise(resolve => setTimeout(resolve, 4000)); await render(stream); }; } });']],
  ])],
  ['editing to add a URL renders its card live and the edited marker on load',new Map([
    ['hidden-url-loading-text',opacity('.x-post-card__loading { visibility: hidden !important; }')],
    ['transparent-url-editor',opacity(`${editor} { opacity: 0 !important; }`)],
    ['delayed-url-edited-marker',opacity('.message[data-message-id="935962058"] .message__edited { opacity: 0; animation: ws8bmMarker 0.001s linear 4s forwards !important; } @keyframes ws8bmMarker { to { opacity: 1; } }')],
  ])],
  ['the emoji picker searches and reacts',new Map([
    ['transparent-picker-fill',opacity('#emoji-picker-panel input[aria-label="Search emoji and icons"] { opacity: 0 !important; }')],
  ])],
]);
for(const [name,variants] of categoryMutations) {
  if(!reviewMutations.has(name)) reviewMutations.set(name,new Map());
  for(const [variant,mutation] of variants) reviewMutations.get(name).set(variant,mutation);
}
const pickerAsset='controllers/emoji_picker_controller-';
const stripLiteralAttribute=label=>[pickerAsset,'connect() {',`connect() {
  const strip=()=>document.querySelectorAll('.message__toolbar button[aria-label="${label}"]').forEach(button=>button.removeAttribute('aria-label'));
  new MutationObserver(strip).observe(document,{subtree:true,childList:true,attributes:true,attributeFilter:['aria-label']});strip();`];
export const literalAttributeMutations=new Map([
  ['the toolbar stays hidden until hover or focus and labels every action',new Map([
    ['missing-thumb-aria-label',stripLiteralAttribute('React with thumbs up')],
    ['missing-picker-aria-label',stripLiteralAttribute('Add reaction')],
    ['missing-reply-aria-label',stripLiteralAttribute('Reply to message')],
    ['missing-thread-aria-label',stripLiteralAttribute('Open thread')],
    ['missing-more-aria-label',stripLiteralAttribute('More message actions')],
  ])],
  ['the emoji picker searches and reacts',new Map([
    ['missing-option-aria-label',[pickerAsset,'connect() {',`connect() {
      const strip=()=>document.querySelectorAll('.emoji-picker__option[aria-label="Grinning face"]').forEach(button=>{
        const label=document.createElement('span');label.id='ws8bm-option-label';label.textContent='Grinning face';label.hidden=true;document.body.append(label);
        button.setAttribute('aria-labelledby',label.id);button.removeAttribute('aria-label');
      });
      new MutationObserver(strip).observe(document,{subtree:true,childList:true,attributes:true,attributeFilter:['aria-label']});strip();`]],
  ])],
]);
for(const [name,variants] of literalAttributeMutations) {
  if(!reviewMutations.has(name)) reviewMutations.set(name,new Map());
  for(const [variant,mutation] of variants) reviewMutations.get(name).set(variant,mutation);
}
export const mutationNames=[...new Set([...mutations.keys(),...reviewMutations.keys()])];
export function mutationVariants(caseName,selected=process.env.WS8BM_MUTANT) {
  const variants=[...(mutations.has(caseName)?['default']:[]),...(reviewMutations.get(caseName)?.keys()||[])];
  return selected?variants.filter(name=>name===selected):variants;
}
export async function installMutation(page,caseName,probe,variant='default') {
  const mutation=variant==='default'?mutations.get(caseName):reviewMutations.get(caseName)?.get(variant)||hiddenScopeProbes.get(caseName)?.get(variant);
  assert.ok(mutation,`no discrimination mutant for ${caseName}`);
  if(process.env.WS8BM_NEGATIVE==='1') probe.target=mutationTarget(caseName,variant);
  const [asset,,replacement]=mutation;
  if(variant==='default') {
    probe.requiredAction=caseName==='the picker shows category tabs and switches between them'
      ?{action:'chooseTab',target:'emoji-picker-tab-people'}:
      caseName==='a tap outside closes the menu'?{action:'outsideTap',target:'room-header'}:undefined;
    if(probe.requiredAction) {
      await page.addInitScript(()=>{window.__ws8bmMutatedActions=[];});
      (probe.observers??=[]).push(()=>page.evaluate(()=>window.__ws8bmMutatedActions||[]).catch(()=>[]));
    }
  }
  if(caseName==='a release click landing on the just-opened menu does not activate it'&&variant==='default') {
    probe.requiresReleaseClick=true;
    await page.addInitScript(()=>{
      window.__ws8bmReleaseClicks=[];
      document.addEventListener('click',event=>{
        const menu=event.target.closest?.('#message-actions-menu');
        if(menu) window.__ws8bmReleaseClicks.push({releaseClick:true,brokenGuard:window.__ws8bmBrokenReleaseGuard===true,menuVisible:!menu.hidden});
      },true);
    });
    (probe.observers??=[]).push(()=>page.evaluate(()=>window.__ws8bmReleaseClicks||[]).catch(()=>[]));
  }
  const cssSelector=asset==='messages-'&&replacement?.includes('{')?(replacement.startsWith('@media')?replacement.match(/@media[^\{]+\{\s*([^\{]+)\{/)[1].trim():replacement.split('{')[0].trim()):null;
  const scriptedSelector=['transparent-cancelled-draft','transparent-saved-draft','transparent-newer-draft'].includes(variant)?editor:
    ['transparent-device-text','transparent-drive-text','transparent-phone-drive-text'].includes(variant)?'.attach-menu span[style]':
    ['missing-const','missing-def'].includes(variant)?'.missing-keyword-token':
    asset==='models/code_highlighter-'&&replacement?.includes('code.dataset.highlighted = "no"')?'code[data-highlighted="no"]':
    asset==='models/code_highlighter-'&&replacement?.includes('missing-code-token')?'.missing-code-token':null;
  const injectedStyleSelector=asset==='controllers/thread_panel_controller-'?replacement?.match(/style\.textContent = '([^'{]+)\{/ )?.[1].trim():null;
  const missingLabels={'missing-thumb-aria-label':'React with thumbs up','missing-picker-aria-label':'Add reaction',
    'missing-reply-aria-label':'Reply to message','missing-thread-aria-label':'Open thread','missing-more-aria-label':'More message actions'};
  const attributeSelector=missingLabels[variant]?`.message__toolbar button[title="${missingLabels[variant]}"]:not([aria-label])`:
    variant==='missing-option-aria-label'?'.emoji-picker__option[aria-labelledby="ws8bm-option-label"]:not([aria-label])':null;
  const initialConversationSelector=variant==='hidden-initial-composer-conversation'?'#thread-panel [data-thread-panel-target="conversation"]':null;
  const stateSelector=cssSelector||scriptedSelector||injectedStyleSelector||attributeSelector||initialConversationSelector;
  if(variant==='hidden-submitted-body-visible-strong') probe.requiredMessageText='First message stays exact.';
  if(initialConversationSelector) probe.requiresHiddenState=true;
  probe.requiresHiddenState ||= !!injectedStyleSelector&&/opacity:\s*0/.test(replacement)|| !!scriptedSelector&&variant.startsWith('transparent-')||!!cssSelector&&/opacity:\s*0(?:[ ;}]|$)|visibility:\s*hidden|display:\s*none/.test(replacement);
  if(stateSelector) (probe.observers??=[]).push(async()=>{
    if(page.isClosed()) return [];
    return page.locator(stateSelector).evaluateAll((elements,selector)=>elements.map(element=>({
      selector,messageId:element.closest('.message[data-message-id]')?.dataset.messageId,
      messageText:element.closest('.message[data-message-id]')?.querySelector('[data-reply-target="body"]')?.textContent.replace(/\s+/g,' ').trim(),
      opacity:getComputedStyle(element).opacity,
      visibility:getComputedStyle(element).visibility,display:getComputedStyle(element).display,
      fontSize:getComputedStyle(element).fontSize,background:getComputedStyle(element).backgroundColor,position:getComputedStyle(element).position,
      width:element.getBoundingClientRect().width,height:element.getBoundingClientRect().height,
      seleniumVisible:window.__ws8bmSeleniumVisible(element),
    })),stateSelector).catch(()=>[]);
  });
  await page.route('**/*',async route=>{
    const request=route.request(),url=new URL(request.url());
    const [asset,needle,replacement]=mutation;
    // Icons share names with stylesheets. Never mutate an SVG whose name
    // happens to start with messages- or flash-.
    if((asset==='messages-'||asset==='flash-')&&!url.pathname.endsWith('.css')) return route.continue();
    const controller=['work-controller-json-response','work-controller-permission-response'].includes(asset)&&/^\/rooms\/654632876\/threads\/\d+\.json$/.test(url.pathname);
    const orphan=asset==='workspace-toggle-response'&&url.pathname==='/users/me/profile'&&request.method()==='GET';
    if(controller||orphan) {
      const response=await route.fetch();let body=await response.text();
      if(controller&&asset==='work-controller-permission-response'&&[403,422].includes(response.status())) {probe.applied++;return route.fulfill({response,status:200});}
      if(controller&&asset==='work-controller-json-response'&&response.status()===200) {const value=JSON.parse(body);value.thread.name='Wrong discussion';body=JSON.stringify(value);probe.applied++;}
      else if(orphan){assert.ok(body.includes('</nav>'));body=body.replace('</nav>','<button aria-label="Open workspace navigation">Open workspace navigation</button></nav>');probe.applied++;}
      return route.fulfill({response,body});
    }
    const refresh=asset==='refresh-content-type'&&url.searchParams.get('reason')==='connection';
    const mention=asset==='mention-response'&&url.pathname.includes('/autocompletable/users');
    const search=asset==='search-page-response'&&url.pathname==='/searches'&&url.searchParams.has('before');
    const forward=asset==='forward-response'&&request.method()==='POST'&&/\/forwards(?:\.json)?$/.test(url.pathname);
    const profile=asset==='profile-button-response'&&url.pathname==='/users/712064548';
    const tombstone=asset==='reply-tombstone-response'&&url.pathname.startsWith('/rooms/')&&url.pathname.includes('/messages/')&&request.method()==='DELETE';
    const boost=asset==='boost-create-response'&&/\/messages\/\d+\/boosts$/.test(url.pathname)&&request.method()==='POST';
    const delayedBoost=asset==='boost-delay-write'&&/\/messages\/\d+\/boosts$/.test(url.pathname)&&request.method()==='POST';
    const boostDelete=asset==='boost-delete-response'&&/\/messages\/\d+\/boosts\/\d+$/.test(url.pathname)&&(request.method()==='DELETE'||request.postData()?.includes('_method=delete'));
    const githubThread=asset==='github-thread-response'&&/^\/rooms\/654632876\/threads\/\d+$/.test(url.pathname)&&request.method()==='GET';
    if(!refresh&&!mention&&!search&&!forward&&!profile&&!tombstone&&!boost&&!delayedBoost&&!boostDelete&&!githubThread&&!url.pathname.includes('/assets/'+asset)) return route.continue();
    if(delayedBoost) {
      // Hold the request before forwarding: neither its write nor its Cable
      // delivery can happen during the original ten-second assertion budget.
      probe.applied++;probe.delayedWriteStarted=Date.now();
      await new Promise(resolve=>setTimeout(resolve,11000));
      if(page.isClosed()) return;
      const response=await route.fetch();
      probe.delayedWriteCompleted=true;
      return route.fulfill({response});
    }
    // Reject the request before forwarding it: a deliberately failed write
    // cannot secretly reach the real app and then pass through its Cable frame.
    if(forward||tombstone) {probe.applied++;return route.fulfill({status:422,contentType:'application/json',body:'{"error":"injected failed write"}'});}
    if(boost||boostDelete) {probe.applied++;return route.fulfill({status:422,contentType:'text/html',body:'<div>injected failed boost</div>'});}
    const response=await route.fetch();let body=await response.text(),headers=response.headers();
    if(refresh) {headers['content-type']='text/vnd.turbo-stream.html';body=' ';}
    else if(mention) {assert.ok(body.includes('Kevin'));body=body.replaceAll('Kevin','Wrong member');}
    else if(search) {assert.ok(body.includes('system paging alpha'));body=body.replaceAll('system paging alpha','wrong older result');}
    else if(profile) {assert.ok(body.includes('aria-label="Message Kevin"'));body=body.replace('aria-label="Message Kevin"','aria-label="Wrong recipient"');}
    else if(githubThread) {assert.ok(body.includes('github-pr-thread-header'));body=body.replaceAll('github-pr-thread-header','missing-pr-header');}
    else {assert.ok(body.includes(needle),`mutant source needle missing: ${asset}`);body=body.replace(needle,replacement);}
    if(probe.requiresReleaseClick) body=body.replace('this.#suppressClickUntil = 0','this.#suppressClickUntil = 0; window.__ws8bmBrokenReleaseGuard = true');
    // Diagnostic composition: the intended delayed marker still exists, but
    // hide loading text to reproduce an earlier, unrelated timeout. This must
    // be INVALID under strict attribution, never another registered negative.
    if(process.env.WS8BM_UNRELATED_FAILURE_PROBE==='1'&&variant==='delayed-url-edited-marker'&&asset==='messages-') {
      body=body.replace('.message__quick-reaction {','.x-post-card__loading { visibility: hidden !important; }\n.message__quick-reaction {');
    }
    probe.applied++;
    await route.fulfill({response,status:refresh?200:response.status(),headers,body});
  });
}
