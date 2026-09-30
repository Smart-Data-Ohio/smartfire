#!/usr/bin/env python3
"""Own seeded servers; no pre-existing storage, binary copies or copied node_modules.

Build campfire, build both parity seeds, npm ci --prefix rust/parity, and install
Playwright chromium first. The target binary is CARGO_TARGET_DIR/debug/campfire.
"""
from pathlib import Path
import os
import sys
import shutil
import subprocess
import tempfile
import time
import urllib.request

root = Path(__file__).resolve().parents[3]
rust = root / 'rust'
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
target = Path(os.environ.get('CARGO_TARGET_DIR', rust / 'target'))
reference_port, candidate_port, target_port = 52160, 52161, 52162
reference = rust / 'parity/bin/reference'
env = os.environ.copy()
for line in (rust / 'parity/.env.reference').read_text().splitlines():
    if line and not line.startswith('#'):
        key, value = line.split('=', 1)
        env[key] = value
env.update({'HTTP_PORT': str(candidate_port), 'TARGET_PORT': str(target_port),
            'TARGET_BIND': '127.0.0.1', 'CAMPFIRE_FROZEN_TIME': '2026-03-02T16:00:00Z',
            'INBOUND_EMAIL_DOMAIN': 'mail.campfire.test', 'RAILS_INBOUND_EMAIL_PASSWORD': 'browser-fixture',
            'CAMPFIRE_LOG': 'error', 'PARITY_NAMESPACE': 'ws8br-browser', 'PARITY_OWNER': 'ws8br',
            'PARITY_IMAGE': 'ws8br-browser-reference-d7c7de92'})
def run_browser(script: str, args: tuple[str, ...] = ()):
    subprocess.run(['docker', 'build', '-f', str(rust / 'reference-tools/rooms/browser.Dockerfile'),
                    '-t', env['PARITY_IMAGE'], str(rust / 'parity/docker')], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix=script.removesuffix('.mjs') + '-', dir=scratch) as work:
        work = Path(work)
        shutil.copytree(rust / 'parity/.seed/default/db', work / 'db')
        shutil.copytree(rust / 'parity/.seed/default/storage', work / 'files')
        env['CAMPFIRE_STORAGE_PATH'] = str(work)
        process = None
        with (scratch / f'{script.removesuffix(".mjs")}-servers.log').open('w') as log:
            try:
                subprocess.run([str(reference), 'up', '--seed', 'default', '--port', str(reference_port),
                    '--time', '2026-03-02T16:00:00Z', '--freeze', '-e', 'INBOUND_EMAIL_DOMAIN=mail.campfire.test',
                    '-e', 'RAILS_INBOUND_EMAIL_PASSWORD=browser-fixture'], env=env, cwd=root, stdout=log, stderr=log, check=True)
                process = subprocess.Popen([str(target / 'debug/campfire'), 'server'], env=env, cwd=root, stdout=log, stderr=log)
                deadline = time.monotonic() + 120
                while True:
                    if process.poll() is not None:
                        raise RuntimeError('candidate stopped; see .scratch/inbound-browser-servers.log')
                    try:
                        with urllib.request.urlopen(f'http://127.0.0.1:{candidate_port}/up', timeout=2) as response:
                            if response.status == 200:
                                break
                    except OSError:
                        pass
                    if time.monotonic() > deadline:
                        raise TimeoutError('candidate not ready')
                    time.sleep(.2)
                subprocess.run(['node', str(rust / 'reference-tools/rooms' / script),
                    f'http://127.0.0.1:{reference_port}', f'http://127.0.0.1:{candidate_port}', *args], cwd=root, env=env, check=True)
            finally:
                if process is not None:
                    process.terminate()
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                subprocess.run([str(reference), 'down', '--port', str(reference_port)], cwd=root, env=env, stdout=log, stderr=log, check=True)

if __name__ == "__main__":
    run_browser("inbound_browser.mjs", tuple(sys.argv[1:]))
