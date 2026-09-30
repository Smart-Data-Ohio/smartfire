#!/usr/bin/env python3
"""Compile agent authority/identity/outcome defects against the real domain and HTTP client."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
env=os.environ.copy()
env.update(CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'.scratch/target'),GITHUB_TEST_PORT_RANGE='51550-51594',CABLE_TEST_PORT_RANGE='51550-51594')
mutants=[
 ('if !a.grant {','if false {','ws15e_fizzy_agent_execution_rechecks_and_records_once'),
 ('|| a.fizzy_user != account.fizzy_user_id','|| false','ws15e_fizzy_agent_execution_rechecks_and_records_once'),
 ('action_claims::rewrite_running(tx, event, metadata, message.map(str::to_owned))?','(action_claims::rewrite_running(tx, event, metadata, message.map(str::to_owned))? || true)','ws15e_fizzy_agent_worker_loses_to_sweep_and_duplicate_jobs_do_no_http'),
]
path=root/'crates/campfire/src/integrations/fizzy/agent_job.rs'
for index,(old,new,test) in enumerate(mutants,1):
 original=path.read_text();assert original.count(old)==1,old
 try:
  path.write_text(original.replace(old,new))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','-j','4','-p','campfire',test,'--','--nocapture'],cwd=root,env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (root.parent/'.scratch'/f'fizzy-agent-mutation-{index}.log').write_text(run.stdout)
  assert run.returncode and 'test result: FAILED' in run.stdout,run.stdout[-2500:]
  print(f'{index} agent_job.rs {test}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')),flush=True)
 finally:path.write_text(original)
print(f'WS15e Fizzy agent mutation checks: {len(mutants)} detected, 0 survived')
