#!/usr/bin/env python3
import json,os,subprocess,sys
from pathlib import Path
root=Path(__file__).resolve().parents[3];output=Path(sys.argv[1]).resolve();output.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,PARITY_IMAGE='ws11api-reference:d7c7de92');env.setdefault('PARITY_NAMESPACE','ws11api-next-2-models');results={}
for kind in ['create_bot','reset_key','hop_limit','human_root','delete_no_owner','budget_viewer','budget_handoff']:
    with (output/(kind+'.log')).open('w') as stderr:
        r=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','bin/rails','runner','/work/reference-tools/agents/next2_model_cases.rb',kind],cwd=root,env=env,stdout=subprocess.PIPE,stderr=stderr,text=True)
    assert r.returncode==0,(kind,r.returncode)
    results[kind]=json.loads(r.stdout)['result']
(output/'agents_next2_models.json').write_text(json.dumps({'reference_pin':'d7c7de92','results':results},ensure_ascii=False,indent=2)+'\n')
print('WS11 remaining model oracle: 7 cases; installed model, WS12 handoff and viewer paths; 0 masks')
