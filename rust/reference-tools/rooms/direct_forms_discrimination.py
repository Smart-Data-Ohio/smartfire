#!/usr/bin/env python3
"""Compile wrong per-viewer facts; fail actual HTTP tests; always restore the source."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[3]
source=root/'rust/crates/campfire/src/controllers/rooms/directs.rs'
original=source.read_text()
mutants=[('administrator:current_user.is_administrator()', 'administrator:true', 'direct_forms_keep_human_membership_and_group_delete_gates'),('users.sort_by_key(|u|!u.starred);', 'for user in &mut users { user.starred=false; } users.sort_by_key(|u|!u.starred);', 'direct_picker_uses_live_viewer_stars_agent_facts_and_active_people')]
env=os.environ.copy();env.update(CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
try:
 for old,new,test in mutants:
  assert original.count(old)==1,old
  source.write_text(original.replace(old,new))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path',str(root/'rust/Cargo.toml'),'-p','campfire',test,'--','--test-threads=4'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  lines=run.stdout.splitlines(); summaries=[line for line in lines if line.startswith('test result:')]
  assert run.returncode and any('FAILED' in line for line in summaries),run.stdout
  print('\n'.join(summaries),flush=True)
finally: source.write_text(original)
print('Direct form discrimination: group-delete visibility and viewer-star mutants rejected; source restored')
