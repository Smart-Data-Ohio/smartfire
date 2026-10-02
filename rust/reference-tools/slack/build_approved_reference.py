#!/usr/bin/env python3
"""Build the pinned Rails oracle with exactly the approved 2e20b24c layout inputs."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
scratch = root / '.scratch/ws16-approved-reference'
scratch.mkdir(parents=True, exist_ok=True)
paths = ['app/views/layouts/application.html.erb', 'app/assets/stylesheets/people.css',
         'app/javascript/controllers/profile_card_controller.js']
lines = ['FROM ws16-reference:d7c7de92', 'USER root']
for index, path in enumerate(paths):
    (scratch / f'input{index}').write_bytes(subprocess.check_output(
        ['git', '-C', str(root), 'show', '2e20b24c:' + path]))
    lines.append(f'COPY --chown=rails:rails input{index} /rails/{path}')
lines += ['USER rails', 'RUN SECRET_KEY_BASE_DUMMY=1 SKIP_TELEMETRY=1 bin/rails assets:clobber assets:precompile']
(scratch / 'Dockerfile').write_text('\n'.join(lines) + '\n')
subprocess.run(['docker', 'build', '--network', 'none', '-t',
               'ws16-reference:d7c7de92-layout-2e20b24c', str(scratch)], check=True)
print('Slack Rails HTTP oracle: pinned d7c7de92 plus exactly three approved 2e20b24c layout inputs')
