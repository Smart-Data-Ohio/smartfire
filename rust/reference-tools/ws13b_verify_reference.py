#!/usr/bin/env python3
"""Verify the Rails implementation and test declarations inside the oracle image."""
import hashlib
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PATHS = [
    "app/models/huddle_grant.rb", "app/models/huddle_cleanup.rb", "app/models/membership.rb",
    "app/models/session.rb", "app/models/user.rb", "app/models/user/bannable.rb", "app/models/room.rb",
    "app/jobs/room/destroy_job.rb", "app/models/rooms/voice.rb", "test/models/huddle_revocation_test.rb",
    "app/models/rooms/stage.rb", "test/models/huddle_grant_test.rb",
    "app/models/huddle/join_notifier.rb", "app/models/huddle/join_pusher.rb",
    "app/models/huddle/invitation_resolver.rb", "app/models/activity_item.rb",
    "test/models/huddle_invitation_test.rb", "test/models/huddle/join_notifier_test.rb",
    "app/models/stream.rb", "app/models/message.rb", "app/models/message/searchable.rb",
    "test/models/stream_test.rb", "test/models/rooms/stage_test.rb", "test/models/rooms/voice_test.rb",
    "app/models/notifications/policy.rb", "app/models/huddle/ring_policy.rb",
    "test/models/huddle/join_pusher_test.rb", "test/models/huddle/ring_policy_test.rb",
    "app/jobs/huddle/join_notice_job.rb", "app/jobs/huddle/push_invitation_job.rb",
    "app/jobs/huddle/broadcast_presence_job.rb", "app/services/huddle/reconciler.rb",
    "test/jobs/huddle/join_notice_job_test.rb", "test/jobs/huddle/push_invitation_job_test.rb",
    "test/jobs/huddle/broadcast_presence_job_test.rb", "test/services/huddle/reconciler_test.rb",
    "app/views/rooms/huddles/_participants.html.erb", "app/views/rooms/stage/_live_badge.html.erb",
    "app/views/rooms/stage/_live_dot.html.erb", "app/views/rooms/stage/_panel_body.html.erb",
    "app/views/rooms/stage/_roster.html.erb", "app/views/rooms/stage/_controls.html.erb",
    "app/views/rooms/stage/_stream_event.html.erb", "app/views/rooms/stage/_role_event.html.erb",
    "app/views/users/_mention.html.erb", "app/helpers/users_helper.rb",
    "app/services/activity_items/recorder.rb", "test/test_helpers/mention_test_helper.rb",
    "app/models/user/status_settings.rb", "app/models/calendar/meeting_cache.rb",
    "app/models/dnd_allowed_user.rb",
    "app/controllers/internal/huddle_controller.rb",
]
image = os.environ.get("PARITY_IMAGE", "ws13-reference:d7c7de92")
raw = subprocess.check_output(["docker", "run", "--rm", "--name", "ws13b-source-check", "--network", "none", "--entrypoint", "sha256sum", image, *[f"/rails/{path}" for path in PATHS]], text=True, cwd=ROOT)
for line in raw.splitlines():
    digest, path = line.split()
    local = path.removeprefix("/rails/")
    expected = hashlib.sha256(subprocess.check_output(["git", "show", f"d7c7de92:{local}"], cwd=ROOT)).hexdigest()
    assert digest == expected, local
print(f"WS13b reference identity: {len(PATHS)} files match d7c7de92")
