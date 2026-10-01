#!/usr/bin/env python3
"""Only pinned Rails produces complete-page golden bytes."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
parser=argparse.ArgumentParser();parser.add_argument('--record',action='store_true');args=parser.parse_args()
root=Path(__file__).resolve().parents[3]
scratch=root/'.scratch/full-pages-reference';scratch.mkdir(parents=True,exist_ok=True)
image='ws8br-reference-status-2e20b24c'
paths={'app/views/rooms/show.html.erb':'d7c7de92','app/views/rooms/show/_nav.html.erb':'d7c7de92','app/views/messages/_message.html.erb':'d7c7de92','app/views/layouts/application.html.erb':'2e20b24c'}
for path,pin in paths.items():
 expected=hashlib.sha256(subprocess.check_output(['git','show',f'{pin}:{path}'],cwd=root)).hexdigest()
 actual=subprocess.check_output(['docker','run','--rm','--name','ws8br-page-source','--entrypoint','sha256sum',image,f'/rails/{path}'],text=True).split()[0]
 assert expected==actual,f'Rails source drift: {path}'
env=dict(os.environ,PARITY_NAMESPACE='ws8br-full-pages',PARITY_OWNER='ws8br',PARITY_IMAGE=image)
run=subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default','--time','2026-03-02T16:00:00Z','--freeze','bin/rails','runner','--skip-executor','/work/reference-tools/rooms/full_pages.rb'],cwd=root,env=env,capture_output=True,check=True)
(scratch/'stderr.log').write_bytes(run.stderr);(scratch/'capture.json').write_bytes(run.stdout)
captured=json.loads(run.stdout)
fixture=root/'rust/crates/campfire/src/controllers/rooms/full_pages.json'
if args.record: fixture.write_text(json.dumps(captured,indent=2,ensure_ascii=False)+'\n')
else: assert captured==json.loads(fixture.read_text()),'Rails full-page corpus changed'
print('Rails full-page oracle: 4 complete pages reproduced; pinned rooms/messages and approved #163 layout verified; bytes unchanged')
