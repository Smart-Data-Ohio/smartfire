// Complete ordinary declarations from pinned test/system/voice_channels_test.rb.
// Room setup uses the public CRUD endpoint and actual signed David/Jason sessions.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, visitRoom, text } from './ws13-support.mjs';

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
  const f=await fixture(t,['david','jason']);
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
