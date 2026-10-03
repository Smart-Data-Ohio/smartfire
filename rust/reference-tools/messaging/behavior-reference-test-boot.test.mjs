import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
test('reference test boot restores the literal pinned controller required before engine routes finish',()=>{
  const original=execFileSync('git',['show','d7c7de92:test/support/test_session_controller.rb']);
  const copied=readFileSync(new URL('./browser-test-session-controller.rb',import.meta.url));
  assert.deepEqual(copied,original);
  const build=readFileSync(new URL('./browser.Dockerfile',import.meta.url),'utf8');
  assert.ok(build.includes('reference-tools/messaging/browser-test-session-controller.rb /rails/test/support/test_session_controller.rb'));
});
