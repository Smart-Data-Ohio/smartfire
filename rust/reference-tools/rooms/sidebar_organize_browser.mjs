// The five non-delivery declarations from pinned SidebarOrganizeTest.
// Execute the original interactions and DOM assertions; no pixel comparison.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
const require=createRequire(new URL('../../parity/package.json',import.meta.url));
const {chromium}=require('playwright');
const sessions=JSON.parse(readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url))).sessions;
const browser=await chromium.launch({headless:true});
const hq='201306877', designers='654632876';

async function acceptance(base) {
  const context=await browser.newContext();
  try {
    const [name,...value]=sessions.find(s=>s.user_name==='JZ').cookie_header.split('=');
    await context.addCookies([{name,value:value.join('='),url:base}]);
    const page=await context.newPage();
    page.setDefaultTimeout(10000);
    const row=id=>page.locator(`#sidebar a[data-room-id="${id}"]`);
    const favorites=()=>page.locator('#favorite_rooms a[data-room-id]');
    const names=()=>page.locator('#favorite_rooms .sidebar-item__label, #favorite_rooms .direct__author').allTextContents().then(xs=>xs.map(x=>x.trim()));
    const category=name=>page.locator('#sidebar .sidebar-section--category').filter({has:page.locator('h2',{hasText:new RegExp(`^\\s*${name}\\s*$`,'i')})});
    async function ready() {
      await page.goto(base+`/rooms/${hq}`);
      await row(designers).waitFor({state:'attached'});
    }
    async function menu(id,keyboard=false) {
      await page.waitForFunction(()=>window.Stimulus?.getControllerForElementAndIdentifier(document.querySelector('[data-controller~="room-menu"]'),'room-menu'));
      if(keyboard) await row(id).press('Shift+F10');
      else await row(id).click({button:'right'});
      await page.locator('#room-menu:not([hidden])').waitFor({timeout:5000});
      return page.locator('#room-menu');
    }
    async function favorite(id) {
      await (await menu(id)).getByRole('menuitem',{name:'Add to favourites',exact:true}).click();
      await page.locator(`#favorite_rooms a[data-room-id="${id}"]`).waitFor();
    }
    async function clearFavorites() {
      // Each original Rails declaration runs in its own rollback. Restore through
      // the actual menu between declarations instead of mutating either database.
      while(await favorites().count()) {
        const id=await favorites().first().getAttribute('data-room-id');
        await (await menu(id)).getByRole('menuitem',{name:'Remove from favourites',exact:true}).click();
        await page.locator(`#favorite_rooms a[data-room-id="${id}"]`).waitFor({state:'detached'});
      }
    }
    async function createCategory(name) {
      await page.locator('.room-category__new-toggle').click();
      const form=page.locator('.room-category__new-form');
      await form.getByRole('textbox',{name:'New category name',exact:true}).fill(name);
      await form.getByRole('button',{name:'Create',exact:true}).click();
      await category(name).waitFor();
    }
    async function assign(id,name) {
      await (await menu(id)).getByRole('menuitem',{name,exact:true}).click();
      await category(name).locator(`a[data-room-id="${id}"]`).waitFor();
      assert.equal(await page.locator(`#shared_rooms a[data-room-id="${id}"]`).count(),0);
    }
    async function deleteCategory(name) {
      page.once('dialog',dialog=>dialog.accept());
      await category(name).getByRole('button',{name:`Delete ${name}`,exact:true}).click();
      await category(name).waitFor({state:'detached'});
    }
    const passed=[];
    await ready();
    await clearFavorites();
    await favorite(designers);
    assert.ok((await names()).includes('Designers'));
    assert.equal(await page.locator(`#shared_rooms a[data-room-id="${designers}"]`).count(),0);
    passed.push('favouriting from the room menu moves the room to Favourites');

    await clearFavorites();
    await favorite(designers);
    await favorite(hq);
    assert.deepEqual(await names(),['Designers','HQ']);
    await (await menu(hq,true)).getByRole('menuitem',{name:'Move up',exact:true}).click();
    await page.locator(`#favorite_rooms > a:first-child[data-room-id="${hq}"]`).waitFor();
    assert.deepEqual(await names(),['HQ','Designers']);
    passed.push('favourites reorder from the keyboard with move up and down');

    await clearFavorites();
    await favorite(designers);
    await favorite(hq);
    assert.deepEqual(await names(),['Designers','HQ']);
    // Rails dispatches these same HTML5 events because WebDriver's drag does not.
    // Geometry supplies the drop event position, never an acceptance comparison.
    await page.evaluate(id=>{
      const list=document.getElementById('favorite_rooms');
      const row=list.querySelector(`a[data-room-id="${id}"]`);
      const y=list.querySelector('a[data-room-id]').getBoundingClientRect().top+2;
      row.dispatchEvent(new DragEvent('dragstart',{bubbles:true,cancelable:true,dataTransfer:new DataTransfer()}));
      list.dispatchEvent(new DragEvent('dragover',{bubbles:true,cancelable:true,clientY:y,dataTransfer:new DataTransfer()}));
      list.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,clientY:y,dataTransfer:new DataTransfer()}));
      row.dispatchEvent(new DragEvent('dragend',{bubbles:true,cancelable:true}));
    },hq);
    await page.locator(`#favorite_rooms > a:first-child[data-room-id="${hq}"]`).waitFor();
    assert.deepEqual(await names(),['HQ','Designers']);
    passed.push('favourites reorder with drag and drop');

    await clearFavorites();
    await createCategory('Team');
    await assign(designers,'Team');
    await category('Team').getByRole('button',{name:'Collapse Team',exact:true}).click();
    await page.locator(`#sidebar .sidebar-list:not([hidden]) a[data-room-id="${designers}"]`).waitFor({state:'detached',timeout:5000});
    await ready();
    assert.equal(await page.locator(`#sidebar .sidebar-list:not([hidden]) a[data-room-id="${designers}"]`).count(),0);
    passed.push('channel categories organize, collapse and persist');
    await deleteCategory('Team');
    await page.locator(`#shared_rooms a[data-room-id="${designers}"]`).waitFor();

    // Use real UI setup for the two fixture writes in the original declaration.
    await createCategory('Team');
    await category('Team').locator('.room-category__rename summary').click();
    await category('Team').getByRole('textbox',{name:'New name for Team',exact:true}).fill('Squad');
    await category('Team').getByRole('button',{name:'Save',exact:true}).click();
    await category('Squad').waitFor();
    await assign(designers,'Squad');
    await ready();
    await category('Squad').locator(`a[data-room-id="${designers}"]`).waitFor();
    await deleteCategory('Squad');
    await page.locator(`#shared_rooms a[data-room-id="${designers}"]`).waitFor();
    passed.push('categories rename and delete');
    return passed;
  } finally {await context.close();}
}
try {
  const rails=await acceptance(process.argv[2]);
  assert.deepEqual(await acceptance(process.argv[3]),rails);
  console.log('SidebarOrganize original mapping: 5 passed on Rails; 5 passed on Rust; 0 failed; muted delivery case remains unexecuted');
} finally {await browser.close();}
