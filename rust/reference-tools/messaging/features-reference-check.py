#!/usr/bin/env python3
"""Check the Rails files this slice reads against the accepted source pin."""
import hashlib
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
FILES = [
    "app/models/notifications/policy.rb", "app/models/user/status_settings.rb", "app/models/calendar/meeting_cache.rb", "lib/web_push/pool.rb", "lib/web_push/notification.rb",
    "app/controllers/rooms/message_links_controller.rb", "app/controllers/rooms/files_controller.rb", "app/helpers/message_links_helper.rb", "app/helpers/rooms_helper.rb", "app/views/rooms/message_links/show.html.erb", "app/views/messages/message_links/_card.html.erb", "app/views/messages/message_links/_private.html.erb", "app/views/messages/message_links/_cards.html.erb", "app/views/rooms/files/index.html.erb", "app/models/drive_attachment.rb",
    "app/controllers/rooms/slash_commands_controller.rb", "app/controllers/autocompletable/icons_controller.rb", "app/controllers/autocompletable/slash_commands_controller.rb", "app/controllers/autocompletable/users_controller.rb",
    "app/models/icons.rb", "app/models/sound.rb", "app/services/huddle.rb", "app/services/slash_commands/registry.rb", "app/services/slash_commands/dispatcher.rb", "app/services/slash_commands/handlers.rb",
    "app/views/autocompletable/users/_user.json.jbuilder", "app/views/autocompletable/icons/_icon.json.jbuilder", "app/views/messages/_presentation.html.erb",
    "app/models/message.rb", "app/models/message/mention_preloader.rb", "lib/rails_ext/action_text_attachables.rb",
    "app/controllers/searches_controller.rb", "app/models/search_query.rb", "app/models/search.rb", "app/helpers/searches_helper.rb",
    "app/views/searches/index.html.erb", "app/views/searches/index.turbo_stream.erb", "app/views/searches/clear.turbo_stream.erb",
    "app/views/searches/_filters.html.erb", "app/views/searches/_sections.html.erb", "app/views/searches/_load_older.html.erb", "app/views/searches/_page_recents.html.erb", "app/views/searches/_dropdown_recents.html.erb",
    "app/controllers/scheduled_messages_controller.rb", "app/models/scheduled_message.rb", "app/models/scheduled_message/dispatcher.rb",
    "app/views/scheduled_messages/index.html.erb", "app/views/scheduled_messages/_item.html.erb", "app/views/scheduled_messages/_past_item.html.erb", "app/views/scheduled_messages/_composer_button.html.erb",
    "app/controllers/rooms/polls_controller.rb", "app/controllers/messages/pins_controller.rb",
    "app/controllers/rooms/pins_controller.rb", "app/controllers/concerns/room_scoped.rb",
    "app/models/poll.rb", "app/models/poll_option.rb", "app/models/poll_vote.rb", "app/models/message_pin.rb",
    "app/views/polls/_poll.html.erb", "app/views/messages/_pin_badge.html.erb",
    "app/views/rooms/pins/_count.html.erb", "app/views/rooms/pins/_list.html.erb", "app/views/rooms/pins/index.html.erb",
    "app/helpers/messages_helper.rb", "app/helpers/time_helper.rb",
    "app/controllers/saved_items_controller.rb", "app/models/saved_item.rb",
    "app/models/saved_item/reminder_dispatcher.rb", "app/models/saved_item/reminder_pusher.rb",
    "app/jobs/saved_item/reminder_push_job.rb", "app/views/saved_items/index.html.erb", "app/views/saved_items/_item.html.erb",
]
image = os.environ.get("PARITY_IMAGE", "ws8bm2-reference:d7c7de92")
output = subprocess.check_output(["docker", "run", "--rm", "--name", f"ws8bm2-reference-check-{os.getpid()}",
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
print(f"WS8bm2 reference source check: {len(FILES)} controller, model, helper and template files match d7c7de92")
print("WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected")
