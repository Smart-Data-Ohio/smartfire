#!/usr/bin/env python3
"""Compile broken Fizzy workspace/lifecycle guards; require assertion failures and restore."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch/fizzy-http-mutations'
scratch.mkdir(parents=True,exist_ok=True)
mutations=[
 ('read_workspace','crates/campfire/src/integrations/fizzy/agent_reads.rs',"capability='fizzy' AND room_id IS NULL", "capability='fizzy'",'fizzy_read_wire_errors_and_coercions'),
 ('write_workspace','crates/campfire/src/integrations/fizzy/agent_requests.rs',"capability='external_action' AND room_id IS NULL", "capability='external_action'",'fizzy_action_wire_errors_replays_and_coercions'),
 ('disconnect_owner','crates/campfire/src/jobs.rs','account.mark_disconnected(tx, "Account deactivated")?;', 'let _ = account;', 'rails_deactivating_the_user_disconnects_the_account'),
]
for name,file,before,after,test in mutations:
 p=root/file; original=p.read_text(); assert original.count(before)==1,(name,original.count(before))
 try:
  p.write_text(original.replace(before,after,1))
  result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p','campfire',test,'--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch),INTEGRATION_TEST_PORT_RANGE='52920-52949'),capture_output=True,text=True)
  output=result.stdout+result.stderr; (scratch/(name+'.log')).write_text(output)
  summary=re.search(r'^test result: FAILED\..*$',output,re.M)
  assert result.returncode!=0 and summary and 'assertion' in output and 'error[E' not in output,output[-4000:]
  print('WS11-api Fizzy '+name+' mutation: '+summary.group(),flush=True)
 finally: p.write_text(original)
print('WS11-api Fizzy mutations: 3 broken guards rejected; sources restored',flush=True)
