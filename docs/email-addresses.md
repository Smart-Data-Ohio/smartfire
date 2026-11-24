# Linked email addresses

Each member keeps a primary email address and can confirm additional addresses. Password sign-in,
password reset, eligible Google sign-in, Slack import and authenticated inbound room mail can find
the member through a confirmed address. Notifications and directory entries use the primary.
Existing Google linking eligibility and inbound sender authentication still apply.

## API

Each suffix starts at `/api/v1/email_addresses`. Use `X-CSRF-Token` for writes.
The management routes require a member session. A confirmation link works from a signed-out
browser that loads the confirmation page and uses that browser's CSRF token.

| Method and suffix | JSON request | Success |
| --- | --- | --- |
| `GET` | Empty | 200 `{primary: string or null, secondary: [{id: number, emailAddress: string, createdAt: string}], canAdd: boolean}`. Dates are UTC ISO 8601. |
| `POST` | `{emailAddress: string}` | 200 `{kind: "confirmationSent", emailAddress: string}`. Requires sudo. |
| `GET /confirmations/:token` | Empty | 200 `{emailAddress: string, requiresSignIn: boolean}`. The boolean indicates whether the viewer will need to sign in after confirming. |
| `PUT /confirmations/:token` | Empty | 200 `{kind: "navigate", location: string}`. Goes to `/app/settings` with a session or `/app/session/new` without one. Both carry a flash notice. |
| `DELETE /:id` | Empty | 204. Requires sudo and an owned secondary address. |
| `POST /:id/primary` | Empty | 204. Requires sudo and an owned secondary address. The old primary becomes a secondary if present. |

Management failures use `{error: {_tag, message, ...}}`. The request determines the tag and
status.

| Condition | Status | Tag and fields |
| --- | --- | --- |
| Missing member session | 401 | `Unauthorized` |
| Sudo required | 403 | `SudoRequired` with `reauthentication` |
| Unknown or unowned address ID, or add without mail | 404 | `NotFound` |
| Invalid address or a promotion that would link a legacy shared primary | 422 | `Validation` with `fields.emailAddress` |
| Malformed request body | 422 | `Validation` with an empty `fields` map |
| Missing or stale CSRF token | 422 | `InvalidAuthenticityToken` |
| Per-member request limit exceeded | 429 | `RateLimited` with `retryAfter: 3600` and the `Retry-After` header |

Confirmation failures use `{kind: "error", fieldErrors: {...}}`. An invalid, expired or inactive
account link returns 400 with `base`. If another account claims the address before confirmation,
`PUT` returns 422 with `emailAddress: ["This email address is already linked to another account."]`.

## Confirmation and mail

Adding requires SMTP, a valid `APP_URL` and `MAILER_FROM`. Otherwise the list shows
`canAdd: false` and the add request returns 404. Valid add requests are limited to five per member
per hour. Each response echoes the trimmed address and queues a job. The job sends only if the
member is still active and the address is free. The 24-hour link opens
`/app/email_addresses/confirm/:token`. Confirmation records the address and can be safely retried
with the same link. An unexpired confirmation link can attach the address again after removal.

New address claims ignore surrounding whitespace and ASCII case. Each sign-in or integration
path keeps its primary address comparison. A reset requested through
a secondary goes to its confirmed spelling. Removing the secondary retires reset links issued
to that address, including after re-linking. Deactivation releases secondary addresses.
