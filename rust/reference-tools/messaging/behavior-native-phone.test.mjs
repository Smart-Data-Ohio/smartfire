import {PIN} from './reference-pin.mjs';
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {extractPhone,PHONE_CASE} from './behavior-native-phone.mjs';
test('native phone executes the pin with only the explicitly excluded screenshot removed',()=>{
  const source=execFileSync('git',['show',`${PIN}:test/system/threads_test.rb`],{encoding:'utf8'});
  const {body,helpers}=extractPhone(source);
  const original=source.slice(source.indexOf(`  test "${PHONE_CASE}" do\n`),source.indexOf('\n  test "marks a joined thread'));
  assert.equal(body.replace('def test_phone',`test "${PHONE_CASE}" do`).replace('    click_button "Close threads"','    save_thread_screenshot "mobile-drawer.png"\n    click_button "Close threads"'),original);
  assert.equal(helpers,source.slice(source.indexOf('\n  private\n'),source.lastIndexOf('\nend')));
  const harness=readFileSync(new URL('./behavior-native-phone.rb',import.meta.url),'utf8');
  assert.ok(!harness.includes('assert_focused("#message-actions-menu'));
  assert.ok(!harness.includes('menu.contains(document.activeElement)'));
});
