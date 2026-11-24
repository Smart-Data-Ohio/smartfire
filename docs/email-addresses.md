# Linked email addresses

Each member keeps a primary email address and can confirm additional addresses. Password sign-in,
password reset, eligible Google sign-in, Slack import and authenticated inbound room mail can find
the member through a confirmed address. Notifications and directory entries use the primary.
Existing Google linking eligibility and inbound sender authentication still apply.

## API

Each suffix starts at `/api/v1/email_addresses`. Use `X-CSRF-Token` for writes.
The management routes require a member session. A confirmation link works from a signed-out
browser that loads the confirmation page and uses that browser's CSRF token. The page names
the account the address will join.

| Method and suffix | JSON request | Success |
| --- | --- | --- |
| `GET` | Empty | 200 `{primary: string or null, secondary: [{id: number, emailAddress: string, createdAt: string}], canAdd: boolean}`. Dates are UTC ISO 8601. |
| `POST` | `{emailAddress: string}` | 200 `{kind: "confirmationSent", emailAddress: string}`. Requires sudo. |
| `GET /confirmations/:token` | Empty | 200 `{emailAddress: string, accountName: string, requiresSignIn: boolean, accountMismatch: boolean}`. A signed-in viewer whose account differs from the link's sees `accountMismatch: true`. |
| `PUT /confirmations/:token` | Empty | 200 `{kind: "navigate", location: string}`. Goes to `/app/settings` with a session for the link's account or `/app/session/new` without a session. Both carry a flash notice. |
| `DELETE /:id` | Empty | 204. Requires sudo and an owned secondary address. |
| `POST /:id/primary` | Empty | 204. Requires sudo and an owned secondary address. The old primary is released. If it is free, Smartfire queues a confirmation email so its inbox can link it again. |

Management failures use `{error: {_tag, message, ...}}`. The request determines the tag and
status.

| Condition | Status | Tag and fields |
| --- | --- | --- |
| Missing member session | 401 | `Unauthorized` |
| Sudo required | 403 | `SudoRequired` with `reauthentication` |
| Unknown or unowned address ID, or add without mail | 404 | `NotFound` |
| Invalid address or a promotion when another legacy account shares the current primary | 422 | `Validation` with `fields.emailAddress` |
| Malformed request body | 422 | `Validation` with an empty `fields` map |
| Missing or stale CSRF token | 422 | `InvalidAuthenticityToken` |
| Per-member request limit exceeded | 429 | `RateLimited` with `retryAfter: 3600` and the `Retry-After` header |

Confirmation failures use `{kind: "error", fieldErrors: {...}}`. An invalid, expired or inactive
account link returns 400 with `base`. A confirmation attempt while signed in to a different account
also returns 400 with `base`. If another account claims the address before confirmation,
`PUT` returns 422 with `emailAddress: ["This email address is already linked to another account."]`.

## Confirmation and mail

Adding requires SMTP, a valid `APP_URL` and `MAILER_FROM`. Otherwise the list shows
`canAdd: false` and the add request returns 404. Valid add requests are limited to five per member
per hour. Each response echoes the trimmed address and queues a job. The job sends only if the
member is still active, the address is free, and neither an address removal nor a password change
has superseded the request. The link opens `/app/email_addresses/confirm/:token` and expires
24 hours after the request. A valid link can be retried while the address stays linked. Removing
the address or changing the password retires earlier confirmation links.

Secondary address claims ignore surrounding whitespace and use Unicode case folding. Sign-in
and integrations keep their primary address comparisons. Password reset tries the exact primary
first, then compares primary and secondary addresses using the claim rules. A fallback must
identify one active account. A reset matched to a secondary goes to its confirmed spelling.
Removing the secondary retires reset links issued to that address, including after re-linking.
Deactivation releases secondary addresses.
