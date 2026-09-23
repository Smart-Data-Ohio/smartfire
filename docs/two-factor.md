# Two-step sign-in

Every human signs in with two steps: their password or Google sign-in,
then a 6-digit code from an authenticator app. This applies to Google
sign-in too — Smart Data's Workspace does not require 2-Step
Verification, so Google alone does not count as a second factor. Bots
and agents authenticate with keys and tokens and are exempt.

The codes follow TOTP (RFC 6238) and work with Google Authenticator,
1Password, Authy, and any compatible app.

Enforcement runs inside every session restore, not just the normal
sign-in flow: API and Turbo Stream requests, Drive listings, workspace
icons, and uploads all require a session that completed the second
factor. There is no partially signed-in state to exploit.

## Setting it up

The first time you sign in, you land on the setup page before anything
else. API and Turbo Stream requests cannot bypass it either: they are
rejected until setup is done.

1. Open your authenticator app and add an account.
2. Scan the QR code, or enter the manual key if you cannot scan.
3. Enter the 6-digit code from the app.

Every visit to the setup page issues a fresh secret bound to your
current session, and confirming only ever uses that session's pending
secret — a secret displayed in one browser cannot be confirmed from
another. Pending secrets expire after 15 minutes. A mistyped code does
not rotate the secret, so you can retry without re-scanning.

Next you get **10 backup codes**. Each one signs you in once if you
lose your authenticator. Copy or download them now: they are shown only
once and are stored as one-way digests, so nobody can show them to you
again later. You can make a fresh set at any time from your profile
("New backup codes"), which invalidates the old set.

## Signing in

After your password, Google, or transfer-link sign-in, enter the code
from your app — or one of your backup codes if you lost your phone.
Until the code checks out you have no session at all: the first step
alone opens nothing.

Tick **"Remember this device for 30 days"** to skip the code on a
browser you trust. The remember cookie is bound to a server-side record
and stops working if it is revoked or expires. Changing your password
revokes every remembered device.

Codes tolerate about 30 seconds of clock skew either way, and a code
cannot be used twice. After a few wrong guesses the challenge slows you
down with rate limiting. After 5 wrong codes in a row the challenge
locks: first for 1 minute, then 5, then 15, with each lockout taking 5
fresh failures. Any successful sign-in clears the count. Every lockout
is recorded in the audit log and lands in your inbox ("Several wrong
sign-in codes were entered for your account."), plus an email once
outbound mail is configured (see below).

## Your profile

The profile page shows whether two-step sign-in is on, your remembered
devices (each with a **Revoke** button, plus **Revoke all**), and two
actions:

- **New backup codes** replaces your backup codes. The old set stops
  working immediately.
- **Disable** turns two-step sign-in off — and because it is enforced,
  you land straight back on the setup page to set it up again. Use this
  to move to a new phone: sign in with a backup code, disable, and
  re-enroll.

Each of these — new codes, disabling, revoking one device, revoking
all — asks for your current authenticator code or your password in the
same request, so anyone holding your signed-in browser cannot take over
the account through them. Members without a password (provisioned
through Google) can instead **Confirm with Google**, which proves the
same linked Google account again and arms exactly one action for the
next 10 minutes. Backup codes never count as confirmation. These
actions are rate limited, and disabling, regenerating, and admin resets
drop every live connection, so open tabs and sockets reconnect as
verified sessions or not at all.

## Lost phone or codes (recovery)

1. Sign in with one of your backup codes, then disable and re-enroll
   from your profile as above.
2. If the backup codes are gone too, ask an administrator to reset your
   two-step sign-in (below). You will set it up again at your next
   sign-in.

## For administrators

The account page shows a key button next to every member who has
two-step sign-in on. **Reset** destroys their authenticator secret,
backup codes, and remembered devices, and signs them out everywhere;
they re-enroll at their next sign-in. You cannot reset your own
two-step sign-in this way — use Disable on your profile instead.

Every sensitive step is recorded in the [audit log](audit-log.md):
`two_factor.enable`, `two_factor.disable`, `two_factor.reset`,
`two_factor.backup_codes.regenerate`, `two_factor.reauthenticate`,
`two_factor.devices.revoke_all`, `sign_in.two_factor.failure` for wrong
codes, and `sign_in.two_factor.lockout` for challenge lockouts. Codes
and secrets never reach the log.

## How it is stored

- Authenticator secrets are encrypted at rest with Active Record
  encryption, like the other OAuth tokens.
- Unconfirmed setup secrets live in their own table, one row per
  session at most, encrypted at rest, and are spent at confirm time.
- Backup codes exist as SHA-256 digests only; a database read cannot
  turn them back into codes.
- Remember cookies carry a random token whose digest alone is stored,
  so a database read cannot mint remember cookies.
- Expired remembered devices and expired setup secrets are pruned by
  the daily retention job.

## Local development

The remember cookie is `Secure`, so browsers only send it over HTTPS
and Rails does not even set it over plain HTTP. On a plain-HTTP
development server, ticking "Remember this device" has no effect and
every sign-in asks for a code. Production serves HTTPS, where it works
normally. The test suite enables cookie write-through so integration
and browser tests exercise the real cookie.

## Lockout email

Lockout notices always land in the member's inbox. They are also
emailed (via `TwoFactorMailer`) once outbound mail is configured, which
currently means `action_mailer.smtp_settings` is present. The sender
defaults to `Smartfire <noreply@smartdata.net>`; confirm it with your
mail provider when enabling SMTP.
