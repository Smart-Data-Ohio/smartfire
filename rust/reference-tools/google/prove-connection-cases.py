#!/usr/bin/env python3
"""Reject audience and reconnect-scope regressions through the connection router."""
import os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
scratch = root.parent / '.scratch/ws14g/mutations'
scratch.mkdir(parents=True, exist_ok=True)
def check(name, relative, before, after, test):
    path = root / relative
    source = path.read_text()
    assert before in source, name
    try:
        path.write_text(source.replace(before, after, 1))
        result = subprocess.run(['cargo', 'test', '--locked', '-j', '4', '-p', 'campfire', test, '--', '--nocapture'], cwd=root,
            env=dict(os.environ, CI='1', TMPDIR=str(scratch), CARGO_PROFILE_TEST_DEBUG='0', CARGO_PROFILE_DEV_DEBUG='0'),
            text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (scratch / f'{name}.log').write_text(result.stdout)
        lines = [line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode == 101 and 'assertion' in result.stdout and any('FAILED' in line for line in lines), result.stdout
        print('\n'.join(lines), flush=True)
        print(f'Google connection mutation {name}: rejected', flush=True)
    finally:
        path.write_text(source)
check('connection-token-audience', 'crates/campfire/src/integrations/google/api.rs',
    '|| v["aud"].as_str() != Some(&self.config.client_id)',
    '|| false', 'google_connection_failed_exchanges_and_invalid_id_tokens')
check('connection-reconnect-scopes', 'crates/db/src/models/google_account.rs',
    '.or_else(|| old.as_ref().and_then(|a| a.scopes.clone()))',
    '.or(None)', 'google_connection_scope_retention_and_reconnect')
print('Google connection discrimination: 2 mutations rejected', flush=True)
