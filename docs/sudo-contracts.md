# JSON reauthentication contracts

Sensitive writes return HTTP 403 with `ApiError::SudoRequired` when the session's
`sudo_verified_at` is outside the retained 15-minute window. The error includes
`reauthentication.methods` (`password`, `totp`, `google`) and `reauthentication.retry`.
The retry names the original HTTP method, request path with query, and SPA `returnTo`.
The server stores no JSON write body in the cookie and does not execute the pending write.

The JSON endpoints require the same authenticated session and CSRF token as retained forms.
Their responses use `SudoResponse` from `crates/api_types/src/auth.rs`.

| Endpoint | Response |
| --- | --- |
| `GET /api/v1/sudo` | 200 `ready` with methods and pending retry |
| `POST /api/v1/sudo` with `{"kind":"password","password":"secret"}` or `{"kind":"totp","code":"123456"}` | 200 `confirmed` with retry; consumes the pending request |
| `POST /api/v1/sudo/google` | 200 `navigate` with the Google authorization URL |
| `GET /api/v1/sudo/continue` | 200 `confirmed` with retry if verification is fresh; otherwise 403 `ready` |

Password and TOTP refusals return 401 `error` with "Confirmation failed. Try again."
Unavailable methods and malformed submissions return 422 `error`.
The HTML and JSON starts share ten attempts per IP in three minutes, including Google starts.
The next attempt returns 429 `error` with the retained wording.
TOTP also shares its existing replay prevention, lockout, and refusal of recovery codes.
All responses disable caching.

Google confirmation started through JSON returns to `/app/sudo/continue`.
That SPA route belongs to slice 70: it consumes the boot flash for failures and calls
`GET /api/v1/sudo/continue`. Google starts through retained forms keep their existing
continuation, even if a JSON write caused the gate. The callback keeps its one-use state,
nonce, PKCE, identity, and fresh `auth_time` checks.

The frontend functions are `readSudo`, `submitSudo`, `startSudoGoogle`, and `resumeSudo`.
Slice 70 owns the dialog, pending-body preservation across Google navigation, and retry.
After `confirmed`, the client reissues the original write with its saved body and current
CSRF token. A consumed retry is `null`; continuation never repeats the write itself.
