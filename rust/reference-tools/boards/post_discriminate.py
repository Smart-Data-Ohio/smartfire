#!/usr/bin/env python3
"""Make the work-policy, revoked-grant and merged deletion-order assertions fail."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
env=os.environ.copy()
env.update(CARGO_BUILD_JOBS='2',TMPDIR=str(scratch),CI='1')
env.setdefault('CARGO_TARGET_DIR',str(scratch/'target'))
mutations=[
 ('assignment-policy','crates/db/src/models/channel_thread/work.rs','self.work_viewable_by(conn, user)? && self.settings_manageable_by(conn, user)?','self.work_viewable_by(conn, user)?','work_policy_separates_managers_owners_and_parent_members'),
 ('revoked-agent-grant','crates/db/src/models/channel_thread/work.rs','&& agent.can(conn, "post_messages", Some(room_id))?','&& true','work_candidates_follow_agent_grants_suspension_and_membership'),
 ('deletion-order','crates/db/src/models/channel_thread/board.rs','tx.after_commit_record("channel_threads", id,','tx.after_commit_record_latest("board_post_destroy", id,','board_deletion_rows_precede_a_failing_agent_ledger_callback'),
]
for name,relative,old,new,test in mutations:
 path=root/relative;source=path.read_text();assert source.count(old)==1,name
 try:
  path.write_text(source.replace(old,new))
  result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--manifest-path',str(root/'Cargo.toml'),'--locked','-p','campfire_db',test,'--','--test-threads=4'],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (scratch/'logs'/f'posts-mutation-{name}.log').write_text(result.stdout)
  summaries=[line for line in result.stdout.splitlines() if line.startswith('test result:')]
  assert result.returncode and any('1 failed' in line for line in summaries),f'mutation did not reach an assertion: {name}\n{result.stdout[-3000:]}'
  print(f'WS12 post mutation {name}: {summaries[-1]}',flush=True)
 finally:path.write_text(source)
print('WS12 post discrimination: 3 mutations rejected; sources restored',flush=True)
