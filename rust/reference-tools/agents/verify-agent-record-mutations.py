#!/usr/bin/env python3
"""Require compiled assertions for agent state and working-presence regressions."""
from pathlib import Path
import os
import re
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch' / 'ws11-agent-record-mutations'
scratch.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, CI='1', TMPDIR=str(root.parent / '.scratch'), CARGO_TARGET_DIR=str(root / 'target'))
path = root / 'crates/db/src/models/agent.rs'
mutations = [
    ('suspension-revocation', 'AgentGrant::revoke_for_agent(tx, self.id)?;', '', 'ws11_agent_descriptions'),
    ('presence-expiry', 'expires > now', 'true', 'ws11_agent_working_presence'),
    ('status-stamp', 'if self.status != before.status {', 'if false {', 'ws11_agent_status_stamp'),
    ('presence-char-limit', 's.chars().count()>limit', 's.len()>limit', 'ws11_agent_record_validation'),
    ('live-agent', 'a.suspended_at IS NULL AND u.status=0', '1=1', 'ws11_agent_descriptions'),
]
for name,before,after,test in mutations:
    original=path.read_text()
    pattern=r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    if not re.search(pattern,original): raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern,lambda _:after,original))
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p','campfire_db',test,'--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'{name}.log').write_text(result.stdout)
        summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
        if result.returncode!=101 or not summary or 'error[E' in result.stdout: raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}',flush=True)
    finally: path.write_text(original)
print(f'WS11 agent record discrimination: {len(mutations)} compiled regressions detected; sources restored')
