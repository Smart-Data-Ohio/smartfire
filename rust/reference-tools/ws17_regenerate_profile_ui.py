#!/usr/bin/env python3
import json, os, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
scratch=root/'.scratch'; scratch.mkdir(exist_ok=True)
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch/'profile-ui-generated.json').open('w') as out,(scratch/'profile-ui-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_profile_ui.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'profile-ui-generated.json').read_text()); assert v['reference']=='d7c7de92'
(root/'rust/crates/views/tests/golden/ws17-profile-ui.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
(root/'rust/crates/views/src/users/profile_zones.json').write_text(json.dumps({k:v[k] for k in ['choices','choice_zones','mapping']},ensure_ascii=False)+'\n')
print(f"Rails profile UI: {len(v['rows'])} complete appearance forms; 1 complete subscription content; {len(v['choices'])} zone choices")
