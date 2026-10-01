#!/usr/bin/env python3
"""Execute WS13 browser cases with PR #172 APIs, without merging its branch.

Run only from an isolated fresh clone under the WS13 scratch directory. The
five exact source files are restored even on failure; production signatures
and the WS13 branch remain unchanged. Pass a reviewed immutable domain SHA.
"""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--domain-sha',required=True);args=p.parse_args()
root=Path(__file__).resolve().parents[3]
assert '/rust-ws13/.scratch/' in str(root), 'use an isolated WS13 scratch checkout'
files=['rust/crates/db/src/models/'+name+'.rs' for name in ['activity_item','huddle_grant','huddle_invitations','huddle_notices']]+['rust/crates/campfire/src/jobs/huddle.rs']
source=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
domain=subprocess.check_output(['git','rev-parse',args.domain_sha+'^{commit}'],cwd=root,text=True).strip()
assert len(domain)==40
original={name:(root/name).read_bytes() for name in files}
for name in files:
    assert original[name]==subprocess.check_output(['git','show',f'HEAD:{name}'],cwd=root),name
try:
    for name in files:
        data=subprocess.check_output(['git','show',f'{domain}:{name}'],cwd=root)
        (root/name).write_bytes(data)
        print(f'WS13b API source: {name} {hashlib.sha256(data).hexdigest()}',flush=True)
    print(f'WS13 source: {source}; WS13b API: {domain}; no merge',flush=True)
    env=dict(os.environ,CI='1',CARGO_BUILD_JOBS='2',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',TMPDIR=str(root/'.scratch'),CABLE_TEST_PORT_RANGE='52300-52349',MAIL_TEST_PORT_RANGE='52350-52399')
    artifacts=[]
    proc=subprocess.Popen(['mise','exec','rust@1.98.1','--','cargo','rustc','--locked','--manifest-path','rust/Cargo.toml','-p','campfire','--tests','--message-format=json','--','--cfg','ws13b_domain_api'],cwd=root,env=env,stdout=subprocess.PIPE,text=True)
    for line in proc.stdout:
        value=json.loads(line)
        if value.get('reason')=='compiler-artifact' and value.get('executable') and value['target']['name']=='campfire' and value['profile']['test']:
            artifacts.append(value['executable'])
        if value.get('reason')=='compiler-message':print(value['message']['rendered'],end='',flush=True)
    assert proc.wait()==0,'domain API build failed'
    assert len(artifacts)==1,artifacts
    image_hash=hashlib.sha256((root/'rust/parity/Dockerfile.playwright').read_bytes()+(root/'rust/parity/package-lock.json').read_bytes()).hexdigest()[:12]
    env['WS13_PLAYWRIGHT_IMAGE']='ws13-parity-playwright:'+image_hash
    subprocess.run([artifacts[0],'huddle_system_cases_in_real_browser','--ignored','--nocapture','--test-threads=8'],cwd=root,env=env,check=True)
finally:
    for name,data in original.items():(root/name).write_bytes(data)
    subprocess.run(['git','diff','--exit-code'],cwd=root,check=True)
    print('WS13b API overlay restored; WS13 source clean',flush=True)
