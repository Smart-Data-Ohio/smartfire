#!/usr/bin/env python3
import json,os,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[2];scratch=root/'.scratch'
# The shared brief explicitly advances only the board tag to #162. Keep every other
# Rails source on the immutable pin; this derivative image changes exactly that file.
context=scratch/'board-tag-reference';context.mkdir(exist_ok=True)
source=subprocess.check_output(['git','show','a6f10a25:app/models/board_automations/nudge_pusher.rb'],cwd=root)
(context/'nudge_pusher.rb').write_bytes(source)
(context/'Dockerfile').write_text('FROM triage-reference-d7c7de92:latest\nCOPY nudge_pusher.rb /rails/app/models/board_automations/nudge_pusher.rb\n')
with (scratch/'board-tag-image.log').open('w') as output:
 subprocess.run(['docker','build','--quiet','-t','ws17-reference-board-a6f10a25:latest',str(context)],cwd=root,stdout=output,stderr=subprocess.STDOUT,check=True)
subprocess.run(['python3','rust/reference-tools/ws17_verify_reference.py','--board-tag'],cwd=root,check=True)
env=dict(os.environ,PARITY_NAMESPACE='ws17',PARITY_OWNER='ws17',PARITY_IMAGE='ws17-reference-board-a6f10a25:latest')
with (scratch/'notification-push-generated.json').open('w') as out,(scratch/'notification-push-generated.log').open('w') as err:
 subprocess.run(['rust/parity/bin/reference','runner','--seed','default','rust/reference-tools/ws17_notification_push.rb'],cwd=root,env=env,stdout=out,stderr=err,check=True)
v=json.loads((scratch/'notification-push-generated.json').read_text());assert v['reference']=='d7c7de92' and v['board_reference']=='a6f10a25'
(root/'rust/vectors/ws17_notification_push.json').write_text(json.dumps(v,ensure_ascii=False)+'\n')
print(f"Rails notification push: {len(v['rows'])} complete source/policy payload cases; board tag reference a6f10a25")
