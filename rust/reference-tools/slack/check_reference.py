#!/usr/bin/env python3
"""Confirm the plain reference image's Slack source matches the pin and main."""
import hashlib
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
pin = (root / 'rust/parity/reference.sha').read_text().strip()
paths = ['app/models/slack', 'app/models/slack_import', 'app/models/slack_import.rb',
         'app/models/slack_workspace.rb', 'app/models/slack_connection.rb',
         'app/jobs/slack_import', 'app/controllers/slack',
         'app/controllers/accounts/slack_imports_controller.rb',
         'app/controllers/accounts/slack_import_runs_controller.rb']
files = subprocess.check_output(['git', '-C', str(root), 'ls-tree', '-r', '--name-only', pin, '--', *paths], text=True).splitlines()
image = os.environ.get('PARITY_IMAGE', 'campfire-reference')
output = subprocess.check_output(['docker', 'run', '--rm', '--name', 'ws16-source-check', '--network', 'none', '--entrypoint', 'sha256sum', image, *['/rails/' + path for path in files]], text=True)
actual = {line.split()[1].removeprefix('/rails/'): line.split()[0] for line in output.splitlines()}
for path in files:
    expected = hashlib.sha256(subprocess.check_output(['git', '-C', str(root), 'show', pin + ':' + path])).hexdigest()
    if actual.get(path) != expected:
        raise RuntimeError('Slack oracle mismatch: ' + path)
drift = subprocess.check_output(['git', '-C', str(root), 'diff', '--name-only', pin, 'origin/main', '--', *paths], text=True).splitlines()
if drift:
    raise RuntimeError('Use origin/main for changed Slack files: ' + ', '.join(drift))
print(f'Slack Rails reference: {len(files)} source files match {pin}; no Slack drift on origin/main')
