# Pinned messages, saved items, and reminders

Discord-style pins per channel, plus Slack-style "Save for later" with
optional reminders.

## Pins

Any member who can post in a room can pin or unpin any message in that
room, from the message menu ("Pin message" / "Unpin message"; the hover
toolbar's "…" opens the same menu). Pins are per room, newest first,
capped at 50 per room; pinning past the cap is rejected with a 422 and
an explanatory message.

The pin icon in the room header shows the pin count and opens the pins
panel, which lists the pinned messages with the pinner and pin time, a
jump-to link for each message, and an unpin action. Pinned messages
carry a small "Pinned" marker in the message list. Pinning posts a
one-line channel note ("📌 pinned a message", with a link to the
message) as the pinner, through the same path event announcements use.

Pins broadcast live over the room messages stream (badge, header
count, and panel list), and access follows the room: only members can
pin, unpin, or list, and pins vanish with the room.

Agents with `post_messages` pin through the agent API; see
[AI agents](agents.md#pins).

## Saved items

"Save for later" in the message menu saves any message you can see.
The Saved view (sidebar, under Work threads) lists your saved messages
following the activity inbox layout, with All / In progress / Done
filters. Each item links back to its message and can be marked done
(or reopened) or removed. Saving is idempotent: saving again updates
the reminder instead of duplicating the item.

Access is re-checked at view time: if you lose access to the room, the
saved item is hidden (not deleted), and it reappears if you regain
access. Acting on a hidden item answers 404.

## Reminders

The save dialog offers "No reminder" (the default), "In 20 minutes",
"In 1 hour", "In 3 hours", "Tomorrow at 9am (your time)", or a custom
date and time. Presets resolve in the viewer's own time zone in the
browser and submit as UTC; the server rejects past or unparseable
times.

At the reminder time, the periodic runner (`bin/periodic`) fires the
reminder: the saver's inbox item for the message transitions to a
"Reminder" (`message_reminder`, under the Reminders inbox filter, kept
read/unread/handled like any other item), and a push notification goes
to the saver's devices. Reminders fire once: the dispatcher claims
each due item through `reminded_at` before notifying, so a second run
or runner cannot double-fire.

A reminder only fires while the saver can still see the room. If the
saver lost room access, the item is claimed without notifying, so a
reminder never leaks message content to a non-member. Deleting the
message deletes its saved items, pins, and inbox items with it.
