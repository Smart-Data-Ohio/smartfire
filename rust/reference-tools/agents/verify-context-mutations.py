#!/usr/bin/env python3
"""Require context assertions to reject visibility and conversation regressions."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2];scratch=root.parent/'.scratch'/'ws11-context-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'))
p=root/'crates/db/src/models/agent_context.rs'
mutations=[('membership','if !exists(','if false && !exists('),('read-grant','if !agent_access::capability_for_agent','if false && !agent_access::capability_for_agent'),('limit-cap','limit = limit.min(100)','limit = limit.min(101)'),('conversation','OR thread_id=?)','OR thread_id=? OR thread_id IS NULL)'),('trigger-cutoff','AND (? IS NULL OR id<=?)','AND (? IS NULL OR ? IS NOT NULL)')]
for name,before,after in mutations:
 original=p.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
 if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
 try:
  p.write_text(re.sub(pattern,lambda _:after,original))
  r=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p','campfire_db','ws11_context','--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(r.stdout);summary=re.search(r'^test result: FAILED\..*$',r.stdout,re.M)
  if r.returncode!=101 or not summary or 'error[E' in r.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 context discrimination: {len(mutations)} compiled regressions detected; sources restored')
