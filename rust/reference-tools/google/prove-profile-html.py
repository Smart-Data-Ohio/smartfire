#!/usr/bin/env python3
"""Reject a real whitespace regression in the complete profile panel fixture."""
from pathlib import Path
import os, subprocess
root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch/ws14g/mutations'
scratch.mkdir(parents=True,exist_ok=True)
path=root/'crates/views/templates/users/profiles/_google_calendar.html'
original=path.read_text()
try:
    path.write_text(original.replace('Connected as {{','Connected  as {{',1))
    result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','-p','campfire_views','--test','google','google_profile_panels_match_pinned_rails','--','--nocapture'],cwd=root,env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_DEBUG='0'),text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (scratch/'profile-html.log').write_text(result.stdout)
    lines=[line for line in result.stdout.splitlines() if line.startswith('test result:')]
    assert result.returncode==101 and 'assertion' in result.stdout and any('FAILED' in line for line in lines),result.stdout
    print('\n'.join(lines))
    print('Google profile HTML discrimination: 1 whitespace mutation rejected')
finally: path.write_text(original)
