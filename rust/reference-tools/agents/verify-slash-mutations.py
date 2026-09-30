#!/usr/bin/env python3
"""Discriminate command ownership, invocation access, flood cap and queue atomicity."""
from pathlib import Path
import os
import re
import subprocess

root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'/'ws11-slash-mutations'
scratch.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(root.parent/'.scratch'),CARGO_TARGET_DIR=str(root/'target'),INTEGRATION_TEST_PORT_RANGE='52250-52299',CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249')
mutations=[
    ('registration-owner','agent_slash_command.rs','c.agent_id!=agent_id','false','campfire_db','ws11_slash_command_registration'),
    ('unregister-owner','agent_slash_command.rs','c.agent_id==agent_id','true','campfire_db','ws11_slash_command_registration'),
    ('invocation-grant','agent_slash_command.rs','if !available {','if false {','campfire_db','ws11_slash_dispatch'),
    ('flood-cap','agent_slash_command.rs','count>=20','count>=21','campfire_db','ws11_slash_dispatch'),
    ('builtin-collision','agent_slash_command.rs','crate::slash_commands::lookup(&name).is_some()','false','campfire_db','ws11_slash_command_model'),
    ('invocation-queue','agent_delivery.rs','enqueue_delivered_webhook(tx,&event);','','campfire','ws11_slash_queue_failure'),
]
for name,filename,before,after,package,test in mutations:
    path=root/'crates/db/src/models'/filename
    original=path.read_text();pattern=r'\s*'.join(re.escape(c) for c in before if not c.isspace())
    if not re.search(pattern,original): raise RuntimeError(f'{name}: missing anchor')
    try:
        path.write_text(re.sub(pattern,lambda _:after,original))
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4','-p',package,test,'--','--nocapture'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'{name}.log').write_text(result.stdout)
        summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
        if result.returncode!=101 or not summary or 'error[E' in result.stdout: raise RuntimeError(f'{name}: no compiled assertion failure')
        print(f'{name}: {summary.group()}',flush=True)
    finally: path.write_text(original)
print(f'WS11 slash discrimination: {len(mutations)} compiled regressions detected; sources restored')
