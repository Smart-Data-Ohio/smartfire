# Meet links and two-way RSVP sync

Two additions to [native events](events.md) and [Google Calendar
publishing](google-calendar.md): optional Google Meet links per event, and
push-based sync that carries RSVP changes made in Google back into
Smartfire attendance.

## Meet links

Creating or editing an event offers **Add a Google Meet link**. The link
is created through Calendar `conferenceData` on the organizer's own
calendar copy, using the existing `calendar.events` scope — no new scopes
or credentials.

- There is no Meet link unless the organizer has connected Google. A
  request made before connecting waits; connecting later provisions every
  still-pending event the member organizes.
- The link appears on the event card ("Join Google Meet") and the event
  page once Google creates it. Repeating events provision one link per
  occurrence; **This and following** carries a new request to later
  occurrences.
- Provisioning runs in `Calendar::MeetLinkJob` (idempotent, transient
  failures retried with backoff). Permanent Google refusals are logged
  and the next event edit retries.

## Two-way RSVP sync

Smartfire stays the source of truth for event times, titles, and
descriptions (one-way publishing, as before). Attendance syncs both ways:
each connected member gets a Google Calendar push channel
(`events.watch`) on their primary calendar, and RSVP-shaped changes they
make in Google update their Smartfire response:

- Cancelling or deleting their calendar copy declines the event locally.
- Restoring a deleted copy (confirmed) re-accepts a declined event.
- Anything else reads back as no change.

The mapping converges rather than flaps: declining locally deletes the
remote copy, which reads back as declined, and going locally confirms it.
Only upcoming events with synced copies are re-read (at most 50 per
notification), only when the member can still respond, and cancelled
events are never touched.

### Setup

Push needs a publicly reachable callback URL from config:

- Set `GOOGLE_CALENDAR_WEBHOOK_URL` to the public notification endpoint,
  for example `https://smartfire.example.com/google/calendar/notifications`.

Without it (or without Google OAuth configured), watching stays disabled
with no errors: no channels open, the health page says what to set, and
one-way publishing is unaffected.

### How it works

- Connecting Google enqueues `Calendar::WatchChannelJob`, which opens one
  channel per member. The channel token is random per channel; Google
  echoes it back on every delivery, and only its SHA-256 digest is
  stored. Jobs take the channel id or user id, never the token.
- `POST /google/calendar/notifications` authenticates each delivery by
  channel id plus token (404 for unknown channels, 403 for wrong tokens),
  acknowledges the `sync` handshake without work, deduplicates `exists`
  notifications by message number (one conditional `UPDATE`, so
  concurrent redeliveries cannot both win), and enqueues
  `Calendar::InboundSyncJob` for the winner.
- Channel expiry is handled four ways: the periodic runner renews
  channels expiring within 24 hours every hour, a `not_exists`
  notification drops the row and enqueues a re-watch, disconnecting
  or deactivating stops and removes the channel, and the same hourly
  sweep opens a channel for any connected account that has none (a
  failed first watch leaves no row to renew).
- Transient Google failures retry with backoff; permanent ones land on
  the channel's `last_error`, visible on the integration health page
  alongside channel counts and upcoming expiries.
