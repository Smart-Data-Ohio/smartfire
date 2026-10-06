#!/usr/bin/env python3
"""Require compiled failures for payload privacy and retained snapshots."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2];scratch=root.parent/'.scratch'/'ws11-payload-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
p=root/'crates/db/src/models/agent_payloads.rs'
mutations=[('private-title','let visible = public','let visible = true'),('owner-access','access.contains(&(id, owner.to_lowercase(), repo.to_lowercase()))','access.iter().any(|(_,o,r)|o==&owner.to_lowercase() && r==&repo.to_lowercase())'),('deleted-snapshot','.get("work_snapshot")','.get("absent_snapshot")'),('compact-false','map.retain(|_, v| !v.is_null())','map.retain(|_, v| !v.is_null() && *v != Value::Bool(false))')]
for name,before,after in mutations:
 original=p.read_text()
 if before not in original:raise RuntimeError(f'{name}: missing anchor')
 try:
  p.write_text(original.replace(before,after))
  result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','ws11_agent_event_payloads_match_rails_bytes','--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(result.stdout);summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
  if result.returncode!=101 or not summary or 'error[E' in result.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 payload discrimination: {len(mutations)} compiled regressions detected; sources restored')
