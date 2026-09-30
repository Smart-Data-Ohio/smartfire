#!/usr/bin/env bash
set -euo pipefail
ws8_root=$(cd "$(dirname "$0")/../../.." && pwd)
set -- models/channel_thread.rb models/thread_membership.rb models/thread_tag.rb models/message.rb services/messages/forwarder.rb models/poll.rb models/poll_option.rb models/poll_vote.rb models/message_pin.rb models/saved_item.rb models/saved_item/reminder_dispatcher.rb models/saved_item/reminder_pusher.rb models/scheduled_message.rb models/scheduled_message/dispatcher.rb models/keyword_alert.rb models/notifications/keyword_matcher.rb models/room_category.rb models/membership.rb models/room.rb models/message_reference.rb models/message/reference_sync.rb models/rooms/direct.rb models/search_query.rb models/audit_log.rb models/message/markdown.rb models/icons.rb jobs/message/quote_cards_refresh_job.rb services/periodic/runner.rb jobs/room/destroy_job.rb jobs/retention/prune_job.rb models/huddle_grant.rb models/huddle_cleanup.rb models/stream.rb models/agent_grant.rb models/event.rb models/event_calendar_entry.rb models/event_attendance.rb models/event_reference.rb models/github/repository_subscription.rb models/github/notification.rb models/work_thread_event.rb models/work_thread_link.rb models/work_handoff.rb models/agent_step.rb services/huddle.rb models/board_sla_nudge.rb models/event/channel_timeline.rb models/github/pull_request_thread.rb
docker run --rm --cpus 2 --name ws8-reference-source-check --entrypoint '' -v "$ws8_root/app:/oracle:ro" -v "$ws8_root/db:/oracle-db:ro" -v "$ws8_root/config/icons.yml:/oracle-icons.yml:ro" "${PARITY_IMAGE:-ws8-reference-models}" sh -ec '
for file in "$@"; do cmp "/rails/app/$file" "/oracle/$file"; done
cmp /rails/config/icons.yml /oracle-icons.yml
diff -r /rails/db /oracle-db
echo "WS8 reference: $# implementation files, icon catalog and Rails db match worktree"
' sh "$@"
