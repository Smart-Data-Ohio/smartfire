# Status, Do Not Disturb, and notifications

People set a presence and custom status, silence push and sounds with Do
Not Disturb or scheduled quiet hours, follow or mute channel threads,
watch keywords, and pick a time zone and theme. One notification policy
(`rust/crates/db/src/models/notification_policy.rs`) gates every push and sound; the pushers call
it instead of re-deciding the rules, and the inbox recorder implements
the same inbox rules in its own candidate flow.

## Presence and custom status

Presence choices:

- **Automatic** (default): online while the browser holds a workspace
  presence lease, idle after 10 minutes without input.
- **Do not disturb**: silences push and sounds (below) and shows a red dot.
- **Invisible**: appear offline everywhere.

Idle detection rides the existing presence heartbeat: the browser reports
whether it saw input in the last minute, and the server stamps
`workspace_presence_leases.last_active_at` only then. A lease that stays
connected but quiet reads idle after `IDLE_AFTER` (10 minutes). The
presence map for a set of users costs one query
(`WorkspacePresenceLease.presence_by_user_id`); reads and lease setup
never prune, since a DELETE takes the SQLite write lock on a hot path.
Expired leases are swept once a minute by the `presence leases` periodic
task instead.

A custom status is an emoji plus text with an expiry of 30 minutes,
1 hour, 4 hours, today, this week, or never. "Today" and "this week" end
at midnight in the member's own time zone. An expired status reads as
blank without a cleanup job; every reader goes through
`User#custom_status_display`.

Presence shows in the member panel (dot plus status text, grouped
online/offline), on DM rows (dots painted by the `dm-presence` Stimulus
controller from `GET /users/presence?ids[]=` so cached rows stay cached),
on profile pages, and through the shared badge below. Agents keep their
self-reported live status: bot rows in the member panel read from the
agent (`status` plus `status_note`), never from presence leases.
Members who opt into [meeting status](meeting-status.md) show "📅 In a
meeting" in the status line while their Google Calendar says they are
busy; a custom status or DND takes precedence over the automatic label.
Members [out of office](out-of-office.md) show "🌴 Out of office until
\<date\>" instead, which takes precedence over custom status, DND, and
the meeting label alike.

### Profile card integration

The profile card renders status through one shared partial
(`users/statuses/_badge`, in `rust/crates/views/templates/`) showing the
presence dot, presence label, and custom status text, so it never queries
leases itself.

## Do Not Disturb and quiet hours

While DND is on, push notifications and sounds stay silent for
everything. Inbox items are still recorded. A member can star people as
**Allow during DND** from their profile page; messages from starred
people still push. Reminders carry no sender, so they stay silent with
no exception.

Quiet hours schedule DND daily: an on/off switch plus a start and end
time, evaluated in the member's time zone (overnight windows work).
Quiet hours share the starred-people exception. So does
[quiet-during-meetings](meeting-status.md): while a busy interval
covers an opted-in member, they read exactly as DND to the policy. So
does [out of office](out-of-office.md), unless the member asked to keep
being notified.

Sounds (`/play` chat sounds, played by the `sound` Stimulus controller)
re-evaluate muting on every play: the layout sends manual DND and the
DND presence as a muted marker plus the quiet-hours window and zone,
and the controller computes the window live, so crossing a quiet-hours
boundary silences or unsilences sounds without a reload. (A DND switch
flipped in another tab still needs a navigation; only the time-based
gate is live.) Quiet-during-meetings sends the cached busy intervals
as epoch windows for the same live treatment; a calendar edit that
moves them needs a navigation. Out-of-office quiet sends its end the
same way unless the member keeps notifications on. The policy's
`sound?` (always equal to `push?`) gates any server-side sound
decision the same way.

## Thread controls

Channel threads keep their three involvement levels, now with matching
behavior across inbox and push:

- **Follow** (`everything`): inbox items and push for every message,
  including replies.
- **Unfollowed** (`mentions`, the default on join): mentions only. A
  reply to an unfollowed member records nothing and pushes nothing.
- **Muted** (`nothing`): nothing at all, not even mentions or keywords.

Joining, leaving, and the involvement select in the thread panel already
expose these; the reply rule above is the behavior change. At room
level, replies still notify `mentions` members (unchanged).

## Keyword alerts

Each member watches up to 20 words or phrases (profile page, one per
line). A new message in a room they belong to matches case-insensitively
on word boundaries ("deploy" matches "Deploy now", not "Redeploying").
A match records a `keyword_alert` inbox item; a keyword never pushes
by itself (an `everything` follower still gets the broadcast push for
the message itself, keyword or not).

Matching runs once per message: thread messages reuse the already-loaded
memberships, root messages match only members holding an alert (a join
from `keyword_alerts` into the room's memberships, so the roster size
never matters), and every distinct phrase compiles into its own pattern
checked independently, so overlapping
phrases held by different users all match. Thread-muted and invisible
members match nothing; members with room notifications off or the room
muted still match, since a keyword is an explicit opt-in like a
mention. A keyword never overrides a mention, reply, or thread item for
the same message. The recorder's query cost stays constant as followers
and the roster grow.

## Time zone

Each member has a time zone, detected from the browser on first visit
(the `timezone` Stimulus controller reports
`Intl.DateTimeFormat().resolvedOptions().timeZone` once, only while none
is saved and none was explicitly chosen) and editable on the profile
page. The form's options carry IANA identifiers (what detection
stores); legacy Rails time zone names (the `Eastern Time (US & Canada)` style)
still validate and map to their identifier for display. Saving the form — even as "Not set" — marks the choice
explicit (`users.time_zone_explicit`), so detection never overwrites a
decision the member made. Every request renders in the member's zone
and quiet hours plus custom status
expiries evaluate in it.

## Theme

Light, Dark, or System (default), picked on the profile page and applied
server-side as `data-theme` on `<html>`, so the first paint already
matches with no flash. `theme.css` re-declares the `colors.css`,
`workspace.css`, and `code.css` tokens under
`:root[data-theme="light"]` and `:root[data-theme="dark"]`; the
`color-scheme` meta tag follows the choice. Component rules follow the
manual theme through those tokens (icon inversion uses
`--icon-filter`) or through their own `data-theme` counterparts, so
only the `system` setting reads the OS. The standalone public pages
have no theme picker and keep following the OS.

## The policy object

The policy answers two questions for one recipient:

- `inbox_event_type`: which item to record (`mention`, `reply`,
  `thread_activity`, `keyword_alert`), or nil.
- `push` / `sound`: whether push and sounds go out.

Every push path (room and thread messages, event reminders, huddle
invitations and huddle joins) asks `push`; sounds follow through the DND marker and
quiet-hours window the layout renders for the `sound` controller. The
inbox recorder calls `inbox_event_type` for each candidate: it batches
keyword matching once per message
(the candidates are the thread members for thread messages, and the
mentionees plus the reply author plus the keyword matches for room
messages), then asks the policy for each candidate's winner with the
already-loaded memberships. The tests in `rust/crates/db/src/tests/`
(`notification_policy_test.rs`, `keyword_alert_test.rs` and the recorder
tests) pin the rules, the recorder's identical behavior and its flat
query cost.

Pushers preload one membership map, one user map, and one DND-exception
set (`dnd_exceptions_for`) per batch, then decide per recipient
in memory.
