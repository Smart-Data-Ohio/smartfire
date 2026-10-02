// Original interactions/assertions from test/system/huddle_presence_test.rb.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, visitRoom, text, absent, observeStreams, headerRenders, waitIssuance } from './ws13-support.mjs';
const sidebar=(room,kind='closed')=>`#list_rooms_${kind}_${room}`;
const header='.room-header__actions';
async function setup(t,kind='designers') {
  const f=await fixture(t,['jason'],{kind});const page=f.pages.jason;
  await page.setViewportSize({width:1400,height:1400});
  await observeStreams(page);return {...f,page,kind:kind==='direct'?'direct':'closed'};
}
async function sight(f) {
  const before=await headerRenders(f.page,f.room,f.kind);
  const grant=await f.issue('david');await waitIssuance(f.page,f.room,f.kind,before);
  await f.mutate({op:'seen_presence',grant:grant.id});return grant;
}
for (const [kind,title] of [['designers','the channel sidebar row and header show participants and empty on revoke'],['direct','the DM sidebar row and header show the peer and empty on revoke']]) {
  test(title,async t=>{
    const f=await setup(t,kind);const p=f.page;const row=sidebar(f.room,f.kind);
    if(kind==='designers') {
      await text(p,'#shared_rooms .sidebar-item','Designers');
      assert.equal(await p.locator(`${row} .voice-stack:not(.voice-stack--live)`).count(),1);
    }
    const grant=await sight(f);
    for(const scope of [row,header]) {
      await p.locator(`${scope} .voice-stack--live`).waitFor({timeout:15000});
      await text(p,`${scope} .voice-stack__count`,'1',15000);
      if(kind==='designers'||scope===row) await p.locator(`${scope} img.voice-stack__avatar[data-user-id="${f.people.david.id}"]`).waitFor({timeout:15000});
    }
    if(kind==='designers') assert.equal(await p.locator(`${row} .voice-stack`).getAttribute('aria-label'),'1 in huddle: David');
    await f.mutate({op:'revoke',grant:grant.id});
    await absent(p,`${row} .voice-stack--live`,15000);await absent(p,`${row} img.voice-stack__avatar`,15000);
    await absent(p,`${header} .voice-stack--live`,15000);
  });
}
test('the sidebar aggregate poll clears quietly expired grants',async t=>{
  const f=await setup(t);const p=f.page;const row=sidebar(f.room);
  assert.equal(await p.locator(`${row} .voice-stack`).getAttribute('data-huddle-participants-interval-value'),'0');
  assert.equal(await p.locator(`${header} .voice-stack`).getAttribute('data-huddle-participants-interval-value'),'15000');
  const g=await sight(f);await text(p,`${row} .voice-stack__count`,'1',15000);
  await countPolls(p);await f.mutate({op:'columns',grant:g.id,seen:-60});
  assert.equal(await p.locator(`${row} .voice-stack__count`).innerText(),'1'); // Rails wait:0
  await absent(p,`${row} .voice-stack--live`,30000);
  assert.ok(await p.evaluate(()=>window.presenceFetches)>=1,'the sidebar never ran its aggregate presence poll');
});
test('the sidebar aggregate poll runs on connect and skips in-flight refreshes',async t=>{
  const f=await setup(t);const p=f.page;const row=sidebar(f.room);
  const g=await f.issue('david');await f.mutate({op:'seen',grant:g.id});await visitRoom(p,f.room);
  await text(p,`${row} .voice-stack--live .voice-stack__count`,'1');
  await f.mutate({op:'columns',grant:g.id,seen:-60});assert.equal(await p.locator(`${row} .voice-stack__count`).innerText(),'1');
  await countPolls(p);
  await p.evaluate(()=>{const e=document.querySelector('[data-controller~="huddle-presence"]');const c=window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-presence');c.disconnect();c.connect();});
  await absent(p,`${row} .voice-stack--live`,10000);
  assert.ok(await p.evaluate(()=>window.presenceFetches)>=1,'reconnecting the sidebar never ran its aggregate presence poll');
  await p.evaluate(()=>{window.presenceFetches=0;const e=document.querySelector('[data-controller~="huddle-presence"]');const c=window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-presence');c.refresh();c.refresh();});
  await p.waitForFunction(()=>window.presenceFetches>=1,null,{timeout:10000,polling:50});
  assert.equal(await p.evaluate(()=>window.presenceFetches),1,'back-to-back refreshes issued duplicate aggregate polls');
});
test('a removed sidebar stack clears once but keeps accepting updates while the header latches',async t=>{
  const f=await setup(t);const p=f.page;const row=sidebar(f.room);
  const g=await f.issue('david');await f.mutate({op:'seen',grant:g.id});await visitRoom(p,f.room);
  for(const scope of [row,header]) await text(p,`${scope} .voice-stack--live .voice-stack__count`,'1');
  const url=`/rooms/${f.room}/huddle/participants`;
  await p.evaluate(url=>window.dispatchEvent(new CustomEvent('huddle-participants:removed',{detail:{url}})),url);
  for(const scope of [row,header]) await absent(p,`${scope} .voice-stack--live`);
  // The original dispatch supplies the server's fresh avatar helper; recover it
  // from the real participants response rather than constructing signed bytes.
  const participants=await (await p.context().request.get(new URL(url,p.url()).href)).json();
  await p.evaluate(({url,participants})=>window.dispatchEvent(new CustomEvent('huddle-participants:updated',{detail:{url,participants}})),{url,participants});
  await text(p,`${row} .voice-stack--live .voice-stack__count`,'1');
  await p.locator(`${row} img.voice-stack__avatar[data-user-id="${f.people.david.id}"]`).waitFor();
  await absent(p,`${header} .voice-stack--live`);
});
test('the aggregate poller skips while hidden and fetches on becoming visible',async t=>{
  const {page:p}=await setup(t);
  const controller=()=>{const e=document.querySelector('[data-controller~="huddle-presence"]');return e&&window.Stimulus?.getControllerForElementAndIdentifier(e,'huddle-presence');};
  await p.waitForFunction(controller,null,{timeout:10000,polling:50});
  const idle=()=>p.waitForFunction(()=>{const e=document.querySelector('[data-controller~="huddle-presence"]');const c=e&&window.Stimulus?.getControllerForElementAndIdentifier(e,'huddle-presence');return !!c&&!c.inFlightRefresh;},null,{timeout:10000,polling:50});
  const refresh=()=>p.evaluate(()=>{const e=document.querySelector('[data-controller~="huddle-presence"]');window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-presence').refresh();});
  await idle();await countPolls(p);await refresh();
  await p.waitForFunction(()=>window.presenceFetches>=1,null,{timeout:10000,polling:50});await idle();
  try {
    await p.evaluate(()=>Object.defineProperty(document,'visibilityState',{configurable:true,get:()=> 'hidden'}));
    const hidden=await p.evaluate(()=>window.presenceFetches);await refresh();await p.waitForTimeout(300);
    assert.equal(await p.evaluate(()=>window.presenceFetches),hidden);
    await p.evaluate(()=>{Object.defineProperty(document,'visibilityState',{configurable:true,get:()=> 'visible'});document.dispatchEvent(new Event('visibilitychange'));});
    await p.waitForFunction(hidden=>window.presenceFetches>hidden,hidden,{timeout:10000,polling:50});
  } finally {await p.evaluate(()=>delete document.visibilityState);}
});
async function countPolls(p) {
  await p.evaluate(()=>{window.presenceFetches=0;const original=window.fetch.bind(window);window.fetch=(...args)=>{const input=args[0];const url=typeof input==='string'?input:input.url;if(new URL(url,window.location.origin).pathname==='/users/huddle_presence')window.presenceFetches++;return original(...args);};});
}
