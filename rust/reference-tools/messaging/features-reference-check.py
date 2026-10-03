#!/usr/bin/env python3
"""Check the Rails files this slice reads against the accepted source pin."""
import hashlib
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
FILES = [
    "app/models/event/channel_timeline.rb", "app/models/event_attendance.rb", "app/models/event_calendar_entry.rb",
    "app/models/calendar/meet_link.rb", "app/models/calendar/entry_sync.rb", "app/models/google/client.rb", "app/jobs/calendar/meet_link_job.rb",
    "app/jobs/link_embed/fetch_job.rb", "app/models/link_embed/fetcher.rb", "app/models/link_embed/metadata_parser.rb", "app/models/opengraph/location.rb", "app/models/opengraph/fetch.rb",
    "app/channels/unread_rooms_channel.rb",
    "app/models/twitter/post.rb", "app/models/twitter/post_url.rb", "app/helpers/twitter/posts_helper.rb", "app/views/twitter/posts/_card.html.erb", "app/views/twitter/posts/_cards.html.erb",
    "app/views/rooms/events/_cards.html.erb", "app/views/rooms/events/_card.html.erb", "app/helpers/rooms/events_helper.rb", "app/models/event.rb",
    "app/views/github/pull_requests/_card.html.erb", "app/models/github/pull_request.rb",
    "app/views/link_embeds/_cards.html.erb", "app/views/link_embeds/_card.html.erb", "app/helpers/link_embeds_helper.rb", "app/models/link_embed.rb", "app/models/link_embed_reference.rb",
    "app/views/linkedin/posts/_cards.html.erb", "app/views/linkedin/posts/_card.html.erb", "app/helpers/linkedin/posts_helper.rb", "app/models/linkedin/post_url.rb",
    "app/services/slash_commands/time_parser.rb",
    "app/views/rooms/pins/_panel.html.erb", "app/views/users/sidebars/show.html.erb", "app/views/rooms/show/_nav.html.erb", "app/views/rooms/show/_composer.html.erb", "app/javascript/controllers/schedule_send_controller.js",
    "app/helpers/github/pull_requests_helper.rb", "app/helpers/fizzy/cards_helper.rb", "app/views/github/pull_requests/_cards.html.erb", "app/views/fizzy/cards/_cards.html.erb",
    "app/controllers/messages_controller.rb", "app/models/message/broadcasts.rb", "app/models/message/reference_sync.rb",
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
revision = os.environ.get("PARITY_REFERENCE_REVISION", "d7c7de92")
image = os.environ.get("PARITY_IMAGE", "ws8bm2-reference:d7c7de92")
output = subprocess.check_output(["docker", "run", "--rm", "--name", f"ws8bm2-reference-check-{os.getpid()}",
                                  "--entrypoint", "sha256sum", image, *[f"/rails/{p}" for p in FILES]], text=True)
expected = {p: hashlib.sha256(subprocess.check_output(["git", "show", f"{revision}:{p}"], cwd=ROOT)).hexdigest() for p in FILES}


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
print(f"WS8bm2 reference source check: {len(FILES)} controller, model, helper and template files match {revision}")
print("WS8bm2 reference check self-test: 2 injected source-byte/file-set differences rejected")
