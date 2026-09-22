# Google Calendar publishing

Members can connect their Google account from their profile. Once connected,
every [native event](events.md) they are **going** or **maybe** to appears
on their primary Google Calendar as a private copy. Publishing is one way,
Smartfire to Google: edits made in Google are never read back, and the app
never reads the calendar, free/busy data, or attendees.

A later time change (or title/description edit) updates that same calendar
entry; declining, cancelling, or leaving the room removes it. The event page
shows "Added to your Google Calendar" while a copy exists for the viewer.

## Setup (Google Cloud Console)

For a deployment using sign-in and Drive too, follow the combined
[Google Workspace setup checklist](google-workspace-setup.md), including
publishing status and Workspace administrator approval.

1. Create (or reuse) a project and configure an **OAuth client** of type
   **Web application**.
2. Add an authorized redirect URI: `<app root URL>/google/callback`
   (for example `https://smartfire.example.com/google/callback`).
3. No extra APIs to enable beyond Google Calendar; the app requests
   `openid email https://www.googleapis.com/auth/calendar.events`.
   The `openid email` part is only used to read the account email from
   the `id_token` returned by the token endpoint (issuer, audience, and
   expiry are verified); the app makes no extra API call to learn it.
   Members who opt into [Drive link previews](google-drive.md) grant the
   additional `https://www.googleapis.com/auth/drive.metadata.readonly`
   scope through the same connect flow.
4. Set the client credentials on the app host:
   `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`.
5. Set `APP_URL` to the public application origin, for example
   `https://smartfire.example.com`. Background jobs use it to generate
   complete event and meeting-room links inside Google Calendar entries.
   It must contain a scheme and host, with an optional port, and no path.

These credentials can also support [Google sign-in](google-sign-in.md).
Sign-in uses its own `/session/google/callback` redirect and identity-only
permissions; signing in does not opt a member into Calendar publishing.

When either variable is missing, the profile shows "Google Calendar is not
configured for this workspace" and the connect routes answer 404.

## What is published and when

Connecting is the explicit opt-in: nothing is published for members who
have not connected. Each published entry carries the event title, the
description plus a "From Smartfire" link back to the event, the start/end
in the event's time zone (events without an end default to one hour), and
Google's default reminders. No Google attendees are added.

Publishing is reconciled by `Calendar::SyncEntryJob`, which recomputes the
desired state from the database on every run: an entry exists exactly when
the member is connected with the calendar scope granted, is going or
maybe, the event is not cancelled, and the member is still in the room.
Each occurrence of a recurring event is its own calendar entry. The job is
enqueued when an RSVP is created or changes, when an event's time, title,
or description changes, when an event is cancelled, when an account is
connected (all upcoming going/maybe RSVPs), and when a membership ends.

Transient failures (Google rate limits, timeouts, connection drops) are
retried by the job with backoff; permanent failures are recorded on the
entry and the next change retries. The Google event id is deterministic
per event and member (`campfire` plus base32hex of the packed ids), and
the local row is reserved before the first request, so a retried insert
reuses the same id and concurrent first runs converge through the
insert-conflict path instead of creating duplicates. If Google reports the
account's grant revoked (`invalid_grant`), the account is marked
disconnected, the local entry being synced is dropped immediately (its
remote copy is unreachable), and the profile offers a reconnect instead of
publishing. The same reconnect prompt appears when the stored grant lacks
the calendar scope (deselected at consent) or when the stored tokens can
no longer be decrypted.

Destroying a local entry outside the reconciler (deleting a room,
shrinking a series, deleting an event) enqueues a remote delete for its
Google copy, so remote copies are removed however the destroy was
triggered. A remote copy already gone (404 or 410) counts as deleted.

## Disconnecting

**Disconnect** on the profile immediately removes the connection and every
local calendar entry for that member, then a background job removes the
remote copies and revokes the Google grant, best effort: entries Google
refuses to remove are logged and forgotten.

Deactivating a member removes their room memberships, revokes the Google
grant in the background, marks the Google account disconnected ("Account
deactivated") so no further syncs run for it, and enqueues one cleanup
sync per calendar entry; the reconciler sees no membership and drops the
local rows. Copies already in Google Calendar are left for the member to
remove.

## Token storage

Refresh and access tokens are stored encrypted (`encrypts` on
`GoogleAccount`) with keys derived from `SECRET_KEY_BASE` (see
`config/initializers/active_record_encryption.rb`), so no separate secret
is needed. Rotating `SECRET_KEY_BASE` invalidates every stored Google
token: affected members must reconnect. Tokens are never logged or
rendered.
