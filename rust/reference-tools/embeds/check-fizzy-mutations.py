#!/usr/bin/env python3
"""Compile and exercise each planted Fizzy security defect, restoring source every time."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.setdefault('TMPDIR', str(root.parent / '.scratch'))
env.setdefault('CARGO_TARGET_DIR', str(root / '.scratch/target'))
mutations = [
    ('client.rs', 'id\n            .bytes()\n            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b\'_\' | b\'-\'))', 'true', 'ws15e_fizzy_path_traversal_ids_fail_before_any_dns'),
    ('client.rs', 'pinned_ip: Some(ip)', 'pinned_ip: None', 'ws15e_fizzy_client_requests_match_rails_and_pin_once'),
    ('accounts.rs', 'let token = crypto.encrypt(input.token);', 'let token = input.token.to_string();', 'ws15e_fizzy_account_encryption_validation_and_unusable_tokens'),
    ('accounts.rs', 'account.chars().all(char::is_whitespace)', 'false', 'ws15e_fizzy_account_encryption_validation_and_unusable_tokens'),
]
for filename, old, replacement, test in mutations:
    path = root / 'crates/campfire/src/integrations/fizzy' / filename
    source = path.read_text()
    assert source.count(old) == 1, (filename, old)
    try:
        path.write_text(source.replace(old, replacement))
        result = subprocess.run(['cargo', 'test', '-j', '4', '-p', 'campfire', test, '--', '--nocapture'], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        (root.parent / '.scratch' / f'fizzy-mutation-{test}-{filename}.log').write_text(result.stdout)
        assert result.returncode != 0 and 'test result: FAILED' in result.stdout, f'Undetected or uncompiled mutation: {test}\n{result.stdout[-2500:]}'
        print(f'{filename} {test}: ' + next(line for line in result.stdout.splitlines() if line.startswith('test result:')), flush=True)
    finally:
        path.write_text(source)
print(f'WS15e Fizzy mutation checks: {len(mutations)} detected, 0 survived')
