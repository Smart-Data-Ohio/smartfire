#!/usr/bin/env python3
"""Compile refused-network, permanent-status and exhausted-attempt mutations."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'),INTEGRATION_TEST_PORT_RANGE='52250-52299',CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249')
for name,file,before,after,test in [
 ('private-url','rust/crates/campfire/src/net/guard.rs','if blocked_at_reference_pin(address) {','if false && blocked_at_reference_pin(address) {','ws11_event_webhook_case_guard_refuses_without_post'),
 ('permanent-status','rust/crates/campfire/src/jobs/agent_jobs.rs','status == 408 || status == 429 || status >= 500','status == 404 || status == 408 || status == 429 || status >= 500','ws11_event_webhook_case_not_found_fails_fast'),
 ('attempt-limit','rust/crates/db/src/models/agent_delivery.rs','e.webhook_attempts >= MAX_ATTEMPTS','e.webhook_attempts > MAX_ATTEMPTS','ws11_event_webhook_case_fifth_failure_exhausts_without_enqueue')]:
 p=root/file;original=p.read_text();assert original.count(before)==1
 log=root/'.scratch'/f'event-webhook-case-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  assert run.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
  print(f'WS11 event webhook mutation: {name}; 1 test failed; original restored')
 finally:p.write_text(original)
