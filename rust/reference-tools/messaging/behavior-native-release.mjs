// Selenium resizes the outer window and uses W3C touch actions. A Playwright
// viewport/CDP touch translation changes the press point and menu hit test.
import assert from 'node:assert/strict';
import {nativePhone} from './behavior-native-phone.mjs';
export const RELEASE_CASE='a release click landing on the just-opened menu does not activate it';
export function extractRelease(source) {
  const start=source.indexOf(`  test "${RELEASE_CASE}" do\n`);
  const end=source.indexOf('\n  test ',start+1);
  assert.ok(start>=0&&end>start,'pinned release source boundaries');
  return {body:source.slice(start,end).replace(`test "${RELEASE_CASE}" do`,'def test_phone'),helpers:''};
}
export async function nativeRelease(base,database,probe={},mutated=false) {
  await nativePhone(base,{sourcePath:'test/system/message_interactions_test.rb',extract:extractRelease,line:55,label:'release',database,probe,
    ...(mutated?{mutation:['controllers/message_list_controller-','this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS','this.#suppressClickUntil = 0; window.__ws8bmBrokenReleaseGuard = true']}: {})});
}
