#!/usr/bin/env python3
"""Compile weakened delivery policies; require the real app comparisons to fail."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
p=root/'rust/crates/db/src/models/agent_delivery.rs'
env=dict(os.environ,CI='1',CARGO_BUILD_JOBS='2',TMPDIR=str(root/'.scratch'),INTEGRATION_TEST_PORT_RANGE='52250-52298')
cases=[('rate','pub const RATE_LIMIT: i64 = 20;','pub const RATE_LIMIT: i64 = 28;','ws11_delivery_path_concurrent_mentions_obey_rate_limit'),('claim','?==1 && configured {enqueue_webhook','?>=0 && configured {enqueue_webhook','ws11_delivery_path_lost_delivery_claim_posts_nothing')]
for name,before,after,test in cases:
 original=p.read_text();assert original.count(before)==1,name
 log=root/'.scratch'/f'delivery-path-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p','campfire',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  summaries=[s for s in log.read_text().splitlines() if s.startswith('test result:')]
  assert run.returncode==101 and any('FAILED. 0 passed; 1 failed;' in s for s in summaries),(name,log)
  print(f'WS11 delivery path mutation: {name}; exit={run.returncode}; '+summaries[-1],flush=True)
 finally:p.write_text(original)
print('WS11 delivery path mutations: 2 compiled mutations caught; source restored')
