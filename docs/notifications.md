# Status, Do Not Disturb, and notifications

People set a presence and custom status, silence push and sounds with Do
Not Disturb or scheduled quiet hours, follow or mute channel threads,
watch keywords, and pick a time zone and theme. One policy object,
`Notifications::Policy`, gates every push and sound; the pushers call
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
(`WorkspacePresenceLease.presence_by_user_id`).

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

### Profile card integration

The profile card renders status through one helper, so it never queries
leases itself:

```erb
<%= render_user_status_badge(user) %>
```

This renders `users/statuses/_badge` (dot, presence label, custom status
text). Pass `presence:` to reuse a preloaded value, or render
`presence_dot_tag(user, presence:)` for the dot alone. Presence labels
come from `Users::PresenceHelper#presence_label`.

## Do Not Disturb and quiet hours

While DND is on, push notifications and sounds stay silent for
everything. Inbox items are still recorded. A member can star people as
**Allow during DND** from their profile page; messages from starred
people still push. Reminders carry no sender, so they stay silent with
no exception.

Quiet hours schedule DND daily: an on/off switch plus a start and end
time, evaluated in the member's time zone (overnight windows work).
Quiet hours share the starred-people exception.

Sounds (`/play` chat sounds, played by the `sound` Stimulus controller)
read the server-rendered `<meta name="notification-sounds">` tag: when
its content is `muted`, `play()` returns without playing. The tag
reflects DND at page render; the policy's `sound?` (always equal to
`push?`) gates any server-side sound decision the same way.

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
memberships, root messages load the roster in a fixed set of queries,
and every distinct phrase compiles into its own pattern checked
independently (`Notifications::KeywordMatcher`), so overlapping phrases
held by different users all match. Muted and invisible members
match nothing; members with room notifications off still match, since a
keyword is an explicit opt-in like a mention. A keyword never overrides
a mention, reply, or thread item for the same message. The query-count
test in `test/services/activity_items/recorder_keyword_test.rb` pins the
constant query cost as followers grow.

## Time zone

Each member has a time zone, detected from the browser on first visit
(the `timezone` Stimulus controller reports
`Intl.DateTimeFormat().resolvedOptions().timeZone` once, only while none
is saved) and editable on the profile page. Every request renders in the
member's zone (`SetTimeZone`, with an around hook that restores the
previous zone even when the action raises), and quiet hours plus custom
status expiries evaluate in it.

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

`Notifications::Policy` answers two questions for one recipient:

- `inbox_event_type`: which item to record (`mention`, `reply`,
  `thread_activity`, `keyword_alert`), or nil.
- `push?` / `sound?`: whether push and sounds go out.

Every push path calls `push?` (`Room::MessagePusher`,
`ChannelThread::MessagePusher`, `Event::ReminderPusher`,
`Huddle::InvitationPusher`); sounds follow through the
`notification-sounds` meta tag the layout renders from the same DND
state. The inbox recorder does not call the policy per recipient:
`ActivityItems::Recorder` reimplements the inbox rules in its own
candidate flow (same winners, same mention-over-reply-over-activity-
over-keyword precedence) so it can batch keyword matching once per
message. Keep the two in sync: the matrix test in
`test/models/notifications/policy_test.rb` pins the policy side, and
`test/services/activity_items/recorder_keyword_test.rb` plus the push
gating tests pin the recorder side.

Pushers preload one membership map, one user map, and one DND-exception
set (`Policy.dnd_exceptions_for`) per batch, then decide per recipient
in Ruby.
