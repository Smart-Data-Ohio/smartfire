import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createRequire} from 'node:module';
import {installVisibility,setVisibilityTimeout,visibleCount,visibleMatch,waitForVisibleCount,waitForVisibility,waitForVisibleProperty,waitForVisibleAttribute,actOnVisible,waitForVisibleContentCount} from './behavior-visibility.mjs';
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
    await assert.rejects(waitForVisibleCount(reactions,9,{timeout:200}),{name:'TimeoutError'});
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


test('property, attribute and explicit lookup checks cannot accept transparent matches',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,200);
    await page.goto('data:text/html,'+encodeURIComponent(`
      <div style="opacity:0"><textarea class="draft">expected</textarea>
        <input id="notify" type="checkbox" checked>
        <a id="link" href="https://example.com/notes">Project notes</a>
        <pre><code>puts :forwarded</code></pre>
        <button id="action" onclick="window.clicked=true">action</button>
      </div>
      <textarea class="draft">wrong visible value</textarea>
    `));
    await assert.rejects(waitForVisibleProperty(page.locator('.draft'),'value','expected',{timeout:100}),{name:'TimeoutError'});
    await assert.rejects(waitForVisibleProperty(page.locator('#notify'),'checked',true,{timeout:100}),{name:'TimeoutError'});
    await assert.rejects(waitForVisibleAttribute(page.locator('#link'),'href','https://example.com/notes',{timeout:100}),{name:'TimeoutError'});
    await assert.rejects(waitForVisibleProperty(page.locator('code'),'textContent','puts :forwarded',{timeout:100}),{name:'TimeoutError'});
    await assert.rejects(actOnVisible(page.locator('#action'),'click',{timeout:100}),{name:'TimeoutError'});
    assert.equal(await page.evaluate(()=>window.clicked),undefined);
    await page.evaluate(()=>{document.querySelector('div').style.opacity='1';});
    await waitForVisibleProperty(page.locator('.draft'),'value','expected',{timeout:200});
    await waitForVisibleProperty(page.locator('#notify'),'checked',true,{timeout:200});
    await waitForVisibleAttribute(page.locator('#link'),'href','https://example.com/notes',{timeout:200});
    await waitForVisibleProperty(page.locator('code'),'textContent','puts :forwarded',{timeout:200});
    await actOnVisible(page.locator('#action'),'click',{timeout:200});
    assert.equal(await page.evaluate(()=>window.clicked),true);
  } finally {await browser.close();}
});


test('visible descendants and ancestors cannot stand in for the selected element',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,100);
    await page.goto('data:text/html,'+encodeURIComponent(`
      <article class="message">
        <h2 style="opacity:0">Design review</h2>
        <pre><code style="visibility:hidden"><span class="code-token" style="visibility:visible">const</span> value = 1;</code></pre>
        <div class="message__body" style="visibility:hidden"><div data-reply-target="body" style="visibility:visible">Delivered text</div></div>
      </article>
    `));
    await waitForVisibility(page.locator('.message'));
    await waitForVisibility(page.locator('.code-token'));
    await waitForVisibility(page.locator('[data-reply-target="body"]'));
    await assert.rejects(waitForVisibility(page.locator('h2')),{name:'TimeoutError'});
    await assert.rejects(waitForVisibility(page.locator('pre code')),{name:'TimeoutError'});
    await assert.rejects(waitForVisibleContentCount(page.locator('.message__body'),'[data-reply-target="body"]','Delivered text',1,{timeout:100}),{name:'TimeoutError'});
    await page.locator('[style]').evaluateAll(nodes=>nodes.forEach(node=>node.removeAttribute('style')));
    await waitForVisibility(page.locator('h2'));
    await waitForVisibility(page.locator('pre code'));
    await waitForVisibleContentCount(page.locator('.message__body'),'[data-reply-target="body"]','Delivered text',1,{timeout:100});
  } finally {await browser.close();}
});

