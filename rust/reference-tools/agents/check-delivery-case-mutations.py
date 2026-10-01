#!/usr/bin/env python3
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
p=root/'rust/crates/db/src/models/agent_delivery.rs'
for name,before,after,test in [
 ('revocation','!= Some(true);','== Some(true);','ws11_delivery_case_revoked_at_perform_time_writes_a_suppression_row'),
 ('hop-limit','if hop >= 3 {','if hop >= 30 {','ws11_delivery_case_hop_limit_suppresses_a_chain_that_reaches_hop_3'),
]:
 original=p.read_text();assert original.count(before)==1
 log=root/'.scratch'/f'delivery-case-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire_db',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  assert result.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
  print(f'WS11 delivery case mutation: {name}; 1 test failed; original restored')
 finally:p.write_text(original)
