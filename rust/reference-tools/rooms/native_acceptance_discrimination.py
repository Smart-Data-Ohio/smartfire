#!/usr/bin/env python3
"""Compile broken production page/recovery paths and require the native checks to fail."""
import os, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[3]
scratch=root/'.scratch/native-acceptance-discrimination';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',CARGO_BUILD_JOBS='2',TMPDIR=str(root/'.scratch'),CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_DEBUG='0',CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
changes=[('full-page-header','rust/crates/campfire/src/controllers/presenters/room_native.rs','let mut show=show.clone();','let mut show=show.clone(); show.room.header.as_mut().unwrap().display_name = "Injected room".into();','full_native_room_pages_match_four_complete_rails_pages'),('stuck-room-recovery','rust/crates/db/src/models/room_delete.rs','let cutoff = tx.now().ago(jiff::SignedDuration::from_secs(grace_seconds));','let cutoff = tx.now().ago(jiff::SignedDuration::from_secs(grace_seconds + 3600));','queue_decision_keeps_atomic_http_failure_and_recovers_a_rails_tombstone')]
for name,file,before,after,test in changes:
 path=root/file; original=path.read_bytes(); source=original.decode();assert source.count(before)==1,name
 try:
  path.write_text(source.replace(before,after))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire',test,'--','--test-threads=4','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/(name+'.log')).write_text(run.stdout)
  summaries=[line for line in run.stdout.splitlines() if line.startswith('test result:')]
  assert run.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0],run.stdout[-6000:]
  print(f'{name}: {summaries[0]}',flush=True)
 finally:path.write_bytes(original)
print('Native acceptance discrimination: 2 broken production paths rejected; source restored')
