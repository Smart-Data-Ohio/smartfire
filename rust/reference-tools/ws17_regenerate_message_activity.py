#!/usr/bin/env python3

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"))
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE=PIN_IMAGE)
with (scratch/'message-activity-generated.json').open('w') as out,(scratch/'message-activity-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_message_activity.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'message-activity-generated.json').read_text());assert v['reference']==PIN
(root/'rust/crates/db/src/tests/ws17_message_activity.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
print(f"Rails message activity: {len(v['rows'])} complete callback/candidate cases")
