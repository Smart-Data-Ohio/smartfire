// Complete declarations from frozen test/system/huddle_roster_test.rb.
// The room/analyser stub below is the original Rails SDK boundary verbatim.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture, text, absent } from './ws13-support.mjs';

async function setup(t) {
  const f=await fixture(t,['jason'],{kind:'designers'});const page=f.pages.jason;
  assert.equal(f.room,654632876);await page.setViewportSize({width:1400,height:1400});
  await page.evaluate(()=>{
    const controller = window.Stimulus
      .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
    window.__huddleController = controller;
    controller.liveKit = {
      Track: { Source: { Microphone: "microphone" } },
      createAudioAnalyser: () => ({
        calculateVolume: () => {
          window.__meterSamples.push(Date.now());
          return window.__meterVolume;
        },
        cleanup: () => { window.__meterCleanedUp = true; }
      })
    };
    controller.roomId = 1;
    controller.state = "connected";
    controller.canPublish = true;
    controller.noiseSuppressionAvailable = false;
    controller.noiseSuppressionEnabled = false;
    window.__micCalls = [];
    window.__cameraCalls = [];
    window.__meterSamples = [];
    window.__meterVolume = 0.4;
    window.__meterCleanedUp = false;
    let micEnabled = true;
    let cameraEnabled = false;
    const mediaStreamTrack = { readyState: "live" };
    window.__mediaStreamTrack = mediaStreamTrack;
    const audioTrack = { mediaStreamTrack };
    const participant = {
      identity: "local-1",
      name: "Jason",
      isSpeaking: false,
      get isMicrophoneEnabled() { return micEnabled; },
      setMicrophoneEnabled: (enabling) => {
        window.__micCalls.push(enabling);
        micEnabled = enabling;
        return Promise.resolve();
      },
      get isCameraEnabled() { return cameraEnabled; },
      setCameraEnabled: (enabling) => {
        window.__cameraCalls.push(enabling);
        cameraEnabled = enabling;
        return Promise.resolve();
      },
      trackPublications: new Map(),
      getTrackPublication: (source) => source === "microphone" ? { audioTrack } : null
    };
    const remoteMic = { isMuted: false };
    const remote = {
      identity: "remote-1",
      name: "David",
      isSpeaking: false,
      trackPublications: new Map(),
      getTrackPublication: (source) => source === "microphone" ? remoteMic : null
    };
    window.__remote = remote;
    window.__remoteMic = remoteMic;
    window.__remoteParticipants = new Map([[ "remote-1", remote ]]);
    controller.room = {
      options: {},
      localParticipant: participant,
      remoteParticipants: window.__remoteParticipants
    };
    controller.element.hidden = false;
    controller.activeControlsTarget.hidden = false;
    controller.peopleTarget.hidden = false;
    controller.muteTarget.disabled = false;
    controller.cameraTarget.disabled = false;
  });
  return page;
}
const row = identity => `li[data-participant-identity='${identity}']`;
async function toggle(page,count,target='mute') {
  await page.locator(`[data-huddle-target='${target}']`).click();
  await page.waitForFunction(({count,target})=>(target==='mute'?window.__micCalls:window.__cameraCalls).length>=count,{count,target},{timeout:10000});
}
async function mark(page,identity) {
  await page.locator(row(identity)).evaluate(row=>{row.dataset.probe='kept';row.title='hover card';});
}
async function kept(page,identity) {
  assert.deepEqual(await page.locator(row(identity)).evaluate(row=>[row.dataset.probe,row.title]),['kept','hover card']);
}
async function activity(page,identity) {
  return page.locator(`${row(identity)} .huddle__participant-activity`).textContent();
}
async function toggleState(page,target,label,pressed,tooltip) {
  await text(page,`[data-huddle-target='${target}'][aria-pressed='${pressed}'][title='${tooltip}']`,label);
}
async function meter(page,running) {
  await page.waitForFunction(running=>(window.__huddleController.microphoneMeter?.running===true)===running,running,{timeout:10000});
}
const samples = page => page.evaluate(()=>window.__meterSamples.length);

