// Complete ordinary Stage declarations from pinned test/system/stage_test.rb.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, text, absent, emptyStackShown, panel, row, controls } from './ws13-support.mjs';
import './ws13-audio.cases.mjs';
import './ws13-roster.cases.mjs';
import './ws13-voice.cases.mjs';

test('stage rooms list in their own section with distinct creation controls and a stage panel',async t => {
  const f=await fixture(t,['jason']); const p=f.pages.jason;
  await text(p,'.room-header__kind','Stage channel'); await text(p,'.huddle-launcher','Join stage');
  await text(p,'#stage_rooms .stage-room','Town Hall');
  await emptyStackShown(p,`#list_rooms_stage_${f.room} .voice-stack:not(.voice-stack--live)`);
  assert.equal(await p.locator('#voice_rooms .stage-room').count(),0);
  for (const [kind,label] of [['voice','Voice'],['stage','Stage']]) {
    const section=p.locator(`.sidebar-section--${kind}`);
    assert.equal((await section.locator('h2').textContent()).trim(),label);
    assert.equal(await section.locator('a.sidebar-section__add').count(),1);
    assert.equal(await section.getByRole('link',{name:`New ${kind} channel`,exact:true}).getAttribute('href'),`/rooms/${kind}s/new`);
  }
  assert.equal(await p.locator('#shared_rooms .sidebar-item').filter({hasText:'Town Hall'}).count(),0);
  await emptyStackShown(p,'.room-header__actions .voice-stack');
  await panel(p); await text(p,'.stage-panel__surface h2','Stage');
  await text(p,".stage-panel__roster [aria-label='Hosts']",'David');
  await text(p,".stage-panel__roster [aria-label='Listeners']",'Jason');
  await text(p,controls(f.room),'You are in the audience.');
  await p.getByRole('button',{name:'Close stage',exact:true}).click();
  await absent(p,'.stage-panel__surface');
});

test('a listener raises and lowers their hand without seeing host controls',async t => {
  const f=await fixture(t,['kevin']);const p=f.pages.kevin;await panel(p);
  for (const name of ['Invite to speak','Make host','Move to audience']) assert.equal(await p.getByRole('button',{name,exact:true}).count(),0);
  await p.getByRole('button',{name:'Raise hand',exact:true}).click();
  await text(p,controls(f.room),'Lower hand');
  await text(p,`${row(f.people.kevin.membership)} .stage-panel__hand-badge`,'Hand raised',15000);
  assert.equal((await f.state()).members.Kevin.hand,true);
  await p.getByRole('button',{name:'Lower hand',exact:true}).click();
  await text(p,controls(f.room),'Raise hand');
  await absent(p,`${row(f.people.kevin.membership)} .stage-panel__hand-badge`,15000);
  assert.equal((await f.state()).members.Kevin.hand,false);
});

test("raised hands appear in the host's panel live, in order",async t => {
  const f=await fixture(t,['david','jason','kevin']);
  for (const p of Object.values(f.pages)) await panel(p);
  await f.pages.jason.getByRole('button',{name:'Raise hand',exact:true}).click();
  await text(f.pages.jason,controls(f.room),'Lower hand');
  await f.pages.kevin.getByRole('button',{name:'Raise hand',exact:true}).click();
  const host=f.pages.david;
  await text(host,`${row(f.people.jason.membership)} .stage-panel__hand-badge`,'Hand raised',15000);
  for (const name of ['jason','kevin']) await host.locator(row(f.people[name].membership)).getByRole('button',{name:'Invite to speak',exact:true}).waitFor({timeout:15000});
  assert.deepEqual(await host.locator("[aria-label='Listeners'] .stage-panel__member").evaluateAll(rows=>rows.map(row=>row.id)),[row(f.people.jason.membership).slice(1),row(f.people.kevin.membership).slice(1)]);
});

