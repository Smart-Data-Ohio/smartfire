#!/usr/bin/env python3
"""Require compiled assertion failures for cleanup regressions, then restore sources."""
from pathlib import Path
import os
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / '.scratch' / 'ws11-removal-mutations'
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, CI='1', TMPDIR=str(ROOT / '.scratch'), CARGO_TARGET_DIR=str(ROOT / 'rust/target'))
MUTATIONS = [
    ('grant-revocation', 'user/removal.rs', 'AgentGrant::revoke_for_user(tx, self.id)?;', 'let _ = self.id;', 'ws11_hard_user_removal'),
    ('agent-cascade', 'user/removal.rs', 'tx.conn().execute("DELETE FROM agents WHERE user_id=?", [self.id])?;', 'if let Some(a) = crate::sql::query_one(tx.conn(), "SELECT id FROM agents WHERE user_id=?", [self.id], |r| r.get::<_,i64>(0))? { crate::Agent::find(tx.conn(),a)?.unwrap().destroy(tx)?; }', 'ws11_hard_user_removal'),
    ('pin-callback', 'user/removal.rs', 'MessagePin::find(tx.conn(), id)?.unpin(tx)?;', 'tx.conn().execute("DELETE FROM message_pins WHERE id=?", [id])?;', 'ws11_hard_user_removal'),
    ('savepoint-rollback', 'user/removal.rs', 'tx.savepoint(|tx| {', "( |tx: &mut Tx<'_>| {", 'ws11_hard_user_dependency_failure'),
]
for name, filename, before, after, test in MUTATIONS:
    path = ROOT / 'rust/crates/db/src/models' / filename
    original = path.read_text()
    pattern = r'\s*'.join(re.escape(char) for char in before if not char.isspace())
    start = 0
    changed, count = re.subn(pattern, lambda _: after, original[start:], count=1)
    changed = original[:start] + changed
    assert count == 1, name
    if name == 'savepoint-rollback':
        changed = changed.replace('        })\n    }\n}', '        })(tx)\n    }\n}', 1)
    try:
        path.write_text(changed)
        output = subprocess.run(['cargo', 'test', '--locked', '-j4', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire_db', test, '--', '--nocapture'], cwd=ROOT, env=ENV, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (SCRATCH / f'{name}.log').write_text(output.stdout)
        assert output.returncode != 0 and 'test result: FAILED.' in output.stdout and 'could not compile' not in output.stdout, name
        print(name + ': ' + next(line for line in output.stdout.splitlines() if line.startswith('test result: FAILED.')), flush=True)
    finally:
        path.write_text(original)
print('WS11 hard-removal discrimination: 4 compiled regressions detected; sources restored')
