#!/usr/bin/env python3
"""Run committed, pinned Rails controller files in isolated reference containers."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import os
import re
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
files = sys.argv[1:] or sorted(str(p.relative_to(root)) for p in (root / 'test/controllers/agents').rglob('*_test.rb')) + [
    'test/controllers/agents_controller_test.rb', 'test/controllers/concerns/agent_authentication_test.rb',
    'test/controllers/messages/by_bots_controller_test.rb', 'test/controllers/messages/boosts/by_bots_controller_test.rb',
]
scratch = root / '.scratch/rails-controllers'
scratch.mkdir(parents=True, exist_ok=True)

def run(file):
    pinned = subprocess.check_output(['git', 'show', 'd7c7de92:' + file], cwd=root)
    assert pinned == (root / file).read_bytes(), file
    name = file.removeprefix('test/controllers/').removesuffix('.rb').replace('/', '-')
    storage = scratch / name
    (storage / 'db').mkdir(parents=True, exist_ok=True)
    (storage / 'files').mkdir(exist_ok=True)
    # Keep the daemonized Redis child owned by Docker init, not Rails. Rails
    # parallel startup must not wait on that unrelated long-lived process.
    result = subprocess.run([
        'docker', 'run', '--rm', '--init', '--name', 'ws11api-test-' + name, '--network', 'none',
        '--user', f'{os.getuid()}:{os.getgid()}', '--env-file', str(root / 'rust/parity/.env.reference'),
        '-e', 'RAILS_ENV=test', '-e', 'PARITY_REDIS=1', '-e', 'BUNDLE_WITHOUT=development',
        '-v', f'{root / "test"}:/rails/test:ro', '-v', f'{storage / "db"}:/rails/storage/db',
        '-v', f'{storage / "files"}:/rails/storage/files', 'ws11api-reference:d7c7de92', 'bin/rails', 'test', file,
    ], capture_output=True, text=True)
    output = result.stdout + result.stderr
    (scratch / (name + '.log')).write_text(output)
    summaries = re.findall(r'^\d+ runs, \d+ assertions, \d+ failures, \d+ errors, \d+ skips$', output, re.M)
    print(f'{file}: {summaries[-1] if summaries else "NO TEST SUMMARY"}; exit {result.returncode}', flush=True)
    return result.returncode

with ThreadPoolExecutor(max_workers=2) as pool:
    results = list(pool.map(run, files))
assert all(code == 0 for code in results), 'inspect per-file logs for reference failures'
