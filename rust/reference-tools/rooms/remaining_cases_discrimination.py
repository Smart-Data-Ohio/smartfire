#!/usr/bin/env python3
"""Reject unsafe preview rendering and per-row HTTP queries, restoring sources in finally."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[3]
presenters=root/'rust/crates/campfire/src/controllers/presenters'
paths=[presenters.with_suffix('.rs'),presenters/'accounts.rs',presenters/'switcher.rs']
originals={path:path.read_bytes() for path in paths}
scratch=root/'.scratch';scratch.mkdir(exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_DEBUG='0',
         CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
base=['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j2','--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire','--']
def reject(name,filters,count):
    run=subprocess.run(base+filters+['--test-threads=4'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
    (scratch/f'remaining-{name}-discrimination.log').write_text(run.stdout)
    summaries=[line for line in run.stdout.splitlines() if line.startswith('test result:')]
    assert run.returncode==101 and len(summaries)==1 and f'0 passed; {count} failed;' in summaries[0],run.stdout[-6000:]
    print(summaries[0],flush=True)
try:
    path=paths[0];source=path.read_text()
    target='Ok(match campfire_richtext::present_message(&body, &ctx) {'
    assert target in source
    # The deliberate regression trusts an attachment's image URL directly and emits its
    # unsanitized body, rather than resolving it through the guarded embed presenter.
    unsafe="""Ok(match Presentation::Html({
        let image=regex::Regex::new(r#"url="([^"]+)""#).unwrap().captures(&body).map(|c|c[1].to_string()).unwrap_or_default();
        format!("{body}<img src=\\\"{image}\\\">")
    }) {"""
    path.write_text(source.replace(target,unsafe))
    reject('previews',['show_renders_a_link_preview','show_renders_an_unfurled_link_preview'],3)
    for path,data in originals.items():path.write_bytes(data)
    for path,variable in [(paths[1],'all'),(paths[2],'memberships')]:
        source=path.read_text()
        target=f'let {variable} = Membership::visible_with_ordered_room(conn, user.id)?;'
        assert target in source
        injection=f'\n    for (membership,_) in &{variable} {{ conn.query_row("SELECT ?1",[membership.room_id],|row|row.get::<_,i64>(0))?; }}'
        path.write_text(source.replace(target,target+injection))
    reject('queries',['show_costs_a_constant_number_of_queries_as_rooms_people_and_threads_grow','sidebar_query_count_does_not_grow_with_group_dms_named_or_not'],2)
finally:
    for path,data in originals.items():path.write_bytes(data)
print('Remaining case discrimination: unsafe previews and per-row HTTP queries rejected; source restored')
