#!/usr/bin/env python3
"""Capture full state with lossless shared before snapshots and changed-table overlays."""
import json, os, subprocess, sys
from pathlib import Path
root=Path(__file__).resolve().parents[3]
out=Path(sys.argv[1]).resolve();out.mkdir(parents=True,exist_ok=True)
env=dict(os.environ,PARITY_IMAGE='ws11api-reference:d7c7de92')
env.setdefault('PARITY_NAMESPACE','ws11api-next3-shapes')
raw=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','bin/rails','runner','/work/reference-tools/agents/array_shapes_contract.rb'],cwd=root,env=env,check=True,stdout=subprocess.PIPE,text=True)
(out/'array-shapes-raw.json').write_text(raw.stdout)
inputs=json.loads(subprocess.check_output(['ruby','-rjson','-e','fixture,manifest=eval(File.read(ARGV.first),binding,ARGV.first); puts JSON.generate(fixture:fixture,manifest:manifest)',str(root/'rust/reference-tools/agents/array_shape_inputs.rb')],text=True))
reference=json.loads(raw.stdout)
baselines=[];cases=[]
for c in reference['cases']:
    before=c['before']
    if before not in baselines:baselines.append(before)
    steps=[]
    for s in c['steps']:
        fields={k:s[k] for k in ['attempt','status','response_body','response_headers','jobs','changed_tables']}
        fields['changed_rows']={t:rows for t,rows in s['state'].items() if rows!=before[t]}
        assert dict(before,**fields['changed_rows'])==s['state']
        steps.append(fields)
    cases.append(dict(name=c['name'],before=baselines.index(before),steps=steps))
gold=dict(reference_pin='d7c7de92',notes=['Every projected table and every column reconstructed from shared before rows and complete changed-table rows; no state/body/job masks. Selected stable headers remain literal.'],fixture=inputs['fixture'],manifest=inputs['manifest'],json_columns=reference['json_columns'],baselines=baselines,cases=cases)
(out/'agent_array_shapes.json').write_text(json.dumps(gold,ensure_ascii=False,indent=2)+'\n')
print(f'WS11 array-shape oracle: {len(cases)} cases; {sum(len(c["steps"]) for c in cases)} responses; all state columns; lossless shared snapshots')
