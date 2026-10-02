// All sixteen original declarations from test/system/huddle_join_notices_test.rb.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, visitRoom, text, absent, waitController, markInCall } from './ws13-support.mjs';
const joinToast='.huddle-join-toast:not(.huddle-join-toast--leave)';
const leaveToast='.huddle-join-toast--leave';
const banner='#huddle-join-banner-slot .huddle-join-banner';
const sleep=(p,ms)=>p.waitForTimeout(ms);
async function setup(t,{group=false,batch=4000,delay=5000,inCall=true,sounds=true}={}) {
  const f=await fixture(t,['jason'],{kind:group?'group':'direct'});const p=f.pages.jason;
  await p.setViewportSize({width:1400,height:1400});
  await waitController(p,'huddle-join-notices','huddle-join-notice');
  await p.evaluate(({batch,delay,sounds})=>{
    const e=document.getElementById('huddle-join-notices');e.setAttribute('data-huddle-join-notice-toast-timeout-value','60000');e.setAttribute('data-huddle-join-notice-batch-window-value',String(batch));e.setAttribute('data-huddle-join-notice-leave-delay-value',String(delay));
    if(sounds){window.__playedSounds=[];window.Audio.prototype.play=function(){window.__playedSounds.push(this.src||this.currentSrc);return Promise.resolve();};}
  },{batch,delay,sounds});
  if(inCall){const viewer=await f.issue('jason');await f.mutate({op:'columns',grant:viewer.id,seen:0});await markInCall(p,f.room);}
  return {...f,page:p};
}
async function david(f) {return f.issue('david',f.room,f.people.david.session);}
const notify=(f,g)=>f.mutate({op:'seen_join',grant:g.id});
const seen=(f,g,value=0)=>f.mutate({op:'columns',grant:g.id,seen:value});
const out=async(f,g)=>assert.equal((await f.mutate({op:'out',grant:g.id})).changed,true);
const sounds=p=>p.evaluate(()=>window.__playedSounds);
async function sound(p,count=1){await p.waitForFunction(count=>window.__playedSounds.length===count,count,{timeout:10000,polling:50});}
async function start(f){const g=await david(f);await notify(f,g);await text(f.page,joinToast,'David joined',10000);return g;}

