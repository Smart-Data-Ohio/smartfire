#!/usr/bin/env python3
"""Check the Rails files this slice reads against the accepted source pin."""
import hashlib
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
FILES = [
    "app/controllers/messages_controller.rb",
    "app/controllers/channel_threads_controller.rb",
    "app/views/channel_threads/index.html.erb",
    "app/views/channel_threads/show.html.erb",
    "app/views/channel_threads/_conversation.html.erb",
    "app/views/rooms/show/_composer.html.erb",
    "app/views/messages/_template.html.erb",
    "app/helpers/rooms_helper.rb",
    "app/models/google_account.rb",
    "app/models/agent_slash_command.rb",
    "app/views/agent_steps/_thread_steps.html.erb",
    "app/models/work_thread_event.rb",
    "app/controllers/channel_thread_messages_controller.rb",
    "app/controllers/message_forwards_controller.rb",
    "app/controllers/message_forward_sources_controller.rb",
    "app/services/messages/forwarder.rb",
    "app/views/channel_thread_messages/index.html.erb",
    "app/views/channel_thread_messages/create.turbo_stream.erb",
    "app/models/thread_membership.rb",
    "app/controllers/concerns/messages/drive_attachable.rb",
    "app/controllers/concerns/room_scoped.rb",
    "app/controllers/concerns/set_current_request.rb",
    "app/helpers/messages_helper.rb",
    "app/helpers/messages/attachment_presentation.rb",
    "app/models/sound.rb",
    "app/controllers/rooms_controller.rb",
    "app/views/rooms/show.html.erb",
    "app/views/messages/_unread_divider.html.erb",
    "app/helpers/message_payload_helper.rb",
    "app/helpers/github/pull_requests_helper.rb",
    "app/helpers/message_links_helper.rb",
    "app/models/message.rb",
    "app/models/message/markdown.rb",
    "app/models/message/legacy_markdown.rb",
    "app/models/message/pagination.rb",
    "app/models/message/broadcasts.rb",
    "app/models/channel_thread.rb",
    "app/models/thread_tag.rb",
    "app/models/agent.rb",
    "app/models/agent_grant.rb",
    "app/models/link_embed.rb",
    "app/helpers/icons_avatar_helper.rb",
    "app/views/messages/edit.html.erb",
    "app/views/messages/_actions.html.erb",
    "app/views/messages/_message.html.erb",
    "app/views/messages/show.html.erb",
    "app/views/messages/index.html.erb",
    "app/views/messages/destroy.turbo_stream.erb",
    "app/views/messages/_system_note.html.erb",
    "app/views/messages/_context.html.erb",
    "app/views/agent_steps/_steps.html.erb",
    "app/helpers/agents/steps_helper.rb",
    "app/views/users/_mention.html.erb",
    "config/icons.yml",
]
image = os.environ.get("PARITY_IMAGE", "triage-reference-d7c7de92")
output = subprocess.check_output(["docker", "run", "--rm", "--name", f"ws8bm-reference-check-{os.getpid()}",
                                  "--entrypoint", "sha256sum", image, *[f"/rails/{p}" for p in FILES]], text=True)
expected = {p: hashlib.sha256(subprocess.check_output(["git", "show", f"d7c7de92:{p}"], cwd=ROOT)).hexdigest() for p in FILES}


def verify(rows):
    actual = {}
    for row in rows:
        digest, path = row.split(maxsplit=1)
        relative = path.removeprefix("/rails/")
        assert relative not in actual, relative
        actual[relative] = digest
    assert actual == expected, "Reference source bytes or file set differ"


rows = output.splitlines()
verify(rows)
changed = list(rows)
changed[0] = "0" * 64 + " " + changed[0].split(maxsplit=1)[1]
for mutation in [changed, rows[:-1]]:
    try:
        verify(mutation)
    except AssertionError:
        pass
    else:
        raise AssertionError("Source verification accepted injected drift")
print(f"WS8bm reference source check: {len(FILES)} controller, model, helper, template and icon files match d7c7de92")
print("WS8bm reference check self-test: 2 injected source-byte/file-set differences rejected")
