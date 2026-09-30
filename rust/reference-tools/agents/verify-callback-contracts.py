#!/usr/bin/env python3
"""Re-record the new Rails failure and named guard vectors without printing secrets."""
from pathlib import Path
import json,os,re,subprocess
root=Path(__file__).resolve().parents[3]
scratch=root/'.scratch';scratch.mkdir(exist_ok=True)
env=dict(os.environ,PARITY_NAMESPACE='ws11',PARITY_OWNER='ws11',PARITY_IMAGE='triage-reference-d7c7de92')
for name,vector in [('finalization_failure_contract','agents_finalization_failure_contract'),('private_guard_cases','agents_private_guard_cases')]:
 path=scratch/(name+'-verified.json')
 with path.open('wb') as out,(scratch/(name+'-verified.log')).open('wb') as err:
  subprocess.run(['rust/parity/bin/reference','runner','--seed','default',f'rust/reference-tools/agents/{name}.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
 assert path.read_bytes()==(root/f'rust/vectors/{vector}.json').read_bytes(),name
source=subprocess.check_output(['git','show','d7c7de92:test/lib/restricted_http/private_network_guard_test.rb'],cwd=root,text=True)
blocks=re.findall(r'  test "(.*?)" do\n(.*?)(?=\n  test |\n  private)',source,re.S)
inputs=json.loads((Path(__file__).parent/'private_guard_case_inputs.json').read_text())
assert len(blocks)==len(inputs)==28
for (name,body),row in zip(blocks,inputs):
 assert name==row['name']
 if row['kind']=='private_ip':
  addresses=[a or b for a,b in re.findall(r'assert_private_ip "([^"]*)"|private_ip\?\("([^"]*)"\)',body)]
  assert row['addresses']==addresses,name
print('WS11 callback Rails oracles: 2 byte-identical vectors; 28 guard case names and address inputs matched the pin')
