#!/usr/bin/env python3
"""Render exact original and tools-only bridge assertion anchors for both sequences."""
import hashlib,json,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[3]
pin=(root/'rust/parity/reference.sha').read_text().strip()
script='rust/reference-tools/users/ledger_browser_lifecycle.mjs'
lines=(root/script).read_text().splitlines()
def oracle(file):return subprocess.check_output(['git','show',pin+':'+file],cwd=root,text=True)
def anchor(needle):
 n=next(i+1 for i,line in enumerate(lines) if needle in line)
 return {'path':f'{script}:{n}','anchor':lines[n-1].strip()}
records=[]
for id,file,name,case,sites in [
 ('muted-room-sequence','test/system/sidebar_organize_test.rb','muted rooms dim and stay quiet until mentioned','muted-delivery',{70:"await selector(file,70",77:"console.log(`ORIGINAL_ASSERTION ${file}:77"}),
 ('calendar-browser-sequence','test/system/meeting_status_test.rb','opting in shows In a meeting for a stubbed busy interval, then clears after it ends','calendar-lifecycle',{24:'await selector(file,24',30:'await selector(file,30'})]:
 src=oracle(file);source=src.splitlines();decl=next(i+1 for i,line in enumerate(source) if line==f'  test "{name}" do')
 r={'id':id,'file':file,'line':decl,'test':name,'source_sha256':hashlib.sha256(src.encode()).hexdigest(),'record_status':'closed','rust_tests':['controllers::ledger_browser_tests::original_ledger_lifecycle_assertions'],'browser_case':case,'browser_mode':'lifecycle','assertions':[],'helper_expansion':[],'setup_expansion':[]}
 for n,needle in sites.items():
  a=anchor(needle);r['assertions'].append({'line':n,'ruby':source[n-1].strip(),'assertion_source':a['path'],'assertion_anchor':a['anchor'],'additional_assertion_sources':[anchor('function equal(')]})
 h='test/test_helpers/system_test_helper.rb';hs=oracle(h).splitlines()
 r['setup_expansion'].append({'ruby_file':h,'line':65,'ruby':hs[64].strip(),'rust_assertion':anchor("await selector('test/test_helpers/system_test_helper.rb',65")})
 if case=='muted-delivery':
  # Both sign-ins invoke the original sidebar assertion independently.
  occurrences=[i+1 for i,line in enumerate(lines) if "await selector('test/test_helpers/system_test_helper.rb',65" in line]
  r['setup_expansion']=[{'ruby_file':h,'line':65,'ruby':hs[64].strip(),'rust_assertion':{'path':f'{script}:{n}','anchor':lines[n-1].strip()}} for n in occurrences]
  r['helper_expansion'].append({'line':148,'ruby':source[147].strip(),'rust_assertion':anchor('await selector(file,148')})
  for n,needle in [(127,"await selector('test/test_helpers/system_test_helper.rb',127"),(131,"await selector('test/test_helpers/system_test_helper.rb',131")]:
   r['helper_expansion'].append({'ruby_file':h,'line':n,'ruby':hs[n-1].strip(),'rust_assertion':anchor(needle)})
 else:
  r['helper_expansion'].append({'line':49,'ruby':source[48].strip(),'rust_assertion':anchor('equal(file,49,true,true)')})
  r['interaction_helper_sources']=[{'ruby_file':file,'line':46,'ruby':source[45].strip(),'rust_helper':anchor('while(database.prepare(')}]
 records.append(r)
manifest={'reference':pin,'records':records,'fixture_bridge':'Real Message::create + Broadcasts::message_create; Calendar::MeetingRefresh fetch/parse/cache + minute dispatcher; both render visits share the injected mutable clock. Rails remains unchanged outside tools-only injected clock/fixture configuration.'}
(root/'rust/plans/ledger-ws8br-ws17-ws11ui-d-lifecycle-receipts.json').write_text(json.dumps(manifest,indent=2)+'\n')
