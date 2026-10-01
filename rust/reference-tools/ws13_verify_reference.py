#!/usr/bin/env python3
"""Check the oracle image's real sources against the frozen Rails pin."""
import hashlib
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
paths=['app/models/huddle_grant.rb','app/models/huddle/join_notifier.rb','app/models/huddle/join_pusher.rb','app/models/huddle/invitation_pusher.rb','app/models/user/inbox_preferences.rb','app/models/membership.rb','app/models/stream.rb','app/controllers/internal/huddle_controller.rb','app/models/activity_item.rb','app/models/huddle/invitation_resolver.rb','app/models/huddle/ring_policy.rb','app/models/rooms/stage.rb','app/models/rooms/voice.rb','app/controllers/rooms/call_moderation_controller.rb','app/controllers/rooms/stage/streams_controller.rb','app/controllers/rooms/stage/roles_controller.rb','app/controllers/rooms/stage/hands_controller.rb','config/initializers/content_security_policy.rb']
raw=subprocess.check_output(['docker','run','--rm','--name','ws13-source-check-current','--network','none','--entrypoint','sha256sum','ws13-reference:d7c7de92',*[f'/rails/{path}' for path in paths]],text=True,cwd=root)
for line in raw.splitlines():
 digest,path=line.split();local=path.removeprefix('/rails/')
 expected=hashlib.sha256(subprocess.check_output(['git','show',f'd7c7de92:{local}'],cwd=root)).hexdigest()
 assert digest==expected,local
 print(f'{local}: pin SHA256 matches reference image')
print(f'Reference identity: {len(paths)} files match d7c7de92')
