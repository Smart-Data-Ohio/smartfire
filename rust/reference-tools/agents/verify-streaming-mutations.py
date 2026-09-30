#!/usr/bin/env python3
"""Reject compiled stream access, claim, trailing-stamp and enqueue regressions."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2];scratch=root.parent/'.scratch'/'ws11-streaming-mutations';scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52900-52919',MAIL_TEST_PORT_RANGE='52920-52949',INTEGRATION_TEST_PORT_RANGE='52920-52949')
mutations=[
 ('membership','agent_streaming.rs','if !exists(','if false && !exists(','campfire_db'),
 ('post-grant','agent_streaming.rs','if !agent_access::capability_for_agent','if false && !agent_access::capability_for_agent','campfire_db'),
 ('locked-thread','agent_streaming.rs','if message.streaming','if false && message.streaming','campfire_db'),
 ('append-precedence','agent_streaming.rs','let source = append.map_or_else','let source = append.filter(|_|markdown_source.is_none()).map_or_else','campfire_db'),
 ('stamp-check','agent_streaming.rs','message.stream_broadcast_at.map(stamp).as_deref() != Some(&job.last_broadcast_at)','false','campfire_db'),
 ('trailing-queue','agent_streaming.rs','tx.emit_after_commit(crate::Event::job(&StreamTrailingBroadcastJob {message_id:message.id,last_broadcast_at:stamp(last),}));','let _ = crate::Event::job(&StreamTrailingBroadcastJob {message_id:message.id,last_broadcast_at:stamp(last),});','campfire'),
 ('overdue-boundary','agent_streaming.rs','messages.streaming_updated_at < ?','messages.streaming_updated_at <= ?','campfire_db'),
]
for name,file,before,after,package in mutations:
 p=root/'crates/db/src/models'/file;original=p.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
 if not re.search(pattern,original):raise RuntimeError(f'{name}: missing anchor')
 try:
  changed=re.sub(pattern,lambda _:after,original)
  p.write_text(changed)
  test='ws11_stream_rejected_trailing' if package=='campfire' else 'ws11_stream'
  r=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p',package,test,'--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  (scratch/f'{name}.log').write_text(r.stdout);summary=re.search(r'^test result: FAILED\..*$',r.stdout,re.M)
  if r.returncode!=101 or not summary or 'error[E' in r.stdout:raise RuntimeError(f'{name}: no compiled assertion failure')
  print(f'{name}: {summary.group()}',flush=True)
 finally:p.write_text(original)
print(f'WS11 streaming discrimination: {len(mutations)} compiled regressions detected; sources restored')
