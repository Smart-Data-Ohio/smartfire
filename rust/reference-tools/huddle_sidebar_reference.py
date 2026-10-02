#!/usr/bin/env python3
"""Build the frozen Rails oracle with only #163's sidebar and application-layout changes.

Tracked context makes fresh-clone regeneration independent of root Rails files
or an untracked overlay. No merge of main is needed.
"""
import hashlib,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2]
context=root/'rust/reference-tools/sidebar_reference'
expected='6fd40c08b6f437ecefac5ab906511093234da3327dcb72c4cad131365ee2f8e8'
assert hashlib.sha256((context/'app/views/users/sidebars/show.html.erb').read_bytes()).hexdigest()==expected
assert hashlib.sha256((context/'app/views/layouts/application.html.erb').read_bytes()).hexdigest()=='53008a50df6bf20f53013c146ef0584690cdd1061b5b748d5bf2035f3fd854c9'
subprocess.run(['docker','build','--label','parity.owner=ws13','-t','ws13-reference:sidebar-2e20b24c',str(context)],check=True,stdout=subprocess.DEVNULL)
print('Sidebar reference: tracked #163 source SHA256 verified; isolated image built')
