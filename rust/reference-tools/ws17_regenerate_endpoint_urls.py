#!/usr/bin/env python3
import json, os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
subprocess.run(['python3', 'rust/reference-tools/ws17_verify_reference.py'], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE='ws17', PARITY_OWNER='ws17', PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch / 'endpoint-urls-generated.json').open('w') as out, (scratch / 'endpoint-urls-generated.log').open('w') as err:
    subprocess.run(['rust/parity/bin/reference', 'runner', '--seed', 'default', 'rust/reference-tools/ws17_endpoint_urls.rb'], cwd=root, env=env, stdout=out, stderr=err, check=True)
v = json.loads((scratch / 'endpoint-urls-generated.json').read_text())
assert v['reference'] == 'd7c7de92' and v['uri'] == '1.1.1'
(root / 'rust/vectors/ws17_endpoint_urls.json').write_text(json.dumps(v, ensure_ascii=False) + '\n')
print(f"Rails endpoint URLs: {len(v['rows'])} real model validation and resolution cases; URI {v['uri']}")
