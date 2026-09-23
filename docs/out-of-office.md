# Out of office, manual and from Google Calendar

Members can mark themselves **"🌴 Out of office"** with an end date, either
by hand (status settings or `/ooo`) or automatically from out-of-office
events on their Google Calendar (opt-in). This document covers setting and
clearing it, the display precedence, the DM notice, notification quiet, and
the refresh and broadcast flow.

## Manual out of office

The status settings' "Out of office" section sets a manual OOO with an end:

- Presets: **Until tomorrow**, **Until Monday**, **1 week**. Presets run to
  the end of the day in the member's own time zone; "Monday" is the next
  one (a week out on Mondays).
- A custom date and time picker, interpreted in the member's time zone.
- An optional note, up to 140 characters, shown beside the label.

`/ooo <when> [note]` does the same from the composer. Durations (`/ooo 3d`,
`/ooo 1 week`) stay exact; bare days (`/ooo tomorrow Back soon`,
`/ooo friday`) and bare dates (`/ooo 2026-10-05`, `/ooo oct 5`) run to the
end of that day in the member's zone, like the form presets (bare weekdays
resolve to the next one, a week out on the same weekday); an explicit clock
time (`/ooo friday 5pm Wrapping up`) keeps that time. `/ooo off` clears it.
There is no profile menu in the app, so the settings section and the slash
command are the two surfaces.

A manual OOO starts immediately and auto-clears at its end: past the end it
reads as off with no cleanup job, and the minute-tick OOO dispatcher
broadcasts the flip and clears the columns. The member can clear it early
from the settings ("Clear out of office") or `/ooo off`; setting or
clearing updates open badges and DM notices at once.

## From Google Calendar (opt-in)

"Use my Google Calendar out-of-office", default off, needs the calendar
connection. Google Calendar events with `eventType: "outOfOffice"` mark the
member OOO for the event's span, with the event end as the "until".
Cancelled OOO events never count; all-day OOO events do (a vacation week is
the main use case), with the dates resolving in the member's zone.

Only the event type, start, and end are read — never the title,
description, or any other detail. The read extends the meeting-status
`events.list` fetch (same request, a 30-day lookahead for OOO members)
and the same cache row, rather than a second fetch; each interval set is
stored only for its own opt-in. OOO events never count as busy for meeting
status either way.

When a manual OOO and a calendar OOO overlap, the later end shows, with the
member's own manual note if one is set. Manual "clear early" ends only the
manual one: a covering calendar interval keeps showing with its own end
(and the note disappears with the manual span). Turning the calendar opt-in
off clears the cached OOO intervals; turning it on fetches them right away.

## Display and precedence

While out of office, the member reads as "🌴 Out of office until \<date\>"
plus their note, everywhere a custom status shows today: the profile badge,
the member panel, profile cards, sidebar DM rows (as the dot tooltip), and
the profile page. The date renders in the OOO member's own zone, so every
viewer sees the same return date. Other viewers see only that plus the
member's own note — no calendar details.

Precedence, top wins (see `User#ooo_status_visible?`):

1. **Invisible** hides everything inferred.
2. **Out of office** wins over a custom status, DND, and the meeting label.
3. **A manually set custom status** wins over the meeting label.
4. **Manual DND** (the toggle, DND presence, quiet hours) wins over the
   meeting label too, since DND already signals unavailability.
5. Otherwise the **meeting** label shows.

OOO quiet itself never suppresses the label, or the two features would
cancel each other out. The presence dot and label are unaffected; only the
status line changes.

## DM notice

Opening a DM or group DM with an OOO person shows a small notice above the
composer: "\<Name\> is out of office until \<date\>. \<note\>". There is one
line per OOO recipient, so a group DM can show several. The notice renders
per viewer on every page load and is never fragment-cached; each line
subscribes to its member's OOO stream, so it appears and disappears live
when their OOO starts or ends. The broadcast names only the return date and
the member's own note — the same string for every viewer.

## Notifications

While out of office, push, sounds, and huddle rings stay silent, exactly
like DND through `Notifications::Policy`, with the same "Allow during DND"
exception. Inbox items are still recorded. The per-member
"Keep notifying me while I'm out of office" switch (notification settings,
default off) opts back into notifications: with it on, OOO shows but never
silences. `/play` chat sounds mute the same way: the layout sends the OOO
end as epoch windows and the sound controller re-evaluates the gate on
every play, so an end crossed mid-page unsilences without a reload.

## Refreshing and broadcasting

Calendar OOO intervals live in `calendar_meeting_caches` beside the busy
intervals and refresh through `Calendar::MeetingRefreshJob`:

- immediately on opt-in, on reconnect, and on calendar push
  notifications (throttled to one fetch per minute per member, with one
  delayed follow-up claimed per burst so the change is never dropped);
- at most every 15 minutes by the periodic sweep (OOO-only members
  refresh through the OOO dispatcher; members with both opt-ins refresh
  through the meeting dispatcher, so one tick never enqueues two).

The `out of office` task in `Periodic::Runner` runs every minute. Each tick
checks every member with a manual OOO or the calendar opt-in; a flip is
claimed with one conditional `UPDATE` (`User#claim_ooo_broadcast!`), so
concurrent ticks announce each boundary exactly once, and a tick with no
flip broadcasts nothing. The broadcast replaces the member's status badge
and DM notice lines live (the `[user, :status]` and `[user, :ooo_notice]`
Turbo streams).

## Failure behavior

A revoked grant, API error, or rate limit never raises into presence: the
refresh stores a gentle notice for the settings page and stamps the fetch
time so the failure backs off to the 15-minute cadence instead of
hot-looping. A dead grant clears the cached intervals (calendar OOO
silently turns off); a transient failure — rate limit, 5xx, timeout,
malformed body — keeps the last good intervals so the status keeps showing.
The next successful refresh clears the notice. The next minute-tick claims
any resulting flip and broadcasts it. Disconnecting Google,
opting out, deactivating, or destroying the member drops the cached
intervals immediately.

## Agents

Agent surfaces expose no per-member status: message and thread payloads
carry the creator's id, name, role, and avatar, but never presence, custom
status, meeting, or OOO state — so there is nothing to add, and no new
tool. (The `set_presence` tool sets the agent's own working presence,
unrelated to member OOO.)
