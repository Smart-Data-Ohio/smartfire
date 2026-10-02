import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createRequire} from 'node:module';
import {installVisibility,setVisibilityTimeout,visibleCount,visibleMatch,waitForVisibleCount,waitForVisibility} from './behavior-visibility.mjs';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');

test('pinned Selenium visibility counts opacity through ancestors and other hidden states',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,200);
    await page.goto('data:text/html,'+encodeURIComponent(`
      <button class="reaction">visible</button>
      <button class="reaction" style="opacity:0">transparent</button>
      <div style="opacity:0"><div style="opacity:1"><button class="reaction">transparent ancestor</button></div></div>
      <div style="opacity:.5"><button class="reaction" style="opacity:.5">partially transparent</button></div>
      <div style="display:none"><button class="reaction">display none ancestor</button></div>
      <button class="reaction" style="visibility:hidden">hidden</button>
      <details><summary>closed details</summary><button class="reaction">closed content</button></details>
      <div style="width:30px;height:30px;overflow:hidden;position:relative"><button class="reaction" style="position:absolute;left:100px">clipped</button></div>
      <div id="shadow"></div>
      <script>document.getElementById('shadow').attachShadow({mode:'open'}).innerHTML='<button class="reaction" style="opacity:0">transparent shadow child</button>';</script>
    `));
    const reactions=page.locator('.reaction');
    assert.equal(await reactions.count(),9);
    assert.equal(await visibleCount(reactions),2);
    assert.equal(await (await visibleMatch(reactions)).textContent(),'visible');
    await waitForVisibleCount(reactions,2);
    await waitForVisibility(page.getByText('transparent ancestor',{exact:true}),{state:'hidden'});
    await assert.rejects(waitForVisibility(page.getByText('transparent',{exact:true})),{name:'TimeoutError'});
    await assert.rejects(waitForVisibleCount(reactions,9),{name:'TimeoutError'});
    await page.getByText('visible',{exact:true}).evaluate(element=>element.remove());
    await waitForVisibility(page.getByText('visible',{exact:true}),{state:'detached'});
    await waitForVisibleCount(reactions,1);
  } finally {await browser.close();}
});

test('visible state waits use the pinned atom after DOM updates and navigation',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,200);
    await page.goto('data:text/html,<div style="opacity:0"><button>action</button></div>');
    const action=page.getByRole('button',{name:'action',exact:true});
    await waitForVisibility(action,{state:'attached'});
    await waitForVisibility(action,{state:'hidden'});
    await page.evaluate(()=>{document.querySelector('div').style.opacity='1';});
    await waitForVisibility(action);
    await page.goto('data:text/html,<button style="opacity:0">action</button>');
    await waitForVisibleCount(action,0);
    await assert.rejects(waitForVisibility(action),{name:'TimeoutError'});
  } finally {await browser.close();}
});
