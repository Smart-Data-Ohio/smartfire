#!/usr/bin/env python3
"""Check every retained WS13 Rails declaration against its source title."""
from pathlib import Path
import re

root=Path(__file__).resolve().parents[2]
s=(root/'rust/plans/ws13-deferred-tests.md').read_text()
rows=re.findall(r'^\| `(test/[^`]+)` \| (\d+) \| (\d+) \| (\d+) \|$',s,re.M)
assert len(rows)==33
assert sum(int(row[1]) for row in rows)==548
total_passed=sum(int(row[2]) for row in rows)
total_remaining=sum(int(row[3]) for row in rows)
assert f'| **Total** | **548** | **{total_passed}** | **{total_remaining}** |' in s
for name,total,passed,remaining in rows:
    assert int(total)==int(passed)+int(remaining)
    raw=(root/name).read_text()
    titles=re.findall(r"^\s*test [\"'](.*?)[\"'] do",raw,re.M)
    assert len(titles)==int(total),(name,len(titles),total)
    section=s.split('## '+name+'\n',1)[1].split('\n## ',1)[0]
    actual=re.findall(r'^- (?:\*\*Passed:\*\* )?(.*)$',section,re.M)
    assert sorted(titles)==sorted(actual),name
    assert section.count('- **Passed:** ')==int(passed),name
print(f'Rails declaration catalogue: 548 titles retained; {total_passed} passed; {total_remaining} partial/deferred; 33 files; source titles match')
