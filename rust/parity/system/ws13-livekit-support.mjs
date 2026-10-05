// Instrumentation from pinned Rails HuddlesTest; native WebRTC and local LiveKit.
import assert from 'node:assert/strict';
import {fixture, text, absent, connected} from './ws13-support.mjs';
import {pollBrowser} from './ws13-browser-poll.mjs';
export const target = name => '[data-huddle-target="'+name+'"]';
export const controller = "window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle')";
export const microphone = controller+"?.room?.localParticipant?.getTrackPublication('microphone')?.audioTrack";
export const evaluate = (p, expression) => p.evaluate(expression);
export const wait = (p, expression, message) => p.waitForFunction(new Function('return ('+expression+');'),null,{timeout:20000,polling:100}).catch(error=>{error.message=message+': '+error.message;throw error;});
// Capybara click_button matches the visible button text, even when a screen-reader
// annotation contributes to its accessible name.
export const click = (p,name) => name.startsWith('Noise suppression ')
  ? p.locator(target('noise')).filter({hasText:name}).click()
  : p.getByRole('button',{name,exact:true}).click();
export async function instrument(p) {
  await p.addInitScript(INSTRUMENT);
  // Rails registers CDP initialization before the later session visits.
  // about:blank has no secure-context mediaDevices; initialize on its real visit.
  if(p.url()!=='about:blank') await p.evaluate(INSTRUMENT);
}
export async function mediaFixture(t,names=['jz'],options={}) {
  const f=await fixture(t,names,{kind:'designers',...options});
  for (const p of Object.values(f.pages)) await instrument(p);
  return f;
}
export async function join(p,label='Join huddle') {
  await click(p,label);
  await wait(p,"['prejoin','connecting','connected'].includes(document.getElementById('channel-huddle').dataset.state)",'the huddle did not start joining');
  if(await p.locator('#channel-huddle[data-state="prejoin"]').count()) {
    await p.locator(target('checkJoin')+':not([disabled])').waitFor({timeout:20000});
    await p.locator(target('checkJoin')).click();
  }
  await connected(p);
  await p.getByRole('button',{name:'Mute microphone',exact:true}).waitFor();
}
export async function leave(p) {
  await click(p,'Leave');
  await absent(p,'#channel-huddle:not([hidden])');
}
export const processor = p => evaluate(p,microphone+"?.getProcessor()?.name ?? null");
export const trackId = p => evaluate(p,microphone+"?._mediaStreamTrack?.id ?? null");
export const trackState = p => evaluate(p,microphone+"?._mediaStreamTrack?.readyState ?? null");
export const storedNoise = p => evaluate(p,"localStorage.getItem('campfire.huddle.noiseSuppression')");
export const constraints = p => evaluate(p,"(window.huddleTestGetUserMediaConstraints||[]).filter(c=>c&&typeof c.audio==='object').at(-1)?.audio??null");
export const setting = (p,name) => evaluate(p,microphone+"?._mediaStreamTrack?.getSettings()["+JSON.stringify(name)+"]??null");
export const meterLevel = p => evaluate(p,"Number(document.querySelector("+JSON.stringify(target('meter'))+").getAttribute('aria-valuenow'))");
export const meterRunning = p => evaluate(p,controller+"?.microphoneMeter?.running??false");
export const stat = (p,name) => evaluate(p,"document.querySelector("+JSON.stringify(target(name))+").textContent");
export const sampling = p => evaluate(p,controller+"?.connectionStatsTimer!==null");
export async function breakNoise(p,name=null) {
  await p.evaluate(name=>{
    const addModule=AudioWorklet.prototype.addModule;
    AudioWorklet.prototype.addModule=function(url,...rest) {
      if(String(url).includes('noise-suppressor-worklet')) return Promise.reject(name?new DOMException('Test noise suppressor refusal',name):new Error('Test noise suppressor failure'));
      return addModule.call(this,url,...rest);
    };
  },name);
}
export async function noise(p,on) {
  await text(p,target('noise')+'[aria-pressed="'+on+'"]',on?'Noise suppression on':'Noise suppression off');
  await wait(p,microphone+"?.getProcessor()?.name "+(on?"=== 'campfire-rnnoise'":"== null"),on?'the RNNoise processor never attached to the microphone':'the RNNoise processor was not removed');
}
export async function media(p,kind,after=0,activeOnly=false) {
  // Playwright 1.63 tests Promise truthiness in waitForFunction, before it
  // settles. Sample with evaluate (which awaits it), like Rails' async script.
  await pollBrowser(p,async ({kind,after,activeOnly})=>{
    const pcs=window.huddleTestPeerConnections.slice(after).filter(pc=>!activeOnly||['connected','completed'].includes(pc.connectionState));
    const reports=await Promise.all(pcs.map(pc=>pc.getStats())).catch(()=>[]);
    return reports.some(report=>[...report.values()].some(s=>s.type==='inbound-rtp'&&s.kind===kind&&s.bytesReceived>0));
  },{kind,after,activeOnly},{timeout:20000,polling:100,message:'no '+kind+' RTP media arrived from LiveKit'});
}
export async function reconnect(p) {
  const result=await p.evaluate(async()=>{
    const c=window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle');
    try{await c.room.simulateScenario('full-reconnect');return {ok:true};}catch(error){return {ok:false,message:error.message};}
  });
  assert.equal(result.ok,true,'SDK could not start a full reconnect');
  await p.locator('#channel-huddle[data-state="connected"]').waitFor({timeout:20000});
}
export async function permission(p,state) {
  await p.evaluate(state=>{
    window.huddleTestPermissionsQuery ||= navigator.permissions.query.bind(navigator.permissions);
    navigator.permissions.query=description=>description?.name==='microphone'?Promise.resolve({state,onchange:null}):window.huddleTestPermissionsQuery(description);
  },state);
}
export const restorePermission = p => p.evaluate(()=>{navigator.permissions.query=window.huddleTestPermissionsQuery;});
export async function participantCount(p,count,timeout=2000) {
  await p.waitForFunction(count=>document.querySelectorAll('.huddle__participant').length===count,count,{timeout,polling:50});
}
export {text,absent};

