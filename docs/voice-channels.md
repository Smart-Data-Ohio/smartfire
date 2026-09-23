# Persistent voice channels

A voice channel is a room with a standing huddle: members join and leave at
will, and nobody has to start a call. Each channel shows who is currently in
the call from the sidebar and the room header, and carries its own text chat
with durable history.

## Membership

Voice channels are a room type (`Rooms::Voice`) with closed-room membership:
explicit members chosen by an administrator or the creator, managed through
the same members UI as closed rooms. Voice rooms keep their type: unlike open
and closed rooms, they never convert to or from another type. Mention
notifications are the default. Stage channels (`Rooms::Stage`) share the
closed membership, the never-converts rule, and the mention default; see
[stage channels](stage-channels.md).

## Text chat

The room page is the ordinary room page — messages, composer, threads — with
the voice presence and join controls in the header. People who are not in the
call read and write the same history as people who are.

## Presence

Who is in the voice channel is derived server-side from active `HuddleGrant`s
the gateway has seen within the last 20 seconds (`HuddleGrant.in_call`, the
same liveness signal DM ringing uses). Members can read it as JSON from
`GET /rooms/:room_id/huddle/participants` (`[{ id, name, avatar_url }]`), and
it renders as an avatar stack in the sidebar row and in the room header.

The stacks refresh over Turbo Streams when a grant is issued, revoked,
first seen in the call, or reported left: the panel reports explicit leaves
and the gateway reports disconnects after its reconnect grace, so the
stacks clear within seconds. Leaving the call does not revoke the
session's grant. Grants that quietly expire with no report — a dead tab the
gateway never saw — fall off through a 15-second browser refresh, which is
the only polling in the feature. On phones the header stack shows at
most three avatars plus the count, yields its width before the room name
shrinks, and steps aside entirely below 360px.

## Joining and leaving

The room header button reads **Join voice** and **Leave voice**. Joining uses
the same `huddle:join` event and huddle panel as every other room, so audio,
screen share, camera, device controls, and reconnect behave identically.
Leaving goes through the panel's own Leave control. Voice and stage rooms
never create invitations, ringing, or missed-call items.

## Microphone modes and shortcuts

Each member chooses how their microphone opens from the **Calls** section of
their profile: **voice activity** (live whenever unmuted, the default) or
**push-to-talk** (open only while the configured key is held, backtick by
default). The default matches the backtick key by position, so it talks on
international layouts where the backtick is a dead key; custom keys match the
typed character. The push-to-talk key never fires while typing, while
composing text, or with Ctrl, Meta, or Alt held, so holding it in the
composer types the character instead of opening the microphone; switching
browser tabs or hiding the page mid-sentence releases a held key rather than
wedging the microphone open.

**Ctrl/Cmd+Shift+M** toggles the microphone from anywhere in the app while in
a call, including while typing. (The `?` shortcut sheet from the navigation
branch is not on `main` yet; when it lands, both shortcuts belong there.)

## Per-person volume and local mute

Every remote row in the call roster carries a volume slider (0–200%) and a
**Mute for me** button. Both are local only: the slider rides the audio
element up to 100% and a Web Audio gain node above it, and the mute
unsubscribes the person's microphone so the server stops sending it. Both are
remembered per person in the browser. Nobody else hears a difference. The
boost gain follows the speaker picker where the browser routes audio
contexts to an output device; where it cannot, the boost caps at 100% while
a non-default speaker is selected, and the slider tooltip says why.

## Host moderation

Voice rooms have no host role, so only administrators moderate: they can
server-mute any member, including another administrator, which revokes
publish until the member is unmuted, or disconnect a member from the call
without touching their membership. A server-muted administrator can unmute
themselves. Muting
and unmuting rejoin the affected browser with a fresh token, exactly like a
stage publish-boundary change; disconnecting sends no rejoin, so the member
stays out until they join again. Enforcement is server-side through grants —
the muted token cannot publish and the gateway's per-second check drops any
grant that no longer matches the membership — with no separate UI in voice
rooms yet: the same endpoints serve the stage roster's Mute, Unmute, and
Disconnect buttons (see [stage channels](stage-channels.md)).

## Reconnection and quality

When the connection drops, the panel shows a reconnection bar with a countdown
and a **Reconnect now** button, which rejoins fresh instead of waiting for the
SDK's own retry. The countdown expiring never forces a failure: the SDK may
still recover, and the manual control stays up. Each roster row carries a poor-
connection badge while the server reports that participant struggling, and the
header connection indicator keeps its details panel with round-trip time,
loss, jitter, bitrates, and transport for your own link.

Whoever is speaking gets a static ring on their avatar in the sidebar and
header stacks, driven by the call's speaker events with no polling of its own.
There is no pulse, so reduced-motion settings need no exception.

## Sidebar

Voice channels list under the **Voice** section below the channels list,
ordered by name. Stage channels list in the same section with a stage glyph.
Each row shows the live participant count and up to three avatars, with a
subtle live treatment while anyone is in the call.

## Access removal

Removing a member, deleting the room, or deactivating the user ends their
voice session through the existing huddle revocation path, and they disappear
from the participant list. Removal also drops the member's sidebar row and
header stack over their existing rooms stream, and the presence poll treats
its 404 as the end of membership: it stops, clears, and never retries.

## Limits

This slice sets no concurrency cap. The desired voice concurrency is still an
open decision (see the roadmap), to be settled with load testing before
capacity expectations are set.
