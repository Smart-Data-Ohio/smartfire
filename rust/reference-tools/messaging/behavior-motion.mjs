// motion_test.rb at d7c7de92: selectors use Selenium visibility, scripted
// geometry retains the original predicates; wait_until polls for 5s at 50ms.
import assert from 'node:assert/strict';
import {performance} from 'node:perf_hooks';
import {CABLE_WAIT} from './behavior-deadlines.mjs';
import {actOnVisible,visibleCount,visibleMatch,isSeleniumVisible,waitForVisibility,filterVisibleText,waitForCondition} from './behavior-visibility.mjs';
import {readySidebar} from './behavior-browser-setup.mjs';
export const motionCases=[
  'mobile drawer animates in, lands in place, and returns focus with motion on',
  'member selection mode moves no rows and resizes nothing',
  'people directory bar shifts no rows when toggling',
  'people directory bar stays stuck while scrolling',
  'room menu measures at full scale when clamping to the viewport edge',
  'mobile drawer keeps the room list scroll position across close and reopen',
  'mobile drawer reveals a current room far down the list on first open',
  'mobile drawer reopens on the current room when it is already in view',
];
async function waitUntil(page,predicate,message,arg) {
  const deadline=performance.now()+5000;
  for(;;) {
    if(await page.evaluate(predicate,arg))return;
    assert.ok(performance.now()<deadline,message);
    await new Promise(resolve=>setTimeout(resolve,50));
  }
}
const drawerVisible=(selector)=>{
  const element=document.querySelector(selector);if(!element)return false;
  const rect=element.getBoundingClientRect();if(rect.width===0||rect.height===0)return false;
  const scroller=document.querySelector('#sidebar .sidebar__scroll');let top=0,bottom=innerHeight;
  if(scroller.contains(element)){top=scroller.getBoundingClientRect().top+scroller.clientTop;bottom=top+scroller.clientHeight;}
  return rect.top>=top-1&&rect.bottom<=bottom+1;
};
const surfaceTx=()=>{
  const transform=getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform;
  if(transform==='none')return 0;
  const match=transform.match(/matrix\((.+)\)/);return match?Number(match[1].split(',').map(part=>part.trim())[4]):null;
};
async function visibleElements(locator) {
  const elements=[];
  for(let index=0;index<await locator.count();index++) {
    const element=locator.nth(index);
    if(await isSeleniumVisible(element))elements.push(element);
  }
  return elements;
}
async function firstOpenCurrentFocus(page,selector) {
  await waitForCondition(()=>page.evaluate(selector=>document.activeElement?.matches(selector),selector));
}
async function reopenedCurrentFocus(page,selector) {
  await waitForCondition(()=>page.evaluate(selector=>document.activeElement?.matches(selector),selector));
}
export async function motion({author:page,base,caseName,fixture}) {
  await page.setViewportSize({width:1400,height:1400});
  const response=await page.goto(`${base}/rooms/${fixture.motion_room_id}`);assert.equal(response.status(),200);
  // SystemTestHelper#join_room waits for every currently mounted stream
  // (at least three, :71-84). This is not a lazy-sidebar readiness wait.
  await waitForCondition(()=>page.locator('turbo-cable-stream-source').evaluateAll(nodes=>nodes.length>=3&&nodes.every(node=>node.hasAttribute('connected'))),{timeout:CABLE_WAIT});
  // Supply only Rails.env.test?'s layout input. The server-emission declaration
  // is checked separately on actual test hosts, never credited by this setup.
  await page.evaluate(()=>document.documentElement.dataset.testMotion='off');
  const click=locator=>actOnVisible(locator,'click');
  const open=async()=>{await click(page.locator('button[aria-label="Open workspace navigation"]'));await waitForVisibility(page.locator('#sidebar.open'));};
  const close=async()=>{await page.keyboard.press('Escape');await waitForVisibility(page.locator('#sidebar.open'),{state:'hidden'});};
  const focused=selector=>waitForCondition(()=>page.evaluate(selector=>document.activeElement?.matches(selector),selector));
  const focusInside=message=>waitUntil(page,()=>document.querySelector('#sidebar').contains(document.activeElement),message);
  const scroll=()=>page.evaluate(()=>document.querySelector('#sidebar .sidebar__scroll').scrollTop);
  if(caseName===motionCases[0]) {
    await page.setViewportSize({width:390,height:844});
    await page.evaluate(()=>{document.documentElement.removeAttribute('data-test-motion');document.documentElement.style.setProperty('--motion-medium','30s');});
    const attached=await page.evaluate(()=>{
      window.__motionRuns=[];let attached=0;
      for(const [selector,name] of [['#sidebar','drawer'],['#sidebar .sidebar__container','surface']]) {
        const node=document.querySelector(selector);if(node){node.addEventListener('transitionrun',event=>window.__motionRuns.push(name+':'+event.propertyName));attached++;}
      }return attached;
    });assert.equal(attached,2);
    await page.evaluate(()=>window.__motionRuns=[]);await open();
    const start=await page.evaluate(surfaceTx);assert.ok(start<-1,'motion: drawer starts off-canvas');
    await waitUntil(page,start=>{
      const transform=getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform;
      const match=transform.match(/matrix\((.+)\)/);const tx=transform==='none'?0:match?Number(match[1].split(',')[4]):null;
      return tx!==null&&tx>start+0.5;
    },'motion: drawer starts sliding',start);
    assert.ok(await page.evaluate(surfaceTx)<-1,'motion: drawer remains mid-travel');
    const running=await page.locator('#sidebar .sidebar__container').evaluate(node=>node.getAnimations().filter(animation=>animation.playState==='running').map(animation=>animation.transitionProperty));
    assert.ok(running.includes('transform'),'motion: running transform');
    await waitUntil(page,()=>window.__motionRuns.includes('drawer:opacity')&&window.__motionRuns.includes('surface:transform'),'motion: enter transitions');
    await page.evaluate(()=>{for(const selector of ['#sidebar','#sidebar .sidebar__container'])document.querySelector(selector).getAnimations().forEach(animation=>animation.finish());document.documentElement.style.removeProperty('--motion-medium');});
    await waitUntil(page,()=>['none','matrix(1, 0, 0, 1, 0, 0)'].includes(getComputedStyle(document.querySelector('#sidebar .sidebar__container')).transform),'motion: drawer lands');
    assert.equal(await page.locator('#sidebar').evaluate(node=>getComputedStyle(node).opacity),'1');
    await focusInside('motion: focus enters drawer');await close();
    await waitUntil(page,()=>document.activeElement?.getAttribute('aria-label')==='Open workspace navigation','motion: focus returns to opener');
  } else if(caseName===motionCases[1]) {
    const show=filterVisibleText(page.locator('button'),'Show members');
    let showButton;try {showButton=await visibleMatch(show,{timeout:5000});}catch(error){if(error.name!=='TimeoutError')throw error;}
    if(showButton)await click(showButton);
    await waitForCondition(async()=>await visibleCount(page.locator('#channel-members .member-panel__member'))>=2,{timeout:10000});
    const boxes=page.locator('#channel-members input[type="checkbox"]');await waitForVisibility(boxes,{state:'hidden'});
    const lefts=()=>page.locator('#channel-members .member-panel__member .member-panel__avatar').evaluateAll(nodes=>nodes.map(node=>node.getBoundingClientRect().left));
    const height=()=>page.locator('#channel-members .member-panel__content').evaluate(node=>node.clientHeight);
    const before=await lefts(),beforeHeight=await height();
    await actOnVisible(page.locator(`#channel-members [data-member-id="${fixture.jason_id}"] button.profile-card-name`),'click',{modifiers:['Control']});
    await waitForVisibility(boxes);await waitForVisibility(page.locator('#channel-members [data-multi-select-target="bar"]'));
    assert.deepEqual(await lefts(),before,'motion: member-select positions');assert.equal(await height(),beforeHeight,'motion: member-select height');
    await click((await visibleElements(page.locator(`#channel-members input[type="checkbox"]:not(#select-member-${fixture.jason_id})`))).at(-1));
    await waitForVisibility(filterVisibleText(page.locator('#channel-members [data-multi-select-target="messageButton"]'),'Message (2)'));
    assert.deepEqual(await lefts(),before,'motion: second-select positions');assert.equal(await height(),beforeHeight);
    for(const box of await visibleElements(boxes))if(await box.isChecked())await click(box);
    await waitForVisibility(page.locator('#channel-members [data-multi-select-target="bar"]'),{state:'hidden'});
    assert.deepEqual(await lefts(),before);assert.equal(await height(),beforeHeight);
  } else if(caseName===motionCases[2]||caseName===motionCases[3]) {
    const sticky=caseName===motionCases[3];if(sticky)await page.setViewportSize({width:1400,height:400});
    assert.equal((await page.goto(base+'/users')).status(),200);
    await waitForCondition(async()=>await visibleCount(page.locator('.people-directory__row'))>=(sticky?10:2));
    const tops=()=>page.locator('.people-directory__row').evaluateAll(nodes=>nodes.map(node=>node.getBoundingClientRect().top));
    const before=await tops();
    if(sticky)assert.ok(await page.locator('#main-content').evaluate(node=>node.scrollHeight>node.clientHeight+100),'motion: sticky overflow');
    const boxes=page.locator('.people-directory__row input[type="checkbox"]');await click(boxes.first());await waitForVisibility(page.locator('.multi-select-bar'));
    if(!sticky) {
      assert.deepEqual(await tops(),before,'motion: directory-select positions');
      for(const box of await visibleElements(boxes))if(await box.isChecked())await click(box);
      await waitForVisibility(page.locator('.multi-select-bar'),{state:'hidden'});assert.deepEqual(await tops(),before);
    } else {
      await page.evaluate(()=>document.querySelector('#main-content').scrollTop=100);
      await waitUntil(page,()=>document.querySelector('#main-content').scrollTop===100,'motion: directory scroll');
      const geometry=await page.evaluate(()=>({barBottom:document.querySelector('.multi-select-bar').getBoundingClientRect().bottom,mainBottom:document.querySelector('#main-content').getBoundingClientRect().bottom}));
      assert.ok(geometry.barBottom<=geometry.mainBottom+1,'motion: sticky bar inside scrollport');
    }
  } else if(caseName===motionCases[4]) {
    await page.evaluate(()=>document.documentElement.removeAttribute('data-test-motion'));
    const row=await visibleMatch(page.locator('#sidebar a[data-room-id]').first());
    await row.evaluate(node=>node.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:innerWidth-10,clientY:400})));
    await waitForVisibility(page.locator('#room-menu:not([hidden])'));
    await waitUntil(page,()=>getComputedStyle(document.querySelector('#room-menu')).transform==='none','motion: menu pop lands');
    const geometry=await page.locator('#room-menu').evaluate(node=>({right:node.getBoundingClientRect().right,limit:innerWidth-8}));
    assert.ok(geometry.right<=geometry.limit+1,'motion: menu clamped inside viewport');
    await page.keyboard.press('Escape');await waitForVisibility(page.locator('#room-menu[hidden]'),{state:'attached'}); // :174 visible: :all
  } else {
    await readySidebar(page);
    await page.setViewportSize({width:390,height:844});
    const current='#sidebar a[aria-current="page"]';
    if(caseName===motionCases[6]) {
      await waitForVisibility(page.locator(current),{state:'attached'}); // :250 visible: :all
      assert.equal(await scroll(),0);assert.equal(await page.evaluate(drawerVisible,current),false);
      await open();
      await firstOpenCurrentFocus(page,current);
      await waitUntil(page,drawerVisible,'motion: first-open reveals current room',current);
    } else if(caseName===motionCases[7]) {
      await open();await focused(current);
      await page.evaluate(()=>window.__motionLastScroll=null);
      await waitUntil(page,()=>{const node=document.querySelector('#sidebar .sidebar__scroll'),current=document.querySelector('#sidebar a[aria-current="page"]'),rect=current.getBoundingClientRect(),top=node.getBoundingClientRect().top+node.clientTop;const now=node.scrollTop,settled=now===window.__motionLastScroll&&rect.top>=top-1&&rect.bottom<=top+node.clientHeight+1;window.__motionLastScroll=now;return settled;},'motion: first reveal settles');
      await close();const before=await scroll();await open();
      await reopenedCurrentFocus(page,current);
      assert.equal(await scroll(),before,'motion: reopen-current keeps offset');
    } else {
      assert.equal(caseName,motionCases[5]);await open();await focusInside('motion: initial drawer focus');await page.evaluate(()=>document.activeElement.blur());
      assert.ok(await page.locator('#sidebar .sidebar__scroll').evaluate(node=>node.scrollHeight>node.clientHeight));
      const max=await page.locator('#sidebar .sidebar__scroll').evaluate(node=>node.scrollHeight-node.clientHeight);assert.ok(max>=400);
      await page.evaluate(()=>document.querySelector('#sidebar .sidebar__scroll').scrollTop=400);
      await waitUntil(page,()=>document.querySelector('#sidebar .sidebar__scroll').scrollTop===400,'motion: drawer scroll lands');
      const before=await scroll();assert.equal(before,400);assert.equal(await page.evaluate(drawerVisible,current),false);
      await close();assert.equal(await scroll(),before,'motion: closed drawer keeps offset');
      await open();await focusInside('motion: reopen drawer focus');assert.equal(await scroll(),before,'motion: reopened drawer keeps offset');
      assert.ok(await page.evaluate(drawerVisible,':focus'),'motion: reopened focus in view');
    }
  }
}
