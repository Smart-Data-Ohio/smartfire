#!/usr/bin/env python3
"""Run owned controller files from pinned Rails; counts do not claim Rust coverage."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
parser=argparse.ArgumentParser();parser.add_argument('--inject-source-drift',action='store_true');args=parser.parse_args()
root=Path(__file__).resolve().parents[3]
scratch=root/'.scratch/room-controller-reference';scratch.mkdir(parents=True,exist_ok=True)
files=['rooms_controller','rooms/opens_controller','rooms/closeds_controller','rooms/directs_controller','rooms/involvements_controller','rooms/refreshes_controller','rooms/reads_controller','rooms/members_controller','rooms/categories_controller','rooms/favorites_controller','rooms/inbound_email_addresses_controller','room_categories_controller','switchers_controller','users/sidebars_controller']
files=[f'test/controllers/{name}_test.rb' for name in files]
archive=subprocess.check_output(['git','archive','d7c7de92','test'],cwd=root)
with tarfile.open(fileobj=io.BytesIO(archive)) as bundle: bundle.extractall(scratch,filter='data')
hashes={file.replace('test/controllers/','app/controllers/').replace('_test.rb','.rb'):hashlib.sha256(subprocess.check_output(['git','show','d7c7de92:'+file.replace('test/controllers/','app/controllers/').replace('_test.rb','.rb')],cwd=root)).hexdigest() for file in files}
if args.inject_source_drift: hashes[next(iter(hashes))]='0'*64
(scratch/'controller-hashes.json').write_text(json.dumps(hashes))
# Run each file separately so failure status cannot be overwritten by a later file.
script='redis-server --daemonize yes\nbin/rails db:prepare >/dev/null\n'
script+='ruby -rjson -rdigest -e \'JSON.parse(File.read("/ws8br-controller-hashes.json")).each { |path,hash| raise "reference drift: #{path}" unless Digest::SHA256.file(path).hexdigest==hash }; puts "Rails controller sources: 14 verified at d7c7de92"\'\n'
for file in files: script+=f"bin/rails test '{file}'\n"
run=subprocess.run(['docker','run','--rm','--cpus','2','--name','ws8br-controller-reference','--entrypoint','sh','--env-file',str(root/'rust/parity/.env.reference'),'-e','RAILS_ENV=test','-e','PARALLEL_WORKERS=1','-e','RAILS_LOG_LEVEL=warn','-v',f"{scratch/'test'}:/rails/test:ro",'-v',f"{scratch/'controller-hashes.json'}:/ws8br-controller-hashes.json:ro",os.environ.get('PARITY_IMAGE','ws8br-reference-d7c7de92'),'-ec',script],cwd=root,capture_output=True,text=True)
(scratch/'run.log').write_text(run.stdout+run.stderr)
if args.inject_source_drift:
    assert run.returncode!=0 and 'reference drift: app/controllers/rooms_controller.rb' in run.stderr,run.stdout+run.stderr
    print('Rails controller source-pin injection: wrong hash rejected before tests')
    raise SystemExit(0)
assert run.returncode==0,f"Rails reference failed; inspect {scratch/'run.log'}"
summaries=re.findall(r'^[0-9]+ runs, .*assertions, 0 failures, 0 errors, 0 skips$',run.stdout,re.M)
assert len(summaries)==len(files),'Missing per-file pass counts'
for file,summary in zip(files,summaries): print(file);print(summary)
print(f'WS8br Rails controller reference: {len(files)} files passed; reference counts only',flush=True)
