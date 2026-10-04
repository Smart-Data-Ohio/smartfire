#!/usr/bin/env python3
"""Verify source against the fixed pin and explicitly adopted #162/#163 blobs."""
import hashlib
from pathlib import Path
import subprocess
import sys
import os

root = Path(__file__).resolve().parents[2]
pin = (root / "rust/parity/reference.sha").read_text().strip()
assert sys.argv[1:] in ([], ["--board-tag"], ["--status-popup"])
status_popup = sys.argv[1:] == ["--status-popup"]
board_tag = bool(sys.argv[1:])
image = os.environ.get("PARITY_IMAGE", "campfire-reference")
status_files = [
    "app/assets/stylesheets/people.css", "app/controllers/users/statuses_controller.rb",
    "app/javascript/controllers/profile_card_controller.js", "app/views/layouts/application.html.erb",
    "app/views/users/cards/show.html.erb", "app/views/users/profiles/_status.html.erb",
    "app/views/users/sidebars/show.html.erb", "app/views/users/statuses/_fields.html.erb",
    "app/views/users/statuses/edit.html.erb", "config/routes.rb",
]
files = [
    "app/views/users/show.html.erb", "app/views/rooms/show/_ooo_notices.html.erb", "app/controllers/rooms_controller.rb",
    "config/initializers/web_push.rb", "config/initializers/vapid.rb",
    "lib/web_push/notification.rb", "lib/web_push/pool.rb",
    "app/models/push/subscription.rb", "app/models/room/message_pusher.rb",
    "app/jobs/event/reminder_push_job.rb", "app/jobs/board_automations/nudge_push_job.rb",
    "app/jobs/huddle/push_invitation_job.rb", "app/jobs/huddle/join_notice_job.rb",
    "app/models/user/inbox_preferences.rb",
    "app/models/channel_thread/message_pusher.rb", "app/models/saved_item/reminder_pusher.rb",
    "app/models/event/reminder_pusher.rb", "app/models/board_automations/nudge_pusher.rb",
    "app/models/huddle/invitation_pusher.rb", "app/models/huddle/join_pusher.rb",
    "app/models/notifications/policy.rb", "app/models/user/status_settings.rb",
    "app/models/workspace_presence_lease.rb", "app/channels/workspace_presence_channel.rb",
    "app/models/calendar/meeting_cache.rb", "app/services/periodic/runner.rb",
    "app/controllers/users/presences_controller.rb",
    "app/controllers/users/push_subscriptions/test_notifications_controller.rb",
    "app/views/pwa/service_worker.js", "public/offline.html",
    "app/controllers/users/statuses_controller.rb", "app/controllers/users/dnd_allowances_controller.rb",
    "app/controllers/users/notification_settings_controller.rb", "app/models/dnd_allowed_user.rb",
    "app/models/keyword_alert.rb", "app/models/notifications/keyword_matcher.rb", "app/views/users/profiles/_status.html.erb",
    "app/views/users/profiles/_notifications.html.erb",
    "app/views/users/statuses/_badge.html.erb", "app/views/rooms/show/_ooo_notice_line.html.erb",
    "app/helpers/users/presence_helper.rb", "app/views/users/profiles/_two_factor.html.erb",
    "app/models/calendar/meeting_dispatcher.rb", "app/models/calendar/ooo_dispatcher.rb",
    "app/jobs/calendar/meeting_refresh_job.rb",
    "app/services/activity_items/recorder.rb", "app/models/activity_item.rb", "app/models/message.rb",
    "app/controllers/users/profiles_controller.rb", "app/helpers/users/profiles_helper.rb",
    "app/views/users/profiles/_appearance.html.erb", "app/controllers/users/push_subscriptions_controller.rb",
    "app/views/users/push_subscriptions/index.html.erb", "app/views/users/push_subscriptions/_push_subscription.html.erb",
]
if status_popup:
    files = list(dict.fromkeys(files + status_files + [
        "app/views/sessions/new.html.erb", "app/views/sessions/incompatible_browser.html.erb",
        "app/views/sessions/transfers/show.html.erb", "app/views/two_factor/setups/show.html.erb",
        "app/views/two_factor/challenges/show.html.erb", "app/views/two_factor/backup_codes/show.html.erb",
        "app/views/users/sessions/index.html.erb", "app/views/sudos/new.html.erb",
        "app/views/sudos/continue.html.erb", "app/views/layouts/public.html.erb",
    ]))
checks = subprocess.check_output([
    "docker", "run", "--rm", "--name", "ws17-verify-reference", "--entrypoint", "sha256sum",
    image, *["/rails/" + path for path in files],
], text=True).splitlines()
assert len(checks) == len(files)
for path, check in zip(files, checks, strict=True):
    source_pin = pin
    source = subprocess.check_output(["git", "show", f"{source_pin}:{path}"], cwd=root)
    assert hashlib.sha256(source).hexdigest() == check.split()[0], f"reference drift: {path}"
print(f"Rails source verified: {len(files)} files match {pin}; one plain reference image")
