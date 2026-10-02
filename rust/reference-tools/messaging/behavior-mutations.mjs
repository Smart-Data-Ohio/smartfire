// Deliberately broken served implementations, never replacement oracle values.
// Each named check must reach its case assertions and reject its specific mutant.
import assert from 'node:assert/strict';
import {actionMutations} from './behavior-action-mutations.mjs';
const list='controllers/message_list_controller-';
const actions='controllers/message_actions_controller-';
const composer='controllers/composer_controller-';
const live='helpers/live_region_helpers-';
const mutations=new Map([
  ...actionMutations,
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
  ['a tap outside closes the menu',['controllers/attach_menu_controller-','this.onDocumentPointerDown = this.#onDocumentPointerDown.bind(this)','this.onDocumentPointerDown = () => {}']],
  ['phone layout keeps the menu above the composer with no horizontal overflow',['controllers/attach_menu_controller-','const top = buttonRect.top - menuHeight - VIEWPORT_PADDING','const top = buttonRect.bottom + VIEWPORT_PADDING']],
  ['device files, paste, and drag-and-drop still preview uploads',[composer,'this.#files.push(...files)','this.#files.push()']],
  ['boosting a message',['boost-create-response']],
  ['a stray create re-entry does not wipe the half-filled thread name',['controllers/thread_panel_controller-','else if (this.createTarget.hidden || parentMessageId !== this.createParentIdTarget.value)','else if (true)']],
  ['deleting a boost',['boost-delete-response']],
  ...['message update preserves the input state','boost by another user preserves the input state'].map(name=>[name,['controllers/messages_controller-','connect() {','connect() { document.addEventListener("turbo:before-stream-render", () => { for (const input of document.querySelectorAll("input[name=\\\"boost[content]\\\"]")) input.value = ""; });']]),
]);
export const mutationNames=[...mutations.keys()];
// Supplement the original one-per-case mutants with the review's escaped
// defects. Keep source text intact while removing only its keyword styling.
const missingKeyword=keyword=>['models/code_highlighter-','span.className = "code-token"',`span.className = token.content.trim() === "${keyword}" ? "missing-keyword-token" : "code-token"`];
const reviewMutations=new Map([
  ['search results highlight code on initial load and after returning to the channel',new Map([['missing-const',missingKeyword('const')]])],
  ['editing a code block replaces its language colors and copied source',new Map([['missing-const',missingKeyword('const')],['missing-def',missingKeyword('def')]])],
  ['opens message actions from context menu and keyboard, and cancels a moving long press',new Map([
    ['hidden-clapping',['messages-','.message__quick-reaction {','.message__quick-reaction[title="Clapping"] { display: none !important; }\n.message__quick-reaction {']],
    ['transparent-clapping',['messages-','.message__quick-reaction {','.message__quick-reaction[title="Clapping"] { opacity: 0 !important; }\n.message__quick-reaction {']],
    ['transparent-reaction-ancestor',['messages-','.message__quick-reaction {','.message__quick-reactions { opacity: 0 !important; }\n.message__quick-reaction {']],
  ])],
  ['quick-react creates a boost from the toolbar',new Map([['delayed-boost-write',['boost-delay-write']]])],
]);
export function mutationVariants(caseName,selected=process.env.WS8BM_MUTANT) {
  const variants=['default',...(reviewMutations.get(caseName)?.keys()||[])];
  return selected?variants.filter(name=>name===selected):variants;
}
export async function installMutation(page,caseName,probe,variant='default') {
  const mutation=variant==='default'?mutations.get(caseName):reviewMutations.get(caseName)?.get(variant);
  assert.ok(mutation,`no discrimination mutant for ${caseName}`);
  await page.route('**/*',async route=>{
    const request=route.request(),url=new URL(request.url());
    const [asset,needle,replacement]=mutation;
    // Icons share names with stylesheets. Never mutate an SVG whose name
    // happens to start with messages- or flash-.
    if((asset==='messages-'||asset==='flash-')&&!url.pathname.endsWith('.css')) return route.continue();
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
    probe.applied++;
    await route.fulfill({response,status:refresh?200:response.status(),headers,body});
  });
}
