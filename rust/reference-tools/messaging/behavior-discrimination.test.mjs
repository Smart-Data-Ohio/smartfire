import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {mutationNames,mutationVariants} from './behavior-mutations.mjs';
import {mutationTarget,rejectionEvidence} from './behavior-discrimination.mjs';
function failure(module,anchor,name='TimeoutError') {
  const url=new URL(module,import.meta.url),source=readFileSync(url,'utf8');
  const index=source.indexOf(anchor);assert.ok(index>=0,anchor);
  const prefix=source.slice(0,index),line=prefix.split('\n').length,column=index-prefix.lastIndexOf('\n');
  return {name,stack:`${name}: probe\n    at case (${url}:${line}:${column})`};
}
const caseName='editing to add a URL renders its card live and the edited marker on load';
const valid={ready:true,applied:1};
test('every registered mutant names an existing intended assertion',()=>{
  let count=0;
  for(const name of mutationNames) for(const variant of mutationVariants(name)) {
    for(const target of mutationTarget(name,variant)) assert.ok(readFileSync(new URL(target.module,import.meta.url),'utf8').includes(target.anchor),`${name}: ${variant}: ${target.anchor}`);
    count++;
  }
  assert.equal(count,179);
});
test('earlier Loading timeout earns no delayed-marker rejection credit',()=>{
  const early=failure('behavior-search-forward.mjs',"waitForVisibility(filterVisibleText(message.locator('.x-post-card'),'Loading post')");
  // Exactly the old 2921b282 rule: this unrelated timeout was credited.
  assert.ok(valid.ready&&valid.applied&&early&&(early.name==='TimeoutError'||early.code==='ERR_ASSERTION'));
  const result=rejectionEvidence(caseName,'delayed-url-edited-marker',valid,early);
  assert.equal(result.valid,false);assert.deepEqual(result.reasons,['intended assertion did not fail']);
  const intended=failure('behavior-search-forward.mjs',"waitForVisibility(filterVisibleText(message.locator('.message__edited'),'(edited)'))");
  assert.equal(rejectionEvidence(caseName,'delayed-url-edited-marker',valid,intended).valid,true);
});
test('startup, network, missing mutation and escaped checks never earn credit',()=>{
  const intended=failure('behavior-search-forward.mjs',"waitForVisibility(filterVisibleText(message.locator('.message__edited'),'(edited)'))");
  for(const probe of [{ready:false,applied:1},{ready:true,applied:0},{...valid,networkFailures:['net::ERR_CONNECTION_RESET']}]) assert.equal(rejectionEvidence(caseName,'delayed-url-edited-marker',probe,intended).valid,false);
  assert.equal(rejectionEvidence(caseName,'delayed-url-edited-marker',valid).valid,false);
});
test('an earlier wait on the same source line cannot credit a later assertion',()=>{
  const error=failure('behavior-toolbar.mjs',"waitForVisibility(option('Grinning face'),{timeout:DELIVERY_WAIT});await focused('Search emoji and icons')");
  const result=rejectionEvidence('picker arrows move through options, Enter selects, and Escape returns focus','default',valid,error);
  assert.equal(result.valid,false);
});

test('method-name stack columns preserve only their own receiver',()=>{
  const error=failure('behavior-message-list.mjs','equal(active,first)');
  assert.equal(rejectionEvidence("a focus move during a stream render survives Turbo's focus restore",'default',valid,error).valid,true);
});

test('release-menu failure requires the broken guard and a click into the mounted menu',()=>{
  const name='a release click landing on the just-opened menu does not activate it';
  const error=failure('behavior-actions.mjs','await assertMenuOpen(page)');
  const probe={...valid,requiresReleaseClick:true};
  assert.equal(rejectionEvidence(name,'default',probe,error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{releaseClick:true,brokenGuard:true,menuVisible:true}]},error).valid,true);
  const geometry=failure('behavior-actions.mjs',"assert.equal(hit,'menu')");
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{releaseClick:true,brokenGuard:true,menuVisible:true}]},geometry).valid,false);
});


test('the redelivery flag names its original visible checkpoint, not a later action',()=>{
  const name='a duplicate delivery does not replace the message while its actions are open';
  const checkpoint=failure('behavior-actions.mjs',"waitForVisibility(page.locator('html[data-duplicate-delivery-rendered]')");
  const later=failure('behavior-actions.mjs',"actOnVisible(page.getByRole('menuitem',{name:'Edit message'");
  assert.equal(rejectionEvidence(name,'transparent-redelivery-flag',valid,checkpoint).valid,true);
  assert.equal(rejectionEvidence(name,'transparent-redelivery-flag',valid,later).valid,false);
});
