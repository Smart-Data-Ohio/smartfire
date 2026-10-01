#!/usr/bin/env python3
"""Discriminate execution-audit recording and delivery despite an audit outage."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch/ws15g';scratch.mkdir(parents=True,exist_ok=True)
env=os.environ|{'CI':'1','CABLE_TEST_PORT_RANGE':'51500-51549','TMPDIR':str(scratch),'CARGO_TARGET_DIR':str(root/'target')}
mutants=[
 ('execution-audit-omitted','crates/campfire/src/integrations/github/agent_actions.rs','    changes.as_object_mut().unwrap().remove("approval_id");','    return;','github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails'),
 ('sweep-audit-outage-aborts-outcome','crates/campfire/src/integrations/action_claims.rs','tracing::error!(%error, event_id, "Stuck integration claim audit failed");','return Err(error);','github_claim_audit_failure_does_not_skip_webhook_and_queue_failure_rolls_back'),
]
for name,file,before,after,test in mutants:
 path=root/file;source=path.read_text();assert source.count(before)==1,name
 try:
  path.write_text(source.replace(before,after))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--manifest-path',str(root/'Cargo.toml'),'--locked','-j4','-p','campfire',test,'--','--nocapture'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
  (scratch/f'round2-mutation-{name}.log').write_text(run.stdout)
  summary=[l for l in run.stdout.splitlines() if l.startswith('test result:')]
  assert run.returncode and summary and '1 failed;' in summary[-1],run.stdout
  print(f'{name}: {summary[-1]}',flush=True)
 finally:path.write_text(source)
print('GitHub execution audit mutations: 2 rejected; 0 survived',flush=True)
