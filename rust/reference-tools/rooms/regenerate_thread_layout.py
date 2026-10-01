#!/usr/bin/env python3
"""Capture unchanged thread pages with Rails' explicitly approved #163 layout.

Only the approved source overlay enters the pinned reference image. Capture uses
WS8bm's original Rails exporter; no rendered Rust response supplies expected bytes.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch/thread-layout10'
scratch.mkdir(parents=True, exist_ok=True)
approved = [
    'app/assets/stylesheets/people.css', 'app/controllers/users/statuses_controller.rb',
    'app/javascript/controllers/profile_card_controller.js', 'app/views/layouts/application.html.erb',
    'app/views/users/cards/show.html.erb', 'app/views/users/profiles/_status.html.erb',
    'app/views/users/sidebars/show.html.erb', 'app/views/users/statuses/_fields.html.erb',
    'app/views/users/statuses/edit.html.erb', 'config/routes.rb',
]
pinned = [
    'app/controllers/channel_threads_controller.rb', 'app/views/channel_threads/show.html.erb',
    'app/views/channel_threads/index.html.erb', 'app/views/messages/_message.html.erb',
    'app/models/channel_thread.rb', 'app/models/message.rb',
]
expected_hashes = {}
for path in approved + pinned:
    pin = '2e20b24c' if path in approved else 'd7c7de92'
    source = subprocess.check_output(['git', 'show', f'{pin}:{path}'], cwd=root)
    expected_hashes['/rails/' + path] = hashlib.sha256(source).hexdigest()
    if path in approved:
        target = scratch / 'overlay' / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(source)
base = os.environ.get('WS8BR_PINNED_REFERENCE_IMAGE', 'ws8br-reference-d7c7de92')
(scratch / 'Dockerfile').write_text(
    f'FROM {base}\nCOPY overlay/ /rails/\n'
    'RUN rm -rf public/assets && SECRET_KEY_BASE_DUMMY=1 bin/rails assets:precompile\n'
)
image = 'ws8br-reference-status-2e20b24c'
with (scratch / 'image.log').open('w') as output:
    subprocess.run(['docker', 'build', '--quiet', '-t', image, str(scratch)],
                   cwd=root, stdout=output, stderr=subprocess.STDOUT, check=True)
checks = subprocess.check_output([
    'docker', 'run', '--rm', '--name', 'ws8br-thread-layout-source-check',
    '--entrypoint', 'sha256sum', image, *expected_hashes,
], text=True).splitlines()
assert len(checks) == len(expected_hashes)
for check in checks:
    digest, path = check.split(maxsplit=1)
    assert digest == expected_hashes[path], f'Rails source drift: {path}'
print('Rails thread layout source: 10 approved #163 files at 2e20b24c; 6 thread/message files at d7c7de92', flush=True)
env = dict(os.environ, PARITY_NAMESPACE='ws8br-thread-layout10', PARITY_OWNER='ws8br', PARITY_IMAGE=image)
with tempfile.TemporaryDirectory(prefix='capture-', dir=scratch) as storage:
    shutil.copytree(root / 'rust/parity/.seed/default', storage, dirs_exist_ok=True)
    subprocess.run([
        'rust/parity/bin/reference', 'exec', '--storage', storage,
        '--time', '2026-03-02T16:00:00Z', '--freeze',
        'bin/rails', 'runner', '--skip-executor',
        '/work/reference-tools/messaging/thread-pages.rb', '/rails/storage/db/thread-pages.json',
    ], cwd=root, env=env, check=True)
    shutil.copyfile(Path(storage) / 'db/thread-pages.json', scratch / 'capture.json')
golden = root / 'rust/vectors/messaging/thread-pages.json'
before = json.loads(golden.read_text())
after = json.loads((scratch / 'capture.json').read_text())
assert len(before['rows']) == len(after['rows'])
changed = []
for old, new in zip(before['rows'], after['rows'], strict=True):
    assert {k: v for k, v in old.items() if k != 'full_body'} == {k: v for k, v in new.items() if k != 'full_body'}, old['name']
    if old['full_body'] != new['full_body']:
        changed.append(old['name'])
        # This diagnostic checks the narrow adopted layout change; the output is
        # the complete raw Rails capture, and the Rust test uses the shared asset
        # verifier to validate actual fingerprints and all surrounding bytes.
        difference = new['full_body'].replace('user-star:changed@window->member-panel#refreshPresence user-status:changed@window->member-panel#refreshPresence',
                                              'user-star:changed@window->member-panel#refreshPresence')
        for old_url, new_url in [('/assets/people-b8926caa.css', '/assets/people-8adb2aea.css'),
                                 ('/assets/controllers/profile_card_controller-25a0f42e.js', '/assets/controllers/profile_card_controller-ca4bd34b.js')]:
            difference = difference.replace(new_url, old_url)
        assert difference == old['full_body'], f'unexpected thread/layout change: {old["name"]}'
assert {k: v for k, v in before.items() if k not in ['rows', 'layout_reference']} == {k: v for k, v in after.items() if k not in ['rows', 'layout_reference']}
after['layout_reference'] = '2e20b24c (#163 application layout and assets only; thread/message source remains d7c7de92)'
golden.write_text(json.dumps(after, indent=2, ensure_ascii=False) + '\n')
print(f'Rails thread layout capture: {len(after["rows"])} original request cases; {len(changed)} complete layout renders updated; all other facts and response bodies unchanged')
