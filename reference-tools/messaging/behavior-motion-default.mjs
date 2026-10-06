// motion_test.rb:19-23: actual RAILS_ENV=test hosts, no injected test attribute.
import assert from 'node:assert/strict';
import {waitForCondition} from './behavior-visibility.mjs';
export const MOTION_DEFAULT='motion is off by default in the test environment';
export async function motionDefault(page,base,fixture) {
  await page.setViewportSize({width:1400,height:1400});
  await page.goto(`${base}/rooms/${fixture.motion_room_id}`);
  // Original setup joins HQ and waits for all mounted cable subscriptions.
  await waitForCondition(()=>page.locator('turbo-cable-stream-source').evaluateAll(nodes=>nodes.length>=3&&nodes.every(node=>node.hasAttribute('connected'))),{timeout:15000});
  const state=await page.evaluate(()=>({motion:document.documentElement.dataset.testMotion,
    medium:getComputedStyle(document.documentElement).getPropertyValue('--motion-medium').trim(),
    fast:getComputedStyle(document.documentElement).getPropertyValue('--motion-fast').trim(),
    quick:getComputedStyle(document.documentElement).getPropertyValue('--motion-quick').trim()}));
  assert.equal(state.motion,'off','motion: server test attribute');
  assert.equal(state.medium,'0ms','motion: medium token');
  assert.equal(state.fast,'0ms','motion: fast token');
  assert.equal(state.quick,'0ms','motion: quick token');
}
