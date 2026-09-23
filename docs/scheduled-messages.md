# Scheduled messages

"Schedule send" next to the composer's send button schedules the
current draft — in channels and in threads, including replies — for a
preset (in 1 hour, tomorrow 9am, next Monday 9am) or a custom time.
Times resolve in the author's time zone in the browser and submit as
UTC. Scheduling consumes the draft like sending does.

The Scheduled view (sidebar) lists upcoming drafts with their channel,
thread, and send time, and past ones (sent or dropped). Upcoming drafts
can be edited (text and time), sent immediately, or cancelled.
Cancelling deletes the draft; sent and dropped rows stay as history,
with sent rows linking to the posted message. Drafts stranded by lost
channel access list separately as no longer sendable so they can still
be cancelled; otherwise they drop with an inbox notice when due.

## Sending

The periodic runner sends due rows as the author: channel drafts post
as channel messages, thread drafts as thread replies, with the same
broadcasts, unread marks, and agent deliveries as typed messages.
Attachments cannot be scheduled — only text (and a reply target).

Each row sends exactly once: the dispatcher claims it through a
conditional timestamp update before posting, so a second run or runner
cannot double-send, and re-checks the send time after claiming, so a
draft moved after selection fires at its new time instead. A claim left
behind by a crashed runner goes stale after five minutes and becomes
sendable again.

Access is re-checked at send time. If the author lost access to the
room, the draft is dropped — never leaked to non-members — and the
author gets a "Scheduled message not sent" inbox item pointing at the
Scheduled view. A locked thread is transient, so its drafts wait for
the next tick instead of dropping.

## Deletion

The `thread_id`, `reply_to_message_id`, and `sent_message_id` links
nullify when their target is deleted, so deletes never fail on these
rows. Deleting a thread drops its pending scheduled replies with an
inbox item ("its thread was deleted") instead of letting them
re-target the channel; sent history keeps its past with the cleared
link. Deleting the room destroys its scheduled rows (and their inbox
items) with everything else.
