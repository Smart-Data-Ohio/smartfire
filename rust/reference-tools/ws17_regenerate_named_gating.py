#!/usr/bin/env python3

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch = Path(os.environ.get("PARITY_PIN_LOG_DIR", root / ".scratch"));scratch.mkdir(exist_ok=True)
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
for name,folder in [('push_gating','notifications'),('room_push','room')]:
 expected=subprocess.check_output(['git','show',PIN_FULL + ':'+ ('test/models/room/push_test.rb' if name=='room_push' else f'test/models/{folder}/{name}_test.rb')],cwd=root)
 assert (root/f'rust/reference-tools/pinned/ws17-{name}_test.rb').read_bytes()==expected
assert (root/'rust/reference-tools/pinned/ws17-mention_test_helper.rb').read_bytes()==subprocess.check_output(['git','show',PIN_FULL + ':test/test_helpers/mention_test_helper.rb'],cwd=root)
assert (root/'rust/reference-tools/pinned/ws17-dns_test_helper.rb').read_bytes()==subprocess.check_output(['git','show',PIN_FULL + ':test/test_helpers/dns_test_helper.rb'],cwd=root)
print(f'pinned gating declarations verified: 4 files byte-identical to {PIN}')
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE=PIN_IMAGE)
with (scratch/'named-gating-generated.json').open('w') as out,(scratch/'named-gating-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','--time','2026-09-23T12:00:00Z','--freeze','rust/reference-tools/ws17_named_gating.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'named-gating-generated.json').read_text()); assert v['reference']==PIN
(root/'rust/vectors/ws17_named_gating.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
for file in sorted({r['file'] for r in v['rows']}):
 rows=[r for r in v['rows'] if r['file']==file]
 print(f"Rails named gating: {file}: {len(rows)} passed cases; {sum(r['assertions'] for r in rows)} original Rails assertions")
