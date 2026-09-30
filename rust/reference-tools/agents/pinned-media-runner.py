#!/usr/bin/env python3
"""Cargo runner: execute storage vectors with the exact Rails media runtime.

CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER points here. Other test binaries
run natively; rustc and cargo still use the machine's configured build queues.
"""
from pathlib import Path
import os,sys
root=Path(__file__).resolve().parents[3]
binary=Path(sys.argv[1]).resolve()
if Path.cwd()==root/'rust/crates/storage' and binary.name.startswith('vectors-'):
 scratch=root/'.scratch';temporary=scratch/'pinned-media';temporary.mkdir(parents=True,exist_ok=True)
 print('WS11 media runner: storage vectors execute in triage-reference-d7c7de92',flush=True)
 args=['docker','run','--rm','--network','none','--name','ws11-fresh-pinned-media','--entrypoint',str(binary),'-e','CI=1','-e','TMPDIR='+str(temporary),'-v',str(root)+':'+str(root)+':ro','-v',str(scratch)+':'+str(scratch)+':rw','--workdir',str(Path.cwd()),'triage-reference-d7c7de92',*sys.argv[2:]]
 os.execvp('docker',args)
os.execv(str(binary),[str(binary),*sys.argv[2:]])
