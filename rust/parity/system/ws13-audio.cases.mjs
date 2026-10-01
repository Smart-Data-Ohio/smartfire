// Complete declarations from frozen test/system/huddle_audio_test.rb.
// Only the SDK/audio track is stubbed, exactly at the original Rails boundary.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fixture } from './ws13-support.mjs';

async function setup(t,noiseEnabled,processorName=null,selectedDevice=null) {
  const f=await fixture(t,['jason'],{kind:'designers'});const page=f.pages.jason;assert.equal(f.room,654632876);
  await page.setViewportSize({width:1400,height:1400});
  if (selectedDevice) await page.evaluate(device=>localStorage.setItem('campfire.huddle.devices',JSON.stringify({audioinput:device})),selectedDevice);
  await page.evaluate(({noiseEnabled,processorName})=>{
    const controller = window.Stimulus
      .getControllerForElementAndIdentifier(document.getElementById("channel-huddle"), "huddle");
    window.__huddleController = controller;
    controller.liveKit = { Track: { Source: { Microphone: "microphone" } } };
    controller.noiseSuppressionAvailable = true;
    controller.noiseSuppressionEnabled = noiseEnabled;
    controller.roomId = 1;
    controller.state = "connected";
    controller.canPublish = true;
    window.__micCalls = [];
    window.__processorCalls = [];
    window.__restartCalls = [];
    window.__constraintCalls = [];
    window.__audioCaptureDefaults = null;
    let micEnabled = true;
    let processor = processorName ? { name: processorName } : null;
    const audioTrack = {
      _constraints: {
        noiseSuppression: !noiseEnabled,
        echoCancellation: true,
        autoGainControl: true,
        voiceIsolation: true,
        deviceId: "stub-device"
      },
      get constraints() { return this._constraints; },
      get isMuted() { return !micEnabled; },
      getSourceTrackSettings: () => ({ deviceId: "live-device", channelCount: 1 }),
      setProcessor: (next) => {
        window.__processorCalls.push([ "set", next?.name || "unidentified" ]);
        processor = next;
        return Promise.resolve();
      },
      stopProcessor: () => {
        window.__processorCalls.push([ "stop" ]);
        processor = null;
        return Promise.resolve();
      },
      getProcessor: () => processor,
      restartTrack: (constraints) => {
        window.__restartCalls.push({ ...constraints });
        if (!micEnabled) return Promise.reject(new DOMException("Test stopped track", "InvalidStateError"));
        audioTrack._constraints = { ...constraints };
        return Promise.resolve();
      },
      applyConstraints: (constraints) => {
        if (!micEnabled) return Promise.reject(new DOMException("Test stopped track", "InvalidStateError"));
        window.__constraintCalls.push({ ...constraints });
        audioTrack._constraints = { ...audioTrack._constraints, ...constraints };
        return Promise.resolve();
      }
    };
    window.__audioTrack = audioTrack;
    const participant = {
      identity: "fake",
      name: "Jason",
      isSpeaking: false,
      get isMicrophoneEnabled() { return micEnabled; },
      setMicrophoneEnabled: (enabling, options) => {
        window.__micCalls.push({
          enabling: enabling,
          noiseSuppression: options?.noiseSuppression,
          deviceId: options?.deviceId
        });
        micEnabled = enabling;
        return Promise.resolve();
      },
      getTrackPublication: (source) => source === "microphone" ? { audioTrack } : null
    };
    controller.room = {
      options: {},
      localParticipant: participant,
      remoteParticipants: new Map()
    };
    const originalSetMicrophoneEnabled = participant.setMicrophoneEnabled;
    participant.setMicrophoneEnabled = (enabling, options) => {
      const result = originalSetMicrophoneEnabled(enabling, options);
      if (enabling) window.__audioCaptureDefaults = controller.room.options.audioCaptureDefaults || null;
      return result;
    };
    controller.element.hidden = false;
    controller.activeControlsTarget.hidden = false;
    controller.settingsTarget.hidden = false;
    controller.settingsRowTarget.hidden = false;
    controller.muteTarget.disabled = false;
    controller.noiseTarget.disabled = false;
  },{noiseEnabled,processorName});
  return page;
}
async function toggle(page,count) {
  await page.locator('[data-huddle-target="mute"]').click();
  await page.waitForFunction(count=>window.__micCalls.length>=count,count,{timeout:10000});
}
async function settle(page) {
  await page.evaluate(()=>(window.__huddleController.noiseOperation || Promise.resolve()).then(()=>true));
}
async function noise(page,processors,restarts) {
  await page.locator('[data-huddle-target="noise"]').click();
  if (processors) await page.waitForFunction(()=>window.__processorCalls.length>0,null,{timeout:10000});
  if (restarts) await page.waitForFunction(()=>window.__restartCalls.length>0,null,{timeout:10000});
  await settle(page);
}
const browserConstraints = deviceId=>({noiseSuppression:true,echoCancellation:true,autoGainControl:true,voiceIsolation:true,deviceId});
const rnnoiseConstraints = deviceId=>({...browserConstraints(deviceId),noiseSuppression:false});
for (const [enabled,title] of [[true,'the capture asks for no browser suppression while RNNoise is on'],[false,'the capture keeps browser suppression while RNNoise is off']]) {
  test(title,async t=>{
    const p=await setup(t,enabled);await toggle(p,1);await toggle(p,2);
    const calls=await p.evaluate(()=>window.__micCalls);
    assert.deepEqual(calls.map(c=>c.enabling),[false,true]);
    assert.deepEqual(calls.map(c=>c.noiseSuppression??true),[!enabled,!enabled]);
  });
}
test('unmuting requests the selected device without rewriting the room defaults',async t=>{
  const p=await setup(t,false,null,'mic-1');
  await toggle(p,1);await toggle(p,2);
  assert.deepEqual(await p.evaluate(()=>window.__micCalls.at(-1).deviceId),{ideal:'mic-1'});
  assert.equal(await p.evaluate(()=>window.__audioCaptureDefaults),null);
});
test('muting keeps the noise processor attached across mute and unmute',async t=>{
  const p=await setup(t,true,'campfire-rnnoise');await toggle(p,1);await toggle(p,2);await settle(p);
  assert.deepEqual(await p.evaluate(()=>window.__processorCalls),[]);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls),[]);
});
test('toggling noise suppression off re-acquires the microphone with browser suppression',async t=>{
  const p=await setup(t,true,'campfire-rnnoise');await noise(p,true,true);
  assert.deepEqual(await p.evaluate(()=>window.__processorCalls),[['stop']]);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls.at(-1)),browserConstraints('stub-device'));
});
test('toggling noise suppression on re-acquires the microphone without browser suppression',async t=>{
  const p=await setup(t,false);await noise(p,true,true);
  assert.deepEqual(await p.evaluate(()=>window.__processorCalls),[['set','campfire-rnnoise']]);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls.at(-1)),rnnoiseConstraints('stub-device'));
});
test('toggling noise suppression off without a stored device re-acquires on the live device',async t=>{
  const p=await setup(t,true,'campfire-rnnoise');
  await p.evaluate(()=>{window.__audioTrack._constraints={noiseSuppression:false,echoCancellation:true,autoGainControl:true};});
  await noise(p,true,true);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls.at(-1)),browserConstraints({ideal:'live-device'}));
});
test('toggling noise suppression off while muted refreshes the stored constraints',async t=>{
  const p=await setup(t,true,'campfire-rnnoise');await toggle(p,1);await noise(p,true,false);await settle(p);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls),[]);
  assert.deepEqual(await p.evaluate(()=>window.__audioTrack._constraints),browserConstraints('stub-device'));
  await toggle(p,2);await settle(p);
  assert.deepEqual(await p.evaluate(()=>window.__restartCalls),[]);
  assert.deepEqual(await p.evaluate(()=>window.__processorCalls),[['stop']]);
});
