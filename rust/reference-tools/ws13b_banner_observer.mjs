// Run one pinned, real Stimulus controller per client, with independent timers.
// Only browser adapters are replaced. No handwritten banner reducer.
import fs from 'node:fs';
import vm from 'node:vm';
const corpus=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const source=fs.readFileSync(new URL('../../app/javascript/controllers/huddle_invitation_controller.js',import.meta.url),'utf8');
async function client() {
  let receive,now=0,next=0;
  const timers=new Map();
  const context=vm.createContext({window:new EventTarget(),document:{visibilityState:'visible'},Event,CustomEvent,queueMicrotask,Symbol,
    setTimeout:(fn,ms)=>{const id=++next;timers.set(id,{fn,due:now+ms});return id;},clearTimeout:id=>timers.delete(id),setInterval:()=>0,clearInterval:()=>{},fetch:async()=>{}});
  const controller=new vm.SyntheticModule(['Controller'],function(){this.setExport('Controller',class{});},{context});
  const cable=new vm.SyntheticModule(['cable'],function(){this.setExport('cable',{subscribeTo:async(_,callbacks)=>{receive=callbacks.received;return{unsubscribe(){}};}});},{context});
  const mod=new vm.SourceTextModule(source,{context});
  await mod.link(spec=>spec==='@hotwired/stimulus'?controller:cable);await mod.evaluate();
  const instance=new mod.namespace.default();
  Object.assign(instance,{element:{hidden:true},titleTarget:{},descriptionTarget:{},activityItemIdValue:0,roomIdValue:0,ringTimeoutValue:45000,endedTimeoutValue:5000});
  await instance.connect();
  return {
    async step(operation,isRecipient) {
      now+=operation.seconds*1000;
      for (;;) {
        const due=[...timers.entries()].filter(([,timer])=>timer.due<=now).sort((a,b)=>a[1].due-b[1].due||a[0]-b[0])[0];
        if(!due)break;timers.delete(due[0]);due[1].fn();
      }
      if(isRecipient && operation.action==='dismiss')await instance.dismiss();
      if(isRecipient && operation.action==='rejoin')context.window.dispatchEvent(new CustomEvent('huddle:changed',{detail:{roomId:186869642,state:'connected'}}));
    },
    receive:payload=>receive(payload),
    snapshot:()=>({hidden:instance.element.hidden,item:instance.activityItemIdValue,room:instance.roomIdValue,
      readPath:instance.readPathValue??'',handledPath:instance.handledPathValue??'',title:instance.titleTarget.textContent??'',description:instance.descriptionTarget.textContent??'',shouldRing:instance.shouldRing??false}),
    disconnect:()=>instance.disconnect()
  };
}
for (const entry of corpus.cases) {
  // The Rails record supplies the actor and all observed stream recipients.
  // Each client sees only its own broadcasts, never another user's frames.
  const ids=[...new Set([String(entry.spec.recipient_id),...entry.phases.flatMap(phase=>phase.frames.map(frame=>frame.stream.match(/^user_(\d+)_/)[1]))])].sort((a,b)=>Number(a)-Number(b));
  const clients=new Map();
  for(const id of ids)clients.set(id,await client());
  for(let i=0;i<entry.phases.length;i++) {
    const phase=entry.phases[i];
    for(const [id,browser] of clients)await browser.step(entry.spec.steps[i],id===String(entry.spec.recipient_id));
    for(const frame of phase.frames)clients.get(frame.stream.match(/^user_(\d+)_/)[1]).receive(frame.payload);
    phase.banners=Object.fromEntries([...clients].map(([id,browser])=>[id,browser.snapshot()]));
  }
  for(const browser of clients.values())browser.disconnect();
}
process.stdout.write(JSON.stringify(corpus));
