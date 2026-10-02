#!/usr/bin/env python3
"""Count executed named comparisons; mapped names alone are not passing tests."""
from pathlib import Path
import json,re,sys,subprocess
root=Path(__file__).resolve().parents[3]
mapping=json.loads((Path(__file__).parent/'case-ports.json').read_text())
log=Path(sys.argv[1]).read_text()
status={name.rsplit('::',1)[-1]:outcome for name,outcome in re.findall(r'^test (\S+) \.\.\. (ok|FAILED|ignored)$',log,re.M)}
subprocess.run([sys.executable, str(Path(__file__).parent/"domain-case-inventory.py")], cwd=root, check=True)
inventory=json.loads((root/".scratch/ws11-domain-case-inventory.json").read_text())
files={g["path"]:(g["cases"],0) for g in inventory}
for group in mapping['files']:
 source=Path(root/group['rails_file']).read_text()
 total=len(re.findall(r'^\s*test "(.*?)" do',source,re.M))
 names=[case['rust'] for case in group['cases']]
 assert all(status.get(name)=='ok' for name in names),group['rails_file']
 count=files.get(group['rails_file'], (total,0))[1]+len(names)
 files[group['rails_file']]=(total,count)
for file,(total,count) in sorted(files.items(),key=lambda g:(-g[1][0],g[0])):
 print(f'WS11 named comparisons: {file}: {count} passed; 0 failed; {total-count} deferred')
print(f'WS11 named comparison totals: {sum(g[1] for g in files.values())} passed; 0 failed; {sum(g[0]-g[1] for g in files.values())} deferred')
