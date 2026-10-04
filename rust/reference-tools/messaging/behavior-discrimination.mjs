// Credit only the assertion named by the served mutation. Source anchors keep
// this contract reviewable without line-number drift. Unknown failures fail closed.
import {readFileSync} from 'node:fs';
import {WORKSPACE_CASE} from './behavior-workspace.mjs';
import {motionCases} from './behavior-motion.mjs';
import {workControllerCases} from './behavior-work-controllers.mjs';
const helpers=new Set(['behavior-visibility.mjs','behavior-text.mjs']);
export function assertionFrames(error) {
  const frames=[];
  for(const match of (error?.stack||'').matchAll(/(?:file:\/\/)?([^\s()]+\/((?:behavior)[^/]*\.mjs)):(\d+):(\d+)/g)) {
    const [,path,module,line,column]=match;
    if(helpers.has(module)||module==='behavior-discrimination.mjs') continue;
    const source=readFileSync(path,'utf8').split('\n')[Number(line)-1]||'';
    // V8 points at the call/await that failed. Exclude earlier calls on a
    // multi-statement line, so a later assertion cannot credit an earlier wait.
    const offset=Number(column)-1;
    // Native assertion/Playwright stacks point at a method name (equal,
    // waitForFunction), while async helper stacks point at await. Preserve
    // that method's receiver without including any earlier statement.
    const receiver=source.slice(0,offset).match(/[A-Za-z_$][\w$]*\.$/)?.[0]||'';
    const suffix=source.slice(offset-receiver.length);
    frames.push({module,line:Number(line),column:Number(column),source:suffix.split(';')[0]});
  }
  return frames;
}
const target=(module,anchor,message)=>({module:`behavior-${module}.mjs`,anchor,message});
const D=new Map();
const add=(names,module,anchor,message)=>{for(const name of names) D.set(name,[target(module,anchor,message)]);};
add(['From Google Drive starts the legacy picker flow'],'attach-menu',"page.locator('.drive-picker__item')");
add(['attach Drive files from the picker, send textless, and remove through edit','edit a room message in the composer and remove one of two attachments'],'drive',"scope.locator('.drive-attachments .drive-chip__name'),'Q3 Planning'");
add(['attach a Drive file from the thread composer'],'drive','await waitForVisibility(link(),{timeout:10000})');
add(['motion is off by default in the test environment'],'motion-default',"assert.equal(state.motion,'off','motion: server test attribute')");
add(['From Google Drive starts the enhanced share flow when sharing is configured'],'attach-menu',"page.locator('.drive-share-dialog .drive-share-dialog__file')");
add([WORKSPACE_CASE],'workspace',"'workspace: profile inside navigation'");
for(const [index,anchor] of [
  [0,"'motion: drawer starts off-canvas'"],
  [1,"'motion: member-select positions'"],
  [2,"'motion: directory-select positions'"],
  [3,"'motion: sticky bar inside scrollport'"],
  [4,"'motion: menu clamped inside viewport'"],
  [5,"'motion: closed drawer keeps offset'"],
  [6,'await firstOpenCurrentFocus(page,current)'],
  [7,'await reopenedCurrentFocus(page,current)'],
]) add([motionCases[index]],'motion',anchor);
D.set(motionCases[0],[{...target('native-motion','await nativePhone'),native:{source:'test/system/motion_test.rb',line:43,message:'expected the drawer to start off-canvas'}}]);
add(['the message list is a single tab stop with a roving tabindex'],'message-list','page.waitForFunction(id=>');
add(['arrow keys move between messages','deleting the focused message moves focus to the surviving tab stop','deleting an older focused message hands focus to its neighbour, not the newest','a late composer autofocus does not steal focus from a message'],'message-list','document.activeElement?.id===id');
add(['a stream replacing the focused message keeps focus and the tab stop on its replacement','a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement','a direct DOM swap of the focused message keeps focus and the tab stop on its replacement'],'message-list','rows.filter(row=>row.tabIndex===0)');
// waitForFunction's stack points to the opening line, not its callback body.
for(const name of ['the message list is a single tab stop with a roving tabindex','a stream replacing the focused message keeps focus and the tab stop on its replacement','a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement','a direct DOM swap of the focused message keeps focus and the tab stop on its replacement']) D.set(name,[target('message-list','page.waitForFunction(id=>{')]);
add(["a focus move during a stream render survives Turbo's focus restore",'a no-change room refresh does not yank focus back to the composer'],'message-list','assert.equal(active,first)');
add(['the ContextMenu key opens the shared menu and Escape returns focus'],'message-list',"page.locator('#message-actions-menu:not([hidden])')");
add(['up arrow from an empty composer still edits my last message'],'message-list',"'Editing Message'");
add(['up-arrow-to-edit shows an error when the actions endpoint fails'],'message-list',"'temporarily unavailable'");
add(['forward reuses the menu-open metadata request instead of fetching again'],'message-list','window.__actionFetches),1');
add(['a menu opened while an action waits does not redirect the pending action'],'message-list',"page.locator('dialog[open]'),0");
add(['the menu closes before Turbo caches the page'],'message-list',"page.locator('.message[data-message-actions-open]'),{state:'hidden'}");
add(['the main message list is a live log'],'message-list','aria-relevant="additions"');
add(['paginated history stays quiet past the insert, then the live region comes back'],'message-list',"timeline.some(entry=>entry.live==='off')");
add(['an edit replacement is not announced as an addition','an own message is not re-announced when its broadcast replaces the pending copy'],'message-list',"window.__liveValues)).includes('off')");
add(['search results keep their menus and focusability','the standalone thread page keeps menus and focusability','the standalone message page keeps its menu and focusability'],'message-destinations','aria-haspopup="menu"');
add(['the message-list top padding does not apply to search results'],'message-destinations',"selector==='.messages'");
add(['the viewport allows pinch zoom'],'message-destinations','meta[name="viewport"]');
add(['profile message and ban buttons have accessible names'],'message-destinations','button[aria-label="Message Kevin"]');
add(['flash persists its 5-second minimum under reduced motion'],'message-destinations','[5000]');
add(['flash dismisses on demand under reduced motion'],'message-destinations',"page.locator('.flash'),{state:'hidden'");
add(['blurring an open autocomplete does not leave a zombie that swallows Enter'],'composer',"page.locator('suggestion-option'),0");
add(['a stale icon response does not poison the suggestion commit'],'composer','field(page,\':openai: \')');
add(['mention queries are URL-encoded'],'composer','requests.some(url=>');
add(['composer autocomplete exposes combobox semantics over a polite listbox'],'composer',"editor.getAttribute('aria-expanded'),'true'");
add(['composing text does not commit a suggestion or send the message'],'composer','field(page,\':open\')');
add(['clicking a reply preview scrolls to the loaded message instead of navigating'],'composer','message--reply-target');
add(['clicking a reply preview falls back to the permalink when the target is not loaded'],'composer','window.__composerTestUrls.some');
add(['deleting a replied-to message turns open reply previews into a tombstone'],'composer',"root,{state:'hidden'");
add(['two typers with the same name do not merge'],'composer','/^David, David$/');
add(['composer drafts persist per room and clear on send'],'composer',"localStorage.getItem('campfire.composer.draft.773523953.654632876.main')");
add(['thread drafts persist per thread without touching the channel draft'],'composer',"waitForVisibleProperty(reply,'value','Thread draft')");
add(['+ shows both attach options when Drive is available'],'attach-menu','aria-expanded=');
add(['From this device triggers the file input','+ opens the file picker directly without Drive'],'attach-menu','window.filePickerClicks),1');
add(['arrow keys move between items and Escape closes back onto +'],'attach-menu',"[role=\"menuitem\"]:nth-child(2)");
add(['a tap outside closes the menu'],'attach-menu','await expanded(false)');
add(['phone layout keeps the menu above the composer with no horizontal overflow'],'attach-menu','geometry.menu.bottom<=geometry.buttonTop');
add(['device files, paste, and drag-and-drop still preview uploads'],'attach-menu',"'hello'");
add(['boosting a message'],'boosts',"value)}");
D.set('deleting a boost',[target('boosts',"boost,{state:'hidden'}"),target('boosts',"browser.locator('.boost[id]').filter")]);
add(['message update preserves the input state','boost by another user preserves the input state'],'boosts',"'value','Hey!'");
add(['the toolbar stays hidden until hover or focus and labels every action'],'toolbar',"page.locator('.message__toolbar'),0");
add(['quick-react creates a boost from the toolbar','keyboard users reach the toolbar from a focused message','the emoji picker searches and reacts','the picker remembers recent reactions'],'toolbar','reaction-chip__count');
add(['reply and thread buttons drive the composer and the thread panel'],'toolbar','[data-thread-panel-target="create"]');
add(['the more button opens the shared menu for its message'],'actions',"page.locator('#message-actions-menu:not([hidden])')");
// onContextMenu is used only by the initial right click. Keyboard and long
// press call the same menu assertion through unaffected handlers.
D.set('opens message actions from context menu and keyboard, and cancels a moving long press',[{
  ...target('actions','await assertMenuOpen(page)'),
  phase:target('actions','await openMenu(page)'),
}]);
add(['the picker shows category tabs and switches between them'],'toolbar',"await tab('people')");
add(['the picker loads its emoji data only on first open'],'toolbar',"assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),false)");
add(['the picker Custom tab reacts with a workspace icon'],'toolbar',"option('Acme Corp').locator('img')");
add(['the picker reacts with a brand icon shortcode'],'toolbar',"option('OpenAI').locator('img')");
add(['picker arrows move through options, Enter selects, and Escape returns focus'],'toolbar','document.activeElement?.getAttribute');
add(['picker tabs move with arrow keys and switch the grid'],'toolbar','document.activeElement?.id');
add(['message action menu is a bottom sheet with touch-sized targets on phones','shows the message action menu as a bottom sheet on phones'],'actions','viewport.height-menu.bottom');
add(['message action menu stays a floating popover on desktop'],'actions','g.menu.width<g.viewport.width');
D.set('a release click landing on the just-opened menu does not activate it',[target('actions','await assertMenuOpen(page)'),target('actions','[data-composer-target="context"][hidden]')]);
add(['edits through the normal composer and restores the saved draft on cancel and success'],'actions','await restoredDraftAfterCancel()');
add(['a duplicate delivery does not replace the message while its actions are open'],'actions','window.originalDeliveredMessage.isConnected');
add(['keeps newer typing through an asynchronous edit and leaves failures in edit mode'],'actions',"field(page,'A newer draft typed while saving'");
add(['replies with notify off and renders a tombstone when the target is deleted'],'actions',"row,{state:'hidden'}");
add(['copies message text and link and forwards to a server-provided thread destination'],'actions','window.interactionsCopied),');
add(['forwarding twice in a row submits only once'],'actions','submit.locator(\':scope:disabled\')');
add(['groups emoji reactions, updates the live count, and highlights the current user'],'actions','reaction-chip__count');
add(['language fences highlight common code without changing its text','unlabelled code is detected while text unknown languages and inline code stay literal','search results highlight code on initial load and after returning to the channel','editing a code block replaces its language colors and copied source'],'code','waitForVisibility(code,');
for(const name of ['search results highlight code on initial load and after returning to the channel','editing a code block replaces its language colors and copied source']) D.set(name,[target('code','code.locator(\'.code-token\')')]);
D.set('unlabelled code is detected while text unknown languages and inline code stay literal',[target('code',"filterVisibleText(marked(row,'text')")]);
add(['code and copying remain available when the highlighter cannot load'],'code','window.copiedCode),expected');
add(['a stray create re-entry does not wipe the half-filled thread name'],'','the completed name must survive until submission');
add(['Markdown messages reach other users and editing preserves the original source'],'','pre code.language-javascript[data-highlighted="yes"]');
add(['Markdown replies and file attachments remain usable'],'',"author.locator('.message[data-message-id] .message__reply-preview'),'A useful point'");
add(['mention suggestions select a room member without sending the unfinished message'],'','suggestion-option');
add(['a rejected message can be recovered corrected and sent'],'','field(author,invalid)');
add(['sending preserves the submitted source and a newer draft'],'','field(author,second)');
add(['search tolerates operators, shows an empty state and pages older results'],'search-forward',"byVisibleText(page.locator('#search-results'),'system paging alpha'");
add(['forwarded Markdown keeps tables and code blocks'],'search-forward','Forwarded to 1 destination');
add(['editing to add a URL renders its card live and the edited marker on load'],'search-forward',"'Loading post'");
add(['few unread render the divider above the first new message and keep the bottom scroll'],'unread','node.nextElementSibling');
add(['mark unread from the message menu points the divider at that message'],'unread','above(fixture.target_id)');
add(['many unread scroll the room to the divider'],'unread','geometry.top<geometry.height/2');
add(['the jump pill shows while the divider is off-screen and returns to it'],'unread',"pill,{state:'hidden'");
add(['unread older than the last page keeps the last page and the pill links to the first unread'],'unread',"searchParams.get('message_id')");
add(['discusses a pull request from its card'],'','github-pr-thread-header');
// Focus helpers are shared by setup and post-mutation assertions. Name the
// post-action call site, so failure of the earlier setup focus earns no credit.
add(['arrow keys move between messages'],'message-list','focused(second)');
add(['deleting the focused message moves focus to the surviving tab stop'],'message-list','focused(third,10000)');
add(['deleting an older focused message hands focus to its neighbour, not the newest'],'message-list','focused(second,10000)');
add(['a late composer autofocus does not steal focus from a message'],'message-list','focused(third)');
for(const name of ['a stream replacing the focused message keeps focus and the tab stop on its replacement','a direct DOM swap of the focused message keeps focus and the tab stop on its replacement']) D.set(name,[target('message-list','focused(second)'),target('message-list','tabStop(second)')]);
D.set('a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement',[target('message-list','tabStop(third)')]);
add(['picker arrows move through options, Enter selects, and Escape returns focus'],'toolbar',"focused('Grinning face with big eyes')");
D.set('the picker remembers recent reactions',[target('toolbar',"await waitForVisibility(option('Grinning face'))")]);
D.set('picker tabs move with arrow keys and switch the grid',[target('toolbar',"await tab('people')")]);
// Continuation -2 retains its additional paired/controller scopes.
add(workControllerCases,'work-controllers',"assert.equal(body.thread.name,'Design discussion')");
add(['thread code stays readable in both themes and scrolls within a narrow screen'],'code',"code.locator('.code-token')");
// The fieldset min-width probe overflows the scrollable main element, not
// the clipped document. This is the original content-overflow assertion
// (mobile_layout_test.rb:56); an earlier document assertion is not its target.
add(['the profile page fits phone widths without scrolling sideways'],'mobile-continuation','assert.ok(result.mainOverflow<=0');
add(['headers outside the workspace shell stay opaque over scrolled content'],'mobile-continuation',"includes(header.background)");
add(['headers outside the workspace shell never cover the page or its scrollbar'],'mobile-continuation','assert.ok(layout.panelTop>=layout.navBottom');
add(['pages outside the workspace shell show no drawer toggle that opens nothing'],'mobile-continuation',"waitForVisibleCount(page.getByRole('button',{name:'Open workspace navigation'");
add(['every drawer destination has one toggle that opens the drawer on itself'],'mobile-continuation',"click(page.getByRole('button',{name:'Open workspace navigation'");
add(['text fields stay at 16px on touch devices without changing the desktop look'],'mobile-continuation','assert.ok(await size(selector)>=16');
add(['tracks work, assigns an owner, completes and reopens it without losing the conversation'],'thread-continuation',"text(target('workStatusLabel'),'Planned')");
add(['shows work-thread guidance in the new-thread form and on the work page'],'thread-continuation',"text(createForm.locator('details.thread-panel__guide:not([open]) summary')");
add(['keeps the new-thread guidance usable on a phone'],'thread-continuation',"visibleMatch(panel.locator('details.thread-panel__guide summary'))");
add(['shows work assignment activity to the owner and opens the exact thread'],'thread-continuation',"text(target('workOwnerLabel'),'Kevin',10000)");
add(['keeps the thread drawer usable on a phone and preserves the channel'],'',"filterVisibleText(panel.locator('[data-thread-panel-target=\"conversationTitle\"]'),name)");
add(['opens a shared thread message link around an older post'],'thread-continuation',"panel.locator('.message-area__return-to-latest'),{timeout:10000}");
add(['marks a joined thread read only while the conversation is visible'],'thread-continuation','await read(false)');
add(['keeps an anchored older thread unread when a new reply arrives'],'thread-continuation','await read(true)');
// behavior.mjs is the dispatcher (there is no dash in its basename).
for(const specs of D.values()) for(const spec of specs) if(spec.module==='behavior-.mjs') spec.module='behavior.mjs';
const V=new Map();
const variant=(names,module,anchor)=>{for(const name of names) V.set(name,[target(module,anchor)]);};
variant(['missing-const'],'code',"code.locator('.code-token')");
variant(['missing-def'],'code',"await highlight(marked(replacement,'python'),'def')");
variant(['hidden-clapping','transparent-clapping','transparent-reaction-ancestor'],'actions',"fixture.reaction_count");
variant(['delayed-boost-write'],'toolbar','reaction-chip__count');
variant(['transparent-edit-field'],'actions',"actOnVisible(editor,'fill'");
variant(['transparent-cancelled-draft'],'actions','await restoredDraftAfterCancel()');
variant(['transparent-saved-draft'],'actions','await restoredDraftAfterSuccess()');
variant(['transparent-redelivery-field'],'actions',`field(page,"Third time's a charm.")`);
// Ruby :166 explicitly asserts this checkpoint with default visibility.
variant(['transparent-redelivery-flag'],'actions',"page.locator('html[data-duplicate-delivery-rendered]')");
variant(['transparent-newer-draft'],'actions',"field(page,'A newer draft typed while saving'");
variant(['transparent-cleared-code-field'],'code','actOnVisible(page.getByRole(\'combobox\'');
variant(['transparent-notify-field'],'actions',"page.getByLabel('Notify author'");
variant(['transparent-menu-owner','transparent-more-owner'],'actions',"waitForVisibility(page.locator('.message[data-message-actions-open]'),{timeout:CAPYBARA_DEFAULT})");
variant(['transparent-ruby-code'],'search-forward',"source.locator('pre code')");
variant(['transparent-forwarded-code'],'search-forward',"forwarded.locator('pre code.language-ruby')");
variant(['transparent-search-back-link'],'code',"name:'Back to Designers'");
variant(['transparent-project-link'],'','message.getByRole(\'link\',');
variant(['transparent-search-field'],'search-forward',"actOnVisible(search,'fill'");
variant(['transparent-forward-destination'],'actions','message-forward-dialog__destination');
variant(['transparent-context-body'],'actions',"row.locator('[data-reply-target=\"body\"]')");
variant(['transparent-forward-checkbox'],'actions','message-forward-dialog__destination input');
variant(['transparent-quick-thumb'],'actions','message__quick-reaction[title="Thumbs up"]');
variant(['transparent-picker-search'],'toolbar',"actOnVisible(search,'press'");
variant(['transparent-flags-lookup'],'toolbar','emoji-picker-tab-flags');
variant(['transparent-hover-body'],'toolbar',"row.locator('[data-reply-target=\"body\"]')");
variant(['transparent-room-header'],'actions',"page.locator('.room-header__name')");
variant(['hidden-code-visible-token'],'','waitForVisibility(code,');
variant(['transparent-initial-heading'],'','message.locator(\'h2\')');
variant(['hidden-body-visible-presentation'],'','waitForVisibleContentCount(');
variant(['hidden-submitted-body-visible-strong'],'',"await text(page,'First message stays exact.')");
variant(['transparent-thread-body'],'','panel.locator(\'.thread-panel__thread-content .message__body\')');
variant(['hidden-conversation-visible-children'],'','panel.locator(\'[data-thread-panel-target="conversation"]\')');
variant(['hidden-safety-body-visible-code'],'',"message.locator('.message__body'),'Safety check'");
variant(['hidden-reply-body-visible-presentation'],'',"parent.locator('.message__body'),'A useful point'");
variant(['transparent-attachment-reply-preview'],'',"author.locator('.message[data-message-id] .message__reply-preview'),'A useful point'");
variant(['transparent-attachment-filename'],'',"author.locator('.message[data-message-id] .message__body'),'markdown-workspace-attachment.txt'");
variant(['transparent-combobox-lookup'],'composer','waitForVisibility(editor,');
// Global field opacity also invalidates the original visible fill_in before
// its later value assertion. Name that causal action, never a setup elsewhere.
variant(['transparent-restored-thread-draft'],'composer',"actOnVisible(panel.getByRole('combobox',{name:'Write a thread reply'");
variant(['transparent-cleared-thread-draft'],'composer',"waitForVisibleProperty(reply,'value','')");
variant(['transparent-boost-draft-after-edit','transparent-boost-draft-after-delivery'],'boosts',"locator('input[name=\"boost[content]\"]'),'fill'");
variant(['transparent-older-search-text'],'search-forward',"byVisibleText(page.locator('#search-results'),'system paging alpha',{exact:true})");
variant(['transparent-initial-url-text'],'search-forward',"'nothing linked yet'");
variant(['hidden-initial-composer-conversation'],'composer','await initialConversationAfterCreate()');
variant(['delayed-negative-removal'],'message-list',"row(id),{state:'hidden'}");
variant(['hidden-url-loading-text'],'search-forward',"'Loading post'");
variant(['transparent-url-editor'],'',"actOnVisible(page.getByRole('combobox'");
variant(['delayed-url-edited-marker'],'search-forward',"message.locator('.message__edited')");
variant(['transparent-picker-fill'],'toolbar',"actOnVisible(search,'fill'");
variant(['transparent-ban-text'],'message-destinations',"browser.locator('button'),'Ban Kevin'");
variant(['transparent-device-text'],'attach-menu',"menu.locator('[role=\"menuitem\"]'),'From this device'");
variant(['transparent-drive-text','transparent-phone-drive-text'],'attach-menu',"menu.locator('[role=\"menuitem\"]'),'From Google Drive'");
variant(['transparent-boost-delete-text'],'boosts',"boost.locator('button'),'Delete this boost'");
variant(['wrong-permission-status','wrong-validation-status'],'work-controllers','assert.equal(response.status,expected');
variant(['delayed-workspace-mobile-draft','unrelated-workspace-mobile-body'],'workspace','page.locator(`.message[data-message-id=');
variant(['slow-workspace-animation'],'workspace','await settled()');
for(const specs of V.values()) for(const spec of specs) if(spec.module==='behavior-.mjs') spec.module='behavior.mjs';
for(const [name,label] of [
  ['missing-thumb-aria-label','React with thumbs up'],['missing-picker-aria-label','Add reaction'],
  ['missing-reply-aria-label','Reply to message'],['missing-thread-aria-label','Open thread'],['missing-more-aria-label','More message actions'],
]) variant([name],'toolbar',`row.locator('button[aria-label="${label}"]`);
variant(['missing-option-aria-label'],'toolbar',"await waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT})");
// More also requires the literal aria-haspopup attribute in the same selector.
variant(['missing-more-aria-label'],'toolbar',`row.locator('button[aria-label="More message actions"][aria-haspopup="menu"]')`);
export function mutationTarget(caseName,variant) {
  const keywordCases={
    'editing a code block replaces its language colors and copied source': "await highlight(marked(row,'ts'),'const')",
    'search results highlight code on initial load and after returning to the channel': "await highlight(marked(row,'javascript'),'const')",
  };
  const specs=(variant==='default'||variant==='missing-const')&&keywordCases[caseName]
    ?[target('code',keywordCases[caseName])]:variant==='default'?D.get(caseName):V.get(variant);
  if(!specs) throw new Error(`Missing intended assertion for ${caseName}: ${variant}`);
  return specs;
}
export function rejectionEvidence(caseName,variant,probe,error) {
  const expected=mutationTarget(caseName,variant),frames=assertionFrames(error);
  const nativeMotion=caseName===motionCases[0]&&variant==='default';
  const nativeMatch=nativeMotion&&probe.nativeFailures?.some(failure=>failure.assertion&&failure.message.includes(expected[0].native.message)&&failure.backtrace.some(frame=>frame.includes(`${expected[0].native.source}:${expected[0].native.line}:`)));
  const matched=nativeMotion?nativeMatch:expected.find(target=>frames.some(frame=>frame.module===target.module&&frame.source.includes(target.anchor))&&(!target.phase||frames.some(frame=>frame.module===target.phase.module&&frame.source.includes(target.phase.anchor))));
  const reasons=[];
  if(nativeMotion&&!probe.observed?.some(state=>state.room==='/rooms/201306877'&&state.open&&state.duration==='0s'&&['none','matrix(1, 0, 0, 1, 0, 0)'].includes(state.transform))) reasons.push('native motion mutation not encountered');
  if(!probe.ready) reasons.push('startup failed');
  if(!probe.applied) reasons.push('mutation not served');
  if(probe.networkFailures?.length) reasons.push('network failed');
  if(!error) reasons.push('mutant escaped');
  else if(error.code!=='ERR_ASSERTION'&&error.name!=='TimeoutError') reasons.push('infrastructure failed');
  if(!matched) reasons.push('intended assertion did not fail');
  if(probe.observers?.length&&!probe.observed?.length) reasons.push('mutated DOM state not encountered');
  if(probe.requiredAction&&!probe.observed?.some(state=>state.action===probe.requiredAction.action&&state.target===probe.requiredAction.target)) reasons.push('mutated action not encountered');
  if(probe.requiresReleaseClick&&!probe.observed?.some(state=>state.releaseClick&&state.brokenGuard&&state.menuVisible)) reasons.push('mutated release click not encountered');
  if(probe.requiredMessageText&&!probe.observed?.some(state=>state.messageText===probe.requiredMessageText&&(state.opacity==='0'||state.visibility==='hidden'||state.display==='none'))) reasons.push('intended message mutation state not encountered');
  if(probe.requiresHiddenState&&!probe.observed?.some(state=>state.opacity==='0'||state.visibility==='hidden'||state.display==='none')) reasons.push('hidden mutation state not encountered');
  if(probe.requiresAttachmentReplyFault&&!probe.observed?.some(state=>/^\d+$/.test(String(state.originalReplyId))&&state.forcedReply===null)) reasons.push('nonnull attachment reply was not discarded by the mutant');
  if(probe.requiresMotionAttribute&&!probe.observed?.some(state=>state.before==='off'&&state.after===undefined)) reasons.push('server attribute was not removed by the mutant');
  if(probe.requiredAnimation&&!probe.observed?.some(state=>state.animations?.some(animation=>animation.name===probe.requiredAnimation.name&&animation.duration===probe.requiredAnimation.duration&&animation.playState==='running'))) reasons.push('intended running animation not encountered');
  return {valid:reasons.length===0,reasons,expected,actual:frames,observed:probe.observed||[]};
}
