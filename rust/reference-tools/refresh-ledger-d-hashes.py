#!/usr/bin/env python3
"""Bind the D maps to their frozen implementation; assertion anchors stay separate."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
users = root / 'rust/reference-tools/users'
paths = set(users.glob('ledger_*.py')) | set(users.glob('ledger_*.mjs')) | set(users.glob('ledger_*.rb'))
paths |= set(users.glob('selenium-*')) | {users / 'selenium_displayed.mjs'}
paths |= {users / name for name in [
    'run_ledger_browser_assertions.py', 'browser_navigation.mjs',
    'browser_diagnostics.mjs', 'original_browser_network.mjs',
    'browser_port_leases.py', 'reference_runtime.py',
    'original-test-session-controller.rb',
    'original-test-session-controller.provenance.json',
]}
paths |= {root / name for name in [
    'rust/parity/seeds/ledger_originals.rb',
    'rust/crates/campfire/src/app.rs', 'rust/crates/campfire/src/jobs.rs',
    'rust/crates/campfire/src/mail.rs', 'rust/crates/campfire/src/concerns.rs',
    'rust/crates/campfire/src/concerns/sudo.rs',
    'rust/crates/campfire/src/concerns/two_factor.rs',
    'rust/crates/campfire/src/controllers/ledger_browser_tests.rs',
    'rust/crates/campfire/src/controllers/ledger_browser_tests/lifecycle_host.rs',
    'rust/crates/campfire/src/controllers/presenters.rs',
    'rust/crates/campfire/src/controllers/presenters/view_context.rs',
    'rust/crates/campfire/src/controllers/presenters/agent_payload.rs',
    'rust/crates/campfire/src/integrations/agent_repositories.rs',
    'rust/crates/views/src/helpers/request_forgery.rs',
    'rust/crates/kit/src/app.rs',
]}
digests = {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
           for p in sorted(paths) if p.is_file()}
assert len(digests) == len(paths), 'every named implementation input must exist'
for mode in ['navigation', 'members', 'surfaces', 'lifecycle']:
    path = root / f'rust/plans/ledger-ws8br-ws17-ws11ui-d-{mode}-receipts.json'
    manifest = json.loads(path.read_text())
    manifest['implementation_sha256'] = {**manifest.get('implementation_sha256', {}), **digests}
    for dependency in manifest.get('browser_dependencies', []):
        dependency['sha256'] = hashlib.sha256((root / dependency['path']).read_bytes()).hexdigest()
    manifest['fixture_foundation'] = 'rust/parity/seeds/ledger_originals.rb'
    manifest['fixture_foundation_sha256'] = digests[manifest['fixture_foundation']]
    path.write_text(json.dumps(manifest, indent=2) + '\n')
print(f'D implementation binding: {len(digests)} frozen dependencies in four maps')
