#!/usr/bin/env python3
"""Prove each inbox lifecycle branch catches a real writer defect."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
for defect in ['reminder','recurrence','deleted-source']:
 result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-p','campfire','ws11ui_next_inbox_lifecycle','--','--test-threads=8','--nocapture'],cwd=root,env={**os.environ,'CARGO_BUILD_JOBS':'2','WS11UI_INBOX_LIFECYCLE_DEFECT':defect},capture_output=True,text=True)
 output=result.stdout+result.stderr
 assert result.returncode and 'test result: FAILED. 0 passed; 1 failed;' in output, output
 print(next(line for line in output.splitlines() if line.startswith('test result:')))
 print(f'Inbox lifecycle discrimination: {defect} writer defect rejected; real producer and HTTP assertions unchanged')
