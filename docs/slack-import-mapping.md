# Slack import mapping

How the Slack importer turns a Slack Pro workspace into Smartfire records.
The run lifecycle (dry run, scoped import, undo, catch-up) is covered in
`docs/slack-import.md`; this file covers what maps to what, what is skipped,
and the limits to expect.

## What maps to what

| Slack | Smartfire |
|---|---|
| Active human member | Active user, matched by email (case-insensitive) or created as a claimable placeholder |
| Deleted member, guest, bot, Slackbot | Deactivated placeholder (name kept, no email, can never sign in) |
| Public channel | Open room |
| Private channel | Closed room |
| Archived channel | Same room type, name suffixed ` (archived)`, every membership invisible |
| DM (`im`) | One-to-one Direct room via `find_or_create_for` (merges with an existing DM) |
| Group DM (`mpim`, ≤10 members) | Direct room for the member set |
| Group DM (`mpim`, >10 members) | Closed room named after the members |
| Message | Message with converted markdown, Slack `ts` as `created_at` (microseconds), `edited.ts` as `edited_at` |
| Thread | ChannelThread on the parent; replies carry `thread_id` |
| Thread in a DM | Flattened: replies are ordinary messages with `reply_to_message_id` on the parent |
| Reaction | Boost (one per user per reaction) |
| Pin (`pinned_to`) | `message_pins` row, at most 50 per room |
| File | A `📎 [name](permalink)` link line (files are not imported) |

## Users and claiming

Matching is by email, case-insensitive, and includes deactivated Smartfire
users. The connection owner always maps to themselves: OAuth proved they are
that Slack user, even when the emails differ.

Placeholders for active humans carry the Slack name (`real_name`, then
`display_name`, then `name`), email, time zone (only when valid) and title as
bio, and no password or avatar. `google_email_link_allowed` is set only when
the email's domain is allowlisted for Google sign-in, so a first Google
sign-in claims the account; otherwise the admin uses the existing transfer
link. Guests must never become active: an active user joins every Open room,
so guests, deleted members and bots are always deactivated placeholders with
no email, created with `skip_open_room_grant`.

Bot authors on messages without a `user` field are keyed by `bot_id` (or
username) so each bot maps once. `USLACKBOT` becomes a deactivated "Slackbot".

If a member signed up with a different email than their Slack one, matching
misses and their Slack identity becomes a separate placeholder. Fix that
before import by changing the member's Smartfire email to the Slack one (the
member can do this from their profile page, password-confirmed), so the
import matches instead of minting a duplicate.

## Rooms and memberships

Merging is narrow, so an import never writes into a room its owner cannot
see. A public channel merges by default into an alive Open room with the same
name (case-insensitive); otherwise the import creates a room. `room_targets`
overrides this per conversation with `"new"`, `"skip"` or an existing room id
(which must be an alive Open or Closed room).

A private channel never auto-merges by name: a workspace run merges one into
an existing Closed room only when an administrator chose that room in
`room_targets`, and otherwise creates a new room. Personal runs never merge
into a pre-existing room at all — the only exceptions are a room an earlier
run's mapping already points at (reused for deduping) and Direct rooms, which
resolve through `find_or_create_for` with the owner as a member. Personal runs
ignore `room_targets` room ids entirely.

Memberships are only ever written in rooms the import created — merged rooms
keep theirs untouched.

In a created Open room, Slack members get the default involvement and every
other active user goes invisible. Invisible Open memberships stay reachable:
the member can open the room by direct URL (room lookup covers every
membership) and its messages appear in search (`reachable_messages` covers
every membership), but the room is hidden from the sidebar and the switcher
(both read `memberships.visible`), and invisible members never accrue unread
or push (`Room#receive` starts from `memberships.visible`). Opening the room
offers the normal involvement control to join visibly.

Imported memberships carry no unread state, and `last_read_message_id` points
at the last imported message. Room `updated_at` becomes the last imported
message time for created rooms (so imported DMs do not jump to the top of the
sidebar), or stays at the later of its time and that for merged rooms. The
room creator is the user who started the run.

Self-DMs and DMs with Slackbot are skipped.

## Messages, threads, reactions, pins

Mentions (`<@U123>`) become `@[Full Name]` tokens, which resolve at save time
to active room members with a unique name — users and memberships are created
before messages for exactly this reason. Duplicate or non-member names stay
literal text, as do `@here`, `@channel` and `@everyone` (Smartfire only turns
`@[Name]` tokens into mentions, so the literals can never notify).

