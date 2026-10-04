// Run one pinned, real Stimulus controller per client, with independent timers.
// Only browser adapters are replaced. No handwritten banner reducer.
import fs from 'node:fs';
import vm from 'node:vm';
import readline from 'node:readline';
import { isDeepStrictEqual } from 'node:util';
import { execFileSync } from 'node:child_process';
// Always execute the pinned UI, including after a main merge changes local files.
const pin=fs.readFileSync(new URL('../parity/reference.sha',import.meta.url),'utf8').trim();
const source=execFileSync('git',['show',`${pin}:app/javascript/controllers/huddle_invitation_controller.js`],{cwd:new URL('../../',import.meta.url),encoding:'utf8'});
async function client(userId) {
  let receive,now=0,next=0;
  const timers=new Map(), requests=[];
  const context=vm.createContext({window:new EventTarget(),document:{visibilityState:'visible',querySelector:()=>null},Event,CustomEvent,queueMicrotask,Symbol,
    setTimeout:(fn,ms)=>{const id=++next;timers.set(id,{fn,due:now+ms});return id;},clearTimeout:id=>timers.delete(id),setInterval:()=>0,clearInterval:()=>{},fetch:async(path,options)=>{requests.push({user_id:Number(userId),path,method:options.method});}});
  const controller=new vm.SyntheticModule(['Controller'],function(){this.setExport('Controller',class{});},{context});
  const cable=new vm.SyntheticModule(['cable'],function(){this.setExport('cable',{subscribeTo:async(_,callbacks)=>{receive=callbacks.received;return{unsubscribe(){}};}});},{context});
  const mod=new vm.SourceTextModule(source,{context});
  await mod.link(spec=>spec==='@hotwired/stimulus'?controller:cable);await mod.evaluate();
  const instance=new mod.namespace.default();
  Object.assign(instance,{element:{hidden:true},titleTarget:{},descriptionTarget:{},activityItemIdValue:0,roomIdValue:0,ringTimeoutValue:45000,endedTimeoutValue:5000});
  await instance.connect();
  return {
    async step(operation,isRecipient,roomId) {
      now+=operation.seconds*1000;
      for (;;) {
        const due=[...timers.entries()].filter(([,timer])=>timer.due<=now).sort((a,b)=>a[1].due-b[1].due||a[0]-b[0])[0];
        if(!due)break;timers.delete(due[0]);due[1].fn();
      }
      // A hidden banner has no clickable dismiss button.
      if(isRecipient && operation.action==='dismiss' && !instance.element.hidden)await instance.dismiss();
      if(isRecipient && operation.action==='rejoin')context.window.dispatchEvent(new CustomEvent('huddle:changed',{detail:{roomId,state:'connected'}}));
      return requests.splice(0);
    },
    receive:payload=>receive(payload),
    snapshot:()=>({hidden:instance.element.hidden,item:instance.activityItemIdValue,room:instance.roomIdValue,
      readPath:instance.readPathValue??'',handledPath:instance.handledPathValue??'',title:instance.titleTarget.textContent??'',description:instance.descriptionTarget.textContent??'',shouldRing:instance.shouldRing??false}),
    disconnect:()=>instance.disconnect()
  };
}
async function session(spec) {
  const ids=spec.client_ids.map(String);
  const clients=new Map();
  for(const id of ids)clients.set(id,await client(id));
  return {
    async before(operation) {
      const requests=[];
      for(const [id,browser] of clients)requests.push(...await browser.step(operation,id===String(spec.recipient_id),spec.room_id));
      return {requests};
    },
    after(phase) {
      for(const frame of phase.frames)clients.get(frame.stream.match(/^user_(\d+)_/)[1]).receive(frame.payload);
      return {banners:Object.fromEntries([...clients].map(([id,browser])=>[id,browser.snapshot()]))};
    },
    disconnect() {for(const browser of clients.values())browser.disconnect();}
  };
}
if(process.env.WS13B_BANNER_STREAM==='1') {
  let current;
  for await(const line of readline.createInterface({input:process.stdin,crlfDelay:Infinity})) {
    try {
      const message=JSON.parse(line);let response;
      if(message.ui_start) {current?.disconnect();current=await session(message.ui_start);response={};}
      else if(message.ui_before)response=await current.before(message.ui_before);
      else if(message.ui_after)response=current.after(message.ui_after);
      else throw new Error('unknown browser command');
      process.stdout.write(JSON.stringify(response)+'\n');
    } catch(error) {process.stdout.write(JSON.stringify({error:String(error)})+'\n');}
  }
  current?.disconnect();
} else {
  const corpus=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
  for(const entry of corpus.cases) {
    const current=await session(entry.spec);
    for(let i=0;i<entry.phases.length;i++) {
      const phase=entry.phases[i],before=await current.before(entry.spec.steps[i]);
      if(!isDeepStrictEqual(before.requests,phase.requests))throw new Error('observed client requests differ: '+entry.spec.name+' step '+i);
      phase.banners=current.after(phase).banners;
    }
    current.disconnect();
  }
  process.stdout.write(JSON.stringify(corpus));
}
