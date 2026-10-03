// The pinned Capybara upload actions/assertions, including original deadlines.
import assert from 'node:assert/strict';
import {nativePhone} from './behavior-native-phone.mjs';
export const ATTACHMENT_CASE='Markdown replies and file attachments remain usable';
export function extractAttachment(source) {
  const declaration=`  test "${ATTACHMENT_CASE}" do\n`;
  const start=source.indexOf(declaration),end=source.indexOf('\n  test ',start+1);
  assert.ok(start>=0&&end>start,'pinned attachment source boundaries');
  let body=source.slice(start,end).replace(`test "${ATTACHMENT_CASE}" do`,'def test_phone');
  const original='Rails.root.join("tmp/markdown-workspace-attachment.txt")';
  assert.equal(body.split(original).length,2,'one shared upload-path substitution');
  // ChromeDriver runs on the host, Ruby in the pinned image. Both see this
  // private mounted file. Its filename/contents/actions/assertions are intact.
  body=body.replace(original,'ENV.fetch("WS8BM_NATIVE_UPLOAD")');
  return {body,helpers:''};
}
export async function nativeAttachment(base,database) {
  await nativePhone(base,{sourcePath:'test/system/workspace_markdown_test.rb',extract:extractAttachment,line:143,label:'attachment',database});
}
