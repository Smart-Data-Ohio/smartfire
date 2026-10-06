#!/usr/bin/env python3
"""Compile broken recovery policies; each named case must actually fail."""
from pathlib import Path
import os, subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CARGO_BUILD_JOBS='2',CI='1',TMPDIR=str(root/'.scratch'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
p=root/'rust/crates/db/src/models/agent_delivery.rs'
cases=[
 ('fresh-grace','let grace = now.ago(SignedDuration::from_mins(2));','let grace = now.ago(SignedDuration::from_secs(30));','ws11_recovery_case_fresh_pending_left_alone'),
 ('retry-snapshot','AND webhook_attempts=? AND webhook_next_attempt_at IS ?','AND webhook_attempts>=? AND (? IS NULL OR webhook_next_attempt_at IS NOT NULL)','ws11_recovery_case_snapshot_cannot_overwrite_new_retry_after'),
 ('stale-attempt',"WHERE id=? AND webhook_attempts=? AND webhook_status='pending'","WHERE id=? AND webhook_attempts>=? AND webhook_status='pending'",'ws11_recovery_case_stale_attempt_exits_without_post'),
]
for name,before,after,test in cases:
 original=p.read_text();assert original.count(before)==1,(name,original.count(before))
 log=root/'.scratch'/f'recovery-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p','campfire',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  summaries=[s for s in log.read_text().splitlines() if s.startswith('test result:')]
  assert run.returncode==101 and any('FAILED. 0 passed; 1 failed;' in s for s in summaries),(name,log)
  print(f'WS11 recovery mutation: {name}; exit={run.returncode}; '+summaries[-1],flush=True)
 finally:p.write_text(original)
print('WS11 recovery mutations: 3 compiled mutations caught; source restored')
