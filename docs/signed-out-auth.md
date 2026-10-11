# Signed-out authentication contracts

Slice 44 adds JSON contracts and a public SPA boot mode. The retained auth pages and
production sign-in navigation keep their current URLs and handlers. This slice adds no
React pages or navigation links.

## Reserved SPA routes

The shell permits visitors without a session only at these auth paths:

| SPA path | Retained page |
| --- | --- |
| `/app/session/new` | `/session/new` |
| `/app/session/transfers/:id` | `/session/transfers/:id` |
| `/app/two_factor/challenge` | `/two_factor_challenge` |

The transfer ID must occupy one nonempty path segment. Other paths under `/app/` still
require authentication. These names follow the existing `/app/` prefix and retained
resource names. The frontend router does not render these pages until slice 45.

A signed-in visitor to the challenge gets the retained page's redirect to `/`. The retained
sign-in and transfer GETs accept signed-in visitors without a redirect, so their SPA
counterparts serve authenticated boot data. A visitor without a session gets public boot
data at all three reserved routes, including a challenge with no pending sign-in.
The challenge JSON endpoint decides whether the pending state is valid.

## Public boot

`GET /api/v1/session/boot` returns the same public DTO the signed-out shell inlines.
The shell also puts its `csrfToken` in the CSRF meta tag. Both responses use `Cache-Control:
no-store`.

```json
{
  "kind": "signedOut",
  "workspace": { "name": "Smart Data", "logoUrl": null, "description": "" },
  "signInMethods": { "password": true, "google": false },
  "firstRunPending": false,
  "csrfToken": "masked token"
}
```

The workspace name is null before an account exists. The description is plain text from
account settings. Password sign-in is always available. Google is available when its
provider configuration is complete. `firstRunPending` uses the retained sign-in page's
check for no users. Unlike `/session/new`, the reserved SPA sign-in shell remains available
before first run so slice 73 can use this flag.

The DTO contains no users, rooms, messages, session identity, pending identity, second-factor
secret, recovery codes, custom workspace CSS, cable endpoint or service-worker URL.
Authenticated `GET /api/v1/boot` keeps its existing contract and access requirement.

## Session endpoints

All writes require the existing session's authenticity token in `X-CSRF-Token`. Typed JSON
request fields use camelCase. The retained handlers and JSON endpoints share session
creation, verification, cookie handling, rate limits, push-subscription removal and audits.

| Method and path | Request | Operation |
| --- | --- | --- |
| `POST /api/v1/session` | `PasswordSignIn`: `emailAddress`, `password` | Password sign-in |
| `POST /api/v1/session/google` | `GoogleSignInStart`: `{}` | Google authorization URL |
| `GET /api/v1/two_factor/challenge` | No body | Current challenge or a navigation action |
| `POST /api/v1/two_factor/challenge` | `ChallengeSubmission`: `code`, `rememberDevice` | TOTP or recovery code |
| `PUT /api/v1/session/transfers/:id` | `TransferSignIn`: `{}` | Verify transfer ID and begin sign-in |
| `DELETE /api/v1/session` | `SignOut`: `pushSubscriptionEndpoint`, nullable | Remove the browser's subscription and session |

Google start uses the retained `/session/google/callback`. Its signed state, nonce, PKCE,
provider restriction and cookie session are unchanged. The callback still completes the
retained flow, including second-factor enforcement.

Transfer IDs are signed user IDs with a four-hour expiry. They are reusable until expiry
and require an active user. JSON preserves that behavior. Recovery codes are single use.

## Next actions and errors

Successful responses use HTTP 200 and the discriminated `AuthResponse` union:

- `signedIn` carries the same validated `location` as the retained success redirect.
- `secondFactorRequired` carries `challenge`, with methods `totp` and `recoveryCode` and
  support for `rememberDevice`. The pending identity remains in the encrypted cookie.
- `navigate` carries the next URL, including Google authorization or a retained auth page.
- `error` carries `fieldErrors`, keyed by `base` or `code`, with the retained user-facing
  rejection text. Invalid transfer IDs add a `base` error because the retained handler
  returns an empty 400. Empty access-gate refusals have an empty field-error map.

Password refusals keep HTTP 401, rate limits keep 429, invalid codes keep 422, and invalid
transfer IDs keep 400. Invalid JSON bodies use 422. CSRF rejection keeps the kit's 422.

Return destinations come from `return_to_after_authenticating` in the cookie session and
use the same SPA mapping and redirect checks as the retained flow. A `return_to` query on
the sign-in page does not override that state. Neither endpoint introduces a body field
for an arbitrary redirect destination.

`frontend/src/api/auth-endpoints.ts` decodes these next actions. Its auth requests interpret
credential 401 responses as field errors and refresh CSRF through `/api/v1/session/boot`.
The existing application client keeps its sign-in redirect for ordinary API 401 responses.
