#!/usr/bin/env python3
"""Compile agent authority/identity/outcome defects against the real domain and HTTP client."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[
 (' AND room_id IS NULL','', 'ws15e_fizzy_agent_requests_match_pinned_service_results'),
 ('if count < limit {','if true {','ws15e_fizzy_agent_requests_match_pinned_service_results'),
 ('account.id,account.fizzy_user_id,','account.id,None::<String>,','ws15e_fizzy_agent_requests_match_pinned_service_results'),
]
path=root/'crates/campfire/src/integrations/fizzy/agent_requests.rs'

for index,(old,new,test) in enumerate(mutants,1):
 original=path.read_text();assert original.count(old)==1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','-j','4','-p','campfire',test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'fizzy-request-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} agent_requests.rs {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e Fizzy request mutation checks: {len(mutants)} detected, 0 survived')
