#!/usr/bin/env python3
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch=root/'.scratch'
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
pinned=subprocess.check_output(['git','show','d7c7de92:test/models/notifications/policy_test.rb'],cwd=root)
assert (root/'rust/reference-tools/pinned/ws17-policy_test.rb').read_bytes()==pinned
print('pinned policy test declarations verified: byte-identical to d7c7de92')
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch/'named-policy-generated.json').open('w') as out,(scratch/'named-policy-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','--time','2026-09-23T12:00:00Z','--freeze','rust/reference-tools/ws17_named_policy.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'named-policy-generated.json').read_text());assert v['reference']=='d7c7de92'
(root/'rust/vectors/ws17_named_policy.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
count=len(v['rows']);assertions=sum(row['assertions'] for row in v['rows']);observations=sum(len(row['calls']) for row in v['rows'])
print(f"Rails named policy: {count} passed cases; {assertions} original Rails assertions; {observations} constructor observations")