test('an in-call member sees a join toast and hears the join sound',async t=>{
  const f=await setup(t);await start(f);await sound(f.page);const played=await sounds(f.page);assert.equal(played.length,1);assert.ok(played[0].includes('incoming'));
});
test('rapid joins batch into one toast with one sound',async t=>{
  const f=await setup(t,{group:true,batch:30000});const d=await david(f);const k=await f.issue('kevin');
  await notify(f,d);await text(f.page,joinToast,'David joined',10000);await notify(f,k);
  await text(f.page,joinToast,'David and Kevin joined',10000);await sound(f.page);assert.equal((await sounds(f.page)).length,1);
});
test('a join toast stays silent with DND on',async t=>{
  const f=await setup(t);const p=f.page;
  await p.evaluate(()=>{const meta=document.createElement('meta');meta.name='notification-dnd';meta.content='muted';document.head.append(meta);});
  await start(f);await sleep(p,500);assert.deepEqual(await sounds(p),[]);
});
test('join and leave toasts announce through the container live region',async t=>{
  const f=await setup(t,{delay:200,sounds:false});const p=f.page;
  assert.equal(await p.locator('.huddle-join-toasts[aria-live="polite"]').count(),1);
  const g=await start(f);await out(f,g);await text(p,leaveToast,'David left',10000);
  await absent(p,'.huddle-join-toast[role]');await absent(p,'.huddle-join-toast[aria-live]');
});
test('a leave toasts quietly in the call without the join sound',async t=>{
  const f=await setup(t,{delay:200});const g=await david(f);await seen(f,g);await out(f,g);
  await text(f.page,leaveToast,'David left',10000);await sleep(f.page,500);assert.deepEqual(await sounds(f.page),[]);
});
test('a server mute cycle toasts neither left nor joined',async t=>{
  const f=await setup(t,{delay:1500});const g=await david(f);await seen(f,g);
  await f.mutate({op:'revoke',grant:g.id});const rejoined=await david(f);await notify(f,rejoined);
  await sleep(f.page,2000);await absent(f.page,'.huddle-join-toast');assert.deepEqual(await sounds(f.page),[]);
});
async function joinFirst(f){
  const g=await david(f);await seen(f,g);await f.mutate({op:'columns',grant:g.id,revoked:true});
  const rejoined=await david(f);await notify(f,rejoined);await sleep(f.page,1000);
  await seen(f,rejoined,null);await f.mutate({op:'notify_leave',grant:g.id});await seen(f,rejoined);
  await sleep(f.page,2000);await absent(f.page,'.huddle-join-toast');return rejoined;
}
test('a server mute cycle stays silent when the join arrives before the leave',async t=>{
  const f=await setup(t,{delay:1500});await joinFirst(f);assert.deepEqual(await sounds(f.page),[]);
});
test('a rejoin after the leave toasted toasts joined again',async t=>{
  const f=await setup(t,{delay:1500});const g=await david(f);await seen(f,g);await f.mutate({op:'revoke',grant:g.id});
  await text(f.page,leaveToast,'David left',10000);const rejoined=await david(f);await notify(f,rejoined);
  await text(f.page,joinToast,'David joined',10000);await text(f.page,leaveToast,'David left');await sound(f.page);assert.equal((await sounds(f.page)).length,1);
});
test('a genuine leave after a join-first mute cycle still toasts',async t=>{
  const f=await setup(t,{delay:1500});const rejoined=await joinFirst(f);await out(f,rejoined);
  await text(f.page,leaveToast,'David left',10000);assert.deepEqual(await sounds(f.page),[]);
});
test('a genuine rejoin after the leave delay toasts joined again',async t=>{
  const f=await setup(t,{batch:200,delay:200});const g=await start(f);await out(f,g);await text(f.page,leaveToast,'David left',10000);
  await notify(f,g);await sound(f.page,2);await text(f.page,joinToast,'David joined');await text(f.page,leaveToast,'David left');
});
test('a server mute cycle across a room switch stays silent',async t=>{
  const f=await setup(t,{group:true,delay:30000});const p=f.page;const g=await david(f);await seen(f,g);
  await f.mutate({op:'revoke',grant:g.id});await sleep(p,1000);
  await p.evaluate(()=>Turbo.visit('/rooms/486777696'));
  await p.waitForFunction(()=>window.location.pathname==='/rooms/486777696',null,{timeout:10000,polling:50});
  await p.waitForFunction(()=>[...document.querySelectorAll('turbo-cable-stream-source')].every(e=>e.hasAttribute('connected')),null,{timeout:15000,polling:50});
  await waitController(p,'huddle-join-notices','huddle-join-notice');
  await p.waitForFunction(()=>!!window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('huddle-join-notices'),'huddle-join-notice')?.subscription,null,{timeout:10000,polling:50});
  await sleep(p,1000);await markInCall(p,f.room);
  const rejoined=await david(f);await notify(f,rejoined);const kevin=await f.issue('kevin');await notify(f,kevin);
  await p.locator('.huddle-join-toast').filter({hasText:/^Kevin joined$/}).waitFor({timeout:10000});
  await absent(p,'.huddle-join-toast:has-text("David")');assert.equal((await sounds(p)).length,1);
});
async function outSetup(t,group=false){
  const f=await setup(t,{group,inCall:false,sounds:false});const d=await david(f);const k=group?await f.issue('kevin'):null;
  await f.mutate({op:'missed',user:f.people.jason.id});await visitRoom(f.page,f.room);
  await waitController(f.page,'huddle-join-notices','huddle-join-notice');
  await f.page.locator(`#list_rooms_direct_${f.room}`).waitFor({timeout:10000});return {...f,d,k};
}
async function joins(p){await p.evaluate(()=>{window.huddleJoinEvents=[];window.addEventListener('huddle:join',e=>window.huddleJoinEvents.push(e.detail));});}
async function checkJoined(f){
  await f.page.waitForFunction(()=>window.huddleJoinEvents.length>0,null,{timeout:10000,polling:50});
  assert.deepEqual(await f.page.evaluate(()=>window.huddleJoinEvents),[{roomId:f.room,roomName:'David'}]);
  await absent(f.page,banner,10000);await absent(f.page,'.huddle-join-pill',10000);
}
test('joining a sidebar pill from another room navigates then joins',async t=>{
  const f=await outSetup(t);const p=f.page;await visitRoom(p,486777696);await waitController(p,'huddle-join-notices','huddle-join-notice');await joins(p);
  await p.locator(`#list_rooms_direct_${f.room}`).waitFor({timeout:10000});await notify(f,f.d);
  const pill=`#list_rooms_direct_${f.room} .huddle-join-pill`;await text(p,pill,'David is in your huddle',10000);await p.locator(pill).getByRole('button',{name:'Join',exact:true}).click();
  await p.waitForFunction(room=>window.location.pathname===`/rooms/${room}`,f.room,{timeout:10000,polling:50});await checkJoined(f);
});
test('an out-of-call member sees the banner and sidebar pill and joins from the banner',async t=>{
  const f=await outSetup(t);const p=f.page;await joins(p);await notify(f,f.d);
  await text(p,banner,'David is in your huddle',10000);await text(p,`#list_rooms_direct_${f.room} .huddle-join-pill`,'David is in your huddle',10000);
  await p.locator('#huddle-join-banner-slot').getByRole('button',{name:'Join',exact:true}).click();await checkJoined(f);
});
async function twoJoins(f){await notify(f,f.d);await text(f.page,banner,'David is in your huddle',10000);await notify(f,f.k);await text(f.page,banner,'David and Kevin are in your huddle',10000);}
test('the join banner drops each leaver and hides when empty',async t=>{
  const f=await outSetup(t,true);await twoJoins(f);await out(f,f.k);
  await text(f.page,banner,'David is in your huddle',10000);await text(f.page,`#list_rooms_direct_${f.room} .huddle-join-pill`,'David is in your huddle',10000);
  await out(f,f.d);await absent(f.page,banner,10000);await absent(f.page,'.huddle-join-pill',10000);
});
test('the join banner reconciles its roster with presence refreshes',async t=>{
  const f=await outSetup(t,true);await twoJoins(f);const url=`/rooms/${f.room}/huddle/participants`;
  await f.page.evaluate(({url,id})=>window.dispatchEvent(new CustomEvent('huddle-participants:updated',{detail:{url,participants:[{id,name:'David'}]}})),{url,id:f.people.david.id});
  await text(f.page,banner,'David is in your huddle',10000);
  await f.page.evaluate(url=>window.dispatchEvent(new CustomEvent('huddle-participants:updated',{detail:{url,participants:[]}})),url);
  await absent(f.page,banner,10000);await absent(f.page,'.huddle-join-pill',10000);
});
test('the join banner clears when the huddle ends',async t=>{
  const f=await outSetup(t);await notify(f,f.d);await text(f.page,banner,'David is in your huddle',10000);await out(f,f.d);
  await absent(f.page,banner,10000);await absent(f.page,'.huddle-join-pill',10000);
});
