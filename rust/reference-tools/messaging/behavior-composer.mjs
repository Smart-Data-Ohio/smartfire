// Behaviour equivalents of composer_test.rb at d7c7de92. Real requests,
// delivered replies, Cable typing and Turbo visits; no screenshot assertions.
import assert from 'node:assert/strict';
export async function composer({author:page,recipient,base,caseName,fixture,viewer,send,text,field}) {
  const editor=page.getByRole('combobox',{name:'Write a message',exact:true});
  const root=page.locator('.message[data-message-id="607264868"]');
  async function openMenu(message) {
    await message.locator(':scope[aria-haspopup="menu"]').waitFor();
    await message.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await page.locator('#message-actions-menu:not([hidden])').waitFor();
  }
  async function reply(body) {
    await openMenu(root);await page.getByRole('menuitem',{name:'Reply',exact:true}).click();
    await page.locator('[data-composer-target="contextLabel"]').filter({hasText:'Replying to'}).waitFor();
    await send(page,body);await text(recipient,body);
    const message=page.locator('.message[data-message-id]').filter({has:page.locator('[data-reply-target="body"]').filter({hasText:body})});
    await message.locator('.message__reply-preview').filter({hasText:"Third time's a charm."}).waitFor();
    return message;
  }
  async function room(id) {
    // Follow a real same-origin Turbo link, preserving drafts across visits.
    await page.locator(`a[href="/rooms/${id}"]`).first().click();
    await page.waitForURL(base+`/rooms/${id}`);
    await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('composer'),'composer'));
    await page.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]').waitFor({state:'attached'});
  }
  if(caseName==='blurring an open autocomplete does not leave a zombie that swallows Enter') {
    await editor.fill(':thu');await page.locator('suggestion-option').filter({hasText:'thumbsup'}).waitFor();
    await editor.evaluate(node=>node.blur());await page.waitForFunction(()=>!document.querySelector('suggestion-option'));
    await editor.click();await editor.fill('hello');await editor.press('Enter');
    await text(page,'hello');await text(recipient,'hello');await field(page,'');
  } else if(caseName==='a stale icon response does not poison the suggestion commit') {
    await page.evaluate(()=>{
      const original=window.fetch;
      window.fetch=(input,init)=>{
        const url=typeof input==='string'?input:input.url;
        if(url.includes('/autocompletable/icons')&&/[?&]q=zx(&|$)/.test(url)) return new Promise(resolve=>setTimeout(()=>resolve(original(input,init)),1500));
        return original(input,init);
      };
    });
    await editor.fill(':zx');await page.waitForTimeout(600);await editor.fill(':open');
    await page.locator('suggestion-option').filter({hasText:'OpenAI'}).waitFor();
    await page.waitForTimeout(1600);await editor.press('Enter');await field(page,':openai: ');
  } else if(caseName==='mention queries are URL-encoded') {
    const requests=[];page.on('request',request=>{if(request.url().includes('/autocompletable/users')) requests.push(request.url());});
    const responded=page.waitForResponse(response=>response.url().includes('/autocompletable/users'));
    await editor.fill('@kev+in');
    await responded;
    assert.ok(requests.some(url=>url.includes('query=kev%2Bin')),`encoded query expected in ${JSON.stringify(requests)}`);
  } else if(caseName==='composer autocomplete exposes combobox semantics over a polite listbox') {
    assert.equal(await editor.getAttribute('role'),'combobox');assert.equal(await editor.getAttribute('aria-autocomplete'),'list');assert.equal(await editor.getAttribute('aria-expanded'),'false');
    await editor.fill(':open');await page.locator('suggestion-option').filter({hasText:'OpenAI'}).waitFor();
    assert.equal(await editor.getAttribute('aria-expanded'),'true');
    const id=await editor.getAttribute('aria-controls');assert.ok(id);
    await page.locator(`[id="${id}"][role="listbox"][aria-live="polite"]`).waitFor();
    await page.locator(`[id="${id}"] suggestion-option[role="option"]`).first().waitFor();
    const first=await editor.getAttribute('aria-activedescendant');assert.ok(first);
    await page.locator(`suggestion-option[id="${first}"][selected]`).waitFor();
    await editor.press('ArrowDown');const second=await editor.getAttribute('aria-activedescendant');assert.ok(second);assert.notEqual(second,first);
    await editor.press('Escape');assert.equal(await editor.getAttribute('aria-expanded'),'false');assert.equal(await editor.getAttribute('aria-activedescendant'),null);
  } else if(caseName==='composing text does not commit a suggestion or send the message') {
    assert.equal(await page.evaluate(()=>new KeyboardEvent('x',{isComposing:true}).isComposing),true);
    await editor.fill(':open');await page.locator('suggestion-option').filter({hasText:'OpenAI'}).waitFor();
    await editor.evaluate(node=>node.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',keyCode:13,bubbles:true,cancelable:true,isComposing:true})));
    await field(page,':open');await page.locator('suggestion-option').filter({hasText:'OpenAI'}).waitFor();
    await page.waitForTimeout(500);assert.equal(await page.locator('.message[data-message-id]').filter({hasText:/^:open$/}).count(),0);
    await editor.press('Enter');await field(page,':openai: ');
  } else if(caseName==='clicking a reply preview scrolls to the loaded message instead of navigating') {
    const message=await reply('A reply for preview click');
    await message.locator('.message__reply-preview-link').click();await root.locator(':scope.message--reply-target').waitFor();
    assert.equal(new URL(page.url()).pathname,'/rooms/654632876');
  } else if(caseName==='clicking a reply preview falls back to the permalink when the target is not loaded') {
    const message=await reply('A reply for preview fallback');await root.evaluate(node=>node.remove());await root.waitFor({state:'detached'});
    await page.evaluate(()=>{
      window.__composerTestUrls=[];
      document.addEventListener('turbo:before-fetch-request',event=>window.__composerTestUrls.push(String(event.detail.url)));
    });
    await message.locator('.message__reply-preview-link').click();
    await page.waitForFunction(()=>window.__composerTestUrls.some(url=>url.includes('/@')));
    assert.equal(await page.locator('.message--reply-target').count(),0);
  } else if(caseName==='deleting a replied-to message turns open reply previews into a tombstone') {
    const message=await reply('A reply whose source goes away');await openMenu(root);
    page.once('dialog',dialog=>dialog.accept());await page.getByRole('menuitem',{name:'Delete message',exact:true}).click();await root.waitFor({state:'detached'});
    for(const browser of [page,recipient]) {
      const delivered=browser.locator('.message[data-message-id]').filter({has:browser.locator('[data-reply-target="body"]').filter({hasText:'A reply whose source goes away'})});
      await delivered.locator('.message__reply-preview').filter({hasText:'Replying to a deleted message'}).waitFor();
      assert.equal(await delivered.locator('.message__reply-preview-link').count(),0);
    }
  } else if(caseName==='two typers with the same name do not merge') {
    const david=await viewer('David');
    for(const browser of [recipient,david]) await browser.waitForFunction(()=>{
      const node=document.querySelector('[data-controller~="typing-notifications"]');
      return window.Stimulus?.getControllerForElementAndIdentifier(node,'typing-notifications')?.channel;
    });
    await david.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from david');
    await recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from kevin');
    await page.waitForFunction(()=>document.querySelector('[data-typing-notifications-target="author"]').textContent==='David, David');
    await recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('hi from kevin');
    await david.getByRole('combobox',{name:'Write a message',exact:true}).fill('');
    await page.waitForFunction(()=>document.querySelector('[data-typing-notifications-target="author"]').textContent==='David');
    await recipient.getByRole('combobox',{name:'Write a message',exact:true}).fill('');
    await page.locator('.typing-indicator--active').waitFor({state:'detached'});
  } else if(caseName==='composer drafts persist per room and clear on send') {
    await editor.fill('Designers draft');
    assert.equal(await page.evaluate(()=>localStorage.getItem('campfire.composer.draft.773523953.654632876.main')),'Designers draft');
    await room(fixture.pets_id);await field(page,'');await editor.fill('Pets draft');
    await room(654632876);await field(page,'Designers draft');
    await room(fixture.pets_id);await field(page,'Pets draft');await page.getByRole('button',{name:'Send Message',exact:true}).click();await text(page,'Pets draft');
    await room(654632876);await field(page,'Designers draft');await room(fixture.pets_id);await field(page,'');
  } else if(caseName==='thread drafts persist per thread without touching the channel draft') {
    const panel=page.locator('#thread-panel');
    async function threads() {
      await page.locator('[data-thread-panel-target="browserToggle"]:visible').click();await panel.locator(':scope[aria-hidden="false"]').waitFor();
    }
    async function thread() {
      await threads();await panel.locator('[data-thread-panel-target="browserList"] .thread-panel__thread-item').filter({hasText:'Composer draft thread'}).click();
      await panel.locator('[data-thread-panel-target="conversation"]:visible').waitFor();
      await panel.locator('turbo-cable-stream-source[channel="RoomMessagesChannel"][connected]').waitFor({state:'attached'});
    }
    await threads();await panel.getByRole('button',{name:'New thread',exact:true}).click();
    await panel.locator('[data-thread-panel-target="create"]:visible').waitFor();
    await page.waitForFunction(()=>document.activeElement===document.querySelector('[data-thread-panel-target="createMessage"]'));
    await panel.locator('[data-thread-panel-target="createName"]').fill('Composer draft thread');
    await panel.locator('[data-thread-panel-target="createMessage"]').fill('The thread for draft persistence.');
    assert.equal(await panel.locator('[data-thread-panel-target="createName"]').inputValue(),'Composer draft thread');
    await panel.locator('[data-thread-panel-target="createSubmit"]').click();
    await panel.locator('[data-thread-panel-target="conversationTitle"]').filter({hasText:'Composer draft thread'}).waitFor();
    await panel.getByRole('combobox',{name:'Write a thread reply',exact:true}).fill('Thread draft');await editor.fill('Channel draft');
    await room(fixture.pets_id);await room(654632876);await field(page,'Channel draft');await thread();
    const reply=panel.getByRole('combobox',{name:'Write a thread reply',exact:true});assert.equal(await reply.inputValue(),'Thread draft');
    await reply.fill('Thread draft sent');await panel.getByRole('button',{name:'Send Reply',exact:true}).click();
    await panel.locator('.message__body').filter({hasText:'Thread draft sent'}).waitFor();
    await room(fixture.pets_id);await room(654632876);await thread();assert.equal(await reply.inputValue(),'');
  } else throw new Error(`unimplemented composer case ${caseName}`);
}
