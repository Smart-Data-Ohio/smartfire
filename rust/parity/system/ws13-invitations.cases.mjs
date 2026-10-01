// Original ten declarations use the merged WS13b APIs. The two inbox cases
// remain ready to enable once WS11-UI lands its public read/handled routes.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, visitRoom, text, waitController, markInCall } from './ws13-support.mjs';
const inbox={skip:process.env.WS13_ENABLE_INBOX_CASES==='1'?false:'Deferred to WS11-UI: activity unread_count/read/handled endpoints'};
const visible='#huddle-invitation:not([hidden])';
const hidden='#huddle-invitation[hidden]';
async function setup(t){
  const f=await fixture(t,['jason'],{kind:'designers'});const p=f.pages.jason;await p.setViewportSize({width:1400,height:1400});
  await waitController(p,'huddle-invitation','huddle-invitation');return {...f,page:p,dm:186869642};
}
const issue=f=>f.issue('david',f.dm,f.people.david.session);
async function item(f,g){const items=(await f.mutate({op:'inspect',grant:g.id})).activities;return items.find(i=>i.user===f.people.jason.id);}
async function shown(p,label='David started a huddle'){await text(p,visible,label,10000);}
async function dismissed(p){await p.locator(hidden).waitFor({state:'attached',timeout:10000});}
const ringing=(p,key)=>p.evaluate(key=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('huddle-invitation'),'huddle-invitation')[key]===true,key);
async function leaves(f,g){await f.mutate({op:'columns',grant:g.id,seen:0});await f.mutate({op:'revoke',grant:g.id});}
test('the recipient sees an incoming huddle banner and dismissing it marks the item read',inbox,async t=>{
  const f=await setup(t);const p=f.page;const g=await issue(f);assert.ok(await item(f,g));await shown(p);await text(p,visible,'Join the huddle in David');await text(p,'.workspace-activity-count','1',10000);
  await p.locator(visible).getByRole('button',{name:'Dismiss',exact:true}).click();await dismissed(p);await p.locator('.workspace-activity-count[hidden]').waitFor({state:'attached',timeout:10000});assert.equal((await item(f,g)).read,true);
});
test('joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel',inbox,async t=>{
  const f=await setup(t);const p=f.page;await p.evaluate(()=>{window.huddleJoinEvents=[];window.addEventListener('huddle:join',e=>window.huddleJoinEvents.push(e.detail));});const g=await issue(f);assert.ok(await item(f,g));await p.locator(visible).waitFor({timeout:10000});
  await p.locator(visible).getByRole('button',{name:'Join',exact:true}).click();await p.waitForURL(url=>url.pathname===`/rooms/${f.dm}`,{timeout:10000});await text(p,'.room--current','David');
  await p.waitForFunction(()=>window.huddleJoinEvents.length>0,null,{timeout:2000,polling:50});assert.deepEqual(await p.evaluate(()=>window.huddleJoinEvents),[{roomId:f.dm,roomName:'David'}]);assert.equal((await item(f,g)).handled,true);
});
test('the banner flips to caller-left when the starter hangs up, then dismisses',async t=>{
  const f=await setup(t);const p=f.page;const g=await issue(f);assert.ok(await item(f,g));await shown(p);await leaves(f,g);
  await shown(p,'David left the huddle');await text(p,visible,'Missed call in David');await dismissed(p);assert.equal((await item(f,g)).event,'huddle_started');
});
for(const [suppressed,title]of [[false,'a ring stops itself after the ring timeout'],[true,'a banner-only ring stops itself after the ring timeout']])test(title,async t=>{
  const f=await setup(t);const p=f.page;
  if(suppressed){await f.mutate({op:'preferences',user:f.people.jason.id,preferences:{huddle_invitations:false}});await visitRoom(p,f.room);await waitController(p,'huddle-invitation','huddle-invitation');}
  await p.evaluate(()=>document.getElementById('huddle-invitation').setAttribute('data-huddle-invitation-ring-timeout-value','300'));
  const g=await issue(f);await shown(p);await dismissed(p);if(suppressed)assert.equal(await item(f,g),undefined);
});
test("the banner stays hidden while already in the room's huddle",async t=>{
  const f=await setup(t);const p=f.page;await markInCall(p,f.dm);const g=await issue(f);assert.ok(await item(f,g));
  await p.locator(visible).waitFor({state:'hidden',timeout:5000});assert.equal(await p.locator(hidden).count(),1);const state=await item(f,g);assert.equal(state.read,false);assert.equal(state.handled,false);
});
test('an incoming call rings audibly until it is answered',async t=>{
  const f=await setup(t);const p=f.page;await issue(f);await shown(p);assert.equal(await ringing(p,'shouldRing'),true,'expected the banner to want its ringtone');
  await p.locator('#huddle-invitation [data-huddle-invitation-target="title"]').click();await p.waitForFunction(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('huddle-invitation'),'huddle-invitation').ringing===true,null,{timeout:10000,polling:50});
  await p.locator(visible).getByRole('button',{name:'Dismiss',exact:true}).click();await dismissed(p);assert.equal(await ringing(p,'shouldRing'),false);assert.equal(await ringing(p,'ringing'),false);
});
test('a silent invitation shows the banner without any sound',async t=>{
  const f=await setup(t);const p=f.page;await f.mutate({op:'preferences',user:f.people.jason.id,preferences:{},quiet:true});await issue(f);await shown(p);assert.equal(await ringing(p,'shouldRing'),false);assert.equal(await ringing(p,'ringing'),false);
});
test('a hidden tab raises a system notification for the call',async t=>{
  const f=await setup(t);const p=f.page;
  await p.evaluate(()=>{window.__notifications=[];window.__nativeNotification=window.Notification;window.__NotificationStub=class{static permission='granted';constructor(title,options){window.__notifications.push({title,options});this.closed=false;}close(){this.closed=true;}};window.Notification=window.__NotificationStub;Object.defineProperty(document,'visibilityState',{configurable:true,get:()=> 'hidden'});});
  try{await issue(f);await shown(p);await p.waitForFunction(()=>window.__notifications.length>=1,null,{timeout:10000,polling:50});const n=await p.evaluate(()=>window.__notifications[0]);assert.equal(n.title,'David started a huddle');assert.match(n.options.body,/Join the huddle in David/);}finally{await p.evaluate(()=>{window.Notification=window.__nativeNotification;delete document.visibilityState;});}
});
test('the ring stops when the caller leaves',async t=>{
  const f=await setup(t);const p=f.page;const g=await issue(f);await shown(p);assert.equal(await ringing(p,'shouldRing'),true);await leaves(f,g);await shown(p,'David left the huddle');assert.equal(await ringing(p,'shouldRing'),false);assert.equal(await ringing(p,'ringing'),false);
});
