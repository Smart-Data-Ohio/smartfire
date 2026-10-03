import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {mutationNames,mutationVariants} from './behavior-mutations.mjs';
import {actionMutations} from './behavior-action-mutations.mjs';
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
    for(const spec of mutationTarget(name,variant)) for(const target of [spec,...(spec.phase?[spec.phase]:[])]) assert.ok(readFileSync(new URL(target.module,import.meta.url),'utf8').includes(target.anchor),`${name}: ${variant}: ${target.anchor}`);
    count++;
  }
  assert.equal(count,194);
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

test('the profile width probe targets main content overflow, not the earlier document check',()=>{
  const name='the profile page fits phone widths without scrolling sideways';
  const content=failure('behavior-mobile-continuation.mjs','assert.ok(result.mainOverflow<=0');
  const document=failure('behavior-mobile-continuation.mjs','assert.ok(result.documentOverflow<=0');
  assert.equal(rejectionEvidence(name,'default',valid,content).valid,true);
  assert.equal(rejectionEvidence(name,'default',valid,document).valid,false);
});

test('initial Recent-tab setup cannot credit the post-click people assertion',()=>{
  const name='the picker shows category tabs and switches between them';
  const early=failure('behavior-toolbar.mjs',"await waitForVisibility(page.locator('#emoji-picker-tab-recent:not([aria-selected=\"true\"])'))");
  assert.equal(rejectionEvidence(name,'default',valid,early).valid,false);
  const post=failure('behavior-toolbar.mjs',"await tab('people')");
  assert.equal(rejectionEvidence(name,'default',valid,post).valid,true);
});

test('outside-tap setup cannot credit the post-outside closed assertion',()=>{
  const name='a tap outside closes the menu';
  const early=failure('behavior-attach-menu.mjs','async function expanded(value)');
  const post=failure('behavior-attach-menu.mjs','await expanded(false)');
  assert.equal(rejectionEvidence(name,'default',valid,early).valid,false);
  assert.equal(rejectionEvidence(name,'default',valid,post).valid,true);
});

test('the initial const checkpoint cannot credit the missing def variant',()=>{
  const name='editing a code block replaces its language colors and copied source';
  const early=failure('behavior-code.mjs',"await waitForVisibility(filterVisibleText(code.locator('.code-token')");
  assert.equal(rejectionEvidence(name,'missing-def',valid,early).valid,false);
  assert.equal(rejectionEvidence(name,'missing-def',valid,failure('behavior-code.mjs',"await highlight(marked(replacement,'python'),'def')")).valid,true);
});

test('the restored draft checkpoint cannot credit the cleared-draft variant',()=>{
  const name='thread drafts persist per thread without touching the channel draft';
  const early=failure('behavior-composer.mjs',"await waitForVisibleProperty(reply,'value','Thread draft')");
  assert.equal(rejectionEvidence(name,'transparent-cleared-thread-draft',valid,early).valid,false);
  assert.equal(rejectionEvidence(name,'transparent-cleared-thread-draft',valid,failure('behavior-composer.mjs',"await waitForVisibleProperty(reply,'value','')")).valid,true);
});


test('post-action source alone cannot credit an unexercised mutated handler',()=>{
  const name='the picker shows category tabs and switches between them';
  const error=failure('behavior-toolbar.mjs',"await tab('people')");
  const probe={...valid,requiredAction:{action:'chooseTab',target:'emoji-picker-tab-people'}};
  assert.equal(rejectionEvidence(name,'default',probe,error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{action:'chooseTab',target:'emoji-picker-tab-flags'}]},error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{action:'chooseTab',target:'emoji-picker-tab-people'}]},error).valid,true);
});

test('lazy data mutation names its initial-connect assertion, not its later fetch assertion',()=>{
  const name='the picker loads its emoji data only on first open';
  const early=failure('behavior-toolbar.mjs',"assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),false)");
  const later=failure('behavior-toolbar.mjs',"assert.equal((await resources()).some(url=>url.endsWith('.json')&&url.includes('emoji')),true)");
  assert.equal(rejectionEvidence(name,'default',valid,early).valid,true);
  assert.equal(rejectionEvidence(name,'default',valid,later).valid,false);
});

test('the post-save draft variant cannot credit cancellation or shared field setup',()=>{
  const name='edits through the normal composer and restores the saved draft on cancel and success';
  const early=failure('behavior-actions.mjs',"await field(page,'A draft that must survive editing')");
  assert.equal(rejectionEvidence(name,'transparent-saved-draft',valid,early).valid,false);
});

test('older-search text mutation cannot credit the pre-pagination absence assertion',()=>{
  const name='search tolerates operators, shows an empty state and pages older results';
  const early=failure('behavior-search-forward.mjs',"await waitForVisibleCount(filterVisibleText(page.locator('#search-results .message'),'system paging alpha'),0)");
  assert.equal(rejectionEvidence(name,'transparent-older-search-text',valid,early).valid,false);
});

test('menu-owner opacity mutation cannot credit an unrelated closed-menu assertion',()=>{
  const early=failure('behavior-actions.mjs',"await waitForVisibility(page.locator('.message[data-message-actions-open]'),{state:'hidden'})");
  for(const [name,variant] of [['the more button opens the shared menu for its message','transparent-more-owner'],['copies message text and link and forwards to a server-provided thread destination','transparent-menu-owner']])
    assert.equal(rejectionEvidence(name,variant,valid,early).valid,false);
});


test('the served tab-handler witness is syntactically executable JavaScript',()=>{
  const replacement=actionMutations.find(([name])=>name==='the picker shows category tabs and switches between them')[1][2];
  assert.doesNotThrow(()=>new Function(`return class {${replacement}}}`));
});
