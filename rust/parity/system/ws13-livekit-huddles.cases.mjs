// Original Rails huddles declarations. Real local audio/video and native SDK only.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mediaFixture,join,leave,click,target,controller,microphone,evaluate,wait,processor,trackId,trackState,storedNoise,constraints,setting,meterLevel,meterRunning,stat,sampling,breakNoise,noise,media,reconnect,permission,restorePermission,participantCount,text,absent} from './ws13-livekit-support.mjs';

test('noise suppression runs on the microphone, can be switched off, and is remembered',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);await noise(p,true);
  await click(p,'Noise suppression on');await noise(p,false);assert.equal(await storedNoise(p),'off');
  await leave(p);await join(p);await noise(p,false);assert.equal(await processor(p),null);
});
test('noise suppression can be switched back on without leaving the huddle',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);await noise(p,true);
  await click(p,'Noise suppression on');await noise(p,false);await click(p,'Noise suppression off');await noise(p,true);assert.equal(await storedNoise(p),'on');
});
test("the capture's browser suppression follows the RNNoise processor",async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);await noise(p,true);
  assert.equal(await setting(p,'noiseSuppression'),false);assert.equal((await constraints(p)).noiseSuppression,false);
  const device=await setting(p,'deviceId');const first=await trackId(p);
  await click(p,'Noise suppression on');await noise(p,false);
  await wait(p,microphone+"?._mediaStreamTrack?.id && "+microphone+"._mediaStreamTrack.id !== "+JSON.stringify(first),'the microphone was not re-acquired after switching suppression off');
  assert.equal(await setting(p,'noiseSuppression'),true);assert.equal((await constraints(p)).noiseSuppression,true);assert.equal((await constraints(p)).deviceId,device);
  const second=await trackId(p);await click(p,'Noise suppression off');await noise(p,true);
  await wait(p,microphone+"?._mediaStreamTrack?.id && "+microphone+"._mediaStreamTrack.id !== "+JSON.stringify(second),'the microphone was not re-acquired after switching suppression on');
  assert.equal(await setting(p,'noiseSuppression'),false);assert.equal((await constraints(p)).noiseSuppression,false);
  await leave(p);await breakNoise(p);await join(p);
  await text(p,target('status'),'Noise suppression couldn’t start');assert.equal(await processor(p),null);
  assert.equal(await setting(p,'noiseSuppression'),true);assert.equal((await constraints(p)).noiseSuppression,true);
});
test('muting and unmuting keeps the noise suppressor on the microphone',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);
  assert.equal(await evaluate(p,controller+"?.room?.options?.publishDefaults?.stopMicTrackOnMute===true"),true,'expected the room to stop the mic track on mute');
  await noise(p,true);await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="true"]','Mute microphone');await p.locator('#channel-huddle.huddle--muted').waitFor();
  await wait(p,microphone+"?._mediaStreamTrack?.readyState==='ended'",'the mic track was not stopped on mute');assert.equal(await processor(p),'campfire-rnnoise');
  await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="false"]','Mute microphone');
  await wait(p,microphone+"?._mediaStreamTrack?.readyState==='live'",'the microphone was not re-acquired on unmute');await noise(p,true);
});
test('a noise suppressor that fails to load still connects the huddle and stays retryable',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(k);await breakNoise(p);await join(p);
  await participantCount(p,2);await media(p,'audio');await text(p,target('status'),'Noise suppression couldn’t start');assert.equal(await processor(p),null);await media(k,'audio');
  await text(p,target('noise')+':not([disabled])','Noise suppression off');assert.equal(await storedNoise(p),null);
});
test('a browser that cannot run the noise suppressor turns the control off for good',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;await join(f.pages.kevin);await breakNoise(p,'NotSupportedError');await join(p);
  await media(p,'audio');await text(p,target('noise')+'[disabled]','Noise suppression unavailable');assert.equal(await processor(p),null);
});
test('the microphone meter follows the fake microphone and rests while muted',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);await p.locator(target('meter')+':not([hidden])').waitFor();
  const level="Number(document.querySelector("+JSON.stringify(target('meter'))+").getAttribute('aria-valuenow'))";
  await wait(p,level+'>0','the microphone meter never rose');await click(p,'Mute microphone');await absent(p,target('meter')+':not([hidden])');
  assert.equal(await meterLevel(p),0);assert.equal(await meterRunning(p),false,'the meter kept polling while muted');
  await click(p,'Mute microphone');await p.locator(target('meter')+':not([hidden])').waitFor();await wait(p,level+'>0','the microphone meter never came back after unmuting');
  await evaluate(p,"window.huddleTestMeterContext="+controller+".microphoneMeter.analyser.analyser.context");assert.equal(await evaluate(p,'window.huddleTestMeterContext.state'),'running');
  await leave(p);assert.equal(await meterRunning(p),false,'the meter kept polling after leaving');assert.equal(await evaluate(p,'window.huddleTestMeterContext.state'),'closed','leaving the huddle left the meter AudioContext alive');
});
test('the microphone meter restarts after a full reconnect',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);
  const level="Number(document.querySelector("+JSON.stringify(target('meter'))+").getAttribute('aria-valuenow'))";
  await wait(p,level+'>0','the microphone meter never rose');const track=await trackId(p);await evaluate(p,"window.huddleTestMeterAnalyser="+controller+".microphoneMeter.analyser");
  const after=await evaluate(p,'window.huddleTestPeerConnections.length');await reconnect(p);await media(p,'audio',after,true);
  assert.notEqual(await trackId(p),track,'the full reconnect did not restart the microphone track');
  await wait(p,controller+".microphoneMeter.analyser && "+controller+".microphoneMeter.analyser!==window.huddleTestMeterAnalyser",'the microphone meter was not restarted after the reconnect');
  await wait(p,level+'>0','the microphone meter never came back after the reconnect');await media(k,'audio');
});
test('the device check appears for a first join and is skipped once permissions were granted',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await permission(p,'prompt');const before=await evaluate(p,'window.huddleTestCredentials.length');
  await click(p,'Join huddle');await p.locator('#channel-huddle[data-state="prejoin"]').waitFor();await text(p,target('devicesBlock'),'Check your devices');
  for(const name of ['microphoneSelect','cameraSelect'])await p.locator(target(name)+' option').first().waitFor({state:'attached'});
  assert.equal(await evaluate(p,'window.huddleTestCredentials.length'),before);
  await p.locator(target('checkJoin')+':not([disabled])').waitFor({timeout:20000});
  await wait(p,"Number(document.querySelector("+JSON.stringify(target('prejoinMeter'))+").getAttribute('aria-valuenow'))>0",'the pre-join meter never rose');
  await wait(p,"(()=>{const v=document.querySelector("+JSON.stringify(target('preview'))+");return !!v&&!v.closest("+JSON.stringify(target('previewWrap'))+").hidden&&v.readyState>=2&&v.videoWidth>0;})()", 'the camera preview did not show video');
  const tracks=await evaluate(p,'window.huddleTestLocalTracks.length');await p.locator(target('checkJoin')).click();await p.locator('#channel-huddle[data-state="connected"]').waitFor({timeout:20000});await participantCount(p,1);
  await wait(p,'window.huddleTestLocalTracks.slice(0,'+tracks+").every(track=>track.readyState==='ended')",'the preview tracks were not stopped on join');
  await leave(p);await restorePermission(p);await click(p,'Join huddle');await p.locator('#channel-huddle[data-state="connected"]').waitFor({timeout:20000});
  await click(p,'Check devices');await p.locator(target('devicesBlock')+':not([hidden])').waitFor();assert.ok(await p.locator(target('microphoneSelect')+' option').count()>=1);
  await click(p,'Done');await absent(p,target('devicesBlock')+':not([hidden])');
});
test('the connection indicator renders and the details panel shows sampled statistics',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;await join(p);await join(f.pages.kevin);
  await p.locator(target('connection')+':not([hidden])').waitFor();assert.ok(['good','fair','poor'].includes(await p.locator(target('connection')).getAttribute('data-quality')));
  assert.equal(await sampling(p),false,'connection statistics were sampled before the panel opened');await p.locator(target('connection')).click();await p.locator(target('connectionDetails')+':not([hidden])').waitFor();
  assert.equal(await sampling(p),true,'connection statistics were not sampled while the panel was open');
  await wait(p,"document.querySelector("+JSON.stringify(target('statRtt'))+").textContent!=='–'",'round-trip time was never sampled');
  assert.match(await stat(p,'statRtt'),/^[\d.]+ ms$/);assert.match(await stat(p,'statLoss'),/^[\d.]+%$/);assert.match(await stat(p,'statJitter'),/^[\d.]+ ms$/);
  await wait(p,"document.querySelector("+JSON.stringify(target('statRx'))+").textContent!=='–'&&document.querySelector("+JSON.stringify(target('statSent'))+").textContent!=='–'",'bitrates were never sampled');
  for(const name of ['statRx','statSent'])assert.match(await stat(p,name),/^[\d.]+ (kbps|Mbps)$/);assert.equal(await stat(p,'statTransport'),'Direct');
  await p.locator(target('connection')).click();await absent(p,target('connectionDetails')+':not([hidden])');assert.equal(await sampling(p),false,'connection statistics kept sampling after the panel closed');
});
test('reopening the connection panel samples bitrates from scratch',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;await join(p);await join(f.pages.kevin);await p.locator(target('connection')).click();await p.locator(target('connectionDetails')+':not([hidden])').waitFor();
  await wait(p,"document.querySelector("+JSON.stringify(target('statRx'))+").textContent!=='–'&&document.querySelector("+JSON.stringify(target('statSent'))+").textContent!=='–'",'bitrates were never sampled');
  await p.locator(target('connection')).click();await absent(p,target('connectionDetails')+':not([hidden])');await new Promise(resolve=>setTimeout(resolve,2500));
  const reopened=Date.now();await p.locator(target('connection')).click();await p.locator(target('connectionDetails')+':not([hidden])').waitFor();
  await wait(p,"("+controller+"?.connectionStatsSummary?.previous?.at??0)>="+reopened,'no fresh sample landed after reopening');
  assert.equal(await stat(p,'statSent'),'–');assert.equal(await stat(p,'statRx'),'–');
});
test('the connection panel shows no received bitrate while alone in the call',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);await p.locator(target('connection')).click();await p.locator(target('connectionDetails')+':not([hidden])').waitFor();
  await wait(p,"document.querySelector("+JSON.stringify(target('statSent'))+").textContent!=='–'",'the sent bitrate was never sampled');assert.equal(await stat(p,'statRx'),'–');
});
test('denied microphone leaves no ghost participant and can be retried',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(k);
  await p.evaluate(()=>{window.huddleTestGetUserMedia=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);navigator.mediaDevices.getUserMedia=()=>Promise.reject(new DOMException('Test permission denial','NotAllowedError'));});
  const before=await evaluate(p,'window.huddleTestCredentials.length');await click(p,'Join huddle');
  await p.locator('#channel-huddle[data-state="failed"]').waitFor();await text(p,target('notice'),'Microphone access was denied');assert.ok(await evaluate(p,'window.huddleTestCredentials.length')>before);
  await participantCount(k,1,5000);await p.evaluate(()=>{navigator.mediaDevices.getUserMedia=window.huddleTestGetUserMedia;});await click(p,'Try again');
  await p.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(p,2);await media(p,'audio');
});
test('denied microphone stops the device check before anything is published and can be retried',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await permission(p,'prompt');
  await p.evaluate(()=>{window.huddleTestGetUserMedia=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);navigator.mediaDevices.getUserMedia=()=>Promise.reject(new DOMException('Test permission denial','NotAllowedError'));});
  const before=await evaluate(p,'window.huddleTestCredentials.length');await click(p,'Join huddle');await p.locator('#channel-huddle[data-state="prejoin"]').waitFor();
  await text(p,target('checkError'),'Microphone access was denied');assert.equal(await evaluate(p,'window.huddleTestCredentials.length'),before);
  await p.evaluate(()=>{navigator.mediaDevices.getUserMedia=window.huddleTestGetUserMedia;});await restorePermission(p);await click(p,'Try again');
  await p.locator(target('checkJoin')+':not([disabled])').waitFor({timeout:20000});await p.locator(target('checkJoin')).click();await p.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(p,1);
  await join(k);await participantCount(k,2);await media(k,'audio');await participantCount(p,2);await media(p,'audio');
});
const storedDevices=p=>evaluate(p,"JSON.parse(localStorage.getItem('campfire.huddle.devices')||'{}')");
const options=(p,name)=>p.locator(target(name)).evaluate(select=>[...select.options].map(o=>({value:o.value,label:o.text})));
const selected=(p,name)=>p.locator(target(name)).inputValue();
async function switchedPreference(p,kind,value,message){await wait(p,"JSON.parse(localStorage.getItem('campfire.huddle.devices')||'{}')["+JSON.stringify(kind)+"]==="+JSON.stringify(value),message);}
const remoteVideo="Array.from(document.querySelectorAll('.huddle__camera:not(.huddle__camera--local) video')).some(v=>v.videoWidth>0&&v.readyState>=2)";
test('device pickers list the fake devices and switching keeps media flowing',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await click(p,'Check devices');await p.locator(target('devicesBlock')+':not([hidden])').waitFor();
  for(const [name,prefix] of [['microphoneSelect','Microphone '],['cameraSelect','Camera '],['speakerSelect','Speaker ']]) {
    const devices=await options(p,name);assert.ok(devices.length>0);assert.ok(devices.every(d=>!d.label.startsWith(prefix)));
  }
  await p.locator(target('speakerRow')+':not([hidden])').waitFor();
  const micCurrent=await selected(p,'microphoneSelect');const mic=(await options(p,'microphoneSelect')).find(d=>d.value!==micCurrent);assert.ok(mic,'expected the fake backend to offer at least two microphones');
  await p.locator(target('microphoneSelect')).selectOption({label:mic.label});await switchedPreference(p,'audioinput',mic.value,'the microphone switch was not remembered');await media(k,'audio');
  await wait(p,"Number(document.querySelector("+JSON.stringify(target('meter'))+").getAttribute('aria-valuenow'))>0",'the meter never came back after the microphone switch');
  const speakerCurrent=await selected(p,'speakerSelect');const speaker=(await options(p,'speakerSelect')).find(d=>d.value!==speakerCurrent);assert.ok(speaker,'expected the fake backend to offer at least two speakers');
  await p.locator(target('speakerSelect')).selectOption({label:speaker.label});await switchedPreference(p,'audiooutput',speaker.value,'the speaker switch was not remembered');await media(p,'audio');
  await click(p,'Camera');await k.locator('.huddle__camera video').waitFor();await wait(k,remoteVideo,'the camera did not decode video');
  await p.locator(target('cameraSelect')).dispatchEvent('change');const camera=await selected(p,'cameraSelect');await switchedPreference(p,'videoinput',camera,'the camera switch was not remembered');await wait(k,remoteVideo,'the camera switch stopped the video');
  await p.locator(target('speakerSelect')).evaluate(select=>{const o=document.createElement('option');o.value='missing-device';o.text='Missing device';select.appendChild(o);select.value='missing-device';select.dispatchEvent(new Event('change',{bubbles:true}));});
  await text(p,target('status'),'speaker could not be switched');await media(p,'audio');await click(p,'Done');await absent(p,target('devicesBlock')+':not([hidden])');
});
test('an SDK device retarget keeps the stored preference and updates the picker',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await click(p,'Check devices');await p.locator(target('devicesBlock')+':not([hidden])').waitFor();
  const speakers=(await options(p,'speakerSelect')).filter(d=>d.value!=='default');assert.ok(speakers.length>=2,'expected the fake backend to offer at least two speakers');const [preferred,fallback]=speakers;
  await p.locator(target('speakerSelect')).selectOption({label:preferred.label});await switchedPreference(p,'audiooutput',preferred.value,'the speaker switch was not remembered');
  const retarget=await p.evaluate(async id=>{try{await window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle').room.switchActiveDevice('audiooutput',id);return true;}catch(e){return 'switch failed: '+e.message;}},fallback.value);assert.equal(retarget,true);
  await wait(p,controller+".room.getActiveDevice('audiooutput')==="+JSON.stringify(fallback.value),'the SDK retarget did not take effect');assert.equal((await storedDevices(p)).audiooutput,preferred.value,'the SDK fallback overwrote the stored speaker preference');
  await wait(p,"document.querySelector("+JSON.stringify(target('speakerSelect'))+").value==="+JSON.stringify(fallback.value),'the picker did not follow the SDK retarget');await media(k,'audio');
});
test('the mute and camera toggles keep stable labels with pressed state and tooltips',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);
  const toggle=async(name,label,pressed,tooltip)=>text(p,target(name)+'[aria-pressed="'+pressed+'"][title="'+tooltip+'"]',label);
  await toggle('mute','Mute microphone',false,'Microphone live');await toggle('camera','Camera',false,'Camera off');
  await click(p,'Mute microphone');await toggle('mute','Mute microphone',true,'Microphone muted');await click(p,'Mute microphone');await toggle('mute','Mute microphone',false,'Microphone live');
  await click(p,'Camera');await toggle('camera','Camera',true,'Camera on');await click(p,'Camera');await toggle('camera','Camera',false,'Camera off');
});
test('a camera that fails to start keeps the huddle connected and stays retryable',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(k);await join(p);await participantCount(p,2);await media(p,'audio');
  await p.evaluate(()=>{window.huddleTestGetUserMedia=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);navigator.mediaDevices.getUserMedia=c=>c?.video?Promise.reject(new DOMException('Test permission denial','NotAllowedError')):window.huddleTestGetUserMedia(c);});
  await click(p,'Camera');await p.locator('#channel-huddle[data-state="connected"]').waitFor();await text(p,target('notice'),'Camera wasn’t started');await text(p,target('camera')+':not([disabled])','Camera');await participantCount(p,2);await media(p,'audio');
  await participantCount(k,2);await media(k,'audio');await absent(k,'.huddle__camera video');
  await p.evaluate(()=>{navigator.mediaDevices.getUserMedia=window.huddleTestGetUserMedia;});await click(p,'Camera');await text(p,target('camera')+'[aria-pressed="true"]','Camera');await absent(p,target('notice')+':not([hidden])');await p.locator('.huddle__camera--local video').waitFor();
  await k.locator('.huddle__camera video').waitFor();await wait(k,remoteVideo,'the retried camera did not decode video');await media(k,'video');
});
test('a camera switched in-call falls back silently when it is unplugged',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await click(p,'Check devices');await p.locator(target('cameraSelect')).dispatchEvent('change');
  const camera=await selected(p,'cameraSelect');await wait(p,controller+".room.getActiveDevice('videoinput')==="+JSON.stringify(camera),'the camera switch never completed');
  await p.evaluate(missingId=>{window.huddleTestGetUserMedia=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);navigator.mediaDevices.getUserMedia=c=>{const wanted=c?.video?.deviceId;const exact=wanted?.exact??(typeof wanted==='string'?wanted:undefined);return exact===missingId?Promise.reject(new DOMException('Test unplugged camera','OverconstrainedError')):window.huddleTestGetUserMedia(c);};},camera);
  await click(p,'Camera');await text(p,target('camera')+'[aria-pressed="true"]','Camera');await p.evaluate(()=>{navigator.mediaDevices.getUserMedia=window.huddleTestGetUserMedia;});await k.locator('.huddle__camera video').waitFor();await wait(k,remoteVideo,'the fallback camera did not decode video');
  assert.deepEqual(await evaluate(p,controller+"?.room?.options?.videoCaptureDefaults?.deviceId??null"),{ideal:camera});
});
const count=(p,selector,total)=>p.waitForFunction(({selector,total})=>document.querySelectorAll(selector).length===total,{selector,total},{timeout:2000,polling:50});
const localVideo="Array.from(document.querySelectorAll('.huddle__camera--local video')).some(v=>v.videoWidth>0&&v.readyState>=2)";
const screenVideo="Array.from(document.querySelectorAll('.huddle__screen video')).some(v=>v.videoWidth>0&&v.readyState>=2)";
const screenWidth=p=>evaluate(p,"document.querySelector('.huddle__screen video')?.clientWidth||0");
const signalURLs=p=>p.evaluate(()=>window.huddleTestWebSocketUrls.filter(url=>/^\/rtc(?:\/v1)?$/.test(new URL(url).pathname)));
const noAudio=p=>count(p,'#channel-huddle audio',0);
test('two users exchange audio and a screen while navigating and muting',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await participantCount(p,2);await media(p,'audio');
  const pcs=await evaluate(p,'window.huddleTestPeerConnections.length');await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();
  await participantCount(k,2);await media(k,'audio');await k.locator('.huddle__screen video').waitFor();await wait(k,screenVideo,'the remote screen did not decode video');await media(k,'video');
  await p.locator('#sidebar').getByRole('link',{name:'HQ',exact:true}).click();await text(p,'.room--current','HQ');await p.locator('#channel-huddle[data-state="connected"]').waitFor();await text(p,'#huddle-room-name','Designers');
  assert.equal(await evaluate(p,'window.huddleTestPeerConnections.length'),pcs);await media(p,'audio');
  await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="true"]','Mute microphone');await k.locator('.huddle__participant').filter({hasText:/JZ.*Muted/s}).waitFor();
  await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="false"]','Mute microphone');await click(p,'Stop sharing');await p.getByRole('button',{name:'Share screen',exact:true}).waitFor();await absent(k,'.huddle__screen video');
  await leave(p);await noAudio(p);await wait(p,"window.huddleTestLocalTracks.every(track=>track.readyState==='ended')",'local media was not stopped on leave');await participantCount(k,1);
});
test('two users exchange camera video while navigating, muting, toggling, and leaving',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await participantCount(p,2);await media(p,'audio');
  await text(p,target('camera')+'[aria-pressed="false"]','Camera');await absent(p,'.huddle__camera video');await click(p,'Camera');await text(p,target('camera')+'[aria-pressed="true"]','Camera');
  await p.locator('.huddle__camera--local video').waitFor();await text(p,'.huddle__camera figcaption','JZ (you)');await wait(p,localVideo,'the local camera preview did not decode video');
  await k.locator('.huddle__camera:not(.huddle__camera--local) video').waitFor();await text(k,'.huddle__camera figcaption','JZ');await wait(k,remoteVideo,'the remote camera did not decode video');await media(k,'video');
  await click(k,'Camera');await text(k,target('camera')+'[aria-pressed="true"]','Camera');await count(p,'.huddle__camera',2);await wait(p,remoteVideo,'the remote camera did not decode video');await media(p,'video');
  await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="true"]','Mute microphone');await count(p,'.huddle__camera',2);await wait(p,remoteVideo,'muting stopped the remote camera');
  await click(p,'Mute microphone');await text(p,target('mute')+'[aria-pressed="false"]','Mute microphone');await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();await p.locator('.huddle__screen video').waitFor();await count(p,'.huddle__camera',2);
  await k.locator('.huddle__screen video').waitFor();await k.locator('[data-huddle-screen-expand]').click();await k.locator('#channel-huddle.huddle--theater').waitFor();await count(k,'.huddle__camera',2);
  const overlap=await k.evaluate(()=>{const screen=document.querySelector('.huddle__screen--expanded video');if(!screen)return 'missing expanded screen';const box=screen.getBoundingClientRect();const tiles=[...document.querySelectorAll('.huddle__camera')];if(!tiles.length)return 'missing camera tiles';return tiles.some(tile=>{const r=tile.getBoundingClientRect();return r.left<box.right&&r.right>box.left&&r.top<box.bottom&&r.bottom>box.top;})?'overlap':'none';});assert.equal(overlap,'none');
  await k.locator('body').press('Escape');await absent(k,'#channel-huddle.huddle--theater');await click(p,'Stop sharing');await p.getByRole('button',{name:'Share screen',exact:true}).waitFor();
  const pcs=await evaluate(p,'window.huddleTestPeerConnections.length');await p.locator('#sidebar').getByRole('link',{name:'HQ',exact:true}).click();await text(p,'.room--current','HQ');await p.locator('#channel-huddle[data-state="connected"]').waitFor();await count(p,'.huddle__camera',2);assert.equal(await evaluate(p,'window.huddleTestPeerConnections.length'),pcs);await wait(p,remoteVideo,'the remote camera did not survive navigation');
  await click(p,'Camera');await text(p,target('camera')+'[aria-pressed="false"]','Camera');await count(p,'.huddle__camera',1);await count(k,'.huddle__camera',1);
  await leave(k);await wait(k,"window.huddleTestLocalTracks.every(track=>track.readyState==='ended')",'local camera was not stopped on leave');await participantCount(p,1);await absent(p,'.huddle__camera video');
  await p.locator('#sidebar').getByRole('link',{name:'Designers',exact:false}).click();await leave(p);await join(p);await text(p,target('camera')+'[aria-pressed="false"]','Camera');await absent(p,'.huddle__camera video');
});
test('two direct message participants exchange audio and screen while navigating and reconnecting',async t=>{
  const f=await mediaFixture(t,['david','jason'],{kind:'direct'});const p=f.pages.david;const j=f.pages.jason;await join(p);await join(j);
  await text(p,'.room-header__kind','Direct message');await participantCount(p,2);await media(p,'audio');const pcs=await evaluate(p,'window.huddleTestPeerConnections.length');
  await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();await participantCount(j,2);await media(j,'audio');await j.locator('.huddle__screen video').waitFor();await wait(j,screenVideo,'the direct-message screen did not decode video');await media(j,'video');
  await p.locator('#sidebar').getByRole('link',{name:'HQ',exact:true}).click();await text(p,'.room--current','HQ');await p.locator('#channel-huddle[data-state="connected"]').waitFor();assert.equal(await evaluate(p,'window.huddleTestPeerConnections.length'),pcs);
  await p.locator('#list_rooms_direct_'+f.room).click();await text(p,'.room-header__kind','Direct message');await text(p,'.room-header__name','Jason');await p.locator('#channel-huddle[data-state="connected"]').waitFor();await media(p,'audio');
  const sockets=(await signalURLs(p)).length;const after=await evaluate(p,'window.huddleTestPeerConnections.length');await reconnect(p);
  await wait(p,"window.huddleTestWebSocketUrls.filter(url=>/^\\/rtc(?:\\/v1)?$/.test(new URL(url).pathname)).length>"+sockets,'the direct-message huddle did not reconnect');await media(p,'audio',after,true);
  await j.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(j,2);await media(j,'audio');await leave(p);await noAudio(p);await participantCount(j,1);
});
test('a viewer enlarges a shared screen into theater mode and leaves it with escape',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();
  await k.locator('.huddle__screen video').waitFor();await text(k,'.huddle-share-indicator','is sharing');await text(k,target('sharing'),'is sharing a screen');const width=await screenWidth(k);assert.ok(width>0);
  await k.locator('[data-huddle-screen-expand]').click();await k.locator('#channel-huddle.huddle--theater').waitFor();await k.locator('.huddle__screen--expanded video').waitFor();await text(k,'[data-huddle-screen-expand][aria-expanded="true"]','Collapse');
  await wait(k,"(document.querySelector('.huddle__screen video')?.clientWidth||0)>"+(width*1.5),'the expanded screen did not grow');
  await wait(k,"(()=>{const v=document.querySelector('.huddle__screen--expanded video');return !!v&&v.videoWidth>0&&v.readyState>=2;})()", 'the expanded screen stopped decoding video');await media(k,'video');
  await k.locator('body').press('Escape');await absent(k,'#channel-huddle.huddle--theater');await text(k,'[data-huddle-screen-expand][aria-expanded="false"]','Expand');assert.equal(await evaluate(k,'document.activeElement.textContent'),'Expand');
});
test('the room header shares-screen button opens the shared screen',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await absent(k,'.huddle-share-indicator');
  await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();await text(k,'.huddle-share-indicator','JZ is sharing');await k.locator('.huddle-share-indicator').click();
  await k.locator('#channel-huddle.huddle--theater').waitFor();await k.locator('.huddle__screen--expanded video').waitFor();await click(p,'Stop sharing');await absent(k,'.huddle-share-indicator');await absent(k,'#channel-huddle.huddle--theater');
});
test('the full screen control asks for the figure and falls back to the video element',async t=>{
  const f=await mediaFixture(t);const p=f.pages.jz;await join(p);
  await p.evaluate(()=>{window.huddleTestFullscreenRequests=[];Element.prototype.requestFullscreen=function(){window.huddleTestFullscreenRequests.push(this.tagName);return Promise.reject(new DOMException('Test full screen refusal','NotAllowedError'));};});
  await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();await p.locator('.huddle__screen video').waitFor();await p.locator('[data-huddle-screen-fullscreen]').click();
  await wait(p,'window.huddleTestFullscreenRequests.length>0','no element was asked for full screen');assert.deepEqual(await evaluate(p,'window.huddleTestFullscreenRequests'),['FIGURE','VIDEO'],'the figure keeps the caption, so it is tried before the bare video element');
  await p.locator('#channel-huddle.huddle--theater').waitFor();await text(p,target('status'),'Full screen isn’t available');
});
test('a second shared screen stays reachable while the first one is expanded',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);await join(k);await click(p,'Share screen');await p.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();
  await k.locator('.huddle__screen video').waitFor();await click(k,'Share screen');await k.getByRole('button',{name:'Stop sharing',exact:true}).waitFor();await count(k,'.huddle__screen',2);await text(k,target('sharing'),'2 people are sharing a screen');
  await click(k,'View');await k.locator('#channel-huddle.huddle--theater').waitFor();const first=await k.locator('[data-huddle-screen-expand][aria-expanded="true"]').getAttribute('aria-label');assert.ok(first);await count(k,'[data-huddle-screen-expand][aria-expanded="false"]',1);
  await click(k,'Next screen');await wait(k,"(()=>{const l=document.querySelector('[data-huddle-screen-expand][aria-expanded=true]')?.getAttribute('aria-label');return !!l&&l!=="+JSON.stringify(first)+";})()", 'the banner did not move to the other shared screen');await k.locator('#channel-huddle.huddle--theater').waitFor();await count(k,'[data-huddle-screen-expand][aria-expanded="true"]',1);
});
async function credentials(p,index=0) {
  await wait(p,'window.huddleTestCredentials.length>'+index,'the browser did not capture huddle credentials');
  return evaluate(p,'window.huddleTestCredentials['+index+']');
}
async function refreshed(p,original) {
  await wait(p,controller+"?.room?.engine?.token && "+controller+".room.engine.token!=="+JSON.stringify(original),'LiveKit did not give the SDK a refreshed token');
  return evaluate(p,controller+'.room.engine.token');
}
const claims=token=>JSON.parse(Buffer.from(token.split('.')[1],'base64url').toString());
function unexpired(...tokens) {for(const token of tokens) assert.ok(claims(token).exp>Date.now()/1000,'gateway must reject an unexpired token because its authorization was revoked');}
async function rejected(p,template,token) {
  const result=await p.evaluate(({template,token})=>new Promise(done=>{
    const url=new URL(template);url.searchParams.set('access_token',token);
    let finished=false;let opened=false;
    const finish=details=>{if(finished)return;finished=true;clearTimeout(timeout);done({opened,...details});};
    const socket=new WebSocket(url);socket.addEventListener('open',()=>{opened=true;socket.close();});socket.addEventListener('error',()=>{});
    socket.addEventListener('close',event=>finish({code:event.code,timedOut:false}));
    const timeout=setTimeout(()=>{socket.close();finish({code:null,timedOut:true});},5000);
  }),{template,token});
  assert.equal(result.timedOut,false,'gateway left an unauthorized signal attempt pending');assert.equal(result.opened,false,'gateway accepted a signal socket with a revoked token');
}
async function accessEnded(p) {
  await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:20000});await text(p,target('status'),'Huddle ended');await noAudio(p);
  await wait(p,"window.huddleTestLocalTracks.every(track=>track.readyState==='ended')","revoked participant's local media was not stopped");
}
async function remaining(p) {
  await p.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(p,1);
  await wait(p,"window.huddleTestPeerConnections.some(pc=>pc.connectionState==='connected')",'the remaining participant lost their peer connection');
}
test('server removal disconnects only the targeted participant and stops their media',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);const identity=(await credentials(p)).identity;await join(k);await participantCount(p,2);await media(p,'audio');
  await f.mutate({op:'server_remove',room:f.room,identity});await p.locator('#channel-huddle[data-state="failed"]').waitFor({timeout:10000});await noAudio(p);
  await wait(p,"window.huddleTestLocalTracks.every(track=>track.readyState==='ended')","revoked participant's local media was not stopped");await k.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(k,1);
});
test("the SDK reconnects through the gateway with LiveKit's refreshed token",async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);const issued=await credentials(p);await join(k);await participantCount(p,2);await media(p,'audio');
  const original=issued.token;const token=await refreshed(p,original);const before=claims(original);const after=claims(token);assert.notEqual(original,token,'LiveKit did not replace the original token');
  assert.equal(issued.identity,before.sub);assert.equal(before.sub,after.sub);assert.equal(before.video.room,after.video.room);
  const sockets=(await signalURLs(p)).length;const pcs=await evaluate(p,'window.huddleTestPeerConnections.length');await reconnect(p);
  await wait(p,"window.huddleTestWebSocketUrls.filter(url=>/^\\/rtc(?:\\/v1)?$/.test(new URL(url).pathname)).slice("+sockets+").some(url=>new URL(url).searchParams.get('access_token')==="+JSON.stringify(token)+")",'the SDK did not reconnect with LiveKit’s refreshed token');
  await media(p,'audio',pcs,true);await k.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(k,2);
});
test('membership revocation rejects original and refreshed tokens even after membership is restored',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);const issued=await credentials(p);const signal=(await signalURLs(p))[0];const original=issued.token;const token=await refreshed(p,original);
  await join(k);await participantCount(p,2);await media(p,'audio');await f.mutate({op:'revoke_member',room:f.room,user:f.people.jz.id});
  await accessEnded(p);await remaining(k);unexpired(original,token);await rejected(p,signal,original);await rejected(p,signal,token);
  await f.mutate({op:'grant_member',room:f.room,user:f.people.jz.id});await join(p,'Try again');const replacement=await credentials(p,1);
  assert.notEqual(issued.grant_id,replacement.grant_id);assert.notEqual(issued.identity,replacement.identity);unexpired(original,token);await rejected(p,signal,original);await rejected(p,signal,token);
  await participantCount(k,2);await media(k,'audio');
});
test('session revocation rejects both tokens and leaves the other participant connected',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);const issued=await credentials(p);const signal=(await signalURLs(p))[0];const original=issued.token;const token=await refreshed(p,original);
  await join(k);await participantCount(p,2);await media(p,'audio');await f.mutate({op:'destroy_session',grant:issued.grant_id});
  await accessEnded(p);await remaining(k);unexpired(original,token);await rejected(p,signal,original);await rejected(p,signal,token);await remaining(k);
});
async function inboundBytes(p,kind) {
  return p.evaluate(async kind=>{try{const reports=await Promise.all(window.huddleTestPeerConnections.map(pc=>pc.getStats()));return reports.reduce((total,report)=>total+[...report.values()].filter(s=>s.type==='inbound-rtp'&&s.kind===kind).reduce((bytes,s)=>bytes+s.bytesReceived,0),0);}catch{return -1;}},kind);
}
async function mediaStopped(p,kind) {
  const deadline=Date.now()+20000;let previous=await inboundBytes(p,kind);assert.ok(previous>=0,'could not read inbound '+kind+' RTP statistics');let stable=0;
  for(;;) {
    await new Promise(resolve=>setTimeout(resolve,250));const current=await inboundBytes(p,kind);assert.ok(current>=0,'could not read inbound '+kind+' RTP statistics');
    stable=current===previous?stable+1:0;if(stable>=4)return;
    assert.ok(Date.now()<deadline,kind+' RTP bytes kept increasing after server revocation');previous=current;
  }
}
test('server enforcement removes a revoked participant that ignores browser access checks',async t=>{
  const f=await mediaFixture(t,['jz','kevin']);const p=f.pages.jz;const k=f.pages.kevin;await join(p);const issued=await credentials(p);await join(k);await participantCount(p,2);await media(p,'audio');assert.ok(await inboundBytes(p,'audio')>0);
  await p.evaluate(()=>{
    const nativeAccessFetch=window.fetch.bind(window);
    window.fetch=(...args)=>{
      const input=args[0];const options=args[1]||{};const url=typeof input==='string'?input:input.url;const method=(options.method||input.method||'GET').toUpperCase();
      if(method==='GET'&&/^\/rooms\/\d+\/huddle$/.test(new URL(url,location.origin).pathname))return Promise.resolve(new Response('{}',{status:200,headers:{'Content-Type':'application/json'}}));
      return nativeAccessFetch(...args);
    };
  });
  await f.mutate({op:'revoke',grant:issued.grant_id});await k.locator('#channel-huddle[data-state="connected"]').waitFor();await participantCount(k,1,20000);await mediaStopped(p,'audio');
});
