// Source-anchored, served implementation failures for every new action flow.
const toolbar='controllers/message_toolbar_controller-';
const picker='controllers/emoji_picker_controller-';
const actions='controllers/message_actions_controller-';
const list='controllers/message_list_controller-';
const composer='controllers/composer_controller-';
export const actionMutations=[
  ['discusses a pull request from its card',['github-thread-response']],
  ...['language fences highlight common code without changing its text','unlabelled code is detected while text unknown languages and inline code stay literal','search results highlight code on initial load and after returning to the channel','editing a code block replaces its language colors and copied source'].map(name=>[name,['models/code_highlighter-','code.dataset.highlighted = "yes"','code.dataset.highlighted = "no"']]),
  ['code and copying remain available when the highlighter cannot load',['models/message_formatter-','navigator.clipboard.writeText(sourceText)','navigator.clipboard.writeText("wrong code")']],
  ['the toolbar stays hidden until hover or focus and labels every action',['messages-','.message__toolbar {','.message__toolbar { display: inline-flex !important;']],
  ...['quick-react creates a boost from the toolbar','keyboard users reach the toolbar from a focused message'].map(name=>[name,[toolbar,'react() {','react() { return;']]),
  ['reply and thread buttons drive the composer and the thread panel',[toolbar,'thread() {','thread() { return;']],
  ['the more button opens the shared menu for its message',[toolbar,'more(event) {','more(event) { return;']],
  ['the emoji picker searches and reacts',['boost-create-response']],
  ['the picker shows category tabs and switches between them',[picker,'chooseTab(event) {','chooseTab(event) { return;']],
  ['the picker loads its emoji data only on first open',[picker,'this.#connected = true','this.#connected = true; void this.#ensureData()']],
  ['the picker remembers recent reactions',[picker,'localStorage.setItem(RECENT_KEY, JSON.stringify(entries.slice(-100)))','void(entries)']],
  ['the picker Custom tab reacts with a workspace icon',[picker,'else if (this.#tab === "custom") void this.#renderCustom()','else if (this.#tab === "custom") this.#renderRecent()']],
  ['the picker reacts with a brand icon shortcode',[picker,'for (const icon of servers || [])','for (const icon of [])']],
  ['picker arrows move through options, Enter selects, and Escape returns focus',[picker,'nextIndex = Math.min(options.length - 1, currentIndex + 1)','nextIndex = currentIndex']],
  ['picker tabs move with arrow keys and switch the grid',[picker,'next = (current + 1) % tabs.length','next = current']],
  ...['message action menu is a bottom sheet with touch-sized targets on phones','shows the message action menu as a bottom sheet on phones'].map(name=>[name,[actions,'if (this.#isMobileSheet()) {','if (this.#isMobileSheet()) { this.menuTarget.style.bottom = "80px";']]),
  ['message action menu stays a floating popover on desktop',[actions,'const point = this.#menuPoint || { x: 0, y: 0 }','this.menuTarget.style.maxInlineSize = "none"; this.menuTarget.style.inlineSize = "100vw"; const point = this.#menuPoint || { x: 0, y: 0 }']],
  ['opens message actions from context menu and keyboard, and cancels a moving long press',[list,'this.onContextMenu = this.#onContextMenu.bind(this)','this.onContextMenu = () => {}']],
  ['a release click landing on the just-opened menu does not activate it',[list,'this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS','this.#suppressClickUntil = 0']],
  ['edits through the normal composer and restores the saved draft on cancel and success',[composer,'const content = submittedContent !== undefined && current !== submittedContent ? current : saved?.content || ""','const content = submittedContent !== undefined && current !== submittedContent ? current : ""']],
  ['a duplicate delivery does not replace the message while its actions are open',['controllers/messages_controller-','if (action === "append" && this.#alreadyDelivered(streamElement)) return','if (false && this.#alreadyDelivered(streamElement)) return']],
  ['keeps newer typing through an asynchronous edit and leaves failures in edit mode',[composer,'submittedContent !== undefined && current !== submittedContent ? current : saved?.content || ""','submittedContent !== undefined && false ? current : saved?.content || ""']],
  ['replies with notify off and renders a tombstone when the target is deleted',['reply-tombstone-response']],
  ['copies message text and link and forwards to a server-provided thread destination',[actions,'navigator.clipboard.writeText(text)','navigator.clipboard.writeText("wrong message")']],
  ['forwarding twice in a row submits only once',[actions,'this.forwardSubmitTarget.disabled = true','this.forwardSubmitTarget.disabled = false']],
  ['groups emoji reactions, updates the live count, and highlights the current user',['boost-create-response']],
];
