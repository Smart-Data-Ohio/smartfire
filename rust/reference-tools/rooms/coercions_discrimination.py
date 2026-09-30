#!/usr/bin/env python3
"""Compile the received channel casts, redirects and request-independent partial lookup, then restore."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
paths=[root/'crates/campfire/src/controllers/rooms.rs',root/'crates/campfire/src/controllers/rooms/directs.rs',root/'crates/campfire/src/controllers/rooms/closeds.rs']
originals={p:p.read_bytes() for p in paths}
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'))
try:
    for path in paths:
        relative=path.relative_to(root.parent)
        source=subprocess.check_output(['git','show',f'8bdb43ec:{relative}'],cwd=root.parent)
        if path.name=='rooms.rs':source+=b'\n#[cfg(test)]\nmod coercions_tests;\n'
        path.write_bytes(source)
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire','controllers::rooms::coercions_tests','--','--test-threads=4'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/'coercions-discrimination.log').write_text(result.stdout)
    summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
    assert result.returncode==101 and len(summaries)==1 and '0 passed; 5 failed;' in summaries[0],result.stdout
    print(summaries[0])
finally:
    for path,source in originals.items():path.write_bytes(source)
print('Coercion discrimination: five compiled HTTP regressions rejected 8bdb43ec; source restored')
