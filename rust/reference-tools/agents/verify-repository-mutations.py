#!/usr/bin/env python3
"""Show owner/privacy and production webhook wiring assertions rejecting regressions."""
from pathlib import Path
import os
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch' / 'ws11-repository-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI='1', TMPDIR=str(ROOT / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'), CABLE_TEST_PORT_RANGE='52200-52249', MAIL_TEST_PORT_RANGE='52200-52249', INTEGRATION_TEST_PORT_RANGE='52250-52299')
MUTATIONS = [
    ('owner-identity', 'agent_repositories.rs', 'and_then(|a| a.owner_id)', 'map(|a| a.user_id)', 'ws11_repository_owner'),
    ('disconnect', 'agent_repositories.rs', 'if reason.as_deref().is_some_and(|s| !campfire_richtext::ruby::is_blank(s))', 'if false', 'ws11_repository_disconnected'),
    ('production-access', 'agent_jobs.rs', '&access,', '&Default::default(),', 'ws11_repository_webhook'),
]
for name, filename, before, after, test in MUTATIONS:
    path = ROOT / 'rust/crates/campfire/src/integrations' / filename
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    changed, count = re.subn(pattern, lambda _: after, original, count=1)
    assert count == 1, name
    try:
        path.write_text(changed)
        output = subprocess.run(['cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire', test, '--', '--nocapture'], cwd=ROOT, env=ENV, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (SCRATCH / f'{name}.log').write_text(output.stdout)
        assert output.returncode != 0 and 'test result: FAILED.' in output.stdout and 'could not compile' not in output.stdout, name
        print(name + ': ' + next(line for line in output.stdout.splitlines() if line.startswith('test result: FAILED.')), flush=True)
    finally:
        path.write_text(original)
print('WS11 repository discrimination: 3 compiled regressions detected; sources restored')
