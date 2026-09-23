# GitHub App identity

Members connect GitHub through the workspace GitHub App instead of
pasting a personal access token. The App issues short-lived user-to-server
tokens with refresh; pasted fine-grained PATs keep working as a fallback
during migration (see [Connect your GitHub account](github.md#connect-your-github-account)).

## Creating the App (GitHub org settings)

Do this once per GitHub organization, outside this repo:

1. In the org: Settings → Developer settings → GitHub Apps → New GitHub App.
2. Name it (for example `smartfire-workspace`), set the homepage URL to the
   workspace root (for example `https://smartfire.example.com`).
3. Under **Identifying and authorizing users**, set the callback URL to
   `<app root URL>/github/app/callback`
   (for example `https://smartfire.example.com/github/app/callback`).
4. Request user authorization on install, and expire user authorization
   tokens (short-lived tokens with refresh).
5. Permissions (user-level, same as the PAT fallback):
   - **Pull requests: Read and write**
   - **Issues: Read and write** (PR comments use the issues API)
   - **Metadata: Read** (included automatically)
6. Install the App on the org (or selected repositories), then copy the
   **Client ID** and generate a **Client secret**.

On the app host, set:

- `GITHUB_APP_CLIENT_ID`
- `GITHUB_APP_CLIENT_SECRET`

While either is missing, the App flow is disabled cleanly: the connect
routes answer 404, the profile offers only the PAT form, and the
[integration health page](#integration-health-page) names the variables
to set. Nothing errors.

## What connecting does

**Connect with GitHub** on the profile page runs the standard OAuth web
flow with signed state. The callback exchanges the code, validates the
token with `GET /user`, and stores the confirmed login with the encrypted
short-lived token, its refresh token, and its expiry. The token is never
logged or shown again; unlinking deletes it.

## Refresh, visibility, and agents

- Expired App tokens refresh in place wherever they are used (PR cards,
  write actions, agent actions), rotating the stored refresh token. A
  rejected refresh marks the account disconnected with a reconnect prompt;
  a transport failure records `last_error` and the next call retries.
- Per-viewer PR card visibility uses the viewer's own token, refreshed
  first: cards for private repositories render only for viewers whose
  linked account can read the repository.
- Agent GitHub actions use the owner's App token when the owner has a
  usable one, else the agent's own linked account
  (`Github::AgentIdentity`). The request, the approval card ("Acts on
  GitHub as @login"), and execution all resolve the same identity, so the
  decider's approval can never run as a different GitHub user.
- Tokens are encrypted at rest, never passed as job arguments (jobs take
  account and approval ids), and revocation is best effort (the save
  proceeds however revocation goes): a full disconnect deletes the whole
  App authorization (`DELETE /applications/{client_id}/grant`), while a
  relink revokes only the replaced token
  (`DELETE /applications/{client_id}/token`) so the new token survives.

## Integration health page

Administrators get an **Integration health** page under Account settings
showing GitHub (workspace token, App configuration, webhook secret,
connected account counts with App/PAT split, disconnected accounts,
recent errors, 24h webhook deliveries, PR fetch errors), Google Calendar
(connected/disconnected accounts, calendar entry errors, push channel
count and expiry), Fizzy (not configured yet — a seam for a future
integration), webhook/agent delivery (pending and failed counts with
recent errors), and forward-to-room email (domain configuration and rooms
with addresses). Every unconfigured item names the variable or step that
enables it.
