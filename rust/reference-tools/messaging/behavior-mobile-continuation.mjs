// Non-screenshot assertions from mobile_layout_test.rb:48-164 and
// message_list_a11y_test.rb:510-552 at d7c7de92. Scripted geometry stays scripted.
import assert from 'node:assert/strict';
import {actOnVisible,waitForVisibility,filterVisibleText,waitForVisibleCount,waitForDomCount,waitForCondition} from './behavior-visibility.mjs';
export const mobileCases=[
  'the profile page fits phone widths without scrolling sideways',
  'headers outside the workspace shell stay opaque over scrolled content',
  'headers outside the workspace shell never cover the page or its scrollbar',
  'pages outside the workspace shell show no drawer toggle that opens nothing',
  'every drawer destination has one toggle that opens the drawer on itself',
];
export async function mobileContinuation({author:page,base,caseName,fixture}) {
  const profile='/users/me/profile',account='/account/edit';
  const visit=async path=>{const response=await page.goto(base+path);assert.equal(response.status(),200);};
  const text=(node,value,timeout=2000)=>waitForVisibility(filterVisibleText(node,value),{timeout});
  const click=node=>actOnVisible(node,'click');
  if(caseName.startsWith('the profile page')) {
    const selectors=['fieldset','fieldset p','fieldset .flex > span','fieldset input:not([type=hidden])','fieldset select','fieldset .btn'].map(value=>'#main-content '+value).join(',');
    for(const width of [320,375,414]) {
      await page.setViewportSize({width,height:812});await visit(profile);await waitForVisibility(page.locator('#user_quiet_hours_start'));
      const result=await page.evaluate(selectors=>{const main=document.querySelector('#main-content'),limit=main.getBoundingClientRect().left+main.clientWidth+0.5;
        const offenders=[...document.querySelectorAll(selectors)].filter(node=>{const rect=node.getBoundingClientRect();return rect.width!==0&&rect.height!==0&&(rect.right>limit||rect.left<-.5);}).map(node=>node.outerHTML.slice(0,120));
        return {documentOverflow:document.documentElement.scrollWidth-document.documentElement.clientWidth,mainOverflow:main.scrollWidth-main.clientWidth,offenders};},selectors);
      assert.ok(result.documentOverflow<=0,JSON.stringify(result));assert.ok(result.mainOverflow<=0,JSON.stringify(result));assert.deepEqual(result.offenders,[]);
    }
  } else if(caseName.startsWith('headers outside')&&caseName.includes('opaque')) {
    await page.setViewportSize({width:375,height:812});
    for(const path of [profile,account,'/users/127326141']) {
      await visit(path);await waitForVisibility(page.locator('#nav a').filter({hasText:'Go Back'}),{state:'attached'}); // :83 visible: :all (DOM text).
      await page.evaluate(()=>document.querySelector('#main-content').scrollTop=400);
      const header=await page.evaluate(()=>{const nav=document.querySelector('#nav'),rect=nav.getBoundingClientRect(),probe=document.elementFromPoint(rect.left+rect.width/2,rect.top+rect.height/2);return{background:getComputedStyle(nav).backgroundColor,hit:!!probe&&nav.contains(probe),height:rect.height};});
      assert.ok(header.height>0);assert.ok(!['transparent','rgba(0, 0, 0, 0)'].includes(header.background));assert.doesNotMatch(header.background,/rgba\(.*, 0(\.\d+)?\)$/);assert.ok(header.hit);
    }
  } else if(caseName.startsWith('headers outside')) {
    for(const [width,height] of [[1400,1000],[375,812]]) {
      await page.setViewportSize({width,height});
      for(const path of [profile,account,'/rooms/opens/201306877/edit']) {
        await visit(path);await waitForVisibility(page.locator('#main-content .panel'));
        const layout=await page.evaluate(()=>{const nav=document.querySelector('#nav').getBoundingClientRect(),main=document.querySelector('#main-content'),r=main.getBoundingClientRect(),panel=document.querySelector('#main-content .panel').getBoundingClientRect(),probe=document.elementFromPoint(r.right-3,nav.bottom+2);return{navBottom:nav.bottom,mainTop:r.top,panelTop:panel.top,scrollbarHit:!!probe&&main.contains(probe)};});
        assert.ok(layout.panelTop>=layout.navBottom-.5,JSON.stringify(layout));assert.ok(layout.mainTop>=layout.navBottom-.5,JSON.stringify(layout));assert.ok(layout.scrollbarHit);
      }
    }
  } else if(caseName.startsWith('pages outside')) {
    await page.setViewportSize({width:375,height:812});
    for(const path of [profile,account]) {await visit(path);await waitForVisibility(page.locator('#nav a').filter({hasText:'Go Back'}),{state:'attached'});await waitForVisibleCount(page.getByRole('button',{name:'Open workspace navigation',exact:true,includeHidden:true}),0);}
  } else if(caseName.startsWith('every drawer')) {
    await page.setViewportSize({width:375,height:812});await visit('/activity');
    for(const [destination,title] of [['People','People'],['Agents','Agents'],['Work threads','Work'],['Saved','Saved for later'],['Scheduled','Scheduled messages'],['Activity inbox','Activity inbox']]) {
      await waitForVisibility(page.locator('#main-content'));await click(page.getByRole('button',{name:'Open workspace navigation',exact:true,includeHidden:true}));await waitForVisibility(page.locator('#sidebar.open'));
      await click(page.locator('#sidebar').getByRole('link',{name:destination,exact:false,includeHidden:true}));await waitForVisibility(page.locator('#sidebar.open'),{state:'hidden'});
      await waitForCondition(async()=> (await page.title()).split(' | ')[0]===title,{timeout:10000});
      const controls=await page.evaluate(()=>[...document.querySelectorAll('#nav a, #nav button')].filter(node=>node.getClientRects().length>0&&!node.closest('.global-search, .help-menu')).map(node=>node.getAttribute('aria-label')||node.textContent.trim()));
      assert.deepEqual(controls,['Open workspace navigation']);await click(page.getByRole('button',{name:'Open workspace navigation',exact:true,includeHidden:true}));await waitForVisibility(page.locator('#sidebar.open'));
      await text(page.locator('#sidebar .workspace-destinations a[aria-current="page"]'),destination,10000);await waitForVisibleCount(page.locator('#sidebar .workspace-destinations a[aria-current="page"]'),1);
      await page.keyboard.press('Escape');await waitForVisibility(page.locator('#sidebar.open'),{state:'hidden'});
    }
  } else if(caseName.startsWith('text fields')) {
    await visit(`/rooms/${fixture.board_id}/threads/${fixture.post_id}`);await click(filterVisibleText(page.locator('summary'),'Update work'));
    const fields=['.board-post__form input[name="thread[tags]"]','.board-post__form select[name="thread[work_status]"]'];
    for(const selector of fields) await waitForVisibility(page.locator(selector));
    const size=selector=>page.evaluate(selector=>parseFloat(getComputedStyle(document.querySelector(selector)).fontSize),selector);
    for(const selector of fields)assert.ok(await size(selector)<16,selector);
    const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:5});assert.ok(await page.evaluate(()=>matchMedia('(pointer: coarse)').matches));
    for(const selector of fields)assert.ok(await size(selector)>=16,selector);
    await page.evaluate(()=>document.body.insertAdjacentHTML('beforeend',"<div id='coarse-probe' style='font-size: 10px'><input id='coarse-input' type='text' aria-label='probe input'><select id='coarse-select' aria-label='probe select'><option>probe</option></select><textarea id='coarse-area' aria-label='probe textarea'></textarea></div>"));
    for(const id of ['coarse-input','coarse-select','coarse-area'])assert.ok(await size('#'+id)>=16,id);
    await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
  } else throw new Error(`Unimplemented mobile continuation: ${caseName}`);
}
