#!/usr/bin/env python3
"""Owner lifecycle must refuse both credential and legacy-key HTTP requests."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[2]
p=root/'crates/db/src/models/agent_access.rs'
scratch=root.parent/'.scratch/owner-http-mutations'
scratch.mkdir(parents=True,exist_ok=True)
original=p.read_text()
baseline=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire','agent_owner_lifecycle_','--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch)),capture_output=True,text=True)
output=baseline.stdout+baseline.stderr
(scratch/'baseline.log').write_text(output)
summary=re.search(r'^test result: ok\..*$',output,re.M)
assert baseline.returncode==0 and summary,output[-2500:]
print('WS11-api owner baseline: '+summary.group(),flush=True)
mutations=[('credentials','WHERE id=? AND suspended_at IS NULL','WHERE id=?','agent_owner_lifecycle_credentials'),('bot_keys','if suspended_at.is_some()','if false && suspended_at.is_some()','agent_owner_lifecycle_bot_keys')]
for name,before,after,test in mutations:
    assert before in original,name
    try:
        p.write_text(original.replace(before,after,1))
        result=subprocess.run(['cargo','test','--locked','-j','4','-p','campfire',test,'--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch)),capture_output=True,text=True)
        output=result.stdout+result.stderr
        (scratch/(name+'.log')).write_text(output)
        summary=re.search(r'^test result: FAILED\..*$',output,re.M)
        assert result.returncode!=0 and summary and 'assertion' in output and 'error[E' not in output,output[-2500:]
        print('WS11-api owner '+name+' mutation: '+summary.group(),flush=True)
    finally:
        p.write_text(original)
print('WS11-api owner lifecycle mutations: 2 broken guards rejected; source restored')
