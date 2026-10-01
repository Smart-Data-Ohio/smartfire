#!/usr/bin/env python3
"""Run the five new HTTP tests against the received slice, then restore both controllers."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
router=root/'crates/campfire/src/controllers.rs'
refresh=root/'crates/campfire/src/controllers/rooms/refreshes.rs'
originals={p:p.read_bytes() for p in [router,refresh]}
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
try:
    source=router.read_text()
    for action in ['create','destroy']:
        line=f'        "rooms/reads#{action}" => arc(rooms::reads::{action}),\n'
        assert source.count(line)==1
        source=source.replace(line,'')
    router.write_text(source)
    refresh.write_bytes(subprocess.check_output(['git','show','c4849d54:rust/crates/campfire/src/controllers/rooms/refreshes.rs'],cwd=root.parent))
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire','controllers::rooms::reads_tests','--','--test-threads=4'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/'reads-discrimination.log').write_text(result.stdout)
    summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
    assert result.returncode==101 and len(summaries)==1 and '0 passed; 5 failed;' in summaries[0],result.stdout
    print(summaries[0])
finally:
    for path,content in originals.items():path.write_bytes(content)
print('Reads discrimination: five compiled HTTP regressions rejected c4849d54; source restored')
