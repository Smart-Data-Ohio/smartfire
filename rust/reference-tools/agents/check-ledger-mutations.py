#!/usr/bin/env python3
"""Compile broken budget, event and step policies against named Rails cases."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CARGO_BUILD_JOBS='2',CI='1',TMPDIR=str(root/'.scratch'))
cases=[
 ('count-openers','agent_posting.rs','AND board_post_opener=0','AND board_post_opener IN (0,1)','ws11_budget_case_board_opener_counts_only_as_post'),
 ('repeat-notice','agent_posting.rs','ON CONFLICT(agent_id,cap,day) DO NOTHING RETURNING id','RETURNING id','ws11_budget_case_denial_notifies_owner_once_per_day'),
 ('message-scope','agent_delivery.rs','Self::of_types(conn, agent_id, &MESSAGE_TYPES)','Self::of_types(conn, agent_id, &DELIVERABLE_TYPES)','ws11_event_case_message_deliverable_scope'),
 ('foreign-step','agent_step.rs','&& Some(message.creator_id) != user','&& Some(message.creator_id) == user','ws11_step_case_rejects_foreign_message'),
]
for name,file,before,after,test in cases:
 p=root/'rust/crates/db/src/models'/file;original=p.read_text();assert original.count(before)==1,(name,original.count(before))
 log=root/'.scratch'/f'ledger-mutation-{name}.log'
 try:
  p.write_text(original.replace(before,after))
  with log.open('w') as out:
   run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p','campfire_db',test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  summaries=[s for s in log.read_text().splitlines() if s.startswith('test result:')]
  assert run.returncode==101 and any('FAILED. 0 passed; 1 failed;' in s for s in summaries),(name,log)
  print(f'WS11 ledger mutation: {name}; exit={run.returncode}; '+summaries[-1],flush=True)
 finally:p.write_text(original)
print('WS11 ledger mutations: 4 compiled mutations caught; source restored')
