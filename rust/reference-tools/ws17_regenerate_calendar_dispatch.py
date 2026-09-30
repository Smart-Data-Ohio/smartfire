#!/usr/bin/env python3
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch=root/'.scratch'
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch/'calendar-dispatch-generated.json').open('w') as out,(scratch/'calendar-dispatch-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_calendar_dispatch.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'calendar-dispatch-generated.json').read_text());assert v['reference']=='d7c7de92'
(root/'rust/vectors/ws17_calendar_dispatch.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
print(f"Rails calendar dispatch: {len(v['rows'])} complete two-tick cases")
