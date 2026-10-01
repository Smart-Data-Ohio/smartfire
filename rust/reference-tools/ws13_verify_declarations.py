#!/usr/bin/env python3
"""Retain the 548 Rails titles while scoring only WS13's own declarations."""
from pathlib import Path
import re

root=Path(__file__).resolve().parents[2]
s=(root/'rust/plans/ws13-deferred-tests.md').read_text()
rows=re.findall(r'^\| `(test/[^`]+)` \| (\d+) \| (\d+|WS13b) \| (\d+|WS13b) \|$',s,re.M)
assert len(rows)==33
assert sum(int(row[1]) for row in rows)==548
owned=[r for r in rows if r[2]!='WS13b']
domain=[r for r in rows if r[2]=='WS13b']
assert sum(int(r[1]) for r in owned)==332
assert sum(int(r[1]) for r in domain)==216
passed=sum(int(r[2]) for r in owned)
remaining=sum(int(r[3]) for r in owned)
assert f"| **WS13 total** | **332** | **{passed}** | **{remaining}** |" in s
for name,total,complete,open_count in rows:
    raw=(root/name).read_text()
    titles=re.findall(r"^\s*test [\"'](.*?)[\"'] do",raw,re.M)
    assert len(titles)==int(total),(name,len(titles),total)
    section=s.split('## '+name+'\n',1)[1].split('\n## ',1)[0]
    actual=re.findall(r'^- (?:\*\*Passed:\*\* )?(.*)$',section,re.M)
    assert sorted(titles)==sorted(actual),name
    if complete=='WS13b':
        assert open_count=='WS13b' and name.startswith(('test/models/','test/jobs/','test/services/'))
        assert section.count('- **Passed:** ')==0,name
    else:
        assert int(total)==int(complete)+int(open_count)
        assert section.count('- **Passed:** ')==int(complete),name
print(f'Rails declaration catalogue: 548 titles retained; WS13 {passed} passed / {remaining} open; WS13b 216 owned, unscored; 33 files; source titles match')
