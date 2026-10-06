// Pinned threads_test.rb:159-489. No screenshots; native focus, menus and read state.
import assert from 'node:assert/strict';
import {actOnVisible,waitForVisibility,filterVisibleText,waitForVisibleCount,waitForCondition,visibleCount,visibleMatch,waitForVisibleProperty} from './behavior-visibility.mjs';
export const continuationCases=[
  'tracks work, assigns an owner, completes and reopens it without losing the conversation',
  'shows work-thread guidance in the new-thread form and on the work page',
  'keeps the new-thread guidance usable on a phone',
  'shows work assignment activity to the owner and opens the exact thread',
  'keeps the thread drawer usable on a phone and preserves the channel',
  'marks a joined thread read only while the conversation is visible',
  'opens a shared thread message link around an older post',
  'keeps an anchored older thread unread when a new reply arrives',
];
export async function threadContinuation({author,recipient,base,caseName,fixture,create,finishCreate,assertThreadMessage,reply,threadMenu,close}) {
  const panel=author.locator('#thread-panel');
  const target=(name,page=author)=>page.locator(`#thread-panel [data-thread-panel-target="${name}"]`);
  const text=async(node,value,timeout=2000)=>waitForVisibility(filterVisibleText(node,value),{timeout});
  const click=async node=>actOnVisible(node,'click');
  async function open(page=author) {
    if(!await visibleCount(page.locator('body.thread-panel-open'))){
      const toggle=page.locator('[data-thread-panel-target="browserToggle"]');
      if(await visibleCount(toggle)) await click(toggle);
      else {await click(page.getByRole('button',{name:'More actions',exact:true,includeHidden:true}));await click(page.locator('#header-overflow-menu [data-thread-panel-target="browserToggle"]'));}
    }
    await waitForVisibility(page.locator('#thread-panel[aria-hidden="false"]'),{timeout:10000});
  }
  async function conversation(page,name) {
    await waitForVisibility(target('conversation',page),{timeout:10000});
    await text(target('conversationTitle',page),name,10000);
  }
  const overflow=async()=>assert.ok(await author.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no horizontal overflow (threads_test.rb:617)');
  async function work() {
    await click(target('manage').locator('summary'));
    await click(target('convertToWork'));
    await waitForVisibility(target('work'),{timeout:10000});
  }
  async function owner() {
    await click(target('workManage').locator('summary'));
    await actOnVisible(target('workOwner'),'selectOption',{},[{label:'Kevin'}]);
    // Original model reload plus subsequent rendered label, :175-176/:269-273.
    const id=await panel.locator('form[data-controller~="composer"]').getAttribute('data-composer-thread-id-value');
    await waitForCondition(()=>author.evaluate(async id=>{const r=await fetch(`/rooms/654632876/threads/${id}.json`);return(await r.json()).thread.work_owner_id===712064548;},id),{timeout:10000});
    await text(target('workOwnerLabel'),'Kevin',10000);
  }
  async function progress() {
    await click(target('workManage').locator('summary'));
    await actOnVisible(target('workStatus'),'selectOption',{},['in_progress']);
    await text(target('workStatusLabel'),'In progress',10000);
  }
  if(caseName.startsWith('tracks work')) {
    const name='Work handoff thread',body='The work conversation must survive completion.';
    await create(name,body,false);await finishCreate(name);await work();await text(target('workStatusLabel'),'Planned');await owner();await progress();
    await click(target('workHistory').locator('summary'));
    await text(target('workHistory'),/Owner: Unassigned.*Kevin/,10000);
    await click(target('workManage').locator('summary'));await click(panel.getByRole('button',{name:'Complete work',exact:true,includeHidden:true}));
    await text(target('workStatusLabel'),'Done',10000);await assertThreadMessage(body);
    await click(target('workManage').locator('summary'));await click(panel.getByRole('button',{name:'Reopen work',exact:true,includeHidden:true}));
    await text(target('workStatusLabel'),'Planned',10000);await assertThreadMessage(body);
    await author.goto(base+'/work');await text(author.locator('.work-threads__item'),name,10000);await text(author.locator('.work-threads__item'),'Planned');
  } else if(caseName.startsWith('shows work-thread guidance')) {
    await open();await click(panel.getByRole('button',{name:'New thread',exact:true,includeHidden:true}));await waitForVisibility(target('create'),{timeout:10000});
    const createForm=target('create');await text(createForm.locator('details.thread-panel__guide:not([open]) summary'),'How to start a work thread');
    await click(filterVisibleText(createForm.locator('summary'),'How to start a work thread'));
    await text(createForm.locator('details.thread-panel__guide[open]'),/Track as work/,10000);await text(createForm.locator('details.thread-panel__guide[open]'),/Fill in this form/);
    await waitForVisibleCount(filterVisibleText(createForm.locator('li'),/Open a channel and choose/),0);
    await author.goto(base+'/work');const guide=author.locator('details.work-threads__guide');await text(guide.locator('summary'),'How to start a work thread');
    await text(guide.locator('li'),/Track as work/);await text(guide.locator('li'),/Open a channel and choose/);
  } else if(caseName.startsWith('keeps the new-thread guidance')) {
    await author.setViewportSize({width:390,height:844});await open();await click(panel.getByRole('button',{name:'New thread',exact:true,includeHidden:true}));
    await waitForVisibility(target('create'),{timeout:10000});await waitForVisibility(panel.locator('details.thread-panel__guide:not([open])'));
    const summary=await visibleMatch(panel.locator('details.thread-panel__guide summary'));await summary.evaluate(node=>node.focus());await actOnVisible(summary,'press',{},['Enter']);
    await text(panel.locator('details.thread-panel__guide[open]'),/Track as work/,10000);await overflow();
    const submit=await visibleMatch(target('createSubmit'));await submit.evaluate(node=>node.focus());await actOnVisible(submit,'press',{},['Tab']);
    await waitForCondition(()=>author.evaluate(()=>{const p=document.querySelector('#thread-panel');return p.contains(document.activeElement)&&document.activeElement!==p.querySelector('[data-thread-panel-target="createSubmit"]');}),{timeout:10000});await overflow();
  } else if(caseName.startsWith('shows work assignment')) {
    const name='Cross-feature work handoff',body='The assigned work message remains available.';
    await recipient.goto(base+'/activity');await text(recipient.locator('#activity-inbox-title'),'Activity inbox',10000);
    await create(name,body,false);await finishCreate(name);await work();await owner();await progress();
    await text(recipient.locator('.activity-item'),/Work assignment/,10000);await text(recipient.locator('.activity-item'),/Owner: unassigned.*Kevin/,10000);
    const status=filterVisibleText(recipient.locator('article.activity-item'),/Status: Planned → In progress/);await waitForVisibility(status,{timeout:10000});
    await click(status.getByRole('button',{name:'Open',exact:true,includeHidden:true}));await conversation(recipient,name);
    await text(recipient.locator('#thread-panel .thread-panel__thread-content .message__body'),body,10000);
    const url=new URL(recipient.url());assert.equal(url.pathname,'/rooms/654632876');assert.match(url.search,/^\?thread=\d+$/);
    const id=await recipient.locator('#thread-panel form[data-controller~="composer"]').getAttribute('data-composer-thread-id-value');assert.equal(url.search,`?thread=${id}`);
  } else if(caseName.startsWith('keeps the thread drawer')) {
    await create('Mobile thread','The mobile thread starter.',false);await finishCreate('Mobile thread');await close(author);
    await author.setViewportSize({width:390,height:844});await open();
    await waitForCondition(()=>author.locator('button[aria-label="Close threads"]').evaluate(node=>node===document.activeElement));assert.ok(await panel.evaluate(node=>node.contains(document.activeElement)));
    const item=filterVisibleText(target('browserList').locator('.thread-panel__thread-item'),'Mobile thread');await waitForVisibility(item,{timeout:10000});await click(item);
    await waitForVisibility(target('conversation'),{timeout:10000});await click(panel.getByRole('button',{name:'Back to thread list',exact:true,includeHidden:true}));await waitForVisibility(target('browser'));
    await click(panel.getByRole('button',{name:'New thread',exact:true,includeHidden:true}));await waitForVisibility(target('create'));
    // fill_in_thread_name: wait for beginCreate's animation-frame focus, then fill and check the name.
    await waitForCondition(()=>author.evaluate(()=>document.activeElement?.matches('[data-thread-panel-target="createMessage"]')),{timeout:10000});
    await actOnVisible(target('createName'),'fill',{},['Mobile second thread']);await waitForVisibleProperty(target('createName'),'value','Mobile second thread');
    await actOnVisible(target('createMessage'),'fill',{},['A second mobile thread.']);await finishCreate('Mobile second thread');
    await threadMenu('A second mobile thread.');assert.ok(await author.locator('#message-actions-menu:not([hidden])').evaluate(menu=>{const r=menu.getBoundingClientRect();return r.left>=0&&r.top>=0&&r.right<=innerWidth&&r.bottom<=innerHeight;}));
    await author.keyboard.press('Escape');await waitForVisibility(panel.locator(':scope[aria-hidden="false"]'));await close(author);
    await waitForCondition(()=>author.locator('#header-overflow-button').evaluate(node=>node===document.activeElement));await text(author.locator('.room-header__name'),'Designers');await overflow();
  } else if(caseName.startsWith('opens a shared')) {
    await author.goto(`${base}/rooms/654632876?thread=${fixture.thread_id}&message_id=${fixture.anchor_id}`);await conversation(author,'Shared anchor thread');await assertThreadMessage('Shared anchor post 5');
    const area=panel.locator(`.message-area[data-messages-anchor-message-id-value="${fixture.anchor_id}"]`);await waitForVisibility(area);await waitForVisibility(panel.locator('.message-area__return-to-latest'),{timeout:10000});
    assert.equal(await area.getAttribute('data-messages-anchor-message-id-value'),String(fixture.anchor_id));
  } else if(caseName.startsWith('marks a joined')) {
    const name='Unread thread coverage';await create(name,'The first unread check.',false);await finishCreate(name);
    // Match the original setup's explicit involvement/unread reset through its normal settings endpoint.
    await click(target('preferences').locator('summary'));await actOnVisible(target('involvement'),'selectOption',{},['everything']);
    await click(target('preferences').locator('summary'));await open(recipient);
    await click(filterVisibleText(target('browserList',recipient).locator('.thread-panel__thread-item'),name));await waitForVisibility(target('join',recipient),{timeout:10000});await click(target('join',recipient));await waitForVisibility(target('leave',recipient),{timeout:10000});
    async function incoming(body){await actOnVisible(recipient.getByRole('combobox',{name:'Write a thread reply',exact:true,includeHidden:true}),'fill',{},[body]);await click(recipient.getByRole('button',{name:'Send Reply',exact:true,includeHidden:true}));}
    await incoming('A visible second-user reply.');await assertThreadMessage('A visible second-user reply.');
    const id=await panel.locator('form[data-controller~="composer"]').getAttribute('data-composer-thread-id-value');
    const read=async unread=>waitForCondition(async()=>author.evaluate(async({base,id,unread})=>{const r=await fetch(`${base}/rooms/654632876/threads/${id}.json`);return (await r.json()).thread.unread===unread;},{base,id,unread}),{timeout:10000});
    await read(false);await close(author);await incoming('A hidden second-user reply.');await read(true);await open();await text(target('browserList').locator('.thread-panel__thread-item[data-unread="true"]'),name,10000);
  } else if(caseName.startsWith('keeps an anchored')) {
    const name='Anchored unread race',id=fixture.thread_id;await author.goto(`${base}/rooms/654632876?thread=${id}&message_id=${fixture.anchor_id}`);await conversation(author,name);
    await waitForVisibility(panel.locator('turbo-cable-stream-source[connected]'),{state:'attached',timeout:10000});const list=panel.locator('.thread-panel__thread-content .messages');
    await waitForVisibility(list.locator(':scope[data-messages-at-latest="false"]'),{timeout:10000});await list.evaluate(node=>{node.scrollTop=node.scrollHeight;node.dispatchEvent(new Event('scroll',{bubbles:true}));});
    await assertThreadMessage(`Anchored race post ${5+fixture.page_size*2}`);await waitForVisibility(list.locator(':scope[data-messages-at-latest="false"]'),{timeout:10000});
    await recipient.goto(`${base}/rooms/654632876?thread=${id}`);await conversation(recipient,name);if(await visibleCount(target('join',recipient)))await click(target('join',recipient));await waitForVisibility(target('leave',recipient),{timeout:10000});
    await actOnVisible(recipient.getByRole('combobox',{name:'Write a thread reply',exact:true,includeHidden:true}),'fill',{},['A reply during the anchored page.']);await click(recipient.getByRole('button',{name:'Send Reply',exact:true,includeHidden:true}));
    await waitForVisibleCount(filterVisibleText(panel.locator('.thread-panel__thread-content .message__body'),'A reply during the anchored page.'),0,{timeout:1000});await waitForVisibility(list.locator(':scope[data-messages-at-latest="false"]'),{timeout:10000});
    const read=unread=>waitForCondition(()=>author.evaluate(async({id,unread})=>{const response=await fetch(`/rooms/654632876/threads/${id}.json`);return(await response.json()).thread.unread===unread;},{id,unread}),{timeout:10000});
    await read(true);await click(panel.locator('.message-area__return-to-latest'));await waitForVisibility(list.locator(':scope[data-messages-at-latest="true"]'),{timeout:10000});await assertThreadMessage('A reply during the anchored page.');await read(false);
  } else throw new Error(`Unimplemented thread continuation: ${caseName}`);
}
