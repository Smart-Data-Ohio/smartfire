import assert from 'node:assert/strict';
import {test} from 'node:test';
import {createRequire} from 'node:module';
import {readySidebar} from './behavior-browser-setup.mjs';
const {chromium}=createRequire(new URL('../../parity/package.json',import.meta.url))('playwright');
test('sidebar readiness does not accept an incomplete frame or a stale current link',async()=>{
  const browser=await chromium.launch({headless:true});
  try {
    const page=await browser.newPage();
    await page.setContent('<meta name="current-room-id" content="201306877"><turbo-frame id="user_sidebar"><div class="sidebar__scroll"><a aria-current="page" data-room-id="654632876">Designers</a></div></turbo-frame>');
    // This would incorrectly open the drawer against the unfinished frame.
    await assert.rejects(readySidebar(page),{name:'TimeoutError'});
    await page.locator('#user_sidebar').evaluate(node=>node.setAttribute('complete',''));
    // Turbo completion alone must not accept a link for the previous room.
    await assert.rejects(readySidebar(page),{name:'TimeoutError'});
    await page.locator('a').evaluate(node=>node.dataset.roomId='201306877');
    await readySidebar(page);
  }finally{await browser.close();}
});
