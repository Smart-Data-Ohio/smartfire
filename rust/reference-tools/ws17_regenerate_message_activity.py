#!/usr/bin/env python3
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch=root/'.scratch'
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch/'message-activity-generated.json').open('w') as out,(scratch/'message-activity-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_message_activity.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'message-activity-generated.json').read_text());assert v['reference']=='d7c7de92'
(root/'rust/crates/db/src/tests/ws17_message_activity.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
print(f"Rails message activity: {len(v['rows'])} complete callback/candidate cases")
