# People, group DMs, and group calls

## Profile cards

Clicking (or pressing Enter on) any avatar or display name opens a
Discord-style profile card in one shared popover, loaded lazily from
`GET /users/:id/card`. Covered surfaces: the DM list, member panel,
presence stacks, and stage roster; message authors, mentions, the reaction
reactor list, and thread participants. The in-call huddle roster is the
one exception: LiveKit participant identities are deliberately opaque, so
those rows carry no user id to load a card from.

The card shows avatar, name, presence (bots with an agent read live from
the agent), an Admin/Agent/Bot badge, and "Agent owned by X" for bots with
an agent. Actions: Message (opens or creates the 1:1 DM), Start call
(opens the 1:1 DM and auto-joins its huddle, ringing them; hidden for
bots), View profile, and Copy mention (copies `@[Name]` for the
composer). Your own card shows Edit profile instead. The popover traps
focus, closes on Esc or outside click, and returns focus to the trigger.

## Multi-select

The member panel, the people directory (`/users`), and the new-DM picker
offer checkbox multi-select with a sticky bar: Message (n), Start huddle
(n), and Clear. Shift-click extends a checkbox range; Ctrl/Cmd-click
toggles; long-press selects on touch. One person opens the 1:1 DM; two to
nine others open the group DM with exactly that set plus you, reusing it
when it exists; more than nine disables the bar with the reason. Agents
can be messaged but never join calls: Start huddle counts humans only and
says how many agents stay in the DM without being rung. Start huddle
lands in the DM with `?huddle=start`, which the join control consumes
exactly once.

## Group DMs

Ad hoc group DMs hold 3–10 people including you. They live in the DM
sidebar section with stacked avatars (up to 3) and either their custom
name or a default first-names preview ("Riel, Jon, Chris +2"). Members
can rename the group, add people up to the cap, and leave from the DM
settings page; leaving removes only your membership, and the group keeps
working. Renames, adds, and leaves post quiet system notes in the
timeline (`messages.system_note`, shared with pin notes), which render as
one compact centered line and broadcast live but stay quiet: no unread,
push, agent or bot delivery, inbox items, or search indexing (see the
quiet contract on Message). Rename notes are rate-limited to one per
room per minute; repeated renames still land the latest name. Adds,
renames, and leaves also re-render every member's sidebar row and room
header live, so newcomers see the group and everyone sees the new name
without reloading.

One-to-one history stays private: adding members to a one-to-one DM is
rejected, so groups are always born from the selection path, never by
widening a private conversation. A group that shrank to two members keeps
its identity (and any custom name) instead of collapsing into a
one-to-one DM.

## Member-set lookup

`Rooms::Direct.find_or_create_for` matches on the exact member set through
an indexed `direct_member_key`: `dm:` plus the SHA-256 of the sorted
member ids, under a partial unique index (NULL keys and deleted rooms
stay outside it). Adding or removing a member recomputes the key. A
mutated group whose new set collides with another room's key keeps its
own history under a room-suffixed key — only the create/open-from-selection
path ever reuses a room; membership changes never merge two rooms.

## Group calls

Every DM can huddle. Starting one rings every other human member with the
same banner-plus-push flow as one-to-one calls, and unanswered rings
become per-recipient missed-call inbox items after the ring window. Grants
go only to current members: removing a member revokes their grants
immediately, and their join tokens stay denied afterwards. See
[huddles](huddles.md) for invitations and the
[authorization boundary](huddle-enforcement.md) for the enforcement model.
