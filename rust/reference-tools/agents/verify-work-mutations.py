#!/usr/bin/env python3
"""Reject compiled work-event privacy, hop, snapshot and queue regressions."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'/'ws11-work-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
mutations=[
 ('read-grant','agent_work_events.rs','!agent_access::capability_for_agent(tx.conn(), agent.id, "read_messages", Some(room_id),)?','false','campfire_db','ws11_work_events_assign'),
 ('membership','agent_work_events.rs','!exists(tx.conn(), "SELECT 1 FROM memberships WHERE user_id=? AND room_id=?", params![agent.user_id,room_id],)?','false','campfire_db','ws11_work_events_assign'),
 ('hop-limit','agent_work_events.rs','hop>=bot_webhook_fanout::HOP_LIMIT','hop>bot_webhook_fanout::HOP_LIMIT','campfire_db','ws11_work_events_assign'),
 ('self-trigger','bot_webhook_fanout.rs','AND (actor_id IS NULL OR actor_id != ?)','AND ? IS NOT NULL','campfire_db','ws11_work_events_assign'),
 ('deleted-snapshot','agent_work_events.rs','json!({"work_snapshot":deleted.snapshot})','json!({"work_snapshot":null,"unused":deleted.snapshot})','campfire_db','ws11_work_events_assign'),
 ('work-queue','agent_work_events.rs','tx.emit_after_commit(crate::Event::job(&EventWebhookJob {event_id:event.id,attempt:Some(event.webhook_attempts),}))','let _ = crate::Event::job(&EventWebhookJob {event_id:event.id,attempt:Some(event.webhook_attempts)})','campfire','ws11_work_delete_queue'),
]
for name,file,before,after,package,test in mutations:
 p=root/'crates/db/src/models'/file;original=p.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
 if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
 try:
  p.write_text(re.sub(pattern,lambda _:after,original))
  r=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p',package,test,'--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(r.stdout);summary=re.search(r'^test result: FAILED\..*$',r.stdout,re.M)
  if r.returncode!=101 or not summary or 'error[E' in r.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 work discrimination: {len(mutations)} compiled regressions detected; sources restored')
