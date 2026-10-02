// Behaviour equivalents of message_interactions, message_toolbar and
// message_actions_mobile at d7c7de92. No screenshots or pixel comparisons.
import assert from 'node:assert/strict';
export const originalMessage=page=>page.locator('.message[data-message-id="607264868"]');
export async function openMenu(page) {
  const row=originalMessage(page);
  await row.locator(':scope[aria-haspopup="menu"]').waitFor();
  await row.locator('[data-reply-target="body"]').click({button:'right'});
  await page.locator('#message-actions-menu:not([hidden])').waitFor();
  // Other viewers may react/copy while edit remains author-only. Geometry
  // waits separately for the author's metadata-dependent Edit action.
  await page.getByRole('menuitem',{name:'Copy text',exact:true}).waitFor();
}
export async function closedMenu(page) {
  await page.locator('.message[data-message-actions-open]').waitFor({state:'detached'});
}
export async function menuGeometry(page) {
  await page.locator('#message-actions-menu .message__edit-action').waitFor();
  await page.locator('#message-actions-menu').evaluate(async menu=>{
    await Promise.all(menu.getAnimations().map(animation=>animation.finished));
  });
  return page.locator('#message-actions-menu').evaluate(menu=>{
    const bounds=el=>{const r=el.getBoundingClientRect();return {left:r.left,top:r.top,right:r.right,bottom:r.bottom,width:r.width,height:r.height};};
    const sizes=selector=>[...menu.querySelectorAll(selector)].filter(el=>el.getClientRects().length).map(bounds);
    const reactions=sizes('.message__quick-reaction');
    return {menu:bounds(menu),reactions,reactionRows:new Set(reactions.map(r=>Math.round(r.top))).size,actions:sizes('.message__menu-action'),viewport:{width:innerWidth,height:innerHeight}};
  });
}
export function bottomSheet(geometry,singleRow=false) {
  const {menu,viewport,reactions,actions}=geometry;
  assert.ok(Math.abs(viewport.width-menu.width)<=1,'phone menu spans the viewport');
  assert.ok(Math.abs(viewport.height-menu.bottom)<=1,'phone menu sits at the bottom');
  assert.ok(menu.height<=viewport.height*.7+1);assert.ok(menu.top>=0&&menu.left>=0);
  assert.ok(reactions.length>0&&actions.length>0);
  for(const r of reactions) assert.ok(r.width>=44&&r.height>=44,'44px quick reaction targets');
  for(const a of actions) assert.ok(a.height>=44,'44px action targets');
  if(singleRow) assert.equal(geometry.reactionRows,1);
}
export async function mobileActions({author:page,caseName}) {
  if(caseName.startsWith('message action menu is')) {
    await page.setViewportSize({width:390,height:844});await openMenu(page);
    bottomSheet(await menuGeometry(page),true);
    await page.locator('.room-header__name').click();await closedMenu(page);
  } else {
    await openMenu(page);const g=await menuGeometry(page);
    assert.ok(g.menu.width<g.viewport.width-1,'desktop menu is a floating popover');
    await page.keyboard.press('Escape');await closedMenu(page);
  }
}
async function longPress(page,move=false) {
  const row=originalMessage(page);await row.scrollIntoViewIfNeeded();
  const b=await row.boundingBox(),x=b.x+b.width/2,y=b.y+b.height/2;
  const cdp=await page.context().newCDPSession(page);
  try {
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y}]});
    if(move) await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:x+25,y}]});
    // Exactly the pin's system_test_helper.rb hold, not an enlarged threshold.
    await new Promise(resolve=>setTimeout(resolve,700));
    await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
  } finally {await cdp.detach();}
}
export async function interactions({author:page,recipient,caseName,fixture,openEdit,send,text,field}) {
  const row=originalMessage(page),editor=page.getByRole('combobox',{name:'Write a message',exact:true});
  if(caseName.startsWith('opens message actions')) {
    await openMenu(page);
    assert.equal(await page.locator('#message-actions-menu .message__quick-reaction').count(),fixture.reaction_count);
    await page.keyboard.press('Escape');await closedMenu(page);
    await row.click();await row.dispatchEvent('keydown',{key:'F10',shiftKey:true,bubbles:true,cancelable:true});
    await page.locator('#message-actions-menu:not([hidden])').waitFor();
    await page.keyboard.press('Escape');await page.waitForFunction(id=>document.activeElement?.id===id,await row.getAttribute('id'));
    await page.setViewportSize({width:390,height:844});await longPress(page,true);await closedMenu(page);
    await longPress(page);await page.locator('#message-actions-menu:not([hidden])').waitFor();
    const g=await menuGeometry(page);
    assert.ok(g.menu.left>=0&&g.menu.top>=0&&g.menu.right<=g.viewport.width&&g.menu.bottom<=g.viewport.height);
    page.once('dialog',dialog=>dialog.dismiss());await page.getByRole('menuitem',{name:'Delete message',exact:true}).click();
    await page.locator('#message-actions-menu .message__delete-action').waitFor();await row.waitFor();
  } else if(caseName.startsWith('a release click')) {
    await page.setViewportSize({width:390,height:844});await longPress(page);
    await page.locator('#message-actions-menu:not([hidden])').waitFor();
    const hit=await row.evaluate(message=>{
      const r=message.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2,target=document.elementFromPoint(x,y);
      if(!target?.closest('#message-actions-menu')) return target?.tagName||'none';
      target.dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true,clientX:x,clientY:y,view:window}));return 'menu';
    });
    assert.equal(hit,'menu');await page.locator('#message-actions-menu:not([hidden])').waitFor();
    await page.locator('#composer [data-composer-target="context"][hidden]').waitFor({state:'attached'});
  } else if(caseName.startsWith('shows the message action')) {
    for(const viewport of [{width:390,height:844},{width:320,height:740}]) {
      await page.setViewportSize(viewport);await openMenu(page);bottomSheet(await menuGeometry(page));
      await page.keyboard.press('Escape');await closedMenu(page);
    }
  } else if(caseName.startsWith('edits through')) {
    await editor.fill('A draft that must survive editing');await openEdit(page,row);
    await field(page,"Third time's a charm.");await page.getByRole('button',{name:'Cancel message context',exact:true}).click();
    await field(page,'A draft that must survive editing');await openEdit(page,row);
    await send(page,'Saved through the main composer');await text(recipient,'Saved through the main composer');
    await page.locator('#composer [data-composer-target="context"][hidden]').waitFor({state:'attached'});
    await field(page,'A draft that must survive editing');
  } else if(caseName.startsWith('a duplicate delivery')) {
    await openMenu(page);
    // The original triggers a redelivery. Feed the actual mounted row to the
    // real Turbo render queue; keep the original object identity witness.
    await row.evaluate(message=>{
      window.originalDeliveredMessage=message;
      document.addEventListener('turbo:before-stream-render',function observe(event) {
        const stream=event.detail.newStream;
        if(stream.getAttribute('action')!=='append'||stream.getAttribute('target')!==message.parentElement.id) return;
        document.removeEventListener('turbo:before-stream-render',observe);
        const render=event.detail.render;
        event.detail.render=async el=>{await render(el);document.documentElement.dataset.duplicateDeliveryRendered='true';};
      });
      Turbo.renderStreamMessage(`<turbo-stream action="append" target="${message.parentElement.id}"><template>${message.outerHTML}</template></turbo-stream>`);
    });
    await page.locator('html[data-duplicate-delivery-rendered]').waitFor({state:'attached'});
    assert.equal(await page.evaluate(()=>window.originalDeliveredMessage.isConnected),true,'redelivery preserves active controls');
    await page.getByRole('menuitem',{name:'Edit message',exact:true}).click();await field(page,"Third time's a charm.");
  } else if(caseName.startsWith('keeps newer typing')) {
    await openEdit(page,row);await editor.fill('First edit request');
    // Same original deterministic PATCH gate/failure, isolated from the
    // separate real-write edit case and its database assertions.
    await page.evaluate(()=>{
      window.originalInteractionsFetch=window.fetch;
      window.fetch=(input,options={})=>options.method==='PATCH'?new Promise(resolve=>{window.resolveInteractionsEdit=resolve;}):window.originalInteractionsFetch(input,options);
    });
    await page.getByRole('button',{name:'Send Message',exact:true}).click();
    await page.waitForFunction(()=>typeof window.resolveInteractionsEdit==='function');
    await editor.fill('A newer draft typed while saving');
    await page.evaluate(()=>window.resolveInteractionsEdit(new Response('{}',{status:200})));
    await field(page,'A newer draft typed while saving');
    await page.locator('#composer [data-composer-target="context"][hidden]').waitFor({state:'attached'});
    await page.evaluate(()=>{
      window.fetch=(input,options={})=>options.method==='PATCH'?Promise.resolve(new Response(JSON.stringify({error:'The message could not be saved'}),{status:422,headers:{'Content-Type':'application/json'}})):window.originalInteractionsFetch(input,options);
    });
    await openEdit(page,row);await editor.fill('Failed edit');await page.getByRole('button',{name:'Send Message',exact:true}).click();
    await page.locator('#composer [data-composer-target="feedback"]').filter({hasText:'The message could not be saved'}).waitFor();
    await page.locator('#composer [data-composer-target="context"]:not([hidden])').waitFor();
    await page.getByRole('button',{name:'Cancel message context',exact:true}).click();
  } else if(caseName.startsWith('replies with notify')) {
    await openMenu(page);await page.getByRole('menuitem',{name:'Reply',exact:true}).click();
    await page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Replying to JZ'}).waitFor();
    assert.equal(await page.getByLabel('Notify author',{exact:true}).isChecked(),true);
    await page.getByLabel('Notify author',{exact:true}).uncheck();await send(page,'A reply without a notification');
    for(const browser of [page,recipient]) await browser.locator('.message__reply-preview').filter({hasText:'Replying to JZ'}).waitFor();
    await openMenu(page);page.once('dialog',dialog=>dialog.accept());await page.getByRole('menuitem',{name:'Delete message',exact:true}).click();
    await row.waitFor({state:'detached'});await page.reload();
    await page.locator('.message__reply-preview').filter({hasText:'Replying to a deleted message'}).waitFor();
  } else if(caseName.startsWith('copies message')) {
    await page.evaluate(()=>Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:text=>{window.interactionsCopied=text;return Promise.resolve();}}}));
    await openMenu(page);await page.getByRole('menuitem',{name:'Copy text',exact:true}).click();
    assert.equal(await page.evaluate(()=>window.interactionsCopied),"Third time's a charm.");
    await openMenu(page);await page.getByRole('menuitem',{name:'Copy link',exact:true}).click();
    assert.ok((await page.evaluate(()=>window.interactionsCopied)).includes(fixture.message_permalink));
    await openMenu(page);await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    const dialog=page.locator('dialog[open]');await dialog.waitFor();
    await dialog.locator('.message-forward-dialog__destination').filter({hasText:'Forward destination'}).click();
    await dialog.getByLabel('Add a note (optional)',{exact:true}).fill('Forwarded from the interaction test');
    await dialog.getByRole('button',{name:'Forward',exact:true}).click();
    await page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:'Forwarded to 1 destination'}).waitFor();
  } else if(caseName.startsWith('forwarding twice')) {
    await openMenu(page);await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    await page.locator('dialog[open]').waitFor();await page.locator('.message-forward-dialog__destination input').first().check();
    const requests=[];page.on('request',request=>{if(request.method()==='POST'&&/\/forwards(?:\.json)?$/.test(new URL(request.url()).pathname)) requests.push(request);});
    const submit=page.locator('[data-message-actions-target="forwardSubmit"]');await submit.click();
    await page.locator('[data-message-actions-target="forwardStatus"]').filter({hasText:'Forwarded to 1 destination'}).waitFor();
    assert.equal(await submit.isDisabled(),true);
    // A real second click is suppressed by the disabled control, not a
    // duplicate handler invocation that bypasses browser disabled semantics.
    await submit.evaluate(button=>button.click());await page.locator('dialog[open]').waitFor({state:'hidden'});
    assert.equal(requests.length,1);
  } else if(caseName.startsWith('groups emoji reactions')) {
    async function reaction(browser,count,active) {
      const chip=originalMessage(browser).locator('.reaction-chip[data-reaction="👍"]');
      await chip.locator('.reaction-chip__count').filter({hasText:new RegExp(`^${count}$`)}).waitFor();
      await browser.waitForFunction(({count,active})=>{
        const chip=document.querySelector('.message[data-message-id="607264868"] .reaction-chip[data-reaction="👍"]');
        return chip?.querySelector('.reaction-chip__count')?.textContent.trim()===String(count)&&chip.classList.contains('reaction-chip--active')===active;
      },{count,active});
    }
    async function quick(browser) {await openMenu(browser);await browser.locator('.message__quick-reaction[title="Thumbs up"]').click();}
    await quick(recipient);await reaction(recipient,1,true);await reaction(page,1,false);
    await quick(page);await reaction(page,2,true);await reaction(recipient,2,true);
    await row.locator('.reaction-chip[data-reaction="👍"]').click();await reaction(page,1,false);await reaction(recipient,1,true);
  } else throw new Error(`unimplemented interaction case ${caseName}`);
}
