// Exact assertion scopes of channel_threads_controller_test.rb:304-505.
// Controller declarations use real token-bearing HTTP; browser visibility is not a scope here.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
export const workControllerCases=[
  'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
  'assigned owner can change work status but cannot reassign it',
  'only a thread manager can remove work tracking',
  'a manager can assign an eligible agent and the agent is notified',
  'a member who cannot manage the thread cannot assign an agent',
];
export async function workControllers({author,recipient,base,caseName,fixture}) {
  const url=`${base}/rooms/654632876/threads/${fixture.thread_id}.json`;
  const databases=JSON.parse(process.env.WS8BM_WORK_DATABASES||'{}');
  assert.ok(databases[base],'fresh work-controller database path required');
  const counts=()=>JSON.parse(execFileSync('python3',[
    fileURLToPath(new URL('./behavior_work_rows.py',import.meta.url)),databases[base],String(fixture.eligible_agent_id),
  ],{encoding:'utf8'}));
  async function request(page,input,expected=200,agentDelta=0) {
    const before=input===null?null:counts();
    const response=await page.evaluate(async({url,input})=>{
      const result=await fetch(url,{method:input===null?'GET':'PATCH',headers:{'Content-Type':'application/json','X-CSRF-Token':document.querySelector('meta[name="csrf-token"]').content},...(input===null?{}:{body:JSON.stringify({thread:input})})});
      return {status:result.status,body:await result.text()};
    },{url,input});
    assert.equal(response.status,expected,`${caseName}: ${response.body}`);
    if(before) {
      const after=counts();
      // Pinned Rails :308,315,336,360,379: one GLOBAL event per successful
      // change and zero on refusal. Separate snapshots prevent later writes
      // from cancelling an earlier incorrect delta.
      assert.equal(after.work_events-before.work_events,expected===200?1:0,`work-event-count: ${caseName}: PATCH ${JSON.stringify(input)}`);
      assert.equal(after.agent_events-before.agent_events,agentDelta,`agent-event-count: ${caseName}: PATCH ${JSON.stringify(input)}`);
    }
    const body=response.body?JSON.parse(response.body):null;
    if(expected===200)assert.equal(body.thread.name,'Design discussion');
    return body;
  }
  const thread=async(page=author)=>(await request(page,null)).thread;
  if(caseName.startsWith('converts a thread')) {
    const first=(await request(author,{work_status:'planned'})).thread;assert.equal(first.work,true);assert.equal(first.work_status,'planned');
    const next=(await request(author,{work_owner_id:712064548})).thread;assert.equal(next.work_owner.id,712064548);
    assert.equal(next.work_history.length,2);const event=next.work_history[0];assert.equal(event.event_type,'work_assignment');
    assert.equal(event.before.owner,null);assert.equal(event.after.owner.id,712064548);assert.equal(event.before.status,'planned');assert.equal(event.after.status,'planned');assert.equal(event.actor.id,773523953);
  } else if(caseName.startsWith('assigned owner')) {
    const before=await thread(recipient);const update=(await request(recipient,{work_status:'in_progress'})).thread;assert.equal(update.work_status,'in_progress');assert.equal(update.work_history.length,before.work_history.length+1);
    await request(recipient,{work_owner_id:773523953},403);assert.equal((await thread()).work_owner.id,712064548);
    await request(recipient,{work_status:''},403);assert.equal((await thread()).work_status,'in_progress');
  } else if(caseName.startsWith('only a thread manager')) {
    const before=await thread();const after=(await request(author,{work_status:'',work_owner_id:''})).thread;
    assert.equal(after.work_history.length,before.work_history.length+1);assert.equal(after.work_status,null);assert.equal(after.work_owner,null);
  } else if(caseName.startsWith('a manager can assign')) {
    await request(author,{work_status:'planned'});const after=(await request(author,{work_owner_id:fixture.eligible_id},200,1)).thread;
    assert.equal(after.work_owner.id,fixture.eligible_id);assert.equal(after.work_owner.agent,true);
  } else if(caseName.startsWith('a member who')) {
    await request(recipient,{work_owner_id:fixture.eligible_id},403);assert.equal((await thread()).work_owner,null);
  } else throw new Error(`Unimplemented work controller: ${caseName}`);
}
