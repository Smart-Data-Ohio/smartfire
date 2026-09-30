#!/usr/bin/env python3
"""Show 1021's oracle accepting the pin's missing tag, and the current oracle rejecting it."""
import json, shutil, subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
scratch=root/'.scratch/review-oracle-payload'
scratch.mkdir(parents=True,exist_ok=True)
old=subprocess.check_output(['git','show','1021be6a:rust/reference-tools/ws17_notification_push.rb'],cwd=root)
(scratch/'reviewed.rb').write_bytes(old)
assert b'reverse_merge(tag:nil)' in old
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py'],cwd=root,check=True)
results=[]
for name,script in [('reviewed',scratch/'reviewed.rb'),('current',root/'rust/reference-tools/ws17_notification_push.rb')]:
    storage=scratch/name
    (storage/'db').mkdir(parents=True,exist_ok=True)
    shutil.copy2(root/'rust/parity/.seed/default/db/production.sqlite3',storage/'db/production.sqlite3')
    result=subprocess.run(['docker','run','--rm','--network','none','--name',f'ws17-oracle-{name}','--env-file',str(root/'rust/parity/.env.reference'),'-e','PARITY_REDIS=1','-v',f'{storage}/db:/rails/storage/db','-v',f'{root}/rust/parity/.seed/default/storage:/rails/storage/files:ro','-v',f'{script}:/oracle.rb:ro','triage-reference-d7c7de92:latest','bin/rails','runner','/oracle.rb'],cwd=root,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    (scratch/f'{name}.stdout').write_text(result.stdout)
    (scratch/f'{name}.stderr').write_text(result.stderr)
    results.append(result)
old,new=results
assert old.returncode==0,old.stderr
v=json.loads(next(line for line in old.stdout.splitlines() if line.startswith("{\"reference\":")))
row=next(row for row in v['rows'] if row['name']=='board_nudge')
assert row['deliveries'][0]['payload']['tag'] is None
assert new.returncode!=0 and 'missing keyword: :tag' in new.stderr,new.stderr
print('1021be6a oracle: FAIL property; accepted board payload without required tag (50 rows emitted)')
print('current oracle: PASS property; actual WebPush::Notification raises ArgumentError: missing keyword: :tag')
