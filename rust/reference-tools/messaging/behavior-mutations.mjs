// Deliberately broken served implementations, never replacement oracle values.
// Each named check must reach its case assertions and reject its specific mutant.
import assert from 'node:assert/strict';
const list='controllers/message_list_controller-';
const actions='controllers/message_actions_controller-';
const composer='controllers/composer_controller-';
const live='helpers/live_region_helpers-';
const mutations=new Map([
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
]);
export const mutationNames=[...mutations.keys()];
export async function installMutation(page,caseName,probe) {
  const mutation=mutations.get(caseName);
  assert.ok(mutation,`no discrimination mutant for ${caseName}`);
  await page.route('**/*',async route=>{
    const request=route.request(),url=new URL(request.url());
    const [asset,needle,replacement]=mutation;
    const refresh=asset==='refresh-content-type'&&url.searchParams.get('reason')==='connection';
    const mention=asset==='mention-response'&&url.pathname.includes('/autocompletable/users');
    const search=asset==='search-page-response'&&url.pathname==='/searches'&&url.searchParams.has('before');
    const forward=asset==='forward-response'&&request.method()==='POST'&&url.pathname.endsWith('/forwards');
    if(!refresh&&!mention&&!search&&!forward&&!url.pathname.includes('/assets/'+asset)) return route.continue();
    const response=await route.fetch();let body=await response.text(),headers=response.headers();
    if(refresh) {headers['content-type']='text/vnd.turbo-stream.html';body=' ';}
    else if(mention) {assert.ok(body.includes('Kevin'));body=body.replaceAll('Kevin','Wrong member');}
    else if(search) {assert.ok(body.includes('system paging alpha'));body=body.replaceAll('system paging alpha','wrong older result');}
    else if(forward) {return route.fulfill({status:422,contentType:'application/json',body:'{"error":"injected failed forward"}'}).then(()=>probe.applied++);}
    else {assert.ok(body.includes(needle),`mutant source needle missing: ${asset}`);body=body.replace(needle,replacement);}
    probe.applied++;
    await route.fulfill({response,status:refresh?200:response.status(),headers,body});
  });
}