test('a host invites a listener to speak and moves them back to the audience',async t => {
  const f=await fixture(t,['david','jason']);const host=f.pages.david,listener=f.pages.jason;
  await panel(host);await panel(listener);
  await listener.getByRole('button',{name:'Raise hand',exact:true}).click();await text(listener,controls(f.room),'Lower hand');
  const member=host.locator(row(f.people.jason.membership));
  await member.getByRole('button',{name:'Invite to speak',exact:true}).click({timeout:15000});
  await text(host,"[aria-label='Speakers'] .stage-panel__member",'Jason',10000);
  await text(listener,controls(f.room),'You are speaking',15000);
  assert.equal((await f.state()).members.Jason.role,'speaker');
  await member.getByRole('button',{name:'Move to audience',exact:true}).click();
  await text(host,"[aria-label='Listeners'] .stage-panel__member",'Jason',10000);
  await text(listener,controls(f.room),'You are in the audience',15000);
  assert.equal((await f.state()).members.Jason.role,'listener');
});

test('a host lowers a raised hand without promoting',async t => {
  const f=await fixture(t,['jason','david']);const host=f.pages.david,listener=f.pages.jason;
  await panel(listener);await listener.getByRole('button',{name:'Raise hand',exact:true}).click();await text(listener,controls(f.room),'Lower hand');
  await panel(host);const member=host.locator(row(f.people.jason.membership));
  await member.getByRole('button',{name:'Lower hand',exact:true}).click();
  await absent(host,`${row(f.people.jason.membership)} .stage-panel__hand-badge`,15000);
  await text(host,"[aria-label='Listeners'] .stage-panel__member",'Jason');
  assert.equal((await f.state()).members.Jason.role,'listener');
});

test('the last host cannot demote themselves from the panel',async t => {
  const f=await fixture(t,['david']);const host=f.pages.david;await panel(host);
  await text(host,`${row(f.people.david.membership)} button[disabled][title='The stage needs at least one host']`,'Move to audience');
  await host.locator(row(f.people.jason.membership)).getByRole('button',{name:'Make host',exact:true}).click();
  await text(host,`${row(f.people.david.membership)} button:not([disabled])`,'Move to audience',10000);
  assert.equal((await f.state()).members.Jason.role,'host');
});

test('join stage dispatches huddle:join and toggles while connected',async t => {
  const f=await fixture(t,['jason']);const p=f.pages.jason;
  // The original declaration supplies a synthetic connected event. Keep its
  // unavailable external signaling connection pending, rather than routing TLS
  // to the Rust HTTP server through the capture proxy and failing immediately.
  await p.routeWebSocket('wss://public.example.test/**',()=>{});
  await p.evaluate(()=>{window.stageJoinEvents=[];window.addEventListener('huddle:join',e=>window.stageJoinEvents.push(e.detail));});
  await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.waitForFunction(()=>window.stageJoinEvents.length>0,null,{timeout:2000});
  const event=[{roomId:f.room,roomName:'Town Hall',canPublishHint:false}];
  assert.deepEqual(await p.evaluate(()=>window.stageJoinEvents),event);
  await p.locator('#channel-huddle[data-state="connecting"]').waitFor();
  await p.evaluate(roomId=>window.dispatchEvent(new CustomEvent('huddle:changed',{detail:{roomId,state:'connected'}})),f.room);
  await text(p,'.huddle-launcher','Leave stage',10000);
  assert.equal(await p.locator('.huddle-launcher').getAttribute('aria-label'),'Leave stage');
  await p.getByRole('button',{name:'Leave stage',exact:true}).click();
  assert.deepEqual(await p.evaluate(()=>window.stageJoinEvents),event);
  await text(p,'.huddle-launcher','Join stage',10000);
  assert.equal(await p.locator('#channel-huddle').getAttribute('data-state'),'idle');
});

test('stage rooms carry ordinary text chat',async t => {
  const f=await fixture(t,['jason']);const p=f.pages.jason;
  const editor=p.locator('#message_markdown_source');await editor.click();
  await editor.evaluate(editor=>{editor.value='Hello from the stage';editor.dispatchEvent(new InputEvent('input',{bubbles:true,inputType:'insertFromPaste',data:editor.value}));});
  await p.getByRole('button',{name:'Send Message',exact:true}).click();
  await text(p,'.message[data-message-id] .message__body','Hello from the stage');
});