test('visible text excludes hidden descendants, honors whitespace and can see overridden children',async()=>{
  const {filterVisibleText,byVisibleText,visibleText,waitForVisibleText}=await import('./behavior-visibility.mjs');
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,100);
    await page.goto('data:text/html,'+encodeURIComponent(`
      <article id="card">X<span style="visibility:hidden">Loading post</span></article>
      <div id="body">visible <span style="opacity:0">hidden</span><span style="display:none">secret</span></div>
      <div style="visibility:hidden"><strong style="visibility:visible">overridden</strong>invisible</div>
      <pre><code>one\n  two <span style="visibility:hidden">hidden</span></code></pre>
      <div id="wrap"><div data-body>visible <span style="visibility:hidden">secret</span></div></div>
    `));
    await waitForVisibility(page.locator('#card'));
    await assert.rejects(waitForVisibility(filterVisibleText(page.locator('#card'),'Loading post')),{name:'TimeoutError'});
    await assert.rejects(waitForVisibility(byVisibleText(page,'secret')),{name:'TimeoutError'});
    assert.equal(await visibleText(page.locator('#body')),'visible');
    await waitForVisibleText(page.locator('strong'),'overridden',{timeout:100});
    assert.equal(await visibleText(page.locator('code')),'one\n  two ');
    await assert.rejects(waitForVisibleContentCount(page.locator('#wrap'),'[data-body]','secret',1,{timeout:100}),{name:'TimeoutError'});
    await page.locator('#card span').evaluate(node=>node.style.visibility='visible');
    await waitForVisibility(filterVisibleText(page.locator('#card'),'Loading post'));
    await page.goto('data:text/html,<div id="card"><span style="visibility:hidden">Loading post</span>error</div>');
    await assert.rejects(waitForVisibility(filterVisibleText(page.locator('#card'),'Loading post')),{name:'TimeoutError'});
  } finally {await browser.close();}
});

test('negative queries retry forbidden matches; all-node checks keep hidden nodes',async()=>{
  const {waitForDomCount}=await import('./behavior-visibility.mjs');
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();setVisibilityTimeout(page,150);
    await page.goto('data:text/html,<div id="forbidden">present</div><script>setTimeout(()=>document.querySelector("div").remove(),80)</script>');
    await waitForVisibility(page.locator('#forbidden'),{state:'hidden'});
    assert.equal(await page.locator('#forbidden').count(),0,'absence cannot pass while the visible forbidden node remains');
    await page.goto('data:text/html,<div id="forbidden">present</div>');
    await assert.rejects(waitForVisibility(page.locator('#forbidden'),{state:'hidden'}),{name:'TimeoutError'});
    await page.locator('#forbidden').evaluate(node=>node.style.opacity='0');
    await waitForVisibility(page.locator('#forbidden'),{state:'hidden'});
    await assert.rejects(waitForDomCount(page.locator('#forbidden'),0,{timeout:100}),{name:'TimeoutError'});
    await waitForDomCount(page.locator('#forbidden'),1,{timeout:100});
  } finally {await browser.close();}
});

test('all element actions require Selenium visibility, including opacity through parents',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const context=await browser.newContext();await installVisibility(context);
    const page=await context.newPage();
    await page.goto('data:text/html,'+encodeURIComponent(`<div style="opacity:0"><textarea></textarea><button>Click</button><input type="checkbox"><select><option value="a">A</option></select></div>`));
    for(const [selector,action,args] of [['textarea','fill',['draft']],['button','click',[]],['input','check',[]],['select','selectOption',['a']],['textarea','press',['Enter']],['button','hover',[]]]) {
      await assert.rejects(actOnVisible(page.locator(selector),action,{timeout:100},args),{name:'TimeoutError'});
    }
    assert.equal(await page.locator('textarea').inputValue(),'');
    assert.equal(await page.locator('input').isChecked(),false);
    await page.locator('div').evaluate(node=>node.style.opacity='1');
    await actOnVisible(page.locator('textarea'),'fill',{timeout:100},['draft']);
    await actOnVisible(page.locator('input'),'check',{timeout:100});
    assert.equal(await page.locator('textarea').inputValue(),'draft');
    assert.equal(await page.locator('input').isChecked(),true);
  } finally {await browser.close();}
});
