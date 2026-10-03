#!/usr/bin/env python3
"""Temporary producer mutations for PR 215; restore before committing/building gates.

Install once, compile test binaries, then select a mutant with WS12_PR215_MUTATION.
This changes producers only. Expected vectors and assertions are never mutated.
"""
from pathlib import Path
import argparse
ROOT=Path(__file__).resolve().parents[3]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('action',choices=['install','restore'])
args=parser.parse_args()
backup=ROOT/'.scratch/pr215-fixes/mutation-backups'
files=['rust/crates/campfire/src/controllers/presenters/activity.rs','rust/crates/views/src/activity.rs','rust/crates/db/src/models/agent_work.rs','rust/crates/db/src/models/activity_item/recorder.rs']
if args.action=='restore':
    assert all('WS12_PR215_MUTATION' in (ROOT/name).read_text() for name in files), 'refuse restoring old backups over clean or newly merged production inputs'
    for i,name in enumerate(files): (ROOT/name).write_bytes((backup/str(i)).read_bytes())
    print('WS12_PR215_MUTATIONS restored all 4 production inputs')
    raise SystemExit
assert not backup.exists(),'restore/remove old backup before another install'
backup.mkdir(parents=True)
for i,name in enumerate(files): (backup/str(i)).write_bytes((ROOT/name).read_bytes())
def gate(name): return f'std::env::var("WS12_PR215_MUTATION").as_deref() == Ok("{name}")'
p=ROOT/files[0];s=p.read_text()
for kind,expression in [('missed','format!("You missed a huddle from {caller}")'),('started','format!("{caller} started a huddle")')]:
    assert s.count(expression)==2
    s=s.replace(expression,f'if {gate(kind)} {{ "Wrong huddle copy".into() }} else {{ {expression} }}')
assert '"pr_review_request" => "Review requested"' in s
s=s.replace('"pr_review_request" => "Review requested"',f'"pr_review_request" => if {gate("review_label")} {{"Wrong review label"}} else {{"Review requested"}}')
assert '.unwrap_or("None")' in s
s=s.replace('.unwrap_or("None")',f'.unwrap_or(if {gate("none_status")} {{"Wrong status"}} else {{"None"}})')
p.write_text(s)
p=ROOT/files[1];s=p.read_text();assert s.count('self.type_filter == value')==1
s=s.replace('self.type_filter == value',f'!({gate("html_type")} && value == "events") && self.type_filter == value')
needle='            items: self.items,'
assert needle in s
s=s.replace(needle,f'            items: if {gate("html_rows")} && self.type_filter == "events" {{ &[] }} else {{ self.items }},',1);p.write_text(s)
p=ROOT/files[2];s=p.read_text();needle='    use rusqlite::types::Value as Bind;';assert s.count(needle)==1
s=s.replace(needle,f'    if {gate("drop_done")} && status == "done" {{ return Ok(Outcome::ok(Vec::new(),200)); }}\n'+needle)
needle='    sql.push_str(" ORDER BY last_activity_at DESC,id DESC LIMIT 100");';assert s.count(needle)==1
s=s.replace(needle,f'''    sql.push_str(if {gate("board_cap")} {{ " ORDER BY last_activity_at DESC,id DESC LIMIT 99" }}
        else if {gate("board_order")} {{ " ORDER BY id DESC LIMIT 100" }}
        else {{ " ORDER BY last_activity_at DESC,id DESC LIMIT 100" }});''')
needle='visible.len() == LIST_MAX_LIMIT';assert s.count(needle)==1
s=s.replace(needle,f'visible.len() == if {gate("work_cap")} {{ 99 }} else {{ LIST_MAX_LIMIT }}');p.write_text(s)
p=ROOT/files[3];s=p.read_text();a=s.index('    /// WorkThreadEvent fanout');b=s.index('    /// The newly claimed',a)
part=s[a:b];assert part.count('            None,')==1
s=s[:a]+part.replace('            None,',f'            if {gate("stale_recipient")} {{ Some(user) }} else {{ None }},')+s[b:];p.write_text(s)
print('WS12_PR215_MUTATIONS installed 11 temporary producer mutants in 4 files; vectors/assertions unchanged')
