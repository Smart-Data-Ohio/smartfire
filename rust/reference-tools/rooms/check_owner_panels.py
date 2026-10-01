#!/usr/bin/env python3
"""Re-execute owner recorders against pinned Rails; never capture Rust as a golden."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch/owner-panels-reference'
scratch.mkdir(parents=True, exist_ok=True)
image = 'ws8br-reference-d7c7de92'
env = dict(os.environ, PARITY_NAMESPACE='ws8br-panels', PARITY_OWNER='ws8br', PARITY_IMAGE=image)
paths = ['app/views/rooms/show/_thread_panel.html.erb', 'app/views/work_threads/_guide.html.erb',
         'app/views/polls/_builder.html.erb', 'app/views/rooms/pins/_panel.html.erb',
         'app/views/rooms/pins/_count.html.erb', 'app/models/rooms/direct.rb']
actual = subprocess.check_output(['docker','run','--rm','--network','none','--name','ws8br-panel-source',
    '--entrypoint','sha256sum',image,*['/rails/'+p for p in paths]],text=True)
for line, path in zip(actual.splitlines(), paths, strict=True):
    wanted = hashlib.sha256(subprocess.check_output(['git','show','d7c7de92:'+path],cwd=root)).hexdigest()
    assert line.split()[0] == wanted, 'Rails source drift: '+path
print('Rails panel source: 6 original files match d7c7de92')
for name in ('panels','pins'):
    result = subprocess.run([str(root/'rust/parity/bin/reference'),'exec','--seed','default',
        '--time','2026-03-02T16:00:00Z','--freeze','bin/rails','runner','--skip-executor',
        f'/work/reference-tools/rooms/owner_{name}.rb'],cwd=root,env=env,capture_output=True,check=True)
    (scratch/f'{name}.stdout').write_bytes(result.stdout)
    (scratch/f'{name}.stderr').write_bytes(result.stderr)
    # Rails may log a collection digest warning before the recorder's one JSON document.
    # This parses the transport, preserving every captured HTML byte unchanged.
    output = result.stdout.decode()
    captured = json.loads(output[output.index('\n{')+1:] if not output.startswith('{') else output)
    fixture = json.loads((root/f'rust/crates/campfire/src/controllers/rooms/owner_{name}.json').read_text())
    assert captured == fixture, 'Rails owner corpus changed: '+name
print('Rails owner panel oracle: 30 composition cases and 4 pin panels reproduced; all captured bytes unchanged')
