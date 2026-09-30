#!/usr/bin/env python3
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch=root/'.scratch'
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch/'notification-push-generated.json').open('w') as out,(scratch/'notification-push-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_notification_push.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'notification-push-generated.json').read_text());assert v['reference']=='d7c7de92'
(root/'rust/vectors/ws17_notification_push.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
print(f"Rails notification push: {len(v['rows'])} complete source/policy payload cases")
