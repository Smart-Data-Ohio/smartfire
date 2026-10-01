#!/usr/bin/env python3
"""Compile wrong Unicode validation, group identity and membership authorization."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[3]
mutants=[
 ('rust/crates/db/src/models/direct_room.rs','name.chars().count() > 100','name.len() > 100','direct_rename_coercions_and_rejections_match_real_rails_requests'),
 ('rust/crates/campfire/src/controllers/rooms/directs.rs','let group_capable=room.direct_group_capable(conn)?;','let group_capable=member_ids.len()>2;','named_pair_keeps_group_controls_and_cannot_be_deleted_by_a_plain_member'),
 ('rust/crates/campfire/src/controllers/rooms.rs','Room::find_for_user(conn, user_id, id)','Room::find_by_id(conn, id)','removed_members_and_wrong_room_types_cannot_submit_invalid_names'),
]
env=os.environ.copy();env.update(CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'))
for path,old,new,test in mutants:
 source=root/path;original=source.read_text();assert old in original
 try:
  source.write_text(original.replace(old,new,1))
  run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','--manifest-path',str(root/'rust/Cargo.toml'),'-p','campfire',test,'--','--test-threads=4'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  lines=[line for line in run.stdout.splitlines() if line.startswith('test result:')]
  assert run.returncode and any('FAILED' in line for line in lines),run.stdout
  print('\n'.join(lines),flush=True)
 finally:source.write_text(original)
print('Direct rename discrimination: Unicode length, named-pair identity and membership mutants rejected; sources restored')
