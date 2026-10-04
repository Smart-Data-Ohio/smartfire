#!/usr/bin/env python3

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json, os, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch")); scratch.mkdir(exist_ok=True)
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE=PIN_IMAGE)
with (scratch/'profile-ui-generated.json').open('w') as out,(scratch/'profile-ui-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_profile_ui.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'profile-ui-generated.json').read_text()); assert v['reference']==PIN
(root/'rust/crates/views/tests/golden/ws17-profile-ui.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
(root/'rust/crates/views/src/users/profile_zones.json').write_text(json.dumps({k:v[k] for k in ['choices','choice_zones','mapping']},ensure_ascii=False)+'\n')
print(f"Rails profile UI: {len(v['rows'])} complete appearance forms; {len(v['rows'])} raw metadata snapshots; 1 complete subscription content; {len(v['choices'])} zone choices")
