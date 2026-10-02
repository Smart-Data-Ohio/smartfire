// Complete ordinary declarations from pinned test/system/voice_channels_test.rb.
// Room setup uses the public CRUD endpoint and actual signed David/Jason sessions.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, visitRoom, text, absent, emptyStackShown, observeStreams, headerRenders, waitIssuance } from './ws13-support.mjs';

async function createVoice(f,name,users) {
  const admin=f.pages.david;
  const params=new URLSearchParams({'room[name]':name});
  for(const user of users) params.append('user_ids[]',String(f.people[user].id));
  const response=await admin.context().request.post(new URL('/rooms/voices',admin.url()).href,{
    data:params.toString(),headers:{'Content-Type':'application/x-www-form-urlencoded','X-CSRF-Token':await admin.locator('meta[name="csrf-token"]').getAttribute('content')}
  });
  assert.equal(response.status(),200);
  const path=new URL(response.url()).pathname;assert.match(path,/^\/rooms\/\d+$/);
  return Number(path.split('/').at(-1));
}
async function setup(t) {
  const f=await fixture(t,['david','jason'],{kind:'designers'});
  await f.pages.jason.setViewportSize({width:1400,height:1400}); // Rails' shared system driver.
  const room=await createVoice(f,'Lounge',['david','jason']);
  await visitRoom(f.pages.jason,room);
  return {...f,room,page:f.pages.jason};
}
async function recordJoins(page) {
  await page.evaluate(()=>{window.huddleJoinEvents=[];window.addEventListener('huddle:join',event=>window.huddleJoinEvents.push(event.detail));});
}
test('join voice dispatches huddle:join',async t=>{
  const f=await setup(t);const p=f.page;await recordJoins(p);
  await p.getByRole('button',{name:'Join voice',exact:true}).click();
  await p.waitForFunction(()=>window.huddleJoinEvents.length>0,null,{timeout:2000});
  assert.deepEqual(await p.evaluate(()=>window.huddleJoinEvents),[{roomId:f.room,roomName:'Lounge'}]);
});
test('the button toggles to leave voice while connected and leaves through the panel',async t=>{
  const f=await setup(t);const p=f.page;await recordJoins(p);
  await p.evaluate(roomId=>window.dispatchEvent(new CustomEvent('huddle:changed',{detail:{roomId,state:'connected'}})),f.room);
  await text(p,'.huddle-launcher','Leave voice',10000);
  assert.equal(await p.locator('.huddle-launcher').getAttribute('aria-label'),'Leave voice');
  await p.getByRole('button',{name:'Leave voice',exact:true}).click();
  assert.deepEqual(await p.evaluate(()=>window.huddleJoinEvents),[]);
  await text(p,'.huddle-launcher','Join voice',10000);
  assert.equal(await p.locator('.huddle-launcher').getAttribute('aria-label'),'Join voice');
  assert.equal(await p.locator('#channel-huddle').getAttribute('data-state'),'idle');
});
test('voice rooms carry ordinary text chat',async t=>{
  const {page:p}=await setup(t);
  const editor=p.locator('#message_markdown_source');await editor.click();
  await editor.evaluate(editor=>{editor.value='Hello from the voice lounge';editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste',data:editor.value}));});
  await p.getByRole('button',{name:'Send Message',exact:true}).click();
  await text(p,'.message[data-message-id] .message__body','Hello from the voice lounge');
});
test('a participants 404 stops polling and clears the stack without retrying',async t=>{
  const f=await setup(t);const p=f.page;const strangers=await createVoice(f,'Founders',['david']);
  await p.evaluate(({strangers,david})=>{
    window.voiceRemovalErrors=[];
    window.addEventListener('error',event=>window.voiceRemovalErrors.push(event.message));
    window.addEventListener('unhandledrejection',event=>window.voiceRemovalErrors.push(String(event.reason)));
    window.probeFetches=0;
    window.fetch=((originalFetch)=>(...args)=>{
      const url=String(args[0]&&args[0].url||args[0]);
      if(url.includes('/huddle/participants')) window.probeFetches++;
      return originalFetch(...args);
    })(window.fetch.bind(window));
    document.body.insertAdjacentHTML('beforeend',`
      <span id="probe-voice-stack" class="voice-stack voice-stack--live" role="img" aria-label="1 in voice: David"
            data-controller="huddle-participants"
            data-huddle-participants-url-value="/rooms/${strangers}/huddle/participants"
            data-huddle-participants-max-value="3"
            data-huddle-participants-interval-value="15000">
        <span class="voice-stack__avatars" data-huddle-participants-target="avatars">
          <img width="20" height="20" class="voice-stack__avatar" data-user-id="${david}">
        </span>
        <span class="voice-stack__count" data-huddle-participants-target="count">1</span>
      </span>`);
  },{strangers,david:f.people.david.id});
  await p.waitForFunction(()=>!!window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('probe-voice-stack'),'huddle-participants'),null,{timeout:2000});
  const refresh=()=>p.evaluate(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('probe-voice-stack'),'huddle-participants').refresh());
  await refresh();
  assert.equal(await p.locator('#probe-voice-stack.voice-stack--live').count(),0);
  assert.equal(await p.locator('#probe-voice-stack img.voice-stack__avatar').count(),0);
  assert.equal(await p.locator('#probe-voice-stack [data-huddle-participants-target="count"][hidden]').count(),1);
  assert.equal(await p.evaluate(()=>window.probeFetches),1);
  await refresh();assert.equal(await p.evaluate(()=>window.probeFetches),1,'a removed member must not be polled again');
  assert.deepEqual(await p.evaluate(()=>window.voiceRemovalErrors),[]);
});

