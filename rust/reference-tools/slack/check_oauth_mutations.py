#!/usr/bin/env python3
"""Reject grouped OAuth/view faults through real TLS and authenticated HTTP; restore sources."""
from pathlib import Path
import os
import subprocess
root = Path(__file__).resolve().parents[2]
paths = {
 'oauth': root/'crates/campfire/src/integrations/slack/oauth.rs',
 'grant': root/'crates/campfire/src/integrations/slack/connections.rs',
 'view': root/'crates/views/templates/accounts/slack_imports/show.html',
}
original = {key: path.read_text() for key,path in paths.items()}
faults = {
 'oauth': [('owner == user_id', 'true || owner == user_id'),
           ('Could not reach Slack ({class})', 'Could not reach Slack (Broken Class)')],
 'grant': [('.filter(|s| !scopes.contains(s))', '.filter(|_| false)')],
 'view': [('>Slack import</h1>', '>Broken Slack title</h1>')],
}
required = [
 'slack_oauth_manifest_urls_and_state_match_real_rails',
 'slack_oauth_transport_errors_never_include_details',
 'slack_oauth_exchange_revoke_and_team_info_match_rails_through_real_tls',
 'slack_connections_http_persistence_audits_and_requests_match_rails',
 'slack_setup_views_are_byte_identical_to_rails_and_write_only',
]
try:
 for key,changes in faults.items():
  mutated=original[key]
  for before,after in changes:
   if before not in mutated: raise RuntimeError('missing mutation anchor: '+before)
   mutated=mutated.replace(before,after)
  paths[key].write_text(mutated)
 result=subprocess.run(['cargo','test','--offline','--locked','--manifest-path',str(root/'Cargo.toml'),'-p','campfire','slack_','--','--test-threads=8'],env=dict(os.environ,CARGO_BUILD_JOBS='2',RUST_TEST_THREADS='8',INTEGRATION_TEST_PORT_RANGE='53300-53399'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 for name in required:
  line=next((line for line in result.stdout.splitlines() if line.startswith('test ') and name+' ... FAILED' in line),None)
  if line is None:
   print(result.stdout)
   raise RuntimeError('mutation did not fail its assertion: '+name)
  print(line)
 summary=next((line for line in result.stdout.splitlines() if line.startswith('test result:')),None)
 if result.returncode==0 or not summary: raise RuntimeError('expected assertion failures')
 print(summary)
finally:
 for key,path in paths.items(): path.write_text(original[key])
print('Slack OAuth mutation guards: wrong owner, transport class, scope grant, and view bytes rejected; source restored')
