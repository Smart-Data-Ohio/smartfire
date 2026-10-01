#!/usr/bin/env python3
"""Reject the received array casts and group selector with compiled assertions, then restore."""
import os
from pathlib import Path
import subprocess

root=Path(__file__).resolve().parents[2]
scratch=root.parent/'.scratch'
paths=[root/'crates/campfire/src/controllers/rooms.rs',root/'crates/campfire/src/controllers/rooms/directs.rs']
originals={path:path.read_bytes() for path in paths}
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_TARGET_DIR=str(root/'target'))
try:
    for path in paths:
        source=subprocess.check_output(['git','show',f'66cde771:{path.relative_to(root.parent)}'],cwd=root.parent)
        if path.name=='rooms.rs':source+=b'\n#[cfg(test)]\nmod direct_selection_tests;\n'
        path.write_bytes(source)
    for label,test in [
        ('Direct selection','controllers::rooms::direct_selection_tests::direct_selection_queries_match_rails_and_commit_notes_audits_and_flash'),
        ('Array predicate','controllers::rooms::coercions_tests::closed_grantees_cast_numbers_and_nested_arrays_without_flattening_hashes')
    ]:
        result=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j','4','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','--bin','campfire',test,'--','--test-threads=4'],cwd=root.parent,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'{label.lower().replace(" ","-")}-discrimination.log').write_text(result.stdout)
        summaries=[line for line in result.stdout.splitlines() if line.startswith('test result:')]
        assert result.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0],result.stdout
        print(summaries[0])
        print(f'{label} discrimination: compiled HTTP regression rejected 66cde771')
finally:
    for path,source in originals.items():path.write_bytes(source)
print('Direct selection discrimination: both sources restored')
