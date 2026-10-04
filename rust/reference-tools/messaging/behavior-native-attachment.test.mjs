import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {extractAttachment,ATTACHMENT_CASE} from './behavior-native-attachment.mjs';
const source=execFileSync('git',['show','d7c7de92:test/system/workspace_markdown_test.rb'],{encoding:'utf8'});
test('native attachment retains every pinned action/assertion with only the shared file path adapted',()=>{
  const {body,helpers}=extractAttachment(source);
  const start=source.indexOf(`  test "${ATTACHMENT_CASE}" do\n`),end=source.indexOf('\n  test ',start+1);
  assert.equal(body.replace('def test_phone',`test "${ATTACHMENT_CASE}" do`).replace('ENV.fetch("WS8BM_NATIVE_UPLOAD")','Rails.root.join("tmp/markdown-workspace-attachment.txt")'),source.slice(start,end));
  assert.equal(helpers,'');
  assert.ok(body.includes('wait: 10'));
  assert.ok(body.includes('Message.joins(:attachment_attachment).find_by!'));
});
test('native attachment refuses a changed declaration or upload path',()=>{
  assert.throws(()=>extractAttachment(source.replace(ATTACHMENT_CASE,'unrelated case')));
  assert.throws(()=>extractAttachment(source.replace('tmp/markdown-workspace-attachment.txt','other.txt')));
});
