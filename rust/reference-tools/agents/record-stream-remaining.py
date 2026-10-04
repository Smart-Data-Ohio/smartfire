#!/usr/bin/env python3

from pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json,os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parents[3]; output=Path(sys.argv[1]).resolve();output.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,PARITY_IMAGE=PIN_IMAGE);env.setdefault('PARITY_NAMESPACE','ws11api-next-2-streams')
results={}
for kind in ['start','finalize','append','trailing']:
    with (output/(kind+'.log')).open('w') as stderr:
        r=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','bin/rails','runner','/work/reference-tools/agents/stream_remaining_cases.rb',kind],cwd=root,env=env,stdout=subprocess.PIPE,stderr=stderr,text=True)
    assert r.returncode==0,(kind,r.returncode)
    results[kind]=json.loads(r.stdout)
(output/'agents_stream_remaining.json').write_text(json.dumps({'reference_pin':PIN,'results':results},ensure_ascii=False,indent=2)+'\n')
print('WS11 remaining streaming oracle: 4 cases; full rendered frames, jobs and state; 0 masks')
