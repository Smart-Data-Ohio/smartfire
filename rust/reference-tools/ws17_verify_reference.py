#!/usr/bin/env python3
"""Verify owned Rails source in the reference image against the fixed Git pin."""
import hashlib
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
pin = "d7c7de92"
image = "triage-reference-d7c7de92:latest"
files = [
    "config/initializers/web_push.rb", "config/initializers/vapid.rb",
    "lib/web_push/notification.rb", "lib/web_push/pool.rb",
    "app/models/push/subscription.rb", "app/models/room/message_pusher.rb",
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
    "app/models/keyword_alert.rb", "app/views/users/profiles/_status.html.erb",
    "app/views/users/profiles/_notifications.html.erb",
]
checks = subprocess.check_output([
    "docker", "run", "--rm", "--name", "ws17-verify-reference", "--entrypoint", "sha256sum",
    image, *["/rails/" + path for path in files],
], text=True).splitlines()
assert len(checks) == len(files)
for path, check in zip(files, checks, strict=True):
    source = subprocess.check_output(["git", "show", f"{pin}:{path}"], cwd=root)
    assert hashlib.sha256(source).hexdigest() == check.split()[0], f"reference drift: {path}"
print(f"pinned Rails source verified: {len(files)} files match {pin}")
