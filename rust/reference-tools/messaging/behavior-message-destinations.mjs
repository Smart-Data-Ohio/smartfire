// Mapped navigation/notification assertions from message_list_a11y_test.rb,
// pinned d7c7de92. Geometry and animation timing are behaviour, never pixels.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,actOnVisible,filterVisibleText,waitForVisibleAttribute} from './behavior-visibility.mjs';
export const destinationCases=[
  'search results keep their menus and focusability',
  'the message-list top padding does not apply to search results',
  'the standalone thread page keeps menus and focusability',
  'the standalone message page keeps its menu and focusability',
  'the viewport allows pinch zoom',
  'profile message and ban buttons have accessible names',
  'flash persists its 5-second minimum under reduced motion',
  'flash dismisses on demand under reduced motion',
];
export async function messageDestinations({author:page,recipient,base,caseName,fixture}) {
  async function visit(path) {assert.equal((await page.goto(base+path)).status(),200);}
  async function menu(message) {
    await waitForVisibility(message.locator(':scope[aria-haspopup="menu"]'));
    await actOnVisible(message.locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
    await waitForVisibility(page.locator('#message-actions-menu:not([hidden])'),{timeout:10000});
    await waitForVisibleAttribute(message,'data-message-actions-open','');
  }
  async function close() {
    await page.keyboard.press('Escape');
    await waitForVisibility(page.locator('.message[data-message-actions-open]'),{state:'hidden'});
  }
  async function longPress(message) {
    // Actual browser touch input, with the pinned Rails helper's 700ms hold.
    await actOnVisible(message,'scrollIntoViewIfNeeded',{});
    const r=await message.locator('[data-reply-target="body"]').first().boundingBox();
    const touch=await page.context().newCDPSession(page);
    await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:r.x+Math.min(r.width/2,80),y:r.y+Math.min(r.height/2,20)}]});
    await page.waitForTimeout(700);
    await touch.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
    await touch.detach();
    await waitForVisibility(page.locator('#message-actions-menu:not([hidden])'),{timeout:10000});
    await waitForVisibleAttribute(message,'data-message-actions-open','');
  }
  if(caseName===destinationCases[0]) {
    await visit('/searches?q=searchable');
    const result=filterVisibleText(page.locator('#search-results .message'),'A searchable menu result');
    await waitForVisibility(result);
    await waitForVisibility(page.locator('#search-results .message[tabindex="0"][aria-haspopup="menu"]'));
    await menu(result);await close();await longPress(result);
  } else if(caseName===destinationCases[1]) {
    const rules=await page.evaluate(()=>{
      const hits=[];
      function scan(rules) {for(const rule of rules) {
        if(rule.type===CSSRule.STYLE_RULE&&rule.style.getPropertyValue('padding-block-start')) hits.push([rule.selectorText,rule.style.getPropertyValue('padding-block-start')]);
        if(rule.cssRules) scan(rule.cssRules);
      }}
      for(const sheet of document.styleSheets) {try {scan(sheet.cssRules);} catch {}}
      return hits.filter(([selector])=>selector.includes('.messages'));
    });
    assert.ok(rules.length,'a message-list top padding rule must exist');
    assert.equal(rules.some(([selector])=>selector==='.messages'),false);
    assert.ok(rules.some(([selector])=>selector.includes(':not(.searches__results)')));
  } else if(caseName===destinationCases[2]) {
    await visit(`/rooms/654632876/threads/${fixture.menu_thread_id}`);
    await waitForVisibleCount(page.locator('main.thread .message'),2);
    await waitForVisibility(page.locator('main.thread .message[tabindex="0"][aria-haspopup="menu"]'));
    await menu(page.locator('.message[data-message-id="607264868"]'));await close();
    const reply=page.locator(`.message[data-message-id="${fixture.menu_reply_id}"]`);
    await menu(reply);await close();await longPress(reply);
  } else if(caseName===destinationCases[3]) {
    await visit('/rooms/654632876/messages/607264868');
    const message=filterVisibleText(page.locator('.message'),"Third time's a charm.");
    await waitForVisibility(message);
    await waitForVisibility(message.locator(':scope[tabindex="0"][aria-haspopup="menu"]'));
    await menu(message);
  } else if(caseName===destinationCases[4]) {
    assert.equal(await page.locator('meta[name="viewport"]').getAttribute('content'),'width=device-width, initial-scale=1, interactive-widget=resizes-content');
  } else if(caseName===destinationCases[5]) {
    for(const browser of [page,recipient]) {
      assert.equal((await browser.goto(base+'/users/712064548')).status(),200);
      await waitForVisibility(browser.locator('button[aria-label="Message Kevin"]'));
      await waitForVisibleCount(browser.locator('img[aria-label]'),0);
      if(browser!==page) await waitForVisibility(browser.getByRole('button',{name:'Ban Kevin',exact:true}));
    }
  } else if(caseName===destinationCases[6]||caseName===destinationCases[7]) {
    await page.emulateMedia({reducedMotion:'reduce'});
    await visit('/users/me/profile');
    const bio=page.locator('#user_bio');
    await actOnVisible(bio,'fill',{},[caseName===destinationCases[6]?'Reduced motion flash check':'Reduced motion dismiss check']);
    await actOnVisible(bio.locator('xpath=ancestor::form').getByRole('button',{name:'Save changes',exact:true}),'click',{});
    await waitForVisibility(page.locator('.flash'),{timeout:10000});
    if(caseName===destinationCases[6]) {
      assert.deepEqual(await page.locator('.flash__inner').evaluate(node=>node.getAnimations().map(animation=>animation.effect.getTiming().duration)),[5000]);
      await page.locator('.flash__inner').evaluate(node=>node.getAnimations()[0].currentTime=4000);
      await page.waitForTimeout(300);await waitForVisibleCount(page.locator('.flash'),1);
      await page.locator('.flash__inner').evaluate(node=>node.getAnimations()[0].currentTime=5100);
    } else await actOnVisible(page.locator('.flash__dismiss'),'click',{});
    await waitForVisibility(page.locator('.flash'),{state:'hidden',timeout:caseName===destinationCases[6]?10000:2000});
  } else throw new Error(`unimplemented message destination ${caseName}`);
}