const INSTRUMENT = "(() => {\nif (window.huddleTestInstrumentationInstalled) return;\nwindow.huddleTestInstrumentationInstalled = true;\nconst forceRelay = false;\n\nwindow.huddleTestPeerConnections = [];\nwindow.huddleTestLocalTracks = [];\nwindow.huddleTestCredentials = [];\nwindow.huddleTestWebSocketUrls = [];\nwindow.huddleTestGetUserMediaConstraints = [];\n\nconst nativeFetch = window.fetch.bind(window);\nwindow.fetch = async (...args) => {\n  const response = await nativeFetch(...args);\n  try {\n    const input = args[0];\n    const options = args[1] || {};\n    const url = typeof input === 'string' ? input : input.url;\n    const method = (options.method || input.method || 'GET').toUpperCase();\n    if (method === 'POST' && new URL(url, window.location.origin).pathname.match(/^\\/rooms\\/\\d+\\/huddle$/)) {\n      response.clone().json().then(body => window.huddleTestCredentials.push(body));\n    }\n  } catch (_) {}\n  return response;\n};\n\nconst NativeWebSocket = window.WebSocket;\nwindow.WebSocket = class extends NativeWebSocket {\n  constructor(...args) {\n    super(...args);\n    window.huddleTestWebSocketUrls.push(String(args[0]));\n  }\n};\n\nconst NativePeerConnection = window.RTCPeerConnection;\nwindow.RTCPeerConnection = class extends NativePeerConnection {\n  constructor(...args) {\n    if (forceRelay) {\n      args[0] = { ...(args[0] || {}), iceTransportPolicy: 'relay' };\n    }\n    super(...args);\n    window.huddleTestPeerConnections.push(this);\n  }\n\n  setConfiguration(configuration) {\n    const nextConfiguration = forceRelay\n      ? { ...(configuration || {}), iceTransportPolicy: 'relay' }\n      : configuration;\n    super.setConfiguration(nextConfiguration);\n  }\n};\nconst nativeGetUserMedia = navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);\nnavigator.mediaDevices.getUserMedia = async (...args) => {\n  window.huddleTestGetUserMediaConstraints.push(args[0]);\n  const stream = await nativeGetUserMedia(...args);\n  window.huddleTestLocalTracks.push(...stream.getTracks());\n  return stream;\n};\n// Synthetic browser-generated screen content keeps personal desktop data\n// out of tests; LiveKit's real publish, transport and decode paths run.\nnavigator.mediaDevices.getDisplayMedia = async () => {\n  const canvas = document.createElement('canvas');\n  canvas.width = 640;\n  canvas.height = 360;\n  const context = canvas.getContext('2d');\n  let frame = 0;\n  const draw = () => {\n    context.fillStyle = frame++ % 2 ? '#164e63' : '#0f766e';\n    context.fillRect(0, 0, 640, 360);\n    context.fillStyle = 'white';\n    context.font = '32px sans-serif';\n    context.fillText('Smartfire screen-share test', 40, 180);\n  };\n  draw();\n  const stream = canvas.captureStream(10);\n  const timer = setInterval(draw, 100);\n  stream.getVideoTracks()[0].addEventListener('ended', () => clearInterval(timer));\n  window.huddleTestLocalTracks.push(...stream.getTracks());\n  return stream;\n};\n})();";
