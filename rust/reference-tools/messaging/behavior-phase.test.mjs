// Synthetic source-stack inputs to the actual current installer and classifier.
// No browser, server, Docker, build, or tracked-file mutation occurs.
// The installer really rewrites a read-only in-memory copy of its asset; failure
// stacks are controlled inputs at real initial and later assertion call sites.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {readFileSync} from 'node:fs';
import {installMutation} from './behavior-mutations.mjs';
import {rejectionEvidence} from './behavior-discrimination.mjs';

function frame(module,anchor,after='') {
  const url=new URL(`./${module}`,import.meta.url);
  const source=readFileSync(url,'utf8');
  const lower=after?source.indexOf(after):0;
  assert.ok(lower>=0,`missing scope ${after}`);
  const index=source.indexOf(anchor,lower);
  assert.ok(index>=0,`missing anchor ${anchor}`);
  const prefix=source.slice(0,index);
  return `    at synthetic (${url}:${prefix.split('\n').length}:${index-prefix.lastIndexOf('\n')})`;
}
async function installedProbe(caseName,variant,asset) {
  let handler,served;
  const page={addInitScript:async()=>{},route:async(_,callback)=>{handler=callback;}};
  const probe={ready:true,applied:0};
  await installMutation(page,caseName,probe,variant);
  const css=asset==='messages';
  const source=readFileSync(new URL(css?'../../web/app/assets/stylesheets/messages.css':`../../web/app/javascript/controllers/${asset}.js`,import.meta.url),'utf8');
  const response={text:async()=>source,headers:()=>({'content-type':'text/javascript'}),status:()=>200};
  await handler({
    request:()=>({url:()=>css?'http://synthetic.invalid/assets/messages-synthetic.css':`http://synthetic.invalid/assets/controllers/${asset}-synthetic.js`,method:()=> 'GET'}),
    fetch:async()=>response,
    fulfill:async result=>{served=result.body;},
    continue:async()=>{throw new Error('registered asset unexpectedly not rewritten');},
  });
  assert.equal(probe.applied,1);
  assert.notEqual(served,source);
  return probe;
}
function check(caseName,variant,probe,description,expectedAttribution,frames) {
  const error={name:'TimeoutError',stack:`TimeoutError: synthetic assertion failure\n${frames.join('\n')}`};
  const evidence=rejectionEvidence(caseName,variant,probe,error);
  test(description,()=>assert.equal(evidence.valid,expectedAttribution==='REJECTED',JSON.stringify(evidence)));

}

const thread='thread drafts persist per thread without touching the channel draft';
const variant='hidden-initial-composer-conversation';
const threadProbe=await installedProbe(thread,variant,'thread_panel_controller');
threadProbe.observed=[{visibility:'hidden'}];
const conversation=`await waitForVisibility(panel.locator('[data-thread-panel-target="conversation"]'),{timeout:DELIVERY_WAIT})`;
check(thread,variant,threadProbe,'Initial conversation immediately after creation (affected control)','REJECTED',[
  frame('behavior-composer.mjs','await initialConversationAfterCreate()'),
]);
check(thread,variant,threadProbe,'Later reopen after room navigation; sessionStorage one-shot guard suppresses the mutation','INVALID',[
  frame('behavior-composer.mjs',conversation,'async function thread()'),
  frame('behavior-composer.mjs','await thread();',`await room(fixture.pets_id);await room(654632876);await field(page,'Channel draft')`),
]);

const menu='opens message actions from context menu and keyboard, and cancels a moving long press';
const menuProbe=await installedProbe(menu,'default','message_list_controller');
const menuWait=frame('behavior-actions.mjs',`await waitForVisibility(page.locator('#message-actions-menu:not([hidden])')`);
check(menu,'default',menuProbe,'Initial right-click; onContextMenu is the affected handler (control)','REJECTED',[
  menuWait,frame('behavior-actions.mjs','await assertMenuOpen(page)','export async function openMenu'),
  frame('behavior-actions.mjs','await openMenu(page)',`if(caseName.startsWith('opens message actions'))`),
]);
check(menu,'default',menuProbe,'Later Shift+F10 checkpoint; unaffected onKeydown path','INVALID',[
  menuWait,frame('behavior-actions.mjs','await assertMenuOpen(page)',`await actOnVisible(row,'click',{});await row.dispatchEvent('keydown'`),
]);
check(menu,'default',menuProbe,'Later long-press checkpoint; unaffected onPointerUp path','INVALID',[
  menuWait,frame('behavior-actions.mjs','await assertMenuOpen(page)','await longPress(page,true);await closedMenu(page)'),
]);

const submitted='sending preserves the submitted source and a newer draft';
const submittedVariant='hidden-submitted-body-visible-strong';
const submittedProbe=await installedProbe(submitted,submittedVariant,'messages');
// Controlled observer output from an earlier bold body, not a real DOM read.
// The first message remains in the room when the later plain draft is sent.
submittedProbe.observed=[{
  selector:'.message:has(.markdown-body strong) .message__body',
  subject:'synthetic earlier bold first message, not the later plain draft',
  messageText:'First message stays exact.',opacity:'1',visibility:'hidden',display:'block',seleniumVisible:false,
}];
const bodyWait=frame('behavior.mjs','await waitForVisibleContentCount(');
check(submitted,submittedVariant,submittedProbe,'Initial strong-containing first message (affected control; synthetic observer state)','REJECTED',[
  bodyWait,frame('behavior.mjs',"await text(page,'First message stays exact.')"),
]);
check(submitted,submittedVariant,submittedProbe,'Later plain newer-draft message; earlier bold body supplies the unrelated hidden-state witness','INVALID',[
  bodyWait,frame('behavior.mjs','await text(page,second)'),
]);

// A seeded/earlier bold message cannot witness this submitted-message mutation.
check(submitted,submittedVariant,{...submittedProbe,observed:[{messageText:'A different bold message',visibility:'hidden'}]},
  'Correct phase with unrelated hidden bold body is INVALID','INVALID',[
    bodyWait,frame('behavior.mjs',"await text(page,'First message stays exact.')"),
  ]);

check(menu,'default',menuProbe,'Initial call but failure in the visible right-click lookup before acting is INVALID','INVALID',[
  frame('behavior-actions.mjs',"await actOnVisible(row.locator('[data-reply-target=\"body\"]')",'export async function openMenu'),
  frame('behavior-actions.mjs','await openMenu(page)',"if(caseName.startsWith('opens message actions'))"),
]);
