#!/usr/bin/env python3
"""Compare the vendored engine with the exact official source revision in pinned Rails."""
import hashlib, json, subprocess, sys
from pathlib import Path
root = Path(__file__).resolve().parents[2]
source = Path(sys.argv[1]).resolve()
vendor = root / 'rust/crates/rails_compat/vendor/onigmo'
metadata = json.loads((vendor / 'UPSTREAM.json').read_text())
revision = subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip()
assert revision == metadata['revision']
for row in metadata['files']:
    upstream = (source / row['upstream']).read_bytes()
    actual = (vendor / row['file']).read_bytes()
    assert hashlib.sha256(upstream).hexdigest() == row['upstream_sha256']
    expected = upstream.replace(b'rb_st_', b'onig_st_') if row['file'] == 'st.h' else upstream
    assert actual == expected, row['file']
    assert hashlib.sha256(actual).hexdigest() == row['sha256']
print(f"Pinned Ruby engine: {len(metadata['files'])-1} byte-identical files; 1 hash-table symbol adaptation; revision {revision}; no matching changes")
