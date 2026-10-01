#!/usr/bin/env python3
"""Require assertions to reject missing service writes and oversized presence."""
from pathlib import Path
import os
import re
import subprocess

root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'/'ws11-presence-mutations'
scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'))
mutations=[
    ('presence-write','agent_working_presence.rs','agent.set_working_presence(tx, text)','agent.set_working_presence(tx, Some(""))'),
    ('presence-cap','agent.rs','("working_presence", &a.working_presence, 140)','("working_presence", &a.working_presence, 141)'),
]
for name,filename,before,after in mutations:
    path=root/'crates/db/src/models'/filename
    original=path.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
    if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern,lambda _:after,original))
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p','campfire_db','ws11_presence_service_set','--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'{name}.log').write_text(result.stdout)
        summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
        if result.returncode!=101 or not summary or 'error[E' in result.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}',flush=True)
    finally:path.write_text(original)
print(f'WS11 presence discrimination: {len(mutations)} compiled regressions detected; sources restored')
