#!/usr/bin/env python3
"""The scrub must not re-key any seed; labeled bot keys must authenticate by digest."""
import hashlib
import json
from pathlib import Path
import sqlite3

root = Path(__file__).resolve().parents[2]
seeds = sorted(path for path in (root / 'parity/seeds').glob('*.rb') if path.stem != 'build')
for source in seeds:
    path = root / 'parity/.seed' / source.stem
    assert (path / 'db/production.sqlite3').is_file(), source.stem
    conn = sqlite3.connect(path / 'db/production.sqlite3')
    assert conn.execute('SELECT COUNT(*) FROM users WHERE bot_token IS NOT NULL').fetchone()[0] == 0, source.stem
    labels = json.loads((path / 'labels.json').read_text())
    for name, key in labels.items():
        if name.startswith('bot_keys.'):
            user, secret = key.split('-', 1)
            digest = conn.execute('SELECT bot_token_digest FROM users WHERE id=?', (user,)).fetchone()[0]
            assert hashlib.sha256(secret.encode()).hexdigest() == digest, (source.stem, name)
    conn.close()
print(f'WS11 seeds: {len(seeds)} built; 0 plaintext tokens; every labeled bot key matches its digest')
