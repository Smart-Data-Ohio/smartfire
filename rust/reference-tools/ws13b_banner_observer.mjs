// Run the pinned, real Stimulus controller with observed frames, including timers.
// Only browser adapters are replaced. No handwritten banner reducer.
import fs from 'node:fs';
import vm from 'node:vm';
const corpus=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const source=fs.readFileSync(new URL('../../app/javascript/controllers/huddle_invitation_controller.js',import.meta.url),'utf8');
for (const entry of corpus.cases) {
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
  for (let i=0;i<entry.phases.length;i++) {
    const step=entry.spec.steps[i];now+=step.seconds*1000;
    for (;;) {
      const due=[...timers.entries()].filter(([,timer])=>timer.due<=now).sort((a,b)=>a[1].due-b[1].due||a[0]-b[0])[0];
      if(!due)break;timers.delete(due[0]);due[1].fn();
    }
    if(step.action==='dismiss')await instance.dismiss();
    if(step.action==='rejoin')context.window.dispatchEvent(new CustomEvent('huddle:changed',{detail:{roomId:186869642,state:'connected'}}));
    for (const frame of entry.phases[i].frames) receive(frame.payload);
    entry.phases[i].banner={hidden:instance.element.hidden,item:instance.activityItemIdValue,room:instance.roomIdValue,
      readPath:instance.readPathValue??'',handledPath:instance.handledPathValue??'',title:instance.titleTarget.textContent??'',description:instance.descriptionTarget.textContent??'',shouldRing:instance.shouldRing??false};
  }
  instance.disconnect();
}
process.stdout.write(JSON.stringify(corpus));
