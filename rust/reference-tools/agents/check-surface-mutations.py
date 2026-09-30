#!/usr/bin/env python3
"""Require every new conversation/work contract group to reject bypassed policy."""
from pathlib import Path
import os
import re
import subprocess
root = Path(__file__).resolve().parents[2]
path = root / 'crates/campfire/src/controllers/agents/pending.rs'
original = path.read_text()
needle = '    match op {'
assert needle in original
scratch = root.parent / '.scratch/surface-mutations'
scratch.mkdir(parents=True, exist_ok=True)
try:
    path.write_text(original.replace(needle, '    if agent_id == agent.id { return campfire_db::models::agent_api_pending::execute(tx,agent_id,op,args); }\n' + needle, 1))
    result = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','-p','campfire','agent_surface_','--','--nocapture'], cwd=root, env=dict(os.environ,CI='1',TMPDIR=str(scratch)), capture_output=True,text=True)
    output = result.stdout + result.stderr
    (scratch / 'bypass.log').write_text(output)
    summary = re.search(r'^test result: FAILED\..*$',output,re.M)
    assert result.returncode != 0 and summary and 'assertion' in output and '4 failed' in summary.group(),output[-4000:]
    print('WS11-api preflight mutation: ' + summary.group())
finally:
    path.write_text(original)
print('WS11-api preflight mutation: four new groups rejected bypass; source restored')
path = root / 'crates/campfire/src/controllers/agents/integrations.rs'
original = path.read_text()
needle = '    if op == "github_pull_request_action" {'
assert needle in original
try:
    path.write_text(original.replace(needle, '    if agent_id == agent.id { return Ok(None); }\n' + needle, 1))
    result = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','-p','campfire','agent_surface_integrations_','--','--nocapture'], cwd=root, env=dict(os.environ,CI='1',TMPDIR=str(scratch)), capture_output=True,text=True)
    output = result.stdout + result.stderr
    (scratch / 'integrations.log').write_text(output)
    summary = re.search(r'^test result: FAILED\..*$',output,re.M)
    assert result.returncode != 0 and summary and 'assertion' in output and '2 failed' in summary.group(),output[-4000:]
    print('WS11-api integration mutation: ' + summary.group())
finally:
    path.write_text(original)
print('WS11-api integration mutation: both groups rejected bypass; source restored')
