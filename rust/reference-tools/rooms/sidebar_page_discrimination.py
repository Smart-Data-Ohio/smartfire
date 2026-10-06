#!/usr/bin/env python3
"""Compile broken section state, collection partitions and recipient broadcasts, then restore."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'),CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
base=['cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml')]
def reject(name,path,old,new,package,test):
    original=path.read_text()
    assert original.count(old)==1, 'mutation must apply once'
    try:
        path.write_text(original.replace(old,new))
        result=subprocess.run(base+package+[test,'--','--nocapture'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'sidebar-page-{name}-discrimination.log').write_text(result.stdout)
        summaries=[s for s in result.stdout.splitlines() if s.startswith('test result:')]
        assert result.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0], result.stdout
        print(summaries[0])
    finally:
        path.write_text(original)
    print(f'Sidebar page discrimination: compiled {name} mutation rejected; source restored',flush=True)
reject('collapse',root/'crates/views/templates/users/sidebars/_room_categories.html','{% if category.collapsed %} hidden','{% if false %} hidden',['-p','campfire_views','--test','sidebar'],'complete_sidebar_pages_match_rails')
reject('favorite-partition',root/'crates/campfire/src/controllers/presenters/accounts.rs','filter(|(m,_)|!m.favorited())','filter(|(_,_)|true)',['-p','campfire','--bin','campfire'],'seeded_sidebar_collections_render_the_complete_rails_frame')
reject('recipient-membership',root/'crates/campfire/src/controllers/rooms/involvements.rs','render_membership_sidebar(c, &room, &membership, if previous == Some(Involvement::Invisible) { None } else { Some(membership.unread()) }).await?','crate::controllers::rooms::render_shared_room(c, &room).await?',['-p','campfire','--bin','campfire'],'involvement_callbacks_match_rails_recipient_rows')
