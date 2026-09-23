# In-a-meeting status from Google Calendar

Members can opt into showing **"In a meeting"** beside their name while
their Google Calendar says they are busy, and can separately opt into
silencing notifications during those meetings. Both settings default
off. This document covers the data read, the display precedence, the
refresh and broadcast flow, and the failure behavior.

## What is read, and why events.list

Meeting status needs a connected Google account with the Calendar
scope; without one, the profile explains that and links to connect.
The read uses `events.list` on the primary calendar under the existing
`calendar.events` grant — no new OAuth scope, no re-consent:

- `singleEvents=true` (recurring events expand to instances),
  `timeMin` one hour ago, `timeMax` 24 hours ahead.
- A `fields` mask (`items(eventType,start,end,status,transparency,...)`,
  see `Google::Client::MEETING_STATUS_FIELDS`) keeps titles,
  descriptions, locations, and attendee identities out of the response
  entirely. Only the event type, start/end times, the status, the
  transparency, and each attendee's self/declined flags arrive.
- Busy intervals are derived in `Calendar::MeetingIntervals`: cancelled,
  out-of-office (`eventType: "outOfOffice"` — those feed [calendar
  OOO](out-of-office.md) instead), declined-by-self, "Show as: Free"
  (transparent), and all-day (date-only start) events never count.
  Tentative and needs-action events count, matching Google's own
  free/busy. Malformed items are skipped, never raised.

The `freeBusy` endpoint was considered and rejected: it requires a
scope the app does not request (`calendar.events` is not accepted —
see the authorization table at
https://developers.google.com/workspace/calendar/api/v3/reference/freebusy/query),
and it returns bare busy blocks, so all-day events could not be
excluded and a single all-day event would show "In a meeting" all day.

Only `[start, end]` timestamp pairs are ever stored
(`calendar_meeting_caches`, which also holds the calendar-OOO intervals
beside the busy ones — each set only for its own opt-in), and only the
boolean "in a meeting" leaves the server: other members see the label,
never any meeting detail.

## Display and precedence

While opted in and inside a busy interval, the member reads as "📅 In
a meeting" everywhere a custom status shows today: the profile badge,
the member panel, profile cards, and the sidebar DM rows (as the dot
tooltip). The badge partial, the member list, and the presence lookup
all read through `User#status_text_display`. The presence dot and label
are unaffected; only the status line changes.

Precedence, top wins (see `User#meeting_status_visible?`; the full order
lives in [out of office](out-of-office.md#display-and-precedence)):

1. **Invisible** hides everything inferred: an invisible member never
   shows the automatic label.
2. **Out of office** wins over the automatic label.
3. **A manually set custom status** wins over the automatic label.
4. **Manual DND** (the toggle, DND presence, quiet hours) wins too,
   since DND already signals unavailability.
5. Otherwise the meeting label shows.

Quiet-during-meetings itself never suppresses the label, or the two
features would cancel each other.

## Refreshing and broadcasting

Busy intervals are cached per member in `calendar_meeting_caches` and
refreshed by `Calendar::MeetingRefreshJob` — the same fetch and row as
[calendar OOO](out-of-office.md), which widens the window to 30 days for
its own opt-in:

- immediately on opt-in, on reconnect, and on calendar push
  notifications (throttled to one fetch per minute per member, so push
  bursts never hammer Google);
- at most every 15 minutes by the periodic sweep.

The `meeting status` task in `Periodic::Runner` runs every minute. Each
tick enqueues refreshes for stale caches and checks every opted-in
member's cached boundaries; a flip is claimed with one conditional
`UPDATE` (`MeetingCache#claim_broadcast!`), so concurrent ticks
announce each boundary exactly once, and a tick with no flip
broadcasts nothing. The broadcast replaces the member's status badge
live on open profile pages and profile cards (the `[user, :status]`
Turbo stream). The member panel and DM dots poll the same reader every
12 and 60 seconds, so they converge on their own cadence with no
broadcast. Opting out or disconnecting while a label shows
broadcasts the cleared badge immediately instead of waiting for the
next sweep. Cached rows and fragments hold no per-viewer data: the
label is the same string for every viewer.

## Quiet-during-meetings

"Do not disturb during meetings" is a per-member on/off switch on the
notification settings. It only works while meeting status itself is
on. During a busy interval the member reads exactly as DND through
`Notifications::Policy`: push and huddle rings (via
`Huddle::RingPolicy`) stay silent, inbox items are still recorded,
and people starred with "Allow during DND" still get through.
`/play` chat sounds mute the same way: the layout sends the cached
busy intervals as epoch windows and the sound controller
re-evaluates the gate on every play, so a boundary crossed mid-page
silences (or unsilences) without a reload — but a calendar edit that
moves the intervals needs a navigation, the same as a quiet-hours
edit. Unlike the label, quiet applies through a custom status: the
label may be hidden, but the interval still silences.

## Failure behavior

A revoked grant, API error, or rate limit never raises into presence:
the refresh clears the cached intervals (the status silently turns
off), stores a gentle notice for the settings page, and stamps the
fetch time so the failure backs off to the 15-minute cadence instead
of hot-looping. The next successful refresh clears the notice.
Disconnecting Google, deactivating, or destroying the member deletes
the cached intervals immediately; opting out clears the busy intervals
(the row survives while calendar OOO still wants it).
