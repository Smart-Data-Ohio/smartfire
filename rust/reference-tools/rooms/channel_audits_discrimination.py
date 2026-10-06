#!/usr/bin/env python3
"""Reject missing channel audits and missing row/header callbacks with compiled HTTP/socket tests."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
paths=[root/f'crates/campfire/src/controllers/rooms/{name}.rs' for name in ['opens','closeds']]
originals={p:p.read_bytes() for p in paths}
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
def reject(name,test,count):
    result=subprocess.run(['cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire',test,'--','--test-threads=4'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/f'channel-audits-{name}-discrimination.log').write_text(result.stdout)
    summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
    assert result.returncode==101 and len(summaries)==1 and f'0 passed; {count} failed;' in summaries[0],result.stdout
    print(summaries[0],flush=True)
try:
    for path in paths:
        path.write_bytes(subprocess.check_output(['git','show',f'e015d50e:rust/crates/campfire/src/controllers/rooms/{path.name}'],cwd=root.parent))
    reject('missing-audits','controllers::rooms::channel_audits_tests',3)
finally:
    for path,source in originals.items():path.write_bytes(source)
print('Channel audit discrimination: three compiled HTTP regressions rejected e015d50e; source restored',flush=True)
try:
    for path in paths:
        source=originals[path].decode()
        old='Some(&header)' if path.stem=='opens' else 'header.as_deref()'
        assert source.count(old)==1
        path.write_text(source.replace(old,'None'))
    reject('missing-header','channel_http_rows_and_headers_match_rails_after_audits',1)
finally:
    for path,source in originals.items():path.write_bytes(source)
print('Channel header discrimination: compiled missing-header socket regression rejected; source restored')
