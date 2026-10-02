#!/usr/bin/env python3
"""Prove the real HTML-to-external execution replay rejects a JSON-only controller."""
from pathlib import Path
import os
import subprocess

root=Path(__file__).resolve().parents[3]
source=root/'crates/campfire/src/controllers/agent_approvals.rs'
original=source.read_text()
needle='ApprovalDecision::Applied => match c.respond_to(&[&format::HTML, &format::JSON])?'
assert original.count(needle)==1
try:
    source.write_text(original.replace(needle,'ApprovalDecision::Applied => match c.respond_to(&[&format::JSON])?'))
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-p','campfire','ws11ui_human_','--','--test-threads=8','--nocapture'],cwd=root,env={**os.environ,'CARGO_BUILD_JOBS':'2'},capture_output=True,text=True)
    output=result.stdout+result.stderr
    assert result.returncode and 'test result: FAILED. 0 passed; 2 failed;' in output and '406' in output, output
    print(next(line for line in output.splitlines() if line.startswith('test result:')))
    print('HTML execution discrimination: both real controller/job/transport replays reject a JSON-only controller; source restored')
finally:
    source.write_text(original)
