#!/usr/bin/env python3
"""A compiling missing-authorization mutant must fail the named Rails equivalent case."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[3]
source = root / 'rust/crates/campfire/src/controllers/rooms/inbound_email_addresses.rs'
original = source.read_text()
needle = '    super::ensure_can_administer(c, &room)?;'
assert original.count(needle) == 1
env = os.environ.copy()
env.update({'CI':'1', 'TMPDIR':str(root / '.scratch'), 'CARGO_TARGET_DIR':str(root / 'rust/target')})
try:
    source.write_text(original.replace(needle, '    // injected missing authorization'))
    result = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4',
        '--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire',
        'controllers::rooms::inbound_rails_cases::a_plain_member_is_forbidden','--','--test-threads=1'],
        cwd=root, env=env, capture_output=True, text=True)
    output = result.stdout + result.stderr
    (root / '.scratch/inbound-authorization-mutant.log').write_text(output)
    assert result.returncode != 0 and 'test result: FAILED. 0 passed; 1 failed;' in output, output
    assert 'could not compile' not in output, output
    print(next(line for line in output.splitlines() if line.startswith('test result:')))
    print('Inbound authorization discrimination: compiled missing-admin check rejected; source restored')
finally:
    source.write_text(original)
