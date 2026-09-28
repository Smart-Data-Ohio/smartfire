# Slack import

Move a Slack Pro workspace into Smartfire: an administrator creates an internal Slack app, connects it, dry-runs the migration, then imports. Members connect their own Slack accounts to bring over personal history (DMs, group DMs, private channels).

For how Slack objects map to Smartfire records (users, placeholders, rooms, messages, threads), see [Slack import mapping](slack-import-mapping.md).

## Why an internal app

Slack Pro's built-in export covers public channels only, so the importer reads through the Slack Web API instead. The workspace administrator creates an **internal Slack app from a manifest** that Smartfire generates (`/account/slack_import`). Internal apps keep Tier 3 rate limits (50+ requests per minute) on `conversations.history` and `conversations.replies`; the stricter 2025 limits (1 request per minute, 15 messages) apply only to commercially distributed non-Marketplace apps. See the [Slack rate-limits changelog](https://docs.slack.dev/changelog/2025/06/03/rate-limits-clarity/).

The importer uses **user tokens only** (`xoxp-`, from OAuth v2 `user_scope`). A user token reads every public channel — even ones the member never joined — plus the private channels, DMs, and group DMs the member is in. No bot user, no bot scopes, no events. The requested user scopes are:

`channels:history channels:read groups:history groups:read im:history im:read mpim:history mpim:read users:read users:read.email team:read`

Files are not imported. Each Slack file becomes a line `📎 [file name](permalink)` in its message, so nothing silently vanishes.

## Setup (administrators)

Open **Account settings → Slack import** (`/account/slack_import`) and work the checklist:

1. **Create the Slack app.** At [api.slack.com/apps](https://api.slack.com/apps) choose Create New App → From a manifest, pick the Slack workspace, paste the manifest from the page, and create the app. Then open Basic Information and copy the Client ID and Client Secret.
2. **Save credentials.** Paste them into the form (sudo-gated, like other secret pages). The secret is write-only: the page never renders it back, only offers a replacement field. Saving records `slack.workspace.configure` in the audit log.
3. **Connect your Slack account.** OAuth through the new app. The first admin connection names the workspace being migrated; later grants from any other Slack team are rejected. A grant missing any required scope is rejected with the missing scopes listed.
4. **Dry run and import.** See below.

"Remove Slack credentials" deletes the app credentials and every member's connection (blocked while a run is active). Run history stays for review.

## The smoke-test path

Runs are listed newest-first at `/account/slack_import/runs`, with kind, mode, status, starter, and time. One import or undo runs at a time across the workspace; a member's run waits behind another member's ("queued behind another import"), and undo waits while any other run is queued, running, or undoing (the run page says so until the way is clear).

1. **Dry run.** From the setup page: "Include private channels I'm in" (default on) plus an optional date range. Writes nothing; reports what an import would do.
2. **Review the plan.** From the completed dry run: a table of conversations (name, type, archived, members, messages, threads) with a checkbox per row (default on), a target select (New room, Skip, or an existing Open/Closed room, preselected from the dry run's suggestion), select all/none, and converted samples showing Slack text next to the Markdown Smartfire will render.
3. **Test import.** From the plan: the checked conversations, recent messages (oldest defaults to 14 days ago). Undoable.
4. **Verify in the app.** Read the imported rooms, check members, threads, reactions, and pins.
5. **Undo or keep.** "Undo import" removes everything the run created — rooms, messages, reactions, pins — and nothing else. Threads and rooms that gained newer activity stay (with all their memberships), as do records the run only matched to existing accounts or rooms (a member found by email, a merged room). Available on imports that stopped (completed, failed, cancelled).
6. **Full import.** From the plan: the checked conversations, all messages, no date bounds.
7. **Catch-up at cutover.** On a completed full import, "Run catch-up import" repeats the same conversations and targets to pick up what changed in Slack since. Safe to repeat: already-imported objects are skipped, never duplicated. Only a completed full import opens the 30-day catch-up window — a date-bounded test import leaves the later full import to re-read the whole range (duplicates are still skipped).

Run pages show status, phase, current conversation, counts, the people summary, API calls, issues, timestamps, and errors, and refresh live while the run is active. Cancel stops a run at its next step boundary.

## Members: importing personal history

Tell members to open their profile → **Import from Slack** (`/slack/imports`) once the workspace is connected (until then the page says an administrator needs to set up Slack import first). The page explains the scope: their DMs, group DMs, and private channels, each visible in Smartfire to the other members of it. Connect, run a **Preview** (a personal dry run), review the per-conversation plan, then **Import**. Members see and act only on their own personal runs; administrators can view, cancel, and undo every run from the admin runs pages.

A DM imported by one participant is never duplicated when the other participant imports it: runs share one mapping table. Personal runs never merge into a pre-existing room — a private channel whose name matches one gets its own new room — except for rooms an earlier run already mapped and Direct rooms.

## Placeholders and claiming

Slack members without a Smartfire account become claimable placeholders (active users, matched by email). They claim the account by signing in with Google using the same email address — allowed automatically when the email's domain is in Google sign-in's allowed domains — or an administrator sends them the transfer link from their profile. People deleted in Slack, guests, and bots/apps become deactivated authors: their names stay on the history but they cannot sign in.

If a member signed up with a different email than their Slack one, matching misses and their Slack identity becomes a separate placeholder. Fix that before import by changing the member's Smartfire email to the Slack one (the member can do this from their profile page, password-confirmed).

## Removing credentials afterwards

After cutover, "Remove Slack credentials" on the setup page deletes the app credentials and every connection. Members' **Disconnect** buttons stay available for individual cleanup. Imported history is unaffected either way.

## Troubleshooting

- **Rate limits.** The importer backs off and retries through Slack's limits; a run slowed by throttling still finishes. Check the run's API-call count and issues if one stalls.
- **Token revoked.** If Slack rejects a connection (revoked token, removed app), reconnect from the setup page (admins) or the personal import page (members). A rejection mid-run fails the run with an error; fix the grant and re-run.
- **Missing scopes.** A grant without every required scope is rejected at connect time with the missing scopes listed. Reconnect and approve them all — the import cannot run on a partial grant.
- **Wrong workspace.** Grants from a different Slack team than the first admin connection are rejected. Connect with an account in the migrated workspace.
- **Undo scope.** Undo removes what the run created only. Threads and rooms that members posted in afterwards stay (with all their memberships), and records the run matched rather than created stay too.
- **Undo waits its turn.** Undo never runs while another import is queued, running, or undoing. If the button is disabled, wait for the other run to finish.
- **Private channels need a target to merge.** A private channel never merges into an existing room by name. To import one into an existing Closed room, pick that room in the plan's target select; otherwise the import creates a new room.
