#!/usr/bin/env python3
"""Compile HTTP capability regressions; compiler/setup errors never count as detection."""
import os
from pathlib import Path
import re
import subprocess
ROOT=Path(__file__).resolve().parents[3]
OUT=ROOT/'.scratch/upload-discrimination'
OUT.mkdir(parents=True,exist_ok=True)
source=ROOT/'rust/crates/campfire/src/controllers/messages.rs'
original=source.read_text()
command=['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire','controllers::messages::upload_tests::signed_root_and_thread_attachments_match_rails_response_and_blob_rows','--','--exact']
env=dict(os.environ,CI='1',TMPDIR=str(ROOT/'.scratch'),CARGO_TARGET_DIR=str(ROOT/'rust/target'),CABLE_TEST_PORT_RANGE='52000-52049',MAIL_TEST_PORT_RANGE='52000-52049')
env.pop('RUST_TEST_THREADS',None)
mutations=[
    ('expiry-clock','signed, c.now())','signed, c.now() - jiff::SignedDuration::from_secs(2))','expired_boundary_root'),
    ('signature-verification','.ok_or_else(invalid_attachment)?;','.unwrap_or(13);','tampered_root'),
]
try:
    for name,old,new,case in mutations:
        assert original.count(old)==1,name
        source.write_text(original.replace(old,new))
        result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True)
        output=result.stdout+result.stderr
        (OUT/f'{name}.log').write_text(output)
        summary=re.findall(r'^test result: FAILED\..*$',output,re.M)
        assert result.returncode and summary and f'failed: {case}' in output,name
        print(f'{name}: {summary[0]}',flush=True)
finally:
    source.write_text(original)
result=subprocess.run(command,cwd=ROOT,env=env,capture_output=True,text=True)
output=result.stdout+result.stderr
(OUT/'restored.log').write_text(output)
assert result.returncode==0,'Restored capability differential failed'
for line in output.splitlines():
    if line.startswith('test result:'):print(line,flush=True)
print('WS8bm upload discriminator: 2 compiled expiry/signature regressions rejected; source restored and differential passed',flush=True)
