#!/usr/bin/env python3

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"));scratch.mkdir(exist_ok=True)
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
for name,folder in [('status_settings','user'),('keyword_matcher','notifications')]:
 expected=subprocess.check_output(['git','show',f'{PIN_FULL}:test/models/{folder}/{name}_test.rb'],cwd=root)
 assert (root/f'rust/reference-tools/pinned/ws17-{name}_test.rb').read_bytes()==expected
print(f'pinned reader/matcher declarations verified: 2 files byte-identical to {PIN}')
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE=PIN_IMAGE)
with (scratch/'named-readers-generated.json').open('w') as out,(scratch/'named-readers-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','--time','2026-09-23T12:00:00Z','--freeze','rust/reference-tools/ws17_named_readers.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'named-readers-generated.json').read_text()); assert v['reference']==PIN
(root/'rust/vectors/ws17_named_readers.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
for file in sorted({r['file'] for r in v['rows']}):
 rows=[r for r in v['rows'] if r['file']==file]
 print(f"Rails named readers: {file}: {len(rows)} passed cases; {sum(r['assertions'] for r in rows)} original Rails assertions")
