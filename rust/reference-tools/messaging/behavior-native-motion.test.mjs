import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {extractMotion,NATIVE_MOTION_CASE} from './behavior-native-motion.mjs';
const source=execFileSync('git',['show','d7c7de92:test/system/motion_test.rb'],{encoding:'utf8'});
test('native motion executes every byte of the pinned animation body',()=>{
  const {body,helpers}=extractMotion(source);
  const start=source.indexOf(`  test "${NATIVE_MOTION_CASE}" do\n`),end=source.indexOf('\n  test ',start+1);
  assert.equal(body.replace('def test_phone',`test "${NATIVE_MOTION_CASE}" do`),source.slice(start,end));
  assert.equal(helpers,source.slice(source.indexOf('\n  def motion_token('),source.lastIndexOf('\nend')));
  assert.ok(helpers.includes('raise "transition listeners did not attach:'));
  assert.ok(helpers.includes('def wait_until(message, timeout: 5)'));
});
test('native motion refuses a different body or missing original helpers',()=>{
  assert.throws(()=>extractMotion(source.replace(NATIVE_MOTION_CASE,'unrelated case')));
  assert.throws(()=>extractMotion(source.replace('def motion_token(','def unrelated_helper(')));
});
