#!/usr/bin/env python3
"""Require DM and shared-posting assertions to reject policy and atomicity regressions."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2];scratch=root.parent/'.scratch'/'ws11-dm-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'))
mutations=[
 ('target-rule','agent_direct_messages.rs','if !allowed(tx, &agent, target.id)?','if false && !allowed(tx, &agent, target.id)?'),
 ('existing-room-scope','agent_direct_messages.rs','if !new_room && !agent_access::capability_for_agent','if false && !new_room && !agent_access::capability_for_agent'),
 ('inbound-types','agent_direct_messages.rs',"event_type IN ('mention','reply','direct_message')","event_type IN ('posted','reply','direct_message')"),
 ('replay-before-budget','agent_posting.rs','match prepare(tx, agent_id, a.room_id, a.client_message_id.as_deref())?','match prepare(tx, agent_id, a.room_id, None)?'),
 ('thread-savepoint','agent_posting.rs','match tx.savepoint(|tx| {','match (|tx: &mut Tx<\'_>| {'),
]
for name,file,before,after in mutations:
 p=root/'crates/db/src/models'/file;original=p.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
 if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
 try:
  changed=re.sub(pattern,lambda _:after,original)
  if name=='thread-savepoint':changed=changed.replace('}) {\n        Ok(message)', '})(tx) {\n        Ok(message)',1)
  p.write_text(changed)
  r=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p','campfire_db','agent_direct_messages_test','--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(r.stdout);summary=re.search(r'^test result: FAILED\..*$',r.stdout,re.M)
  if r.returncode!=101 or not summary or 'error[E' in r.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 DM discrimination: {len(mutations)} compiled regressions detected; sources restored')
