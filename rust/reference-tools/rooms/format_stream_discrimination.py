#!/usr/bin/env python3
"""Moving the format failure after publication must fail the compiled socket check."""
import os
from pathlib import Path
import subprocess

root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
path=root/'crates/campfire/src/controllers/rooms/closeds.rs'
original=path.read_bytes()
guard='''    let formats = c.formats()?;
    if !formats.contains(&&campfire_kit::format::HTML) && !formats.contains(&&campfire_kit::format::ALL) {
        return Err(Error::internal(anyhow::anyhow!("Missing partial users/sidebars/rooms/shared for requested format")));
    }
'''
tail='        .map_err(db_error)\n}\n'
try:
    source=original.decode()
    assert source.count(guard)==1 and source.endswith(tail)
    source=source.replace(guard,'')
    source=source[:-len(tail)]+'        .map_err(db_error)?;\n'+guard+'    Ok(())\n}\n'
    path.write_text(source)
    env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52100-52149')
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire','closed_request_partial_failures_commit_but_publish_no_controller_frames','--','--test-threads=1'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/'format-stream-discrimination.log').write_text(result.stdout)
    summaries=[line for line in result.stdout.splitlines() if line.startswith('test result:')]
    assert result.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0] and 'expected no frame' in result.stdout,result.stdout
    print(summaries[0])
finally:
    path.write_bytes(original)
print('Format stream discrimination: compiled socket regression rejected post-publication failure; source restored')
