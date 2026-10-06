#!/usr/bin/env python3
"""Compile discriminating resume, throttle and sweep-plan mutations; restore exactly."""
from pathlib import Path
import os, subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
for name,file,before,after,test in [
 ('resume','message.rs','if self.streaming && !Self::find(conn, self.id)?.streaming {','if false && self.streaming && !Self::find(conn, self.id)?.streaming {','ws11_stream_case_finalized_stream_never_resumes'),
 ('throttle','agent_streaming.rs','last > tx.now().ago(BROADCAST_INTERVAL)','last > tx.now()','ws11_stream_case_broadcasts_coalesce_four_per_second'),
 ('sweep-plan','agent_streaming.rs','messages.streaming_updated_at < ?";','messages.streaming_updated_at < ? ORDER BY id";','ws11_stream_case_sweep_uses_streaming_activity_index')]:
 p=root/'rust/crates/db/src/models'/file;original=p.read_text();assert original.count(before)==1
 log=root/'.scratch'/f'stream-case-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire_db',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  assert run.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
  print(f'WS11 streaming mutation: {name}; 1 test failed; original restored')
 finally:p.write_text(original)
