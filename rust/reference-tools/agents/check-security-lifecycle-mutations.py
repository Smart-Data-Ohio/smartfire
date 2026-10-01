#!/usr/bin/env python3
"""Compiled authority/commit/cache regressions, restored even on failure."""
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[3]
cases=[
 ('membership-revocation','db/src/models/membership.rs','crate::models::AgentGrant::revoke_for_membership(tx, self.user_id, self.room_id)?;','let _ = self.user_id;','campfire_db','ws11_revocation_case_membership_scoped_only'),
 ('quiet-after-commit','db/src/models/agent_lifecycle.rs','let user_id = agent.user_id;','let user_id = -1;','campfire_db','ws11_kill_case_quiet_stream'),
 ('github-kill-authority','campfire/src/integrations/github/agent_actions.rs','if !self.active {','if false {','campfire','ws11_kill_case_approved_github_job'),
]
for name,file,before,after,package,test in cases:
 path=root/'rust/crates'/file
 source=path.read_text();assert source.count(before)==1,name
 log=root/'.scratch'/f'{name}-mutation.log'
 try:
  path.write_text(source.replace(before,after))
  with log.open('w') as out:
   result=subprocess.run(['rust/reference-tools/agents/run-focused.sh','-p',package,test],cwd=root,stdout=out,stderr=subprocess.STDOUT)
  lines=log.read_text().splitlines()
  assert result.returncode and any('test result: FAILED.' in s for s in lines) and not any('could not compile' in s for s in lines),name
  print(name+': '+next(s for s in lines if s.startswith('test result: FAILED.')),flush=True)
 finally:path.write_text(source)
print('WS11 security lifecycle discrimination: 3 compiled regressions rejected; sources restored')
