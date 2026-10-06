#!/usr/bin/env python3
"""Named security comparisons must reject compiled policy regressions."""
from pathlib import Path
import os,subprocess
root=Path(__file__).resolve().parents[3]
env=dict(os.environ,CI='1',TMPDIR=str(root/'.scratch'),CARGO_TARGET_DIR=str(root/'rust/target'),CABLE_TEST_PORT_RANGE='52200-52249',MAIL_TEST_PORT_RANGE='52200-52249',INTEGRATION_TEST_PORT_RANGE='52250-52299')
cases=[
('unknown-capability', 'rust/crates/db/src/models/agent_access.rs', 'if !CAPABILITIES.contains(&capability) {\n        return Ok(false);\n    }', 'if !CAPABILITIES.contains(&capability) {\n        return Ok(true);\n    }','campfire_db','ws11_agent_case_unknown_capabilities_are_denied'),
('nat64-local-use','rust/crates/campfire/src/integrations/net/guard.rs',' || in_v6(ip, NAT64_LOCAL_USE)',' || false','campfire','ws11_private_guard_case_private_ip_returns_true_for_the_whole_local_use_nat64_block_rfc8215'),
]
for name,file,before,after,package,test in cases:
 path=root/file;original=path.read_text();assert before in original,name
 log=root/'.scratch'/f'named-mutation-{name}.log'
 try:
  path.write_text(original.replace(before,after,1))
  with log.open('w') as out:
   result=subprocess.run(['cargo','test','--locked','-j4','--manifest-path','rust/Cargo.toml','-p',package,test,'--','--test-threads=4'],cwd=root,env=env,stdout=out,stderr=subprocess.STDOUT)
  assert result.returncode and 'test result: FAILED. 0 passed; 1 failed;' in log.read_text(),log
  print(f'WS11 named mutation: {name}; 1 test failed; original restored')
 finally:path.write_text(original)