async function permissionFailure(page, permission, hanging=false) {
  await page.evaluate(({permission,hanging}) => {
    window.__stageGumCalls=0;window.__stageHuddleStates=[];
    new MutationObserver(()=>window.__stageHuddleStates.push(document.getElementById('channel-huddle').dataset.state))
      .observe(document.getElementById('channel-huddle'),{attributes:true,attributeFilter:['data-state']});
    const originalFetch=window.fetch;
    // Same credential failure/hang injected by the original Rails system declarations.
    window.fetch=(url,options)=>typeof url==='string' && url.endsWith('/huddle') && options?.method==='POST'
      ? hanging ? new Promise(()=>{}) : Promise.resolve(new Response('{}',{status:503,headers:{'Content-Type':'application/json'}}))
      : originalFetch(url,options);
    navigator.permissions.query=()=>Promise.resolve({state:permission});
    navigator.mediaDevices.getUserMedia=()=>{window.__stageGumCalls++;return Promise.reject(new DOMException('No microphone','NotFoundError'));};
  },{permission,hanging});
}
async function roleEvent(page,room,stageRole,serverMuted) {
  await page.evaluate(({room,stageRole,serverMuted})=>{
    const event=document.createElement('div');event.dataset.huddleRejoinRoomId=room;event.dataset.huddleRejoinStageRole=stageRole;
    if (serverMuted!==undefined) event.dataset.huddleRejoinServerMuted=String(serverMuted);
    document.getElementById('huddle_role_events').appendChild(event);
  },{room,stageRole,serverMuted});
}
async function noPrejoin(page) {
  assert.equal((await page.evaluate(()=>window.__stageHuddleStates)).includes('prejoin'),false);
  assert.equal(await page.evaluate(()=>window.__stageGumCalls),0);
}
test('a listener joins without a microphone or device check',async t=>{
  const f=await fixture(t,['kevin']);const p=f.pages.kevin;
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="false"]').waitFor();
  await permissionFailure(p,'prompt');
  await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:20000});
  await noPrejoin(p);
});
test('a demoted speaker retries as a listener without entering prejoin',async t=>{
  const f=await fixture(t,['kevin'],{speaker:true});const p=f.pages.kevin;
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="true"]').waitFor();
  await permissionFailure(p,'denied');await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:20000});
  await p.evaluate(()=>window.__stageHuddleStates=[]);await roleEvent(p,f.room,'listener');
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="false"]').waitFor({timeout:10000});
  await p.waitForFunction(()=>window.__stageHuddleStates.includes('connecting'),null,{timeout:10000});
  await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:20000});
  await p.evaluate(()=>{window.__stageGumCalls=0;window.__stageHuddleStates=[];navigator.permissions.query=()=>Promise.resolve({state:'prompt'});});
  await p.locator('[data-huddle-target="retry"]').click();
  await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:20000});await noPrejoin(p);
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="false"]').waitFor();
});
test('a muted speaker is told and rejoins without microphone prejoin',async t=>{
  const f=await fixture(t,['kevin'],{speaker:true});const p=f.pages.kevin;
  await permissionFailure(p,'denied',true);await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.locator('#channel-huddle[data-state="connecting"]').waitFor({timeout:20000});
  await roleEvent(p,f.room,'speaker',true);
  await text(p,'[data-huddle-target="notice"]:not([hidden])','A host muted you',10000);
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="false"]').waitFor({timeout:10000});
  assert.equal(await p.evaluate(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle').canPublishHint),false);
  await p.locator('[data-action="huddle#leave"]').click();
  await p.waitForFunction(()=>document.getElementById('channel-huddle').dataset.state==='idle',null,{timeout:10000});
  await p.evaluate(()=>{window.__stageGumCalls=0;window.__stageHuddleStates=[];navigator.permissions.query=()=>Promise.resolve({state:'prompt'});});
  await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.locator('#channel-huddle[data-state="connecting"]').waitFor({timeout:20000});await noPrejoin(p);
  await roleEvent(p,f.room,'speaker',true);await text(p,'[data-huddle-target="notice"]:not([hidden])','A host muted you',10000);
  await roleEvent(p,f.room,'speaker',false);
  await p.waitForFunction(()=>document.querySelector('[data-huddle-target="notice"]').hidden,null,{timeout:10000});
  await p.locator('.huddle-launcher[data-huddle-can-publish-param="true"]').waitFor({timeout:10000});
  assert.equal(await p.evaluate(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle').canPublishHint),true);
});
