#!/usr/bin/env python3
"""Compile each unadapted owner boundary; the Rails byte assertion must reject each."""
import os
from pathlib import Path
import subprocess

root=Path(__file__).resolve().parents[3]
paths=[root/'rust/crates/campfire/src/controllers/presenters'/name for name in ['room_list.rs','room_native.rs']]
originals={path:path.read_bytes() for path in paths}
scratch=root/'.scratch';scratch.mkdir(exist_ok=True)
env=dict(os.environ,CI='1',TMPDIR=str(scratch),CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_DEV_DEBUG='0',
         CABLE_TEST_PORT_RANGE='52100-52149',MAIL_TEST_PORT_RANGE='52100-52149')
changes=[('list',paths[1],'.map(|list|format!("\\n    \\n{list}"))',''),
         ('composer',paths[1],"let footer=format!(\"  {}\", inline.strip_prefix('\\n').unwrap_or(&inline));",'let footer=inline;'),
         ('template',paths[1],'let template=format!("{}\\n",campfire_views::channel_threads::PendingTemplate{ctx,user}.render()?);',
          'let template=campfire_views::channel_threads::PendingTemplate{ctx,user}.render()?;')]
try:
    for name,path,target,replacement in changes:
        source=path.read_text();assert target in source,name
        path.write_text(source.replace(target,replacement))
        run=subprocess.run(['mise','exec','rust@1.98.1','--','cargo','test','--locked','-j4',
            '--manifest-path','rust/Cargo.toml','-p','campfire','--bin','campfire',
            'native_component_capture_matches_rails_root_selection','--','--test-threads=4','--nocapture'],
            cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (scratch/f'native-boundary-{name}-discrimination.log').write_text(run.stdout)
        summaries=[line for line in run.stdout.splitlines() if line.startswith('test result:')]
        assert run.returncode==101 and len(summaries)==1 and '0 passed; 1 failed;' in summaries[0],run.stdout[-6000:]
        print(f'{name}: {summaries[0]}',flush=True)
        path.write_bytes(originals[path])
finally:
    for path,original in originals.items():path.write_bytes(original)
print('Native boundary discrimination: three independently compiled unadapted owner seams rejected; source restored')
