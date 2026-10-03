import assert from 'node:assert/strict';
import {test} from 'node:test';
import {settleVisualTransitions} from './behavior-workspace.mjs';
import {readFileSync} from 'node:fs';
import {installMutation} from './behavior-mutations.mjs';
import {rejectionEvidence} from './behavior-discrimination.mjs';
import {WORKSPACE_CASE} from './behavior-workspace.mjs';

test('finite transitions cannot outlive the pinned Capybara async-script budget',async()=>{
  await assert.rejects(settleVisualTransitions({evaluate:()=>new Promise(()=>{})}),error=>
    error.name==='TimeoutError'&&error.message==='Capybara async script timed out after 2000ms');
});
test('script exceptions remain primary and cancel the deadline timer',async()=>{
  const original=new Error('page closed during transition sampling');
  await assert.rejects(settleVisualTransitions({evaluate:()=>Promise.reject(original)}),error=>error===original);
});
function timeoutAt(anchor) {
  const url=new URL('./behavior-workspace.mjs',import.meta.url),source=readFileSync(url,'utf8');
  const offset=source.indexOf(anchor);assert.ok(offset>=0);
  const prefix=source.slice(0,offset);
  return {name:'TimeoutError',stack:`TimeoutError: synthetic\n    at workspace (${url}:${prefix.split('\n').length}:${offset-prefix.lastIndexOf('\n')})`};
}
test('slow-animation witness selects the actual message, not the keyframes rule',async()=>{
  let selected;
  const page={route:async()=>{},isClosed:()=>false,locator:selector=>({evaluateAll:async()=>{selected=selector;return [];}})};
  const probe={};await installMutation(page,WORKSPACE_CASE,probe,'slow-workspace-animation');
  await probe.observers[0]();assert.equal(selected,'.message:last-child');
  assert.deepEqual(probe.requiredAnimation,{name:'ws8bm-slow-settle',duration:4000});
});
test('animation deadline credit requires its specific active four-second state',()=>{
  const probe={ready:true,applied:1,observers:[()=>{}],requiredAnimation:{name:'ws8bm-slow-settle',duration:4000}};
  for(const animation of [null,{name:'unrelated',duration:4000,playState:'running'},{name:'ws8bm-slow-settle',duration:1000,playState:'running'},{name:'ws8bm-slow-settle',duration:4000,playState:'finished'}]) {
    assert.equal(rejectionEvidence(WORKSPACE_CASE,'slow-workspace-animation',{...probe,observed:[{animations:animation?[animation]:[]}]},timeoutAt('await settled()')).valid,false);
  }
  const observed=[{animations:[{name:'ws8bm-slow-settle',duration:4000,playState:'running'}]}];
  assert.equal(rejectionEvidence(WORKSPACE_CASE,'slow-workspace-animation',{...probe,observed},timeoutAt('await settled()')).valid,true);
  assert.equal(rejectionEvidence(WORKSPACE_CASE,'slow-workspace-animation',{...probe,observed},timeoutAt("'workspace: profile inside navigation'")).valid,false);
});