Skipped subtypes: joins and leaves, topic, purpose, renames, archive and
unarchive (plus the `group_*` equivalents), `pinned_item`, `bot_add`,
`bot_remove`, `tombstone`, `message_deleted` and `huddle_thread`.
`thread_broadcast` is skipped in history and kept once, as the thread reply.

Threads are read per parent (`reply_count > 0`) through `conversations.replies`.
The thread keeps Smartfire's naming default (the parent's first line) with
`last_activity_at` set to the last reply time, the count refreshed once, and
reply authors that belong to the room added as followers. A thread whose
parent falls outside the run's date bounds is skipped, and every message row
stays inside the bounds.

Reactions strip `::skin-tone-N`; standard emoji resolve to their character,
known icons to canonical `:name:`, and unknown custom emoji stay `:name:` when
the shortcode is valid — anything else is skipped with an issue. Pins past 50
per room are recorded as issues. Over-long messages truncate at 50,000
characters with a warning issue.

Imported messages render markdown, resolve mentions, index for search and keep
their DB-only reference rows, but skip unread marks, web push, inbox items,
network reference fetches, agent deliveries, thread-indicator broadcasts, the
stale-thread sweep and room touches.

## Catch-up

Every Slack object maps once per workspace in `slack_import_records`, so a
later run skips everything an earlier run brought over — including DMs and
private channels imported by a different member's personal run.

The 30-day catch-up window opens only when an earlier, completed, full import
(an import run with no `oldest` bound) already covered the conversation:
history is then fetched from 30 days before the newest imported message, which
picks up late thread replies without re-reading all history. Coverage is
decided from the runs themselves — a conversation is covered when a completed
full import finished it (its run stats mark it done) — not from which run owns
its mapping row, since a full import reuses the test import's mapping. A
date-bounded test import never opens the window, so the later full import
re-reads the whole range and the mapping skips duplicates. Each conversation's
bounds are fixed when the run starts that conversation and reused for its
history and thread replies on every later step, so a long conversation keeps
its full window however many steps it spans. Threads found on catch-up pages
are re-read in full; only new replies are created. When a mapped thread or
its parent message was deleted since the import, the run skips that thread's
new replies with one issue and continues, the same as for a deleted room.

## Undo

Undo deletes in batches, in reverse dependency order, and only ever removes
what the run created: reactions, pins and thread follows; messages through the
full destroy path (rich text and search rows go, quietly); threads; memberships;
rooms; placeholder users that never signed in and author nothing left; then the
run's mapping rows. Matched users and pre-existing content are never touched.

A thread is deleted only when every message in it was created by the run.
Otherwise the thread and its parent message stay, with an issue recorded. Each
room's fate is decided before any membership is touched: a room holding
messages the run did not create stays with all its memberships and its
conversation mapping, with an issue recorded. Mappings for everything kept
stay behind too, so a later run reuses the survivors instead of duplicating
them.

One import or undo runs at a time across the workspace. Undo waits while any
other run is queued, running or undoing, and the run page says so until the
way is clear.

## Rate limits and expected duration

Slack paces the list methods at Tier 2 (20+/min) and history/replies at Tier 3
(50+/min) for internal apps. The importer stays under both at about 18/min and
45/min per method family, and a 429 reschedules the run after `Retry-After`.
The arithmetic, at about 50 calls a minute with one call per thread:

- a 10,000-message channel in full pages: ~50 history calls, ~1 minute;
- plus one call per thread (a 300-thread channel adds ~7 minutes);
- users, channel list and member lists are one call per page each.

A workspace with 100 channels averaging 5,000 messages and 150 threads each
needs roughly 100 × (25 + 150) ≈ 17,500 calls, or about 6 hours on the single
import worker. Dry runs cost the same reads minus the replies. Plan the
cutover catch-up (which re-reads only 30 days of history per channel) from the
recent-message volume, not the archive size.

Per-conversation mapping lookups seek the workspace/kind/key unique index
with a key range (`CONV:` to `CONV;`), never a `LIKE` scan. Long loops —
finishing rooms, deciding kept rooms on undo — refresh the run's heartbeat
as they go, so the 5-minute stale sweeper never stacks a second job onto a
live run.

## Known limitations

Files, custom emoji images, avatars, canvases, huddles, channel topics and
scheduled or draft messages are not imported. Bot attachment payloads survive
only as quoted text. Threads re-read in full on catch-up. Extremely large
channels hold one page plus one member list in memory at a time; member lists
above tens of thousands inflate the run's saved state. Undo keeps rooms and
users that gained newer activity instead of deleting them.
