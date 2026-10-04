// Selenium resizes the outer window and uses W3C touch actions. A Playwright
// viewport/CDP touch translation changes the press point and menu hit test.
import assert from 'node:assert/strict';
import {nativePhone} from './behavior-native-phone.mjs';
export const RELEASE_CASE='a release click landing on the just-opened menu does not activate it';
export const RELEASE_SCOPE_MUTATION=['controllers/message_list_controller-',
  'this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS',
  `this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS; {
    const context=document.querySelector('#composer [data-composer-target="context"]');
    const sibling=document.createElement('form');sibling.id='ws8bm-idle-composer';
    const idle=context.cloneNode(true);idle.hidden=true;sibling.append(idle);document.body.append(sibling);
    context.hidden=false;
  }`];
export function extractRelease(source) {
  const start=source.indexOf(`  test "${RELEASE_CASE}" do\n`);
  const end=source.indexOf('\n  test ',start+1);
  assert.ok(start>=0&&end>start,'pinned release source boundaries');
  const body=source.slice(start,end).replace(`test "${RELEASE_CASE}" do`,'def test_phone');
  const context=`assert_selector "[data-composer-target='context'][hidden]", visible: false`;
  assert.equal(body.split(context).length,2,'one pinned hidden context assertion');
  // Preserve main's active-composer scope. The hidden-all predicate, deadline
  // and native source line stay intact; an idle sibling is not the composer.
  return {body:body.replace(context,`assert_selector "#composer [data-composer-target='context'][hidden]", visible: false`),helpers:''};
}
export async function nativeRelease(base,database,probe={},mutated=false,variant='default') {
  await nativePhone(base,{sourcePath:'test/system/message_interactions_test.rb',extract:extractRelease,line:55,label:'release',database,probe,
    ...(mutated?{mutation:variant==='unrelated-hidden-context'?RELEASE_SCOPE_MUTATION:['controllers/message_list_controller-','this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS','this.#suppressClickUntil = 0; window.__ws8bmBrokenReleaseGuard = true']}: {})});
}
