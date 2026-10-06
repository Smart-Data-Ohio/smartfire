import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
const workspace=readFileSync(new URL('./behavior-workspace.mjs',import.meta.url),'utf8');
test('workspace mobile send keeps the pinned persisted-row selector and one two-second budget',()=>{
  assert.ok(workspace.includes('deadline=performance.now()+CAPYBARA_DEFAULT'));
  assert.ok(workspace.includes('WHERE room_id=201306877 AND creator_id=773523953'));
  assert.ok(workspace.includes("('Mobile draft'+chr(10),)"));
  assert.ok(workspace.includes('.message[data-message-id="${messageId}"] .message__body'));
  assert.ok(workspace.includes('timeout:Math.max(0,deadline-performance.now())'));
  assert.ok(!workspace.includes("page.locator('.message__body'),'Mobile draft'"));
});
