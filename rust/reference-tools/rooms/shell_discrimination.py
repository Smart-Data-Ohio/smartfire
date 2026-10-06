#!/usr/bin/env python3
"""Reject visible placeholder markup and the wrong unread scroll threshold, after compilation."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
base=['cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml')]
def reject(name,path,old,new,package,test):
    original=path.read_text()
    assert original.count(old)==1,'mutation must apply once'
    try:
        path.write_text(original.replace(old,new))
        result=subprocess.run(base+package+[test,'--','--nocapture'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'shell-{name}-discrimination.log').write_text(result.stdout)
        summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
        assert result.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0],result.stdout
        print(summaries[0])
    finally:
        path.write_text(original)
    print(f'Room shell discrimination: compiled {name} mutation rejected; source restored',flush=True)
reject('visible-list-placeholder',root/'crates/views/src/rooms.rs','.unwrap_or_else(h::empty)','.unwrap_or_else(||h::raw("Empty room"))',['-p','campfire_views','--test','room_shell'],'empty_room_shell_regions_match_rails')
reject('scroll-threshold',root/'crates/campfire/src/controllers/presenters/room_shell.rs','count > 5','count >= 5',['-p','campfire','--bin','campfire'],'unread_shell_facts_match_rails_pointer_cases')

reject('unread-pill-label',root/'crates/views/src/rooms.rs','"Jump to unread"','"Read more"',['-p','campfire_views','--test','room_shell'],'unread_jump_controls_match_rails')
