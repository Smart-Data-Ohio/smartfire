// composer_attach_menu_test.rb d7c7de92: actual controls and browser file,
// clipboard and drop events. These checks never replace an app response.
import assert from 'node:assert/strict';
import {waitForVisibility,waitForVisibleCount,actOnVisible,filterVisibleText} from './behavior-visibility.mjs';
export async function attachMenu({author,recipient,caseName}) {
  const uploads=[];
  for(const browser of [author,recipient]) browser.on('request',request=>{
    if(request.method()==='POST'&&(/\/messages(?:\.json)?$/.test(new URL(request.url()).pathname)||new URL(request.url()).pathname.endsWith('/direct_uploads'))) uploads.push(request.url());
  });
  const page=caseName==='+ opens the file picker directly without Drive'||caseName==='device files, paste, and drag-and-drop still preview uploads'?recipient:author;
  const button=page.locator('#composer button.composer__attachment-btn');
  const menu=page.locator('#composer .attach-menu');
  async function expanded(value) {await waitForVisibility(button.locator(`:scope[aria-expanded="${value}"]`));}
  async function recorder() {
    await page.evaluate(()=>{
      window.filePickerClicks=0;
      document.querySelector('#composer [data-attach-menu-target="fileInput"]').addEventListener('click',()=>window.filePickerClicks++);
    });
  }
  if(caseName==='+ shows both attach options when Drive is available') {
    await actOnVisible(button,'click',{});await expanded(true);
    await waitForVisibility(menu.getByRole('menuitem',{name:'From this device',exact:true}));
    await waitForVisibility(menu.getByRole('menuitem',{name:'From Google Drive',exact:true}));
  } else if(caseName==='From this device triggers the file input') {
    await recorder();await actOnVisible(button,'click',{});await actOnVisible(menu.getByRole('menuitem',{name:'From this device',exact:true}),'click',{});
    assert.equal(await page.evaluate(()=>window.filePickerClicks),1);await expanded(false);
  } else if(caseName==='+ opens the file picker directly without Drive') {
    await recorder();await waitForVisibleCount(menu,0);await actOnVisible(button,'click',{});assert.equal(await page.evaluate(()=>window.filePickerClicks),1);
  } else if(caseName==='arrow keys move between items and Escape closes back onto +') {
    await actOnVisible(button,'focus',{});await actOnVisible(button,'press',{},['ArrowDown']);await expanded(true);
    await page.waitForFunction(()=>document.activeElement===document.querySelector('.attach-menu [role="menuitem"]:nth-child(1)'));
    await page.keyboard.press('ArrowDown');await page.waitForFunction(()=>document.activeElement===document.querySelector('.attach-menu [role="menuitem"]:nth-child(2)'));
    await page.keyboard.press('ArrowUp');await page.waitForFunction(()=>document.activeElement===document.querySelector('.attach-menu [role="menuitem"]:nth-child(1)'));
    await page.keyboard.press('Escape');await expanded(false);assert.equal(await button.evaluate(node=>document.activeElement===node),true);
  } else if(caseName==='a tap outside closes the menu') {
    await actOnVisible(button,'click',{});await expanded(true);await actOnVisible(page.locator('.room-header__name'),'click',{});await expanded(false);
  } else if(caseName==='phone layout keeps the menu above the composer with no horizontal overflow') {
    await page.setViewportSize({width:390,height:844});await actOnVisible(button,'click',{});await waitForVisibility(menu.getByRole('menuitem',{name:'From Google Drive',exact:true}));
    // Measure an actionable, settled menu, after its enter transform. This
    // is an interaction-state wait, with no fixed sleep or changed bounds.
    await actOnVisible(menu.getByRole('menuitem',{name:'From Google Drive',exact:true}),'click',{trial:true});
    await menu.evaluate(element=>Promise.all(element.getAnimations({subtree:true}).map(animation=>animation.finished)));
    const geometry=await page.evaluate(()=>{
      const menu=document.querySelector('.attach-menu').getBoundingClientRect(),button=document.querySelector('button.composer__attachment-btn').getBoundingClientRect();
      return {menu:{left:menu.left,right:menu.right,top:menu.top,bottom:menu.bottom,height:menu.height},buttonTop:button.top,
        items:[...document.querySelectorAll('.attach-menu [role="menuitem"]')].map(item=>item.getBoundingClientRect().height),
        overflow:document.documentElement.scrollWidth>window.innerWidth+1,viewportWidth:window.innerWidth};
    });
    assert.equal(geometry.overflow,false);assert.ok(geometry.menu.left>=0);assert.ok(geometry.menu.right<=geometry.viewportWidth+1);
    assert.ok(geometry.menu.bottom<=geometry.buttonTop+1,'menu sits above +');assert.ok(geometry.items.length);
    for(const height of geometry.items) {assert.ok(height>=44,JSON.stringify(geometry));assert.ok(height<=64,JSON.stringify(geometry));}
    assert.ok(geometry.menu.height<=geometry.items.reduce((a,b)=>a+b,0)+40);
  } else if(caseName==='device files, paste, and drag-and-drop still preview uploads') {
    await page.evaluate(()=>{
      const transfer=new DataTransfer();transfer.items.add(new File(['hello'],'hello.txt',{type:'text/plain'}));
      const input=document.querySelector('#composer [data-attach-menu-target="fileInput"]');input.files=transfer.files;input.dispatchEvent(new Event('input',{bubbles:true}));
    });
    await waitForVisibility(filterVisibleText(page.locator('#composer .composer__file'),'hello'));
    await page.evaluate(()=>{
      const transfer=new DataTransfer();transfer.items.add(new File(['pasted'],'pasted.png',{type:'image/png'}));
      document.querySelector('#composer textarea').dispatchEvent(new ClipboardEvent('paste',{clipboardData:transfer,bubbles:true,cancelable:true}));
    });
    await waitForVisibility(filterVisibleText(page.locator('#composer .composer__file'),'pasted'));
    await page.evaluate(()=>{
      const transfer=new DataTransfer();transfer.items.add(new File(['dropped'],'dropped.txt',{type:'text/plain'}));
      const event=new DragEvent('drop',{bubbles:true,cancelable:true});Object.defineProperty(event,'dataTransfer',{value:transfer});document.querySelector('#composer').dispatchEvent(event);
    });
    await waitForVisibility(filterVisibleText(page.locator('#composer .composer__file'),'dropped'));await waitForVisibleCount(page.locator('#composer .composer__file'),3);
  } else throw new Error(`unimplemented attach menu case ${caseName}`);
  assert.deepEqual(uploads,[],"pickers and unsent previews must not POST uploads or messages");
}
