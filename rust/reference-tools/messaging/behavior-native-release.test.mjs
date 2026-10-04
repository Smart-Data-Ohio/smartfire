import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {extractRelease,RELEASE_CASE} from './behavior-native-release.mjs';
const source=execFileSync('git',['show','d7c7de92:test/system/message_interactions_test.rb'],{encoding:'utf8'});
test('native release executes the literal pinned body, including center hit and ensure',()=>{
  const {body,helpers}=extractRelease(source);
  const start=source.indexOf(`  test "${RELEASE_CASE}" do\n`),end=source.indexOf('\n  test ',start+1);
  assert.equal(body.replace('def test_phone',`test "${RELEASE_CASE}" do`),source.slice(start,end));
  assert.equal(helpers,'');
  assert.ok(body.includes('assert_equal "menu", hit'));
  assert.ok(body.includes('visible: false'));
  assert.ok(body.includes('page.current_window.resize_to(390, 844)'));
  const helper=execFileSync('git',['show','d7c7de92:test/test_helpers/system_test_helper.rb'],{encoding:'utf8'});
  assert.ok(helper.includes('def long_press(node, move_by: nil, hold: 0.7)'));
  assert.ok(helper.includes('visible: true, wait: 10'));
});
test('positive and negative release use the same native body',()=>{
  const dispatcher=readFileSync(new URL('./behavior.mjs',import.meta.url),'utf8');
  assert.ok(dispatcher.includes('nativeRelease(base,JSON.parse(process.env.WS8BM_WORK_DATABASES)[base],probe,negative||!!selectedMutant)'));
  assert.throws(()=>extractRelease(source.replace(RELEASE_CASE,'different declaration')));
});
