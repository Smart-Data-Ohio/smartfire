#!/usr/bin/env python3
"""Restore the old show lookup and remove join dispatch; all four compiled HTTP tests fail."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
router=root/'crates/campfire/src/controllers.rs'
rooms=root/'crates/campfire/src/controllers/rooms.rs'
originals={p:p.read_text() for p in [router,rooms]}
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'))
try:
    line='        "rooms#join" => arc(rooms::join),\n'
    assert originals[router].count(line)==1
    router.write_text(originals[router].replace(line,''))
    old='    let (room, join_preview) = set_room_for_show(c, Scope::All).await?;'
    assert originals[rooms].count(old)==1
    rooms.write_text(originals[rooms].replace(old,'    let (room, join_preview) = (set_room(c, Scope::All).await?, false);'))
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire','controllers::rooms::join_tests','--','--test-threads=4'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/'join-discrimination.log').write_text(result.stdout)
    summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
    assert result.returncode==101 and len(summaries)==1 and '0 passed; 4 failed;' in summaries[0],result.stdout
    print(summaries[0])
finally:
    for path,content in originals.items():path.write_text(content)
print('Join discrimination: four compiled HTTP regressions rejected legacy lookup and absent dispatch; source restored')
