// Behaviour equivalents of composer_test.rb at d7c7de92. Real requests,
// delivered replies, Cable typing and Turbo visits; no screenshot assertions.
import assert from 'node:assert/strict';
import {CAPYBARA_DEFAULT,DELIVERY_WAIT} from './behavior-deadlines.mjs';
import {waitForVisibility,waitForVisibleCount,waitForVisibleProperty,actOnVisible,filterVisibleText,waitForCondition} from './behavior-visibility.mjs';
export async function composer({author:page,recipient,base,caseName,fixture,viewer,send,text,field}) {
  const editor=page.getByRole('combobox',{name:'Write a message',exact:true});
  const root=page.locator('.message[data-message-id="607264868"]');
  async function openMenu(message) {
    await waitForVisibility(message.locator(':scope[aria-haspopup="menu"]'));
    await actOnVisible(message.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
    await waitForVisibility(page.locator('#message-actions-menu:not([hidden])'),{timeout:10000});
  }
  async function reply(body) {
    await openMenu(root);await actOnVisible(page.getByRole('menuitem',{name:'Reply',exact:true}),'click',{});
    await waitForVisibility(filterVisibleText(page.locator('[data-composer-target="contextLabel"]'),'Replying to'),{timeout:DELIVERY_WAIT});
    await send(page,body,{timeout:DELIVERY_WAIT});await text(recipient,body,1,{timeout:DELIVERY_WAIT});
    const message=page.locator('.message[data-message-id]').filter({has:filterVisibleText(page.locator('[data-reply-target="body"]'),body)});
    await waitForVisibility(filterVisibleText(message.locator('.message__reply-preview'),"Third time's a charm."),{timeout:DELIVERY_WAIT});
    return message;
  }
  async function room(id) {
    // Follow a real same-origin Turbo link, preserving drafts across visits.
    await actOnVisible(page.locator(`a[href="/rooms/${id}"]`).first(),'click',{});
    await page.waitForURL(base+`/rooms/${id}`);
    await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));
    await waitForVisibility(page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]'),{state:'attached'});
  }
  if(caseName==='blurring an open autocomplete does not leave a zombie that swallows Enter') {
    await actOnVisible(editor,'fill',{},[':thu']);await waitForVisibility(filterVisibleText(page.locator('suggestion-option'),'thumbsup'));
    await editor.evaluate(node=>node.blur());await waitForVisibleCount(page.locator('suggestion-option'),0);
    await actOnVisible(editor,'click',{});await actOnVisible(editor,'fill',{},['hello']);await actOnVisible(editor,'press',{},['Enter']);
    await text(page,'hello',1,{timeout:DELIVERY_WAIT});await text(recipient,'hello',1,{timeout:DELIVERY_WAIT});await field(page,'');
  } else if(caseName==='a stale icon response does not poison the suggestion commit') {
    await page.evaluate(()=>{
      const original=window.fetch;
      window.fetch=(input,init)=>{
        const url=typeof input==='string'?input:input.url;
        if(url.includes('/autocompletable/icons')&&/[?&]q=zx(&|$)/.test(url)) return new Promise(resolve=>setTimeout(()=>resolve(original(input,init)),1500));
        return original(input,init);
      };
    });
    await actOnVisible(editor,'fill',{},[':zx']);await page.waitForTimeout(600);await actOnVisible(editor,'fill',{},[':open']);
    await waitForVisibility(filterVisibleText(page.locator('suggestion-option'),'OpenAI'));
    await page.waitForTimeout(1600);await actOnVisible(editor,'press',{},['Enter']);await field(page,':openai: ');
  } else if(caseName==='mention queries are URL-encoded') {
    const requests=[];page.on('request',request=>{if(request.url().includes('/autocompletable/users')) requests.push(request.url());});
    await actOnVisible(editor,'fill',{},['@kev+in']);
    await waitForCondition(()=>requests.length>0);
    assert.ok(requests.some(url=>url.includes('query=kev%2Bin')),`encoded query expected in ${JSON.stringify(requests)}`);
  } else if(caseName==='composer autocomplete exposes combobox semantics over a polite listbox') {
    await waitForVisibility(editor,{timeout:CAPYBARA_DEFAULT});
    assert.equal(await editor.getAttribute('role'),'combobox');assert.equal(await editor.getAttribute('aria-autocomplete'),'list');assert.equal(await editor.getAttribute('aria-expanded'),'false');
    await actOnVisible(editor,'fill',{},[':open']);await waitForVisibility(filterVisibleText(page.locator('suggestion-option'),'OpenAI'));
    assert.equal(await editor.getAttribute('aria-expanded'),'true');
    const id=await editor.getAttribute('aria-controls');assert.ok(id);
    await waitForVisibility(page.locator(`[id="${id}"][role="listbox"][aria-live="polite"]`));
    await waitForVisibility(page.locator(`[id="${id}"] suggestion-option[role="option"]`).first());
    const first=await editor.getAttribute('aria-activedescendant');assert.ok(first);
    await waitForVisibility(page.locator(`suggestion-option[id="${first}"][selected]`));
    await actOnVisible(editor,'press',{},['ArrowDown']);const second=await editor.getAttribute('aria-activedescendant');assert.ok(second);assert.notEqual(second,first);
    await actOnVisible(editor,'press',{},['Escape']);assert.equal(await editor.getAttribute('aria-expanded'),'false');assert.equal(await editor.getAttribute('aria-activedescendant'),null);
  } else if(caseName==='composing text does not commit a suggestion or send the message') {
    assert.equal(await page.evaluate(()=>new KeyboardEvent('x',{isComposing:true}).isComposing),true);
    await actOnVisible(editor,'fill',{},[':open']);await waitForVisibility(filterVisibleText(page.locator('suggestion-option'),'OpenAI'));
    await editor.evaluate(node=>node.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',keyCode:13,bubbles:true,cancelable:true,isComposing:true})));
    await field(page,':open');await waitForVisibility(filterVisibleText(page.locator('suggestion-option'),'OpenAI'));
    await page.waitForTimeout(500);await waitForVisibleCount(filterVisibleText(page.locator('.message[data-message-id]'),/^:open$/),0);
    await actOnVisible(editor,'press',{},['Enter']);await field(page,':openai: ');
  } else if(caseName==='clicking a reply preview scrolls to the loaded message instead of navigating') {
    const message=await reply('A reply for preview click');
    await actOnVisible(message.locator('.message__reply-preview-link'),'click',{});await waitForVisibility(root.locator(':scope.message--reply-target'),{timeout:DELIVERY_WAIT});
    assert.equal(new URL(page.url()).pathname,'/rooms/654632876');
  } else if(caseName==='clicking a reply preview falls back to the permalink when the target is not loaded') {
    const message=await reply('A reply for preview fallback');await root.evaluate(node=>node.remove());await waitForVisibility(root,{state:'hidden'});
    await page.evaluate(()=>{
      window.__composerTestUrls=[];
      document.addEventListener('turbo:before-fetch-request',event=>{
        if(new Headers(event.detail.fetchOptions.headers).get('X-Sec-Purpose')!=='prefetch') window.__composerTestUrls.push(String(event.detail.url));
      });
    });
    const permalink=await message.locator('.message__reply-preview-link').getAttribute('href');
    await actOnVisible(message.locator('.message__reply-preview-link'),'click',{});
    // The reply belongs to the messages Turbo Frame, so the actual click
    // fetch does not emit a Drive before-visit. Ignore hover prefetches and
    // require the exact permalink in a real navigation fetch instead.
    await page.waitForFunction(href=>window.__composerTestUrls.some(url=>new URL(url,location.origin).href===new URL(href,location.origin).href),permalink);
    await page.waitForFunction(()=>window.__composerTestUrls.some(url=>url.includes('/@')));
    await waitForVisibleCount(page.locator('.message--reply-target'),0);
  } else if(caseName==='deleting a replied-to message turns open reply previews into a tombstone') {
    const message=await reply('A reply whose source goes away');await openMenu(root);
    page.once('dialog',dialog=>dialog.accept());await actOnVisible(page.getByRole('menuitem',{name:'Delete message',exact:true}),'click',{});await waitForVisibility(root,{state:'hidden',timeout:DELIVERY_WAIT});
    for(const browser of [page,recipient]) {
      const delivered=browser.locator('.message[data-message-id]').filter({has:filterVisibleText(browser.locator('[data-reply-target="body"]'),'A reply whose source goes away')});
      await waitForVisibility(filterVisibleText(delivered.locator('.message__reply-preview'),'Replying to a deleted message'),{timeout:DELIVERY_WAIT});
      await waitForVisibleCount(delivered.locator('.message__reply-preview-link'),0);
    }
  } else if(caseName==='two typers with the same name do not merge') {
    const david=await viewer('David');
    for(const browser of [recipient,david]) await browser.waitForFunction(()=>{
      const node=document.querySelector('[data-controller~="typing-notifications"]');
      return window.Stimulus?.getControllerForElementAndIdentifier(node,'typing-notifications')?.channel;
    });
    await actOnVisible(david.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['hi from david']);
    await actOnVisible(recipient.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['hi from kevin']);
    await waitForVisibility(filterVisibleText(page.locator('[data-typing-notifications-target="author"]'),/^David, David$/),{timeout:10000});
    await actOnVisible(recipient.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['hi from kevin']);
    await actOnVisible(david.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['']);
    await waitForVisibility(filterVisibleText(page.locator('[data-typing-notifications-target="author"]'),/^David$/),{timeout:10000});
    await actOnVisible(recipient.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['']);
    await waitForVisibility(page.locator('.typing-indicator--active'),{state:'hidden',timeout:10000});
  } else if(caseName==='composer drafts persist per room and clear on send') {
    await actOnVisible(editor,'fill',{},['Designers draft']);
    assert.equal(await page.evaluate(()=>localStorage.getItem('campfire.composer.draft.773523953.654632876.main')),'Designers draft');
    await room(fixture.pets_id);await field(page,'');await actOnVisible(editor,'fill',{},['Pets draft']);
    await room(654632876);await field(page,'Designers draft');
    await room(fixture.pets_id);await field(page,'Pets draft');await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});await text(page,'Pets draft',1,{timeout:DELIVERY_WAIT});
    await room(654632876);await field(page,'Designers draft');await room(fixture.pets_id);await field(page,'');
  } else if(caseName==='thread drafts persist per thread without touching the channel draft') {
    const panel=page.locator('#thread-panel');
    async function threads() {
      await actOnVisible(page.locator('[data-thread-panel-target="browserToggle"]'),'click');await waitForVisibility(panel.locator(':scope[aria-hidden="false"]'),{timeout:DELIVERY_WAIT});
    }
    async function thread() {
      await threads();await actOnVisible(filterVisibleText(panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item'),'Composer draft thread'),'click',{});
      await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT});
      await waitForVisibility(panel.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]'),{state:'attached'});
    }
    await threads();await actOnVisible(panel.getByRole('button',{name:'New thread',exact:true}),'click',{});
    await waitForVisibility(panel.locator('[data-thread-panel-target="create"]'),{timeout:DELIVERY_WAIT});
    await page.waitForFunction(()=>document.activeElement===document.querySelector('[data-thread-panel-target="createMessage"]'));
    await actOnVisible(panel.locator('[data-thread-panel-target="createName"]'),'fill',{},['Composer draft thread']);
    await actOnVisible(panel.locator('[data-thread-panel-target="createMessage"]'),'fill',{},['The thread for draft persistence.']);
    assert.equal(await panel.locator('[data-thread-panel-target="createName"]').inputValue(),'Composer draft thread');
    await actOnVisible(panel.locator('[data-thread-panel-target="createSubmit"]'),'click',{});
    await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT});
    await waitForVisibility(filterVisibleText(panel.locator('[data-thread-panel-target="conversationTitle"]'),'Composer draft thread'));
    await actOnVisible(panel.getByRole('combobox',{name:'Write a thread reply',exact:true}),'fill',{},['Thread draft']);await actOnVisible(editor,'fill',{},['Channel draft']);
    await room(fixture.pets_id);await room(654632876);await field(page,'Channel draft');await thread();
    const reply=panel.getByRole('combobox',{name:'Write a thread reply',exact:true});await waitForVisibleProperty(reply,'value','Thread draft');
    await actOnVisible(reply,'fill',{},['Thread draft sent']);await actOnVisible(panel.getByRole('button',{name:'Send Reply',exact:true}),'click',{});
    await waitForVisibility(filterVisibleText(panel.locator('.message__body'),'Thread draft sent'),{timeout:DELIVERY_WAIT});
    await room(fixture.pets_id);await room(654632876);await thread();await waitForVisibleProperty(reply,'value','');
  } else throw new Error(`unimplemented composer case ${caseName}`);
}
