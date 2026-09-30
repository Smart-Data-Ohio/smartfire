#!/usr/bin/env python3
"""A compiled bypass of the owner-linked reader must disclose private details and fail."""
from pathlib import Path
import os,re,subprocess
root=Path(__file__).resolve().parents[3];p=root/'rust/crates/campfire/src/integrations/agent_repositories.rs';original=p.read_text()
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_BUILD_JOBS='2',CARGO_INCREMENTAL='0',CARGO_PROFILE_DEV_DEBUG='line-tables-only',CARGO_PROFILE_TEST_DEBUG='line-tables-only',INTEGRATION_TEST_PORT_RANGE='52250-52298',WS15E_TEST_PORT_RANGE='52250-52298')
start=original.index('        Box::pin(async move {');end=original.index('\n        })',start)+len('\n        })')
try:
 p.write_text(original[:start]+'        Box::pin(async move {let _ = request; Ok(true)})'+original[end:])
 result=subprocess.run(['cargo','test','--locked','--manifest-path','rust/Cargo.toml','-p','campfire','ws11_live_repository_adapter','--','--test-threads=4'],cwd=root,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 (root/'.scratch/live-reader-mutation-test.log').write_text(result.stdout)
 summary=re.search(r'^test result: FAILED\..*$',result.stdout,re.M)
 assert result.returncode==101 and summary and 'could not compile' not in result.stdout
 print('WS11 live reader privacy bypass: '+summary.group(),flush=True)
finally:p.write_text(original)
print('WS11 live reader mutation: 1 compiled owner-access bypass caught; source restored')
