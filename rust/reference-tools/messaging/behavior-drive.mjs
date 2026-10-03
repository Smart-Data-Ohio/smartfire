// Literal predicates/actions from drive_attachments_test.rb d7c7de92.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {actOnVisible,filterVisibleText,waitForVisibility,waitForVisibleCount,waitForCondition} from './behavior-visibility.mjs';
import {assertMenuOpen} from './behavior-actions.mjs';
export const DRIVE_ONE='1AbcDefGhIjKlMnOpQrSt';
export function driveLast(base,thread=null) {
  const database=JSON.parse(process.env.WS8BM_WORK_DATABASES)[base];
  return JSON.parse(execFileSync('python3',['-c',
    'import sqlite3,json,sys; c=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True); row=c.execute("SELECT id,client_message_id FROM messages"+(" WHERE thread_id=?" if sys.argv[2]!="null" else "")+" ORDER BY id DESC LIMIT 1", (() if sys.argv[2]=="null" else (int(sys.argv[2]),))).fetchone(); ids=c.execute("SELECT file_id FROM drive_attachments WHERE message_id=? ORDER BY id",(row[0],)).fetchall() if row else []; print(json.dumps({"row":row,"files":[r[0] for r in ids]})); c.close()',database,JSON.stringify(thread)],{encoding:'utf8'}));
}
export async function driveAttachments({author:page,base,caseName,fixture}) {
  let scope=page.locator('body');
  async function choose() {
    await actOnVisible(scope.locator('button.composer__attachment-btn'),'click');
    await actOnVisible(filterVisibleText(scope.locator('button'),'From Google Drive'),'click');
  }
  async function attach(name) {
    const item=filterVisibleText(scope.locator('.drive-picker__item'),name);
    await actOnVisible(item,'hover');
    await actOnVisible(item.locator('button.drive-picker__attach'),'click');
  }
  const chips=()=>scope.locator('.composer__drive-attachments .drive-attachment-chip');
  const link=()=>scope.locator(`a.drive-attachment[href="https://drive.google.com/open?id=${DRIVE_ONE}"]`);
  const two=caseName.startsWith('edit a room message');
  if(caseName==='attach a Drive file from the thread composer') {
    await page.goto(`${base}/rooms/654632876?thread=${fixture.drive_thread_id}`);
    scope=page.locator('#thread-panel');
    await waitForVisibility(scope.locator('[data-thread-panel-target="conversation"]'),{timeout:10000});
    await waitForVisibility(scope.locator('button.composer__attachment-btn'),{timeout:10000});
    await waitForCondition(()=>page.evaluate(()=>getComputedStyle(document.querySelector('#thread-panel .thread-panel__surface')).transform==='none'),{timeout:10000});
    await choose();
    await waitForVisibility(scope.locator('[role="dialog"][aria-label="Find a Drive file"]'),{timeout:10000});
    await waitForVisibility(filterVisibleText(scope.locator('.drive-picker__item'),'Q3 Planning'),{timeout:10000});
    await attach('Q3 Planning');
    await waitForVisibility(filterVisibleText(chips(),'Q3 Planning'));
    await actOnVisible(scope.getByRole('combobox',{name:'Write a thread reply',exact:true}),'fill',{},['thread file attached']);
    await actOnVisible(scope.getByRole('button',{name:'Send Reply',exact:true}),'click');
    await waitForVisibility(link(),{timeout:10000});
    assert.deepEqual(driveLast(base,fixture.drive_thread_id).files,[DRIVE_ONE],'drive: persisted thread file');
    return;
  }
  await choose();
  if(!two) {
    await waitForVisibility(filterVisibleText(scope.locator('.drive-picker__item'),'Q3 Planning'));
    const dialog=scope.locator('[role="dialog"][aria-label="Find a Drive file"]');
    await waitForVisibility(dialog.locator('li[role="presentation"] > button[role="option"]'));
    await waitForVisibility(dialog.locator('li[role="presentation"] > button.drive-picker__attach'));
    await waitForVisibleCount(dialog.locator('[role="option"] button'),0);
  }
  await attach('Q3 Planning');
  if(!two) {
    await waitForVisibility(filterVisibleText(chips(),'Q3 Planning'));
    await waitForVisibleCount(scope.locator('[role="dialog"][aria-label="Find a Drive file"]'),0);
  }
  await choose();await attach('Budget 2026');
  await waitForVisibleCount(chips(),2);
  if(!two) {
    const size=await page.locator('.drive-attachment-chip__remove').first().evaluate(button=>{const r=button.getBoundingClientRect();return {width:r.width,height:r.height};});
    assert.ok(size.width>=24,'drive: chip width');assert.ok(size.height>=24,'drive: chip height');
    await actOnVisible(filterVisibleText(chips(),'Budget 2026').locator('button'),'click');
    await waitForVisibleCount(chips(),1);
  } else await actOnVisible(scope.getByRole('combobox',{name:'Write a message',exact:true}),'fill',{},['two files attached']);
  await actOnVisible(scope.getByRole('button',{name:'Send Message',exact:true}),'click');
  if(!two) {await waitForVisibleCount(chips(),0);await waitForVisibility(link());}
  else await waitForVisibleCount(scope.locator('a.drive-attachment'),2);
  await waitForVisibility(filterVisibleText(scope.locator('.drive-attachments .drive-chip__name'),'Q3 Planning'));
  if(two) await waitForVisibility(filterVisibleText(scope.locator('.drive-attachments .drive-chip__name'),'Budget 2026'));
  const saved=driveLast(base);
  assert.ok(saved.row,'drive: saved message identity');
  if(!two) {
    assert.deepEqual(saved.files,[DRIVE_ONE],'drive: persisted textless file');
    await page.goto(`${base}/rooms/654632876/messages/${saved.row[0]}/edit`);
    await waitForVisibility(filterVisibleText(page.locator('.drive-attachment-chip'),'Google Drive file'));
    await actOnVisible(page.getByRole('textbox',{name:'Message',exact:true}),'fill',{},['the file moved elsewhere']);
    await actOnVisible(page.locator('button[aria-label="Remove Google Drive file"]'),'click');
    await waitForVisibleCount(page.locator('.drive-attachment-chip'),0);
    await actOnVisible(page.getByRole('button',{name:'Save changes',exact:true}),'click');
    await waitForVisibility(filterVisibleText(page.locator('.message__body'),'the file moved elsewhere'),{timeout:10000});
    await waitForVisibleCount(page.locator('a.drive-attachment'),0);
    assert.deepEqual(driveLast(base).files,[],'drive: persisted empty attachments');
  } else {
    const message=page.locator(`[id=${JSON.stringify('message_'+saved.row[1])}]`);
    await actOnVisible(message.locator('[data-reply-target="body"]'),'click',{button:'right'});
    await assertMenuOpen(page);
    await actOnVisible(filterVisibleText(page.locator('#message-actions-menu button'),'Edit message'),'click');
    await waitForVisibility(filterVisibleText(page.locator('[data-composer-target="contextLabel"]'),'Editing Message'),{timeout:10000});
    await waitForVisibleCount(chips(),2);
    await actOnVisible(filterVisibleText(chips(),'Budget 2026').locator('button'),'click');
    await waitForVisibleCount(chips(),1);
    await actOnVisible(scope.getByRole('button',{name:'Send Message',exact:true}),'click');
    await waitForVisibleCount(scope.locator('a.drive-attachment'),1);
    await waitForVisibility(link());
    await waitForVisibility(page.locator('[data-composer-target="context"][hidden]'),{state:'attached'});
    assert.deepEqual(driveLast(base).files,[DRIVE_ONE],'drive: persisted remaining file');
  }
}
