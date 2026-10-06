#!/usr/bin/env python3
"""Expired decisions must remain refused even when validation failures are values."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3];p=root/'rust/crates/db/src/models/agent_approval.rs'
original=p.read_text();before='let errors = self.pending_errors(tx.now());\n        if !errors.is_empty() {'
assert original.count(before)==2
log=root/'.scratch/approval-case-mutation.log'
try:
 p.write_text(original.replace(before,'let errors = self.pending_errors(tx.now());\n        if false && !errors.is_empty() {'))
 with log.open('w') as out:
  run=subprocess.run(['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire_db','ws11_approval_case_decide_rejects_expired_and_decided','--','--test-threads=4'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target')),stdout=out,stderr=subprocess.STDOUT)
 assert run.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
 print('WS11 approval mutation: expired or decided request accepted; 1 test failed; original restored')
finally:p.write_text(original)
