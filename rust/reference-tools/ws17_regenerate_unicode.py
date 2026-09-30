#!/usr/bin/env python3
import json, os, subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
scratch = root / '.scratch'
scratch.mkdir(exist_ok=True)
subprocess.run(['python3', 'rust/reference-tools/ws17_verify_reference.py'], cwd=root, check=True)
env = dict(os.environ, PARITY_NAMESPACE='ws17', PARITY_OWNER='ws17', PARITY_IMAGE='triage-reference-d7c7de92:latest')
with (scratch / 'unicode-generated.json').open('w') as out, (scratch / 'unicode-generated.log').open('w') as err:
    subprocess.run(['rust/parity/bin/reference', 'runner', '--seed', 'default', 'rust/reference-tools/ws17_unicode.rb'], cwd=root, env=env, stdout=out, stderr=err, check=True)
v = json.loads((scratch / 'unicode-generated.json').read_text())
assert v['reference'] == 'd7c7de92' and v['unicode'] == '15.0.0'
(root / 'rust/vectors/ws17_unicode.json').write_text(json.dumps(v, ensure_ascii=False) + '\n')
(root / 'rust/crates/rails_compat/src/unicode_tables.json').write_text(json.dumps({key: v[key] for key in ['reference', 'ruby', 'unicode', 'maps', 'alpha_ranges']}, ensure_ascii=False) + '\n')
print(f"Rails Unicode: {len(v['maps']['downcase'])} lowercase mappings; {len(v['maps']['fold'])} full folds; {len(v['word_ranges'])} word ranges; {len(v['matcher'])} matcher cases; {len(v['replacements'])} replacements; {len(v['validations'])} validations")
