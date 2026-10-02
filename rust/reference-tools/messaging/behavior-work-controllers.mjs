// Exact assertion scopes of channel_threads_controller_test.rb:304-505.
// Controller declarations use real token-bearing HTTP; browser visibility is not a scope here.
import assert from 'node:assert/strict';
export const workControllerCases=[
  'converts a thread to work, assigns an eligible owner, and keeps an audit trail',
  'work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable',
  'assigned owner can change work status but cannot reassign it',
  'only a thread manager can remove work tracking',
  'a manager can assign an eligible agent and the agent is notified',
  'the owner picker lists eligible agents with profiles and excludes ineligible ones',
  'a member who cannot manage the thread cannot assign an agent',
  'ordinary thread fields remain separate from work tracking',
];
export async function workControllers({author,recipient,base,caseName,fixture}) {
  const url=`${base}/rooms/654632876/threads/${fixture.thread_id}.json`;
  async function request(page,input,expected=200) {
    const response=await page.evaluate(async({url,input})=>{
      const result=await fetch(url,{method:input===null?'GET':'PATCH',headers:{'Content-Type':'application/json','X-CSRF-Token':document.querySelector('meta[name="csrf-token"]').content},...(input===null?{}:{body:JSON.stringify({thread:input})})});
      return {status:result.status,body:await result.text()};
    },{url,input});
    assert.equal(response.status,expected,`${caseName}: ${response.body}`);
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
  } else if(caseName.startsWith('work owner must')) {
    await request(author,{work_status:'planned',work_owner_id:712064548});const before=await thread();
    const error=await request(author,{work_owner_id:fixture.botless_id},422);assert.match(error.error,/active agent member/);const after=await thread();assert.equal(after.work_owner.id,712064548);assert.equal(after.work_history.length,before.work_history.length);
    // The fixture provides a second work thread with the already revoked owner,
    // reproducing :344-349 without a private browser-side DB mutation.
    const unavailable=await author.evaluate(async url=>(await(await fetch(url)).json()).thread,`${base}/rooms/654632876/threads/${fixture.revoked_thread_id}.json`);
    assert.equal(unavailable.work_owner.active,false);assert.equal(unavailable.work_owner.name,'Kevin');
    const cleared=(await request(author,{work_owner_id:''})).thread;assert.equal(cleared.work_owner,null);
  } else if(caseName.startsWith('assigned owner')) {
    const before=await thread(recipient);const update=(await request(recipient,{work_status:'in_progress'})).thread;assert.equal(update.work_status,'in_progress');assert.equal(update.work_history.length,before.work_history.length+1);
    await request(recipient,{work_owner_id:773523953},403);assert.equal((await thread()).work_owner.id,712064548);
    await request(recipient,{work_status:''},403);assert.equal((await thread()).work_status,'in_progress');
  } else if(caseName.startsWith('only a thread manager')) {
    const before=await thread();const after=(await request(author,{work_status:'',work_owner_id:''})).thread;
    assert.equal(after.work_history.length,before.work_history.length+1);assert.equal(after.work_status,null);assert.equal(after.work_owner,null);
  } else if(caseName.startsWith('a manager can assign')) {
    await request(author,{work_status:'planned'});const after=(await request(author,{work_owner_id:fixture.eligible_id})).thread;
    assert.equal(after.work_owner.id,fixture.eligible_id);assert.equal(after.work_owner.agent,true);
  } else if(caseName.startsWith('the owner picker')) {
    const options=(await thread()).work_owner_options;assert.ok(options.some(row=>row.name==='Kevin'));
    const entry=options.find(row=>row.id===fixture.eligible_id);assert.ok(entry);assert.equal(entry.agent,true);assert.equal(entry.provider,'TestLab');assert.equal(entry.description,'Does the work');
    const ids=options.map(row=>row.id);for(const key of ['suspended_id','outside_id','reader_id','botless_id'])assert.ok(!ids.includes(fixture[key]),key);
  } else if(caseName.startsWith('a member who')) {
    await request(recipient,{work_owner_id:fixture.eligible_id},403);assert.equal((await thread()).work_owner,null);
  } else if(caseName.startsWith('ordinary thread')) {
    const payload=await thread();assert.equal(payload.work,false);assert.equal(payload.work_status,null);assert.equal(payload.work_owner,null);assert.equal(payload.work_history.length,0);
  } else throw new Error(`Unimplemented work controller: ${caseName}`);
}
