#!/usr/bin/env python3
"""Show room integration assertions fail through their real HTTP callers."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch/ws15g';scratch.mkdir(parents=True,exist_ok=True)
env=os.environ|{'CI':'1','CABLE_TEST_PORT_RANGE':'51500-51549','TMPDIR':str(scratch),'CARGO_TARGET_DIR':str(root/'target')}
mutants=[
 ('room-card-caller-omitted','crates/campfire/src/controllers/presenters.rs','github_cards_html: Some(github_cards_html)','github_cards_html: Some({ let _=github_cards_html; String::new() })','github_room_cards_real_pages'),
 ('room-refresh-disabled','crates/campfire/src/controllers/rooms.rs','refresh_after_render(&c.app().db, refreshes).await','refresh_after_render(&c.app().db, { let _=refreshes; Vec::new() }).await','github_room_refresh_claims'),
 ('room-membership-bypassed','crates/campfire/src/controllers/rooms.rs','Room::find_for_user(conn, user_id, id)','Room::find_by_id(conn, id)','github_room_cards_security'),
]
for name,file,before,after,test in mutants:
 path=root/file;source=path.read_text()
 assert source.count(before)==1,name
 try:
  path.write_text(source.replace(before,after))
  run=subprocess.run(['cargo','test','--manifest-path',str(root/'Cargo.toml'),'--locked','-j4','-p','campfire',test,'--','--nocapture'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,timeout=300)
  (scratch/f'round2-mutation-{name}.log').write_text(run.stdout)
  summary=[l for l in run.stdout.splitlines() if l.startswith('test result:')]
  assert run.returncode and summary and '1 failed;' in summary[-1],run.stdout
  print(f'{name}: {summary[-1]}',flush=True)
 finally:path.write_text(source)
print('GitHub room-card mutations: 3 rejected; 0 survived',flush=True)
