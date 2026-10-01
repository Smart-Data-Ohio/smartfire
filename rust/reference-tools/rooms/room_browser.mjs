// Real keyboard and drawer interactions on separate Rails/Rust seed snapshots. No pixels.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const browser=await chromium.launch({headless:true});
const quickSwitcherOnly=process.argv.includes('--quick-switcher-only');
async function acceptance(base) {
 const context=await browser.newContext({viewport:{width:1440,height:1000}});
 try {
  const [name,...value]=sessions.find(s=>s.user_name==='JZ').cookie_header.split('=');
  await context.addCookies([{name,value:value.join('='),url:base}]);
  const page=await context.newPage();
  const designers='/rooms/654632876';
  await page.goto(base+'/rooms/201306877');
  await page.locator('#user_sidebar a[data-room-id="654632876"]').waitFor();
  async function switcher(query) {
   await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.body,'keyboard-shortcuts') && window.Stimulus?.getControllerForElementAndIdentifier(document.getElementById('quick-switcher'),'quick-switcher'));
   await page.keyboard.press('Control+k');
   await page.locator('#quick-switcher[open]').waitFor();
   const field=page.getByRole('combobox',{name:'Jump to a room, person, or thread'});
   await field.fill(query);
   return field;
  }
  let input=await switcher('Desig');
  await page.locator('#quick-switcher-listbox .quick-switcher__title').filter({hasText:/^Designers$/}).waitFor();
  await input.press('Enter');
  await page.waitForURL(base+designers);
  await page.locator('.room-header__name').filter({hasText:'Designers'}).waitFor();
  input=await switcher('');
  await page.locator('#quick-switcher-listbox .quick-switcher__group').filter({hasText:/recent/i}).waitFor();
  await page.locator('#quick-switcher-listbox .quick-switcher__title').filter({hasText:/^HQ$/}).waitFor();
  await input.press('Escape');
  input=await switcher('a');
  await page.locator('#quick-switcher-listbox [aria-selected="true"]').waitFor();
  const titles=await page.locator('#quick-switcher-listbox [role="option"] .quick-switcher__title').allTextContents();
  assert.ok(titles.length>=2);
  const active=()=>page.locator('#quick-switcher-listbox [aria-selected="true"] .quick-switcher__title').textContent();
  assert.equal(await active(),titles[0]);
  await input.press('ArrowDown');assert.equal(await active(),titles[1]);
  await input.press('ArrowUp');assert.equal(await active(),titles[0]);
  await input.press('End');assert.equal(await active(),titles.at(-1));
  await input.press('Escape');
  assert.equal(await page.locator('#quick-switcher[open]').count(),0);
  if (!quickSwitcherOnly) {
  // The member drawer opens automatically on desktop, then traps/restores focus on phones.
  await page.getByRole('button',{name:'Hide members',exact:true}).waitFor();
  await page.locator('#channel-members [data-member-id="712064548"]').waitFor();
  await page.getByRole('button',{name:'Hide members',exact:true}).click();
  await page.waitForFunction(()=>!document.body.classList.contains('member-panel-open'));
  await page.getByRole('button',{name:'Show members',exact:true}).click();
  await page.waitForFunction(()=>document.body.classList.contains('member-panel-open'));
  await page.setViewportSize({width:390,height:844});
  await page.waitForFunction(()=>!document.body.classList.contains('member-panel-open'));
  await page.getByRole('button',{name:'Show members',exact:true}).click();
  await page.waitForFunction(()=>document.activeElement?.getAttribute('aria-label')==='Close members');
  await page.keyboard.press('Shift+Tab');
  assert.equal(await page.evaluate(()=>document.querySelector('#channel-members').contains(document.activeElement)),true);
  await page.keyboard.press('Escape');
  await page.waitForFunction(()=>!document.body.classList.contains('member-panel-open'));
  assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'Show members');
  // Header menu keyboard navigation, dismissal and panel focus hand-off.
  const more=page.getByRole('button',{name:'More actions',exact:true});
  await more.press('ArrowDown');
  await page.locator('#header-overflow-menu:not([hidden])').waitFor();
  assert.equal(await more.getAttribute('aria-expanded'),'true');
  assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('role')),'menuitem');
  await page.keyboard.press('ArrowDown');
  assert.match(await page.evaluate(()=>document.activeElement.getAttribute('href')),/\/events$/);
  await page.keyboard.press('Escape');
  assert.equal(await more.getAttribute('aria-expanded'),'false');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'header-overflow-button');
  await more.click();
  await page.locator('.room-header__name').click();
  await page.waitForFunction(()=>document.getElementById('header-overflow-menu').hidden);
  await more.click();
  await page.locator('#header-overflow-menu').getByRole('menuitem',{name:'Threads',exact:true}).click();
  await page.waitForFunction(()=>document.body.classList.contains('thread-panel-open'));
  await page.locator('.thread-panel__close').click();
  await page.waitForFunction(()=>!document.body.classList.contains('thread-panel-open'));
  assert.equal(await page.evaluate(()=>document.activeElement.id),'header-overflow-button');
  await more.click();
  await page.locator('#header-overflow-menu').getByRole('menuitem',{name:/Pins/}).click();
  await page.locator('.pins-panel[open]').waitFor();
  await page.locator('.pins-panel turbo-frame .pins-panel__list').waitFor();
  await page.getByRole('button',{name:'Close pinned messages',exact:true}).click();
  assert.equal(await page.locator('.pins-panel[open]').count(),0);
  }
  await page.goto(base+'/rooms/201306877');
  const rooms=()=>page.evaluate(async()=>{
    const url=document.getElementById('quick-switcher').dataset.quickSwitcherUrlValue;
    return (await (await fetch(url,{headers:{Accept:'application/json'}})).json()).rooms;
  });
  const before=await rooms();
  assert.ok(!before.some(r=>r.kind==='dm' && r.name==='Kevin'),'fixture has no Kevin DM for JZ');
  input=await switcher('Kevin');
  await page.locator('#quick-switcher-listbox .quick-switcher__title').filter({hasText:/^Kevin$/}).waitFor();
  // The parity seed also has a matching group DM. Choose the actual person by keyboard.
  const options=await page.locator('#quick-switcher-listbox [role="option"]').count();
  for (let i=0;i<options;i++) {
    const selected=page.locator('#quick-switcher-listbox [aria-selected="true"]');
    if ((await selected.locator('.quick-switcher__title').textContent())==='Kevin' &&
        (await selected.locator('.quick-switcher__detail').textContent())==='Open DM') break;
    await input.press('ArrowDown');
  }
  assert.equal(await page.locator('#quick-switcher-listbox [aria-selected="true"] .quick-switcher__title').textContent(),'Kevin');
  assert.equal(await page.locator('#quick-switcher-listbox [aria-selected="true"] .quick-switcher__detail').textContent(),'Open DM');
  await input.press('Enter');
  await page.locator('.room-header__name').filter({hasText:'Kevin'}).waitFor();
  assert.equal((await rooms()).length,before.length+1,'person jump creates exactly one new accessible room');
  const original={roomJump:true,recents:true,arrows:true,escape:true,personCreatesDm:true};
  return quickSwitcherOnly ? original : {...original,memberDrawer:true,focusTrap:true,menuKeyboard:true,outsideDismiss:true,threadFocusReturn:true,pins:true};
 } finally {await context.close();}
}
try {
 const rails=await acceptance(process.argv[2]);
 assert.deepEqual(await acceptance(process.argv[3]),rails);
 console.log(quickSwitcherOnly ? 'QuickSwitcher browser acceptance: 2 targets passed; all five original interactions match' : 'Room browser acceptance: 2 targets passed; keyboard room/person jumps, recent rooms, option navigation, Escape, member drawer/focus trap, header menu keyboard/outside dismissal, thread focus return and native pins match');
} finally {await browser.close();}
