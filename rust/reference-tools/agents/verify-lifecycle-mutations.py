#!/usr/bin/env python3
"""Reject compiled suspension, quiet finalization and rollback regressions."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2];scratch=root.parent/'.scratch'/'ws11-lifecycle-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'))
mutations=[
 ('suspension-revocation','agent.rs','AgentGrant::revoke_for_agent(tx, self.id)?','0'),
 ('quiet-callback','agent_lifecycle.rs','m.finalize_stream_quietly(tx)','{ let _ = m; Ok(false) }'),
 ('expire-before-cancel','agent_lifecycle.rs','approval.expire_if_due(tx)?','false'),
 ('finalize-claim','message.rs','AND "messages"."streaming" = 1','AND "messages"."streaming" IN (0,1)'),
 ('owner-deactivation','user.rs','super::agent_lifecycle::suspend_owned(tx, self.id, audit)?','{ let _=audit; }'),
]
for name,file,before,after in mutations:
 p=root/'crates/db/src/models'/file;original=p.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
 if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
 try:
  p.write_text(re.sub(pattern,lambda _:after,original))
  r=subprocess.run(['cargo','test','--locked','-j4','-p','campfire_db','agent_lifecycle_test','--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(r.stdout);summary=re.search(r'^test result: FAILED\..*$',r.stdout,re.M)
  if r.returncode!=101 or not summary or 'error[E' in r.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 lifecycle discrimination: {len(mutations)} compiled regressions detected; sources restored')
