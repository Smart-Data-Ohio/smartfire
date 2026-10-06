#!/usr/bin/env python3
"""Compile agent authority/identity/outcome defects against the real domain and HTTP client."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[(' AND room_id IS NULL','', 'ws15e_fizzy_agent_reads_reject_room_grants_suspension_and_corrupt_owner_token')]
path=root/'crates/campfire/src/integrations/fizzy/agent_reads.rs'

for index,(old,new,test) in enumerate(mutants,1):
 original=path.read_text();assert original.count(old)==1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['cargo','test','-j','4','-p','campfire',test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'fizzy-read-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} agent_reads.rs {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e Fizzy read mutation checks: {len(mutants)} detected, 0 survived')
