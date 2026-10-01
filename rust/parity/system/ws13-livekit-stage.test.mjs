// Four original LiveKit Stage declarations, using an actual local media server.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {fixture,text,absent,panel,row} from './ws13-support.mjs';
async function join(p){
  await p.getByRole('button',{name:'Join stage',exact:true}).click();
  await p.waitForFunction(()=>['prejoin','connecting','connected'].includes(document.getElementById('channel-huddle').dataset.state),null,{timeout:10000,polling:50});
  if(await p.locator('#channel-huddle[data-state="prejoin"]').count()) {
    await p.locator('[data-huddle-target="checkJoin"]:not([disabled])').waitFor({timeout:20000});await p.locator('[data-huddle-target="checkJoin"]').click();
  }
  await p.locator('#channel-huddle[data-state="connected"]').waitFor({timeout:20000});
}
const canPublish=p=>p.evaluate(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle')?.room?.localParticipant?.permissions?.canPublish??null);
async function permission(p,value){await p.waitForFunction(value=>document.getElementById('channel-huddle').dataset.state==='connected'&&window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle')?.room?.localParticipant?.permissions?.canPublish===value,value,{timeout:30000,polling:50});}
const note=p=>text(p,'#channel-huddle [data-huddle-target="listeningNote"]','You are listening');
const noMute=p=>absent(p,'#channel-huddle [data-huddle-target="mute"]');
test('a listener joins subscribe-only while the host publishes',async t=>{
  const f=await fixture(t,['jason','david']);const p=f.pages.jason;const host=f.pages.david;await join(p);assert.equal(await canPublish(p),false);
  for(const target of ['mute','share','camera'])await absent(p,`#channel-huddle [data-huddle-target="${target}"]`);await note(p);
  await join(host);assert.equal(await canPublish(host),true);await text(host,'#channel-huddle [data-huddle-target="mute"]','Mute microphone');
  for(const page of [host,p])await page.waitForFunction(()=>document.querySelectorAll('.huddle__participant').length===2,null,{timeout:2000});
});
test('inviting a listener to speak rejoins them publishing, and moving them back removes publish',async t=>{
  const f=await fixture(t,['jason','david']);const p=f.pages.jason;const host=f.pages.david;await join(p);assert.equal(await canPublish(p),false);await panel(host);
  await host.locator(row(f.people.jason.membership)).getByRole('button',{name:'Invite to speak',exact:true}).click();await permission(p,true);
  await text(p,'#channel-huddle [data-huddle-target="mute"]','Mute microphone');await absent(p,'#channel-huddle [data-huddle-target="listeningNote"]');
  await host.locator(row(f.people.jason.membership)).getByRole('button',{name:'Move to audience',exact:true}).click({timeout:10000});await permission(p,false);await note(p);await noMute(p);
});
test('a host server-mutes a speaker and they rejoin muted, then unmutes them',async t=>{
  const f=await fixture(t,['jason','david'],{jason_speaker:true});const p=f.pages.jason;const host=f.pages.david;await join(p);assert.equal(await canPublish(p),true);await panel(host);
  await host.locator(row(f.people.jason.membership)).getByRole('button',{name:'Mute',exact:true}).click();await permission(p,false);await note(p);await noMute(p);
  await host.locator(row(f.people.jason.membership)).getByRole('button',{name:'Unmute',exact:true}).click({timeout:10000});await permission(p,true);await text(p,'#channel-huddle [data-huddle-target="mute"]','Mute microphone');
});
test('a listener survives a full reconnect and stays subscribe-only',async t=>{
  const f=await fixture(t,['jason','david']);const p=f.pages.jason;await join(p);assert.equal(await canPublish(p),false);await join(f.pages.david);
  await p.waitForFunction(()=>document.querySelectorAll('.huddle__participant').length===2,null,{timeout:2000});
  const result=await p.evaluate(async()=>{const c=window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle');try{await c.room.simulateScenario('full-reconnect');return {ok:true};}catch(error){return {ok:false,message:error.message};}});
  assert.equal(result.ok,true,'SDK could not start a full reconnect');await p.locator('#channel-huddle[data-state="connected"]').waitFor({timeout:20000});
  await p.waitForFunction(()=>window.Stimulus.getControllerForElementAndIdentifier(document.getElementById('channel-huddle'),'huddle')?.room?.localParticipant?.permissions?.canPublish===false,null,{timeout:10000,polling:50});await note(p);await noMute(p);
});
