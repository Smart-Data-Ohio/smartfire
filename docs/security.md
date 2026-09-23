# Security posture

What Smartfire enforces for workspace administrators, and which knobs
exist. For the audit log's action vocabulary see
[the audit log](audit-log.md).

## Content Security Policy

The policy in `config/initializers/content_security_policy.rb` is
**enforced** (not report-only). Browsers block anything outside it and
report violations to `/csp_reports`, which logs one line per violation,
rate-limited, without query strings. The main pages (room, huddle,
stage, board, search, account settings, profile, event form, and the
profile card) are covered by a system test asserting zero violations;
inline scripts carry the session nonce, and there are no inline event
handlers or `javascript:` URLs.

Uploaded SVG icons are served with their own script-blocking policy
(`default-src 'none'`), so even a missed upload-validation vector
cannot run.

## Security headers

Every response carries `X-Content-Type-Options: nosniff`,
`Referrer-Policy: strict-origin-when-cross-origin`, and a
`Permissions-Policy` of `camera=(self), display-capture=(self),
microphone=(self), notifications=(self)`, pinned in
`config/initializers/security_headers.rb`. `notifications` is not a
real Permissions-Policy directive (the Notifications API is gated by
its own user prompt), so browsers log a console note and ignore that
entry; it is listed for completeness.

Production serves HSTS (`max-age=31556952; includeSubDomains`) through
`force_ssl` with explicit `ssl_options` in
`config/environments/production.rb`.

## Sudo mode

Sensitive actions require the member to have confirmed their identity
in the last 15 minutes (`SudoMode::SUDO_TIMEOUT`). The UX is one
prompt, then the action continues: the intercepted request is stashed,
the member confirms once at `/sudo/new`, and then GETs (such as the
audit CSV export) redirect straight back while small non-GETs replay
automatically through an auto-submitting form. Requests carrying
secrets, uploads, or large bodies are never stashed; those return to
the originating page after confirmation and the member resubmits once.

Gated actions:

- member role changes, bans, and deactivation;
- bot and agent credentials (issue, reset, revoke), grants, webhook URL
  changes (only when the URL actually changes), and webhook secret
  resets, including the agent's own GitHub connection;
- GitHub (personal token and GitHub App), Google, and Fizzy account
  connections and disconnections;
- join-code resets;
- custom styles updates;
- audit log CSV export (browsing stays open).

Confirmation is by password, or by Google re-auth for members with a
linked Google identity (Google-only members have no password); the
re-auth forces a fresh Google login and verifies both that the login
happened within the last 5 minutes and that the Google subject
matches the linked identity, so an older login or a different Google
account is rejected. Confirmations are rate-limited and audit-logged
(`sudo.confirm.success`, `sudo.confirm.failure`); a new sign-in
always starts unverified.

### Two-factor hook point

Once enforced TOTP two-step sign-in lands, a TOTP code also confirms
sudo. The seams are ready: `SudoMode.register_verifier(:totp)` opts
the verifier in, `SudoMode.verify_totp` checks the code,
`SudoMode.verifier_available?` gates it on enrollment, and
`sudos/new` renders the TOTP form when `:totp` is offered. Until then
`:totp` answers `:unsupported`.

## Sessions

Members review their active sessions on the profile ("Your sessions"):
device and browser parsed from the user agent, last-active time, sign-in
time, and last IP address. There is no IP geolocation. Each row signs
that session out, and "Sign out of all other sessions" revokes the
rest. Revocation destroys the session row and drops every Action Cable
connection with reconnect on, so a revoked browser's sockets reconnect
and are rejected at connect while surviving sessions resubscribe.
Revocations are audit-logged (`session.revoke`, `session.revoke_others`).

Administrator sessions expire after 7 days idle (members never
expire). Override with `ADMIN_SESSION_IDLE_TIMEOUT_DAYS` (whole days;
unparseable values fall back to 7). Expired sessions are hidden from
the sessions page, destroyed on their next request or cable connect,
and rejected on the next presence heartbeat, so already-open cables
close within seconds of the timeout.

## New-device sign-in alerts

A sign-in from a browser the account never used creates an inbox item
("Security" filter): "New sign-in to your account from &lt;browser&gt;
on &lt;OS&gt;, &lt;time&gt;. Wasn't you? Review your sessions", linking
to the sessions page. Devices are keyed on a long-lived signed cookie
recorded in `user_devices`, not the IP, so travel and carrier NAT do
not alert; browser upgrades do not alert either (the stored user agent
is refreshed on sign-in). The first-ever sign-in never alerts. Device
rows outlive sessions, so signing out everywhere does not reset
first-ever.

When outbound mail is configured, the same alert is also emailed. Mail
needs all three of `SMTP_ADDRESS`, `MAILER_FROM`, and `APP_URL` (for
absolute links); optional `SMTP_PORT` (default 25), `SMTP_DOMAIN`,
`SMTP_USER_NAME`, `SMTP_PASSWORD`, `SMTP_AUTHENTICATION`, and
`SMTP_ENABLE_STARTTLS` (`"false"` disables; on by default). Without
them the inbox item is the whole alert and nothing is sent.

## Dependency updates

`.github/dependabot.yml` runs weekly Bundler and GitHub Actions
updates with minor and patch updates grouped. There is no npm
ecosystem (importmap with vendored pins, no node build) and no Docker
ecosystem: the base image tag is a build arg the updater cannot read,
so Ruby base image bumps stay a manual, reviewed step (see the file's
comment).
