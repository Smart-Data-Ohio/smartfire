import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {motionCases} from './behavior-motion.mjs';
import {WORKSPACE_CASE} from './behavior-workspace.mjs';
import {mutationTarget,rejectionEvidence} from './behavior-discrimination.mjs';
function failure(anchor) {
  const url=new URL('./behavior-motion.mjs',import.meta.url),source=readFileSync(url,'utf8');
  const index=source.indexOf(anchor);assert.ok(index>=0);
  const prefix=source.slice(0,index);
  return {name:'TimeoutError',stack:`TimeoutError: synthetic\n    at synthetic (${url}:${prefix.split('\n').length}:${index-prefix.lastIndexOf('\n')})`};
}
for(const name of [...motionCases,WORKSPACE_CASE])test(`motion/workspace attribution exists: ${name}`,()=>{
  const target=mutationTarget(name,'default')[0];assert.equal(mutationTarget(name,'default').length,1);
  assert.ok(readFileSync(new URL(target.module,import.meta.url),'utf8').includes(target.anchor));
});
test('reopen-focus cannot borrow the initial-open focus assertion',()=>{
  const name=motionCases[7],probe={ready:true,applied:1};
  assert.equal(rejectionEvidence(name,'default',probe,failure('await focused(current)')).valid,false);
  assert.equal(rejectionEvidence(name,'default',probe,failure('await reopenedCurrentFocus(page,current)')).valid,true);
});
test('first reveal cannot borrow the later viewport predicate or startup',()=>{
  const name=motionCases[6],probe={ready:true,applied:1};
  assert.equal(rejectionEvidence(name,'default',probe,failure("await waitUntil(page,drawerVisible")).valid,false);
  assert.equal(rejectionEvidence(name,'default',probe,failure('await firstOpenCurrentFocus(page,current)')).valid,true);
  assert.equal(rejectionEvidence(name,'default',{...probe,ready:false},failure('await firstOpenCurrentFocus(page,current)')).valid,false);
});
test('member and directory geometry faults cannot borrow their baseline snapshots',()=>{
  for(const [name,wrong,right] of [[motionCases[1],'const before=await lefts()',"'motion: member-select positions'"],[motionCases[2],'const before=await tops()',"'motion: directory-select positions'"]]) {
    const probe={ready:true,applied:1};
    assert.equal(rejectionEvidence(name,'default',probe,failure(wrong)).valid,false);
    assert.equal(rejectionEvidence(name,'default',probe,failure(right)).valid,true);
  }
});
