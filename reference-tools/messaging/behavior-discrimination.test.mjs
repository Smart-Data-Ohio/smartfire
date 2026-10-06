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
  assert.equal(count,199);
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
  const probe={...valid,requiresReleaseClick:true};
  const observed=[{releaseClick:true,brokenGuard:true,menuVisible:true}];
  for(const anchor of ['await longPress(page);await assertMenuOpen(page)','await menuStillOpenAfterRelease()',`waitForVisibility(page.locator('#composer [data-composer-target="context"][hidden]'),{state:'attached'});\n  } else throw`]) {
    const error=failure('behavior-actions.mjs',anchor.startsWith('await longPress')?'assertMenuOpen(page);\n    // The browser':anchor);
    assert.equal(rejectionEvidence(name,'default',probe,error).valid,false,anchor);
    assert.equal(rejectionEvidence(name,'default',{...probe,observed},error).valid,true,anchor);
  }
  const hit=failure('behavior-actions.mjs',"equal(hit,'menu'",'AssertionError');
  assert.equal(rejectionEvidence(name,'default',{...probe,observed},{...hit,code:'ERR_ASSERTION'}).valid,false);
  for(const property of ['brokenGuard','menuVisible']) {
    const wrong=structuredClone(observed);wrong[0][property]=false;
    const error=failure('behavior-actions.mjs','await menuStillOpenAfterRelease()');
    assert.equal(rejectionEvidence(name,'default',{...probe,observed:wrong},error).valid,false);
  }
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

test('attachment reply fault requires an actual nonnull reply discarded by the served constructor',()=>{
  const name='Markdown replies and file attachments remain usable';
  const error=failure('behavior.mjs',"author.locator('.message[data-message-id] .message__reply-preview'),'A useful point'");
  const probe={...valid,requiresAttachmentReplyFault:true};
  assert.equal(rejectionEvidence(name,'default',probe,error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{originalReplyId:null,forcedReply:null}]},error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{originalReplyId:607264869,forcedReply:null}]},error).valid,true);
});

test('motion default fault requires removal of a real server-emitted test attribute',()=>{
  const name='motion is off by default in the test environment';
  const error=failure('behavior-motion-default.mjs',"assert.equal(state.motion,'off','motion: server test attribute')");
  const probe={...valid,requiresMotionAttribute:true};
  assert.equal(rejectionEvidence(name,'default',probe,error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{before:undefined,after:undefined}]},error).valid,false);
  assert.equal(rejectionEvidence(name,'default',{...probe,observed:[{before:'off',after:undefined}]},error).valid,true);
});

test('the first motion case is credited only at its off-canvas assertion',()=>{
  const name='mobile drawer animates in, lands in place, and returns focus with motion on';
  const error={...failure('behavior-motion.mjs',"ok(start<-1,'motion: drawer starts off-canvas')"),code:'ERR_ASSERTION'};
  assert.equal(rejectionEvidence(name,'default',valid,error).valid,true);
  const later={...failure('behavior-motion.mjs','await waitUntil(page,start=>{'),code:'ERR_ASSERTION'};
  assert.equal(rejectionEvidence(name,'default',valid,later).valid,false);
});


test('idle hidden context rejection requires the active composer and original post-click assertion',()=>{
  const name='a release click landing on the just-opened menu does not activate it';
  const error=failure('behavior-actions.mjs',`waitForVisibility(page.locator('#composer [data-composer-target="context"][hidden]'),{state:'attached'});\n  } else throw`);
  const state={contextScopes:[{form:'composer',hidden:false},{form:'ws8bm-idle-composer',hidden:true}]};
  const probe={...valid,requiresContextScopes:true,observed:[state]};
  assert.equal(rejectionEvidence(name,'unrelated-hidden-context',probe,error).valid,true);
  for(const anchor of ['await menuStillOpenAfterRelease()',"equal(hit,'menu'"]) {
    assert.equal(rejectionEvidence(name,'unrelated-hidden-context',probe,failure('behavior-actions.mjs',anchor)).valid,false,anchor);
  }
  for(const scopes of [[{form:'composer',hidden:false}],[{form:'ws8bm-idle-composer',hidden:true}],[]]) {
    assert.equal(rejectionEvidence(name,'unrelated-hidden-context',{...probe,observed:[{contextScopes:scopes}]},error).valid,false);
  }
});

test('upload mutants need the processed video or the delivered row to be reached',()=>{
  const video='uploading a fresh video in the thread composer';
  const poster=failure('behavior-uploads.mjs',"waitForVisibility(page.locator(\"#thread-panel div[style*='aspect-ratio'] video.message__attachment[poster]\")");
  const probe={...valid,requiresVideoFault:true,videoJobPerformed:true,observed:[{videoFaultSeen:true}]};
  assert.equal(rejectionEvidence(video,'default',probe,poster).valid,true);
  assert.equal(rejectionEvidence(video,'default',{...probe,videoJobPerformed:false},poster).valid,false);
  assert.equal(rejectionEvidence(video,'default',{...probe,observed:[]},poster).valid,false);
  assert.equal(rejectionEvidence(video,'default',probe,failure('behavior-uploads.mjs',"waitForVisibility(panel.locator('video.message__attachment')")).valid,false);
  const progress='late upload progress preserves a delivered attachment and reply preview';
  const overwritten={...failure('behavior-uploads.mjs',"equal(await body(),delivered,'progress: delivered body unchanged')",'AssertionError'),code:'ERR_ASSERTION'};
  const delivered={...valid,requiresProgressFault:true,observed:[{progressFault:{delivered:true}}]};
  assert.equal(rejectionEvidence(progress,'default',delivered,overwritten).valid,true);
  assert.equal(rejectionEvidence(progress,'default',{...delivered,observed:[{progressFault:{delivered:false}}]},overwritten).valid,false);
});
