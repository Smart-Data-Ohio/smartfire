// Mapped behaviour assertions from test/system/message_list_a11y_test.rb, d7c7de92.
// Synthetic streams/fetch gates below are the original deterministic regressions;
// delivery/edit/pagination also exercise the actual server on each application.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount} from './behavior-visibility.mjs';
export async function messageList({author:page,recipient,caseName,send,text,openEdit,field}) {
  const list=page.locator('.messages[role="log"]').first();
  if(caseName==='the main message list is a live log') {
    // Rails :313 includes hidden containers and does not first find a visible
    // message. The navigation/focus cases below have their own visible scope.
    await waitForVisibility(list.locator(':scope[aria-live="polite"][aria-relevant="additions"]'),{state:'attached',timeout:10000});
    return;
  }
  const editor=page.getByRole('combobox',{name:'Write a message',exact:true});
  await page.waitForFunction(()=>document.activeElement?.id==='message_markdown_source');
  await waitForVisibility(list.locator(':scope > .message[tabindex="0"]').first());
  const ids=await list.locator(':scope > .message').evaluateAll(rows=>rows.map(row=>row.id));
  const [first,second]=ids,third=ids.at(-1);
  assert.ok(ids.length>=3);
  if(!caseName.startsWith('paginated history')) {
    assert.equal(await page.locator(`[id="${third}"]`).getAttribute('data-message-id'),'607264868',
      'the original newest seed message is the initial tab stop');
  }
  const row=id=>page.locator(`[id="${id}"]`);
  async function focus(id) {await row(id).focus();await focused(id);}
  async function focused(id) {await page.waitForFunction(id=>document.activeElement?.id===id,id);}
  async function tabStop(id) {
    await page.waitForFunction(id=>{
      const rows=[...document.querySelectorAll('.messages[role="log"] > .message')];
      return rows.filter(row=>row.tabIndex===0).map(row=>row.id).join()===id;
    },id);
    assert.equal(await row(id).getAttribute('tabindex'),'0');
  }
  async function menu(id) {
    await row(id).locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await waitForVisibility(page.locator('#message-actions-menu:not([hidden])'));
    await waitForVisibility(row(id).locator(':scope[data-message-actions-open]'));
  }
  async function replace(id,direct=false) {
    await page.evaluate(({id,direct})=>{
      const node=document.getElementById(id),clone=node.cloneNode(true);
      clone.dataset.replaced='true';
      if(direct) node.replaceWith(clone);
      else Turbo.renderStreamMessage(`<turbo-stream action="replace" target="${id}"><template>${clone.outerHTML}</template></turbo-stream>`);
    },{id,direct});
    await waitForVisibility(page.locator(`[id="${id}"][data-replaced="true"]`));
  }
  async function remove(id) {
    await page.evaluate(id=>Turbo.renderStreamMessage(`<turbo-stream action="remove" target="${id}"></turbo-stream>`),id);
    await waitForVisibility(row(id),{state:'detached'});
  }
  async function gate() {
    await page.evaluate(()=>{
      window.__actionFetches=0;window.__actionSettled=0;window.__actionGates=[];window.__origFetch=window.fetch;
      window.fetch=(input,init={})=>{
        const url=typeof input==='string'?input:input.url;
        if(url.includes('/actions')) {
          window.__actionFetches++;
          return new Promise(resolve=>window.__actionGates.push(()=>resolve(window.__origFetch(input,init).finally(()=>window.__actionSettled++))));
        }
        return window.__origFetch(input,init);
      };
    });
  }
  async function release() {await page.evaluate(()=>{window.__actionGates.forEach(release=>release());window.__actionGates=[];});}
  async function liveObserver() {
    await waitForVisibility(list.locator(':scope[aria-live="polite"]'));
    await page.evaluate(()=>{
      window.__liveValues=[];
      new MutationObserver(mutations=>{
        for(const mutation of mutations) window.__liveValues.push(mutation.target.getAttribute('aria-live'));
      }).observe(document.querySelector('.messages[role="log"]'),{attributes:true,attributeFilter:['aria-live']});
    });
  }
  if(caseName==='the message list is a single tab stop with a roving tabindex') {
    await tabStop(third);await focus(third);await page.keyboard.press('Tab');
    assert.equal(await page.evaluate(()=>document.activeElement.classList.contains('avatar')),true);
    await page.keyboard.press('Tab');
    assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'React with thumbs up');
  } else if(caseName==='arrow keys move between messages') {
    await focus(first);await page.keyboard.press('ArrowDown');await focused(second);
    await page.keyboard.press('ArrowDown');await focused(ids[2]);
    await page.keyboard.press('ArrowUp');await focused(second);
    await page.keyboard.press('Home');await focused(first);
    await page.keyboard.press('End');await focused(third);await tabStop(third);
  } else if(caseName==='a stream replacing the focused message keeps focus and the tab stop on its replacement' ||
            caseName==='a direct DOM swap of the focused message keeps focus and the tab stop on its replacement') {
    await focus(second);await replace(second,caseName.startsWith('a direct'));
    await focused(second);await tabStop(second);assert.equal(await row(third).getAttribute('tabindex'),'-1');
  } else if(caseName==='a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement') {
    await replace(third);await tabStop(third);await focused('message_markdown_source');
  } else if(caseName==='deleting the focused message moves focus to the surviving tab stop') {
    await focus(second);await remove(second);await focused(third);await tabStop(third);
    await page.keyboard.press('ArrowUp');await focused(first);
  } else if(caseName==='deleting an older focused message hands focus to its neighbour, not the newest') {
    await focus(first);await remove(first);await focused(second);await tabStop(second);
  } else if(caseName==='a focus move during a stream render survives Turbo\'s focus restore' ||
            caseName==='a no-change room refresh does not yank focus back to the composer') {
    const refresh=caseName.startsWith('a no-change');
    const active=await page.evaluate(async({id,refresh})=>{
      if(refresh) {
        const url=document.querySelector('[data-refresh-room-url-value]').dataset.refreshRoomUrlValue;
        const response=await fetch(`${url}?${new URLSearchParams({since:Date.now(),reason:'connection'})}`,{headers:{Accept:'text/vnd.turbo-stream.html'}});
        const body=await response.text();
        if((response.headers.get('content-type')||'').includes('turbo-stream')) Turbo.renderStreamMessage(body);
      } else Turbo.renderStreamMessage('<turbo-stream action="append" target="main-content"><template><span data-stream-focus-probe="true"></span></template></turbo-stream>');
      document.getElementById(id).focus();
      await new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(()=>setTimeout(done,50))));
      return document.activeElement.id;
    },{id:first,refresh});
    assert.equal(active,first);
    await page.keyboard.press('ArrowDown');await focused(second);
  } else if(caseName==='the ContextMenu key opens the shared menu and Escape returns focus') {
    await focus(third);
    await row(third).evaluate(node=>node.dispatchEvent(new KeyboardEvent('keydown',{bubbles:true,cancelable:true,key:'ContextMenu'})));
    await waitForVisibility(page.locator('#message-actions-menu:not([hidden])'));
    await page.keyboard.press('Escape');await waitForVisibility(page.locator('.message[data-message-actions-open]'),{state:'detached'});await focused(third);
  } else if(caseName==='a late composer autofocus does not steal focus from a message') {
    await focus(third);
    await page.evaluate(()=>{
      const composer=document.querySelector('[data-controller~="composer"]'),parent=composer.parentNode,next=composer.nextSibling;
      parent.removeChild(composer);parent.insertBefore(composer,next);
    });
    await page.waitForTimeout(500);await focused(third);
    assert.notEqual(await page.evaluate(()=>document.activeElement.id),'message_markdown_source');
  } else if(caseName==='up arrow from an empty composer still edits my last message') {
    await editor.click();await editor.press('ArrowUp');
    await waitForVisibility(page.locator('#composer [data-composer-target="contextLabel"]').filter({hasText:'Editing Message'}));
    await field(page,"Third time's a charm.");
  } else if(caseName==='up-arrow-to-edit shows an error when the actions endpoint fails') {
    await page.evaluate(()=>{
      const originalFetch=window.fetch;
      window.fetch=(input,init={})=>(typeof input==='string'?input:input.url).includes('/actions')?
        Promise.resolve(new Response('{}',{status:500})):originalFetch(input,init);
    });
    await editor.click();await editor.press('ArrowUp');
    await waitForVisibility(page.locator('.flash--client[role="alert"]').filter({hasText:'temporarily unavailable'}));
  } else if(caseName==='forward reuses the menu-open metadata request instead of fetching again') {
    await gate();await menu(third);await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    assert.equal(await page.evaluate(()=>window.__actionFetches),1);
    await release();await waitForVisibility(page.locator('dialog[open]'));
  } else if(caseName==='a menu opened while an action waits does not redirect the pending action') {
    await gate();await menu(second);await page.getByRole('menuitem',{name:'Forward',exact:true}).click();
    await menu(third);await release();
    // Wait for both metadata responses, rather than asserting absence before
    // the pending action had a chance to consume the released response.
    await page.waitForFunction(()=>window.__actionSettled===window.__actionFetches);
    await page.evaluate(()=>new Promise(done=>requestAnimationFrame(()=>requestAnimationFrame(done))));
    await waitForVisibleCount(page.locator('dialog[open]'),0);
    assert.equal(await row(third).getAttribute('data-message-actions-open'),'');
  } else if(caseName==='the menu closes before Turbo caches the page') {
    await menu(third);await page.evaluate(()=>document.dispatchEvent(new Event('turbo:before-cache')));
    await waitForVisibility(page.locator('.message[data-message-actions-open]'),{state:'detached'});
    assert.equal(await row(third).getAttribute('aria-expanded'),'false');
  } else if(caseName==='paginated history stays quiet past the insert, then the live region comes back') {
    await waitForVisibleCount(list.locator('.message').filter({hasText:/^History post 0$/}),0);
    await page.evaluate(()=>{
      window.__liveTimeline=[];window.__liveT0=performance.now();
      const list=document.querySelector('.messages[role="log"]');
      new MutationObserver(mutations=>{
        for(const mutation of mutations) {
          const entry={t:Math.round((performance.now()-window.__liveT0)*10)/10};
          if(mutation.type==='attributes') {entry.live=list.getAttribute('aria-live');entry.busy=list.getAttribute('aria-busy');}
          else entry.added=mutation.addedNodes.length;
          window.__liveTimeline.push(entry);
        }
      }).observe(list,{attributes:true,attributeFilter:['aria-live','aria-busy'],childList:true});
      list.scrollTop=0;
    });
    await text(page,'History post 0');
    await page.waitForFunction(()=>document.querySelector('.messages[role="log"]').getAttribute('aria-live')==='polite' && !document.querySelector('.messages[role="log"]').hasAttribute('aria-busy'));
    const timeline=await page.evaluate(()=>window.__liveTimeline),insert=timeline.find(entry=>entry.added>0);
    assert.ok(insert,'pagination inserts actual earlier history');
    assert.ok(timeline.some(entry=>entry.live==='off'));
    assert.ok(timeline.some(entry=>entry.busy==='true'));
    const restore=timeline.find(entry=>entry.live==='polite'&&entry.t>insert.t);
    assert.ok(restore);assert.ok(restore.t-insert.t>=30,'same pinned 30ms quiet-after-insert threshold');
    const history=await list.locator('.message[data-message-id]').evaluateAll(rows=>rows.map(row=>row.dataset.messageId));
    assert.equal(new Set(history).size,history.length,'pagination never duplicates delivered rows');
  } else if(caseName==='an edit replacement is not announced as an addition') {
    await liveObserver();await openEdit(page,row(third));await send(page,'Edited quietly');await text(recipient,'Edited quietly');
    await waitForVisibility(list.locator(':scope[aria-live="polite"]:not([aria-busy])'));
    assert.ok((await page.evaluate(()=>window.__liveValues)).includes('off'));
  } else if(caseName==='an own message is not re-announced when its broadcast replaces the pending copy') {
    await liveObserver();await send(page,'Announce me once');await text(recipient,'Announce me once');
    await waitForVisibility(list.locator(':scope[aria-live="polite"]:not([aria-busy])'));
    assert.ok((await page.evaluate(()=>window.__liveValues)).includes('off'));
  } else throw new Error(`unimplemented message-list case ${caseName}`);
}
