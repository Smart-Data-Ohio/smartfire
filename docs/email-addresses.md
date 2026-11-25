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
| `POST /:id/primary` | Empty | 204. Requires sudo and an owned secondary address. The old primary is released. Smartfire queues a confirmation link for a free old primary when outgoing mail is available at promotion. Delivery checks mail availability again. |

Management failures use `{error: {_tag, message, ...}}`. The request determines the tag and
status.

| Condition | Status | Tag and fields |
| --- | --- | --- |
| Missing member session | 401 | `Unauthorized` |
| Sudo required | 403 | `SudoRequired` with `reauthentication` |
| Unknown or unowned address ID, or add without mail | 404 | `NotFound` |
| Invalid address or a promotion that conflicts with another account's primary | 422 | `Validation` with `fields.emailAddress` |
| Malformed request body | 422 | `Validation` with an empty `fields` map |
| Missing or stale CSRF token | 422 | `InvalidAuthenticityToken` |
| Per-member request limit exceeded | 429 | `RateLimited` with `retryAfter: 3600` and the `Retry-After` header |

Confirmation failures use `{kind: "error", fieldErrors: {...}}`. An invalid, expired or inactive
account link returns 400 with `base`. A confirmation attempt while signed in to a different account
also returns 400 with `base`. If another account claims the address before confirmation,
`PUT` returns 422 with `emailAddress: ["This email address is already linked to another account."]`.
An account can confirm up to 10 secondary addresses. Confirming another address at that limit
returns 422 with `base`. An existing confirmation can be retried while its address stays linked.

## Confirmation and mail

Adding requires SMTP, a valid `APP_URL` and `MAILER_FROM`. Otherwise the list shows
`canAdd: false` and the add request returns 404. Valid add requests are limited to five per member
per hour. Each response echoes the trimmed address and queues a job. The job sends only if the
member is still active, the address is free, and the request is still valid. The link opens
`/app/email_addresses/confirm/:token` and expires 24 hours after the request. A password change
retires earlier requests. Removing a secondary or replacing a primary retires earlier requests
for that address across accounts. Smartfire compares audit IDs and folds address spelling.
Deactivation retires every earlier request for the member, even after an unban. It also retires
earlier requests from other members for every secondary and any primary it releases. Deactivation
records have their own sequence, which links and queued jobs also carry. Smartfire refuses jobs
and links issued before it recorded audit IDs.

Secondary address claims ignore surrounding whitespace and use Unicode case folding. Password
sign-in and reset delivery first look for an exact primary or confirmed secondary address. If
neither matches, they use the claim rules across both address sources and require one active
account. Google sign-in, Slack import, and inbound mail keep their primary address comparisons.
They match a confirmed secondary by its spelling, ignoring surrounding whitespace and ASCII case.
If a lookup misses and the claim rules find an address already in use, Google refuses sign-in
and Slack marks the import failed. Inbound mail treats a sender with a missed lookup as unknown.

A password sign-in through a secondary checks that its linked row still belongs to the member
when the session is created, including after two-step verification. A password sign-in still
waiting for its second step when this check is deployed must start again. A reset matched to a
secondary goes to its confirmed spelling. Removing the secondary retires reset links issued to
that address, including after re-linking.
