#!/usr/bin/env python3
"""Check the oracle image's real sources against the frozen Rails pin."""

from status_pin_identity import PIN, PIN_FULL, PIN_IMAGE
import hashlib
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[2]
paths=['app/controllers/rooms/huddles_controller.rb','app/controllers/users/huddle_presence_controller.rb','app/controllers/rooms/voices_controller.rb','app/controllers/rooms/stages_controller.rb','app/controllers/rooms_controller.rb','app/models/icons.rb','app/views/rooms/voices/_form.html.erb','app/views/rooms/stages/_form.html.erb','app/views/rooms/closeds/_user.html.erb','app/views/users/sidebars/rooms/_voice.html.erb','app/views/users/sidebars/rooms/_stage.html.erb','app/views/layouts/_huddle.html.erb','app/views/layouts/_huddle_invitation.html.erb','app/views/layouts/_huddle_join_notice.html.erb','app/models/huddle_grant.rb','app/models/huddle/join_notifier.rb','app/models/huddle/join_pusher.rb','app/models/huddle/invitation_pusher.rb','app/models/user/inbox_preferences.rb','app/models/membership.rb','app/models/stream.rb','app/controllers/internal/huddle_controller.rb','app/models/activity_item.rb','app/models/huddle/invitation_resolver.rb','app/models/huddle/ring_policy.rb','app/models/rooms/stage.rb','app/models/rooms/voice.rb','app/controllers/rooms/call_moderation_controller.rb','app/controllers/rooms/stage/streams_controller.rb','app/controllers/rooms/stage/roles_controller.rb','app/controllers/rooms/stage/hands_controller.rb','config/initializers/content_security_policy.rb']
paths += ['app/views/rooms/show/_nav.html.erb', 'app/views/rooms/boards/_nav.html.erb', 'app/views/rooms/show/_header_identity.html.erb', 'app/views/rooms/show/_header_overflow.html.erb', 'app/views/rooms/pins/_panel.html.erb', 'app/views/rooms/pins/_count.html.erb', 'app/views/rooms/layouts/_edit.html.erb', 'app/views/rooms/layouts/_new.html.erb', 'app/views/rooms/github_subscriptions/_section.html.erb', 'app/views/rooms/inbound_email_addresses/_section.html.erb', 'config/icons.yml', 'app/helpers/rooms_helper.rb', 'app/helpers/rooms/involvements_helper.rb']
paths += ['app/views/users/sidebars/show.html.erb', 'app/controllers/users/sidebars_controller.rb', 'app/helpers/users/sidebar_helper.rb']
paths += ['test/controllers/rooms/stage/streams_controller_test.rb', 'test/controllers/rooms/stage_view_test.rb', 'test/controllers/rooms/huddles_controller_test.rb', 'test/controllers/internal/huddle_controller_test.rb']
paths += ['test/controllers/rooms/stages_controller_test.rb', 'test/controllers/rooms/voices_controller_test.rb']
paths += ['app/helpers/application_helper.rb', 'app/models/calendar/meeting_cache.rb', 'app/models/google_account.rb', 'app/models/google/picker.rb', 'app/models/search.rb', 'app/models/user/status_settings.rb']
paths += ['app/views/rooms/show/_composer.html.erb', 'app/views/rooms/show/_member_panel.html.erb', 'app/views/rooms/show/_thread_panel.html.erb', 'app/views/polls/_builder.html.erb', 'app/views/scheduled_messages/_composer_button.html.erb', 'app/views/shared/_multi_select_bar.html.erb', 'app/views/work_threads/_guide.html.erb']
paths += ['app/views/users/sidebars/rooms/_shared.html.erb', 'app/views/users/sidebars/rooms/_board.html.erb', 'app/views/users/sidebars/rooms/_direct.html.erb', 'app/views/users/sidebars/rooms/_direct_placeholder.html.erb', 'app/views/users/sidebars/_room_categories.html.erb', 'app/views/users/sidebars/_room_menu.html.erb', 'app/helpers/users_helper.rb']
paths += ['app/views/rooms/show.html.erb', 'app/helpers/messages_helper.rb', 'app/views/messages/_template.html.erb', 'app/views/messages/_drive_attachments.html.erb', 'app/views/messages/_thread_indicator.html.erb', 'app/views/messages/_unread_divider.html.erb', 'app/views/rooms/show/_invitation.html.erb', 'app/views/rooms/show/_ooo_notices.html.erb', 'app/views/rooms/show/_ooo_notice_line.html.erb', 'app/helpers/users/avatars_helper.rb', 'app/views/accounts/_invite.html.erb']
paths += ['app/controllers/rooms/involvements_controller.rb', 'app/models/rooms/direct.rb']
paths += ['app/views/messages/message_links/_card.html.erb','app/views/messages/message_links/_cards.html.erb','app/helpers/message_links_helper.rb','test/system/stage_test.rb','test/system/voice_channels_test.rb','test/system/huddles_test.rb','test/system/huddle_audio_test.rb','test/system/huddle_roster_test.rb','test/system/huddle_join_notices_test.rb','test/system/huddle_invitations_test.rb','test/system/huddle_presence_test.rb']
raw=subprocess.check_output(['docker','run','--rm','--name','ws13-source-check-current','--network','none','--entrypoint','sha256sum',PIN_IMAGE,*[f'/rails/{path}' for path in paths if not path.startswith('test/')]],text=True,cwd=root)
for line in raw.splitlines():
 digest,path=line.split();local=path.removeprefix('/rails/')
 expected=hashlib.sha256(subprocess.check_output(['git','show',f'{PIN_FULL}:{local}'],cwd=root)).hexdigest()
 assert digest==expected,local
 print(f'{local}: pin SHA256 matches reference image')
print(f'Reference identity: {len(paths)} files match {PIN}')

for path in paths:
 if path.startswith('test/'):
  assert (root/path).read_bytes() == subprocess.check_output(['git','show',f'{PIN_FULL}:{path}'],cwd=root),path
print(f'Named declaration sources match current pin {PIN_FULL}; no overlay image')