test('muting patches the local roster row instead of rebuilding it',async t=>{
  const p=await setup(t);await toggle(p,1);await text(p,row('local-1'),'Muted');
  await mark(p,'local-1');await toggle(p,2);await text(p,row('local-1'),'Listening');
  await kept(p,'local-1');assert.equal(await activity(p,'local-1'),'Listening');
  assert.ok((await p.locator(row('local-1')).getAttribute('aria-label')).endsWith(', Listening'));
  await text(p,`${row('local-1')} .huddle__participant-name`,'(you)');
});
test('speaking and mute changes patch the remote row in place',async t=>{
  const p=await setup(t);await toggle(p,1);await text(p,row('remote-1'),'Listening');await mark(p,'remote-1');
  await p.evaluate(()=>{window.__remote.isSpeaking=true;window.__remoteMic.isMuted=true;});
  await toggle(p,2);await text(p,row('remote-1'),'Speaking');await kept(p,'remote-1');
  assert.equal(await activity(p,'remote-1'),'Speaking');
  assert.ok((await p.locator(row('remote-1')).getAttribute('class')).includes('huddle__participant--speaking'));
  await p.evaluate(()=>window.__remote.isSpeaking=false);await toggle(p,3);await text(p,row('remote-1'),'Muted');
  await kept(p,'remote-1');assert.equal(await activity(p,'remote-1'),'Muted');
  assert.equal((await p.locator(row('remote-1')).getAttribute('class')).includes('huddle__participant--speaking'),false);
});
test('joining and leaving adds and removes roster rows only',async t=>{
  const p=await setup(t);await toggle(p,1);await text(p,row('remote-1'),'Listening');await mark(p,'local-1');
  await p.evaluate(()=>{
    const remoteMic={isMuted:false};window.__remoteParticipants.set('remote-2',{
      identity:'remote-2',name:'Alice',isSpeaking:false,trackPublications:new Map(),
      getTrackPublication:source=>source==='microphone'?remoteMic:null
    });
  });
  await toggle(p,2);await kept(p,'local-1');await text(p,row('remote-2'),'Alice');
  assert.equal(await p.locator('[data-huddle-target="participantCount"]').textContent(),'3 participants');
  await p.evaluate(()=>window.__remoteParticipants.delete('remote-1'));await toggle(p,3);
  await kept(p,'local-1');await absent(p,row('remote-1'));await text(p,row('remote-2'),'Alice');
});
test('the mute toggle keeps a stable label with pressed state and tooltip',async t=>{
  const p=await setup(t);await toggleState(p,'mute','Mute microphone',false,'Microphone live');
  await toggle(p,1);await toggleState(p,'mute','Mute microphone',true,'Microphone muted');
  await toggle(p,2);await toggleState(p,'mute','Mute microphone',false,'Microphone live');
});
test('the camera toggle keeps a stable label with pressed state and tooltip',async t=>{
  const p=await setup(t);await toggleState(p,'camera','Camera',false,'Camera off');
  await toggle(p,1,'camera');await toggleState(p,'camera','Camera',true,'Camera on');
  await toggle(p,2,'camera');await toggleState(p,'camera','Camera',false,'Camera off');
});
test('the meter stops once its track ends',async t=>{
  const p=await setup(t);await toggle(p,1);await toggle(p,2);await meter(p,true);assert.ok(await samples(p)>0);
  await p.evaluate(()=>window.__mediaStreamTrack.readyState='ended');await meter(p,false);
  assert.equal(await p.evaluate(()=>window.__meterCleanedUp),true);
  const stopped=await samples(p);await new Promise(resolve=>setTimeout(resolve,300));assert.equal(await samples(p),stopped);
});
test('the meter skips ticks while the tab is hidden',async t=>{
  const p=await setup(t);await toggle(p,1);await toggle(p,2);await meter(p,true);
  let hidden;
  try {
    await p.evaluate(()=>Object.defineProperty(document,'visibilityState',{configurable:true,get:()=> 'hidden'}));
    hidden=await samples(p);await new Promise(resolve=>setTimeout(resolve,350));assert.equal(await samples(p),hidden);
    assert.equal(await p.evaluate(()=>window.__huddleController.microphoneMeter?.running===true),true);
  } finally { await p.evaluate(()=>delete document.visibilityState); }
  await p.waitForFunction(hidden=>window.__meterSamples.length>hidden,hidden,{timeout:10000});
});
test('leaving reports after the disconnect completes',async t=>{
  const p=await setup(t);
  await p.evaluate(()=>{
    window.__leaveOrder=[];
    const csrfMeta=document.createElement('meta');csrfMeta.name='csrf-token';csrfMeta.content='test-csrf-token';
    document.head.appendChild(csrfMeta);
    const nativeLeaveFetch=window.fetch.bind(window);
    window.fetch=(...args)=>{
      const input=args[0];const url=typeof input==='string'?input:input.url;
      if(new URL(url,window.location.origin).pathname.endsWith('/huddle/leave')) window.__leaveOrder.push('reported');
      return nativeLeaveFetch(...args);
    };
    window.__huddleController.room.disconnect=()=>{window.__leaveOrder.push('disconnected');return Promise.resolve();};
  });
  await p.locator('[data-action="huddle#leave"]').click();
  await p.waitForFunction(()=>window.__leaveOrder.length>=2,null,{timeout:10000});
  assert.deepEqual(await p.evaluate(()=>window.__leaveOrder),['disconnected','reported']);
});
