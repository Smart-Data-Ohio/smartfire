import assert from 'node:assert/strict';
import {test} from 'node:test';
import {settleVisualTransitions} from './behavior-workspace.mjs';

test('finite transitions cannot outlive the pinned Capybara async-script budget',async()=>{
  await assert.rejects(settleVisualTransitions({evaluate:()=>new Promise(()=>{})}),error=>
    error.name==='TimeoutError'&&error.message==='Capybara async script timed out after 2000ms');
});
test('script exceptions remain primary and cancel the deadline timer',async()=>{
  const original=new Error('page closed during transition sampling');
  await assert.rejects(settleVisualTransitions({evaluate:()=>Promise.reject(original)}),error=>error===original);
});
