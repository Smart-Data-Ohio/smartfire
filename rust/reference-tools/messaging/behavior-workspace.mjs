// workspace_markdown_test.rb:242-290 and :295-323 at d7c7de92.
import assert from 'node:assert/strict';
import {actOnVisible,waitForVisibility,waitForVisibleProperty,waitForVisibleCount,waitForCondition,filterVisibleText} from './behavior-visibility.mjs';
export const WORKSPACE_CASE='workspace follows the system theme and mobile navigation remains reachable';
export async function workspace({author:page,source,submit}) {
  const literal=source.split("MARKDOWN = <<~'MARKDOWN'.freeze\n")[1].split('  MARKDOWN')[0].split('\n').map(line=>line.slice(4)).join('\n');
  await page.emulateMedia({colorScheme:'light'});
  await submit(page,literal);
  await waitForVisibility(filterVisibleText(page.locator('.message__body h2'),'Design review'));
  const overflow=async()=>assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'workspace: no horizontal overflow');
  const profile=async()=>{
    const inside=await page.evaluate(()=>{const nav=document.querySelector('.sidebar__container').getBoundingClientRect(),bar=document.querySelector('.sidebar__tools').getBoundingClientRect();return Math.abs(bar.left-nav.left)<=1&&Math.abs(bar.right-nav.right)<=1;});
    assert.ok(inside,'workspace: profile inside navigation');
  };
  const compact=async()=>{
    await waitForVisibleProperty(page.locator('#composer textarea[name="message[markdown_source]"]'),'value','');
    await waitForVisibility(page.locator('#composer [role="tablist"], #composer [role="toolbar"], #composer trix-editor'),{state:'hidden'});
    await waitForVisibleCount(filterVisibleText(page.locator('#composer'),'Enter to send'),0);
    await waitForVisibleCount(filterVisibleText(page.locator('#composer button'),'Rich text'),0);
    await waitForVisibility(filterVisibleText(page.locator('#composer button'),'Send Message'));
    await waitForVisibility(page.locator('#composer input[type="file"]'),{state:'attached'}); // :302 visible: :all
    assert.ok(await page.evaluate(()=>{const surface=document.querySelector('#composer .composer__surface').getBoundingClientRect(),field=document.querySelector('#composer textarea').getBoundingClientRect(),send=document.querySelector('#composer button[name="send"]').getBoundingClientRect();return surface.height<=72&&send.top>=surface.top&&send.bottom<=surface.bottom+1&&send.left>=field.right-1;}),'workspace: compact composer');
  };
  const settled=()=>page.evaluate(async()=>{const finite=document.getAnimations().filter(animation=>Number.isFinite(animation.effect.getComputedTiming().endTime));await Promise.all(finite.map(animation=>animation.finished.catch(()=>{})));});
  const background=()=>page.locator('#main-content').evaluate(node=>getComputedStyle(node).backgroundColor);
  const open=()=>actOnVisible(page.locator('button[aria-label="Open workspace navigation"]'),'click');
  await overflow();await compact();await profile();
  await waitForVisibility(page.locator('#channel-members [data-member-id="773523953"][data-online="true"]'),{timeout:5000});
  const light=await background();await settled();await page.emulateMedia({colorScheme:'dark'});
  assert.ok(await page.evaluate(()=>matchMedia('(prefers-color-scheme: dark)').matches));
  assert.notEqual(await background(),light,'workspace: system theme changes background');await settled();await profile();
  await page.setViewportSize({width:390,height:844});await overflow();await compact();await open();
  await waitForVisibility(page.locator('button[aria-label="Close workspace navigation"]'),{state:'detached'}); // :267 visible: :all
  await waitForCondition(()=>page.evaluate(()=>document.activeElement?.matches('#sidebar a[aria-current="page"]')));
  await actOnVisible(page.locator('#sidebar a[href]').first(),'press',{},['Shift+Tab']);
  assert.ok(await page.evaluate(()=>document.querySelector('#sidebar').contains(document.activeElement)),'workspace: drawer traps focus');
  await settled();await profile();
  await actOnVisible(filterVisibleText(page.locator('#sidebar a'),'HQ',{exact:true}),'click');
  await waitForVisibility(filterVisibleText(page.locator('.room--current'),'HQ'));
  await waitForVisibility(page.locator('#sidebar.open'),{state:'hidden'});
  await open();await page.keyboard.press('Escape');await waitForVisibility(page.locator('#sidebar.open'),{state:'hidden'});
  assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')||document.activeElement.textContent.trim()),'Open workspace navigation');await overflow();
  const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:1});
  assert.ok(await page.evaluate(()=>matchMedia('(pointer: coarse)').matches));
  const editor=page.locator('#composer textarea[name="message[markdown_source]"]');
  await actOnVisible(editor,'fill',{},['Mobile draft']);await actOnVisible(editor,'press',{},['Enter']);
  await waitForVisibleProperty(editor,'value','Mobile draft\n');
  // Message.exists? is checked against both live databases by the driver,
  // before Send Message, rather than inferred from a pending browser node.
  const {execFileSync}=await import('node:child_process');
  const db=JSON.parse(process.env.WS8BM_WORK_DATABASES)[new URL(page.url()).origin];
  const count=Number(execFileSync('python3',['-c',"import sqlite3,sys; c=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True); print(c.execute(\"SELECT COUNT(*) FROM messages WHERE markdown_source='Mobile draft'\").fetchone()[0]); c.close()",db],{encoding:'utf8'}));
  assert.equal(count,0,'workspace: coarse Enter creates no message');
  const written=page.waitForResponse(response=>response.request().method()==='POST'&&new URL(response.url()).pathname==='/rooms/201306877/messages',{timeout:10000});
  await actOnVisible(filterVisibleText(page.locator('#composer button'),'Send Message'),'click');
  await waitForVisibility(filterVisibleText(page.locator('.message__body'),'Mobile draft'),{timeout:10000});
  // Capybara drains its app server before closing the session. Drain the real
  // browser write too: an optimistic body alone cannot supply persisted rows.
  const response=await written;assert.equal(response.status(),200);
  await response.finished();

}