const header='.room-header__actions';
async function sight(f,user,session=null){
  const p=f.page;const before=await headerRenders(p,f.room,'voice');const g=await f.issue(user,f.room,session);
  await waitIssuance(p,f.room,'voice',before);await f.mutate({op:'seen_presence',grant:g.id});return g;
}
async function quiet(loads){
  const deadline=Date.now()+30000;
  while(Date.now()<deadline){const before=loads.length;await new Promise(resolve=>setTimeout(resolve,1000));if(loads.length===before)return;}
  assert.fail('sidebar loads never became quiet within the original 30 seconds');
}
function countLoads(p){const loads=[];p.on('response',r=>{if(new URL(r.url()).pathname==='/users/me/sidebar')loads.push(r.status());});return loads;}
test('the sidebar row and header show participants and update when a grant is revoked',async t=>{
  const f=await setup(t);const p=f.page;
  await text(p,'.room-header__kind',/voice channel/i);await text(p,'.huddle-launcher','Join voice');
  await text(p,'#voice_rooms .voice-room','Lounge');await emptyStackShown(p,'#voice_rooms .voice-room .voice-stack:not(.voice-stack--live)');
  await observeStreams(p);const d=await sight(f,'david',f.people.david.session);await text(p,'#voice_rooms .voice-room .voice-stack__count','1',15000);
  const j=await sight(f,'jason');
  for(const scope of ['#voice_rooms .voice-room',header]){
    await p.locator(`${scope} .voice-stack--live`).waitFor({timeout:15000});await text(p,`${scope} .voice-stack__count`,'2',15000);
    for(const user of ['david','jason'])await p.locator(`${scope} img.voice-stack__avatar[data-user-id="${f.people[user].id}"]`).waitFor({timeout:15000});
  }
  const [label,trailing]=await p.evaluate(()=>{const r=document.querySelector('#voice_rooms .voice-room');return [r.querySelector('.sidebar-item__label').getBoundingClientRect().left,r.querySelector('.voice-room__trailing').getBoundingClientRect().left];});
  assert.ok(trailing>label);
  for(const g of [d,j])assert.deepEqual((await f.mutate({op:'inspect',grant:g.id})).activities,[]);
  await f.mutate({op:'revoke',grant:d.id});
  for(const scope of ['#voice_rooms .voice-room',header]){
    await text(p,`${scope} .voice-stack__count`,'1',15000);await absent(p,`${scope} img.voice-stack__avatar[data-user-id="${f.people.david.id}"]`,15000);
  }
  await p.locator(`#voice_rooms .voice-room img.voice-stack__avatar[data-user-id="${f.people.jason.id}"]`).waitFor({timeout:15000});
});
test('the sidebar loads once when the cable connects and reloads on reconnect',async t=>{
  const f=await setup(t);const p=f.page;const loads=countLoads(p);await quiet(loads);const before=loads.length;
  await visitRoom(p,f.room);await quiet(loads);assert.equal(loads.length-before,1,'the sidebar reloaded when the cable connected');
  await f.mutate({op:'reconnect',user:f.people.jason.id});
  const deadline=Date.now()+30000;while(loads.length-before<=1&&Date.now()<deadline)await p.waitForTimeout(200);
  assert.equal(loads.length-before,2);
});
test('rooms stream broadcasts survive the reconnect sidebar reload',async t=>{
  const f=await setup(t);const p=f.page;await observeStreams(p);const beforeRenders=await headerRenders(p,f.room,'voice');const g=await f.issue('jason',f.room);
  await waitIssuance(p,f.room,'voice',beforeRenders);await absent(p,'#voice_rooms .voice-room .voice-stack--live');
  const loads=countLoads(p);await quiet(loads);const before=loads.length;
  await p.evaluate(()=>{window.sidebarReloads=0;document.getElementById('user_sidebar').addEventListener('turbo:frame-load',()=>window.sidebarReloads++);window.preReloadSources=[...document.querySelectorAll('turbo-cable-stream-source')];});
  await f.mutate({op:'reconnect',user:f.people.jason.id});
  await p.waitForFunction(()=>window.sidebarReloads>0,null,{timeout:30000,polling:100});assert.ok(loads.length-before>=1);
  await f.mutate({op:'seen_presence',grant:g.id});
  assert.equal(await p.evaluate(()=>window.preReloadSources.length),3);assert.equal(await p.evaluate(()=>window.preReloadSources.every(e=>e.isConnected)),true,'the sidebar reload replaced the rooms stream sources');
  await text(p,'#voice_rooms .voice-room .voice-stack__count','1',15000);
});
test('presence refreshes once quiet grants expire',async t=>{
  const f=await setup(t);const p=f.page;assert.equal(await p.locator(`${header} .voice-stack`).getAttribute('data-huddle-participants-interval-value'),'15000');
  await observeStreams(p);const g=await sight(f,'david',f.people.david.session);await text(p,`${header} .voice-stack__count`,'1',15000);
  await f.mutate({op:'columns',grant:g.id,seen:-60});assert.equal(await p.locator(`${header} .voice-stack__count`).innerText(),'1');
  await absent(p,`${header} .voice-stack--live`,25000);await p.locator(`${header} .voice-stack__count[hidden]`).waitFor({state:'attached',timeout:10000});
});
test('leaving through the panel clears presence immediately',async t=>{
  const f=await setup(t);const p=f.page;const g=await f.issue('jason',f.room,f.people.jason.session);await f.mutate({op:'columns',grant:g.id,seen:0});
  await visitRoom(p,f.room);await text(p,'#voice_rooms .voice-room .voice-stack__count','1');
  await p.evaluate(room=>{const c=window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle');c.roomId=room;c.leave();},f.room);
  await absent(p,'#voice_rooms .voice-room .voice-stack--live',15000);
  const state=await f.mutate({op:'inspect',grant:g.id});assert.equal(state.last_seen,null);assert.equal(state.revoked,false);
});
test('removing a member drops their sidebar row and header stack without errors',async t=>{
  const f=await setup(t);const p=f.page;await observeStreams(p);
  await p.evaluate(()=>{const e=document.querySelector('[data-controller~="refresh-room"]');window.Stimulus.getControllerForElementAndIdentifier(e,'refresh-room').disconnect();});
  await sight(f,'david',f.people.david.session);await p.locator(`${header} .voice-stack--live`).waitFor({timeout:15000});
  await p.evaluate(()=>{window.voiceRemovalErrors=[];window.addEventListener('error',e=>window.voiceRemovalErrors.push(e.message));window.addEventListener('unhandledrejection',e=>window.voiceRemovalErrors.push(String(e.reason)));window.voiceRemovalInflight=0;const native=window.fetch.bind(window);window.fetch=(...args)=>{window.voiceRemovalInflight++;return native(...args).finally(()=>window.voiceRemovalInflight--);};});
  const admin=f.pages.david;await admin.goto(new URL(`/rooms/voices/${f.room}/edit`,admin.url()).href,{waitUntil:'domcontentloaded'});
  await admin.locator('li[data-value="jason"] label.switch').click({timeout:10000});await admin.locator('button.btn--reversed:visible').click({timeout:10000});await text(admin,'.room-header__name','Lounge',10000);
  await absent(p,'#voice_rooms .voice-room:has-text("Lounge")',30000);
  const element=p.locator(`${header} .voice-stack`);
  try{await element.waitFor({state:'hidden',timeout:5000});}catch{
    await p.evaluate(()=>{const e=document.querySelector('.room-header__actions .voice-stack');const c=e&&window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-participants');return c?.refresh();});
  }
  await p.waitForFunction(()=>{const e=document.querySelector('.room-header__actions .voice-stack');if(!e)return true;const c=window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-participants');return !!c&&!!c.revoked;},null,{timeout:40000,polling:100});
  await absent(p,`${header} .voice-stack--live`,15000);
  await p.waitForFunction(()=>document.querySelectorAll('turbo-cable-stream-source[connected]').length===2,null,{timeout:25000,polling:200});
  await p.waitForFunction(()=>window.voiceRemovalInflight===0,null,{timeout:25000,polling:50});await p.waitForTimeout(500);
  await absent(p,'#voice_rooms .voice-room:has-text("Lounge")');await absent(p,`${header} .voice-stack--live`);assert.deepEqual(await p.evaluate(()=>window.voiceRemovalErrors),[]);
});
async function fits(p){
  assert.equal(await p.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth),true,'the workspace overflows the viewport horizontally');
  const [box,width]=await p.evaluate(()=>[document.querySelector('#nav').getBoundingClientRect().toJSON(),window.innerWidth]);assert.ok(box.left>=0,'the room header starts outside the viewport');assert.ok(box.right<=width,'the room header ends outside the viewport');
}
test('the voice header fits narrow phones and caps the stack',async t=>{
  const f=await setup(t);const p=f.page;await observeStreams(p);
  await sight(f,'david',f.people.david.session);await text(p,`${header} .voice-stack--live .voice-stack__count`,'1',15000);
  await sight(f,'jason');await text(p,`${header} .voice-stack--live .voice-stack__count`,'2',10000);
  try {
    for(const [width,height]of [[320,740],[390,844]]){await p.setViewportSize({width,height});await fits(p);}
    for(const user of ['kevin','jz'])await f.mutate({op:'grant_member',room:f.room,user:f.people[user].id});
    await sight(f,'kevin');await p.locator(`${header} .voice-stack__count`).filter({hasText:'3'}).waitFor({state:'attached',timeout:15000});await sight(f,'jz');
    await p.setViewportSize({width:1400,height:1400});await text(p,`${header} .voice-stack__count`,'4',15000);
    await p.waitForFunction(()=>[...document.querySelectorAll('.room-header__actions img.voice-stack__avatar')].filter(e=>e.checkVisibility()).length===4,null,{timeout:15000});
    for(const [width,height]of [[320,740],[500,800]]){await p.setViewportSize({width,height});assert.equal(await p.locator(`${header} img.voice-stack__avatar:visible`).count(),3);await text(p,`${header} .voice-stack__count`,'4');await fits(p);}
  }finally{await p.setViewportSize({width:1400,height:1400});}
});
test('the room page shares one participants request across its stacks',async t=>{
  const f=await setup(t);const p=f.page;await observeStreams(p);await sight(f,'david',f.people.david.session);
  await p.locator(`${header} img.voice-stack__avatar[data-user-id="${f.people.david.id}"]`).waitFor({timeout:15000});assert.equal(await p.locator('[data-controller="huddle-participants"]:visible').count(),2);
  await p.evaluate(()=>{window.probeFetches=0;const native=window.fetch.bind(window);window.fetch=(...args)=>{if(String(args[0]&&args[0].url||args[0]).includes('/huddle/participants'))window.probeFetches++;return native(...args);};});
  const count=await p.evaluate(async url=>{const controllers=[...document.querySelectorAll(`[data-huddle-participants-url-value="${url}"]`)].map(e=>window.Stimulus.getControllerForElementAndIdentifier(e,'huddle-participants'));await Promise.all(controllers.map(c=>c.refresh()));return window.probeFetches;},`/rooms/${f.room}/huddle/participants`);
  assert.equal(count,1,'the sidebar and header stacks must share one request');
  for(const scope of ['#voice_rooms',header])await p.locator(`${scope} img.voice-stack__avatar[data-user-id="${f.people.david.id}"]`).waitFor();
});
