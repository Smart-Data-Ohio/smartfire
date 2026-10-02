#!/usr/bin/env python3
"""Require the HTTP owner/member tests to reject removal of Rails admin gates."""
import os
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[3]
source = root/'crates/campfire/src/controllers/accounts/bots/github_connections.rs'
original = source.read_text()
needle = '    concerns::ensure_can_administer(c)?;'
assert original.count(needle) == 2
try:
    source.write_text(original.replace(needle, '    // Injected regression: administrator gate removed.'))
    result = subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','--workspace','the_owner_without_admin_rights_can_neither_link_relink_nor_unlink','--','--test-threads=8'],cwd=root,env={**os.environ,'CI':'1','CARGO_BUILD_JOBS':'2'},capture_output=True,text=True)
    output = result.stdout+result.stderr
    assert result.returncode != 0 and 'test result: FAILED.' in output and 'right: 403' in output, output
    print(next(line for line in output.splitlines() if line.startswith('test result:')))
    print('GitHub administrator discrimination: gate removal rejected; source restored')
finally:
    source.write_text(original)
