#!/usr/bin/env python3
"""Compile regressions in the message ledger, atomic queue and claimed POST policy."""
from pathlib import Path
import os, re, subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'/'ws11-delivery-mutations'
scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
mutations=[
 ('ledger-hook','campfire_db','crates/db/src/models/message.rs','crate::models::agent_delivery::enqueue_for_message(tx, &message)?;','','ws11_agent_message_chain_records_pending_and_rate_suppression'),
 ('queue-atomicity','campfire','crates/db/src/models/message.rs','crate::models::agent_delivery::enqueue_for_message(tx, &message)?;','','ws11_agent_delivery_enqueue_failure_rolls_the_http_message_back'),
 ('rate-cap','campfire_db','crates/db/src/models/agent_delivery.rs','pub const RATE_LIMIT: i64 = 20;','pub const RATE_LIMIT: i64 = 99;','ws11_agent_rate_limit_is_atomic_across_concurrent_posts'),
 ('revocation','campfire_db','crates/db/src/models/agent_delivery.rs','if revoked {','if false {','ws11_agent_delivery_claims_revocation_ack_and_recovery'),
 ('attempt-claim','campfire','crates/db/src/models/agent_delivery.rs',"AND webhook_attempts=? AND webhook_status='pending'","AND ? IS NOT NULL AND webhook_status='pending'",'ws11_agent_webhook_http_retries_claims_and_permanent_failures'),
 ('five-attempt-cap','campfire_db','crates/db/src/models/agent_delivery.rs','pub const MAX_ATTEMPTS: i64 = 5;','pub const MAX_ATTEMPTS: i64 = 99;','ws11_five_claimed_attempts_match_rails_exhaustion_vectors'),
 ('future-retry','campfire_db','crates/db/src/models/agent_delivery.rs','now.ago(SignedDuration::from_mins(2))','now.since(SignedDuration::from_hours(2))','ws11_agent_delivery_claims_revocation_ack_and_recovery'),
 ('recovery-isolation','campfire','crates/campfire/src/jobs/periodic.rs','db.write(move |tx|domain::recover_one(tx,candidate)).await','db.write(move |tx| {domain::recover_one(tx,candidate)?; Err::<(),_>(campfire_db::Error::Other("WS11 regression".into()))}).await','ws11_recovery_continues_after_one_durable_enqueue_failure'),
]
for name,package,relative,before,after,test in mutations:
 path=root/relative;original=path.read_text()
 if before not in original:raise RuntimeError(f'{name}: missing anchor')
 try:
  path.write_text(original.replace(before,after))
  result=subprocess.run(['cargo','test','--locked','-j','4','-p',package,test,'--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(result.stdout)
  summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
  if result.returncode!=101 or not summary or f'{test} ... FAILED' not in result.stdout or 'error[E' in result.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:path.write_text(original)
print(f'WS11 delivery discrimination: {len(mutations)} compiled regressions detected; sources restored')
