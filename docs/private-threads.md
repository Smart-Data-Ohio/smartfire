# Private threads

A thread's visibility is `public` or `private`, set at creation. Existing threads stay public. The database rejects changes to visibility. The migration adds no jobs.

`ChannelThread::readable_by` is the access resolver. Its `readable_by_sql` predicate also filters paginated queries and counts. Reading requires an active user with membership in the live parent room, and either a public thread, an access membership, or the administrator role. Creating a private thread adds its creator. Leaving removes that grant; being the creator alone does not permit reads. A creator who retains parent-room access may still invite and remove people, including inviting themselves back. An invitation response to a creator without read access contains only the grant they just made; it does not disclose the roster. Agents and bots need explicit membership in private threads, plus the capabilities their existing APIs require. Owning work or following a thread grants no access.

`thread_access_memberships` stores access. `thread_memberships` stores notification preferences and read state. An invitation initializes notification state to mentions without replacing an existing mute or follow preference. Unfollowing or deleting `/join` preserves access. Removing access clears unread state and disconnects existing sockets. Reconnecting checks access again. Invitations and removals are transactional and idempotent.

The JSON APIs are:

- `POST /api/v1/rooms/:room_id/threads` accepts optional `visibility`, defaulting to `public`.
- `GET /api/v1/threads/:thread_id/members` lists access memberships and their users.
- `POST /api/v1/threads/:thread_id/members` accepts `userId`. The creator, an access member, or an administrator may invite an active member of the parent room.
- `DELETE /api/v1/threads/:thread_id/members/:user_id` removes access. The creator or an administrator may remove others. A member may remove themselves.
- `POST` and `DELETE /api/v1/threads/:thread_id/join` change notification membership only.

`ThreadAccess` errors carry `notPrivate`, `inviteForbidden`, `removeForbidden`, `parentRoomRequired`, or `visibilityImmutable`. An inaccessible thread returns the ordinary `NotFound` error without its title or roster. PATCH requests containing `visibility` are refused before applying other changes.

A private-thread message or edit invites active parent-room users named by resolved mentions before unread, inbox, push, and agent delivery. A forward invites mentions in its new note; snapshot mentions do not invite. A moderator's edit uses the current moderator as the inviter.

Reader filtering covers thread and message APIs, classic JSON, room indicators, search, saved items, inbox activity, badges, pins, work and forwarding pickers, cards, sync subscriptions and replay, attachment URLs, and agent context, history, events and webhooks. Every queued delivery rechecks access. Private pins write their note inside the thread. Shared quote-card payloads omit private sources; authorized previews use a per-viewer fetch. Shared board digests omit private posts because the persisted root note would outlive a recipient's private access.

Private media retains a marker after messages are deleted and while derivative purge jobs wait. Original, representation, preview and disk URLs authenticate and use private caching. Signed identities cannot restore revoked access. Message edits check the current actor. Private signed blobs cannot be reused directly as profile or workspace branding uploads. An explicit authorized forward makes a new blob copy for its destination.

The app has no channel or message content export. Audit CSV exports contain supported audit targets rather than thread content. Slack imports create public threads; their status APIs report import plans, counters and issues. They do not read newly created private-thread content.

The frontend publishes generated visibility and invitation types, Effect schema pins, and member routes in the mock backend. This slice adds no private-thread UI. Mock binary downloads still use their existing upload capability model; runtime byte-serving authorization is covered by HTTP tests.
