#!/usr/bin/env python3
"""Compile a deliberately bypassed adapter; security/rollback checks must reject it."""
from pathlib import Path
import os, subprocess
root=Path(__file__).resolve().parents[3]
p=root/'rust/crates/db/src/database.rs'
source=p.read_text()
before='sink.model_callback(self, crate::callbacks::Callback { phase, record_id })'
after='let _ = (sink, phase, record_id); Ok(())'
assert source.count(before)==1
log=root/'.scratch/peer-callback-mutation.log'
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
try:
 p.write_text(source.replace(before,after))
 with log.open('w') as out:
  result=subprocess.run(['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire_db','ws11_peer','--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
 assert result.returncode and 'test result: FAILED.' in log.read_text(),log
 assert '4 failed' in log.read_text(),log
 print('WS11 peer mutation: bypassed transactional adapter; 4 tests failed; original restored')
finally:p.write_text(source)
