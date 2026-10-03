# WS11-UI approved differences

## Oversized password re-confirmation replay cookies

Approved by the lead in the next-2 instructions on 2026-10-02: keep Rust's
302 instead of copying Rails' cookie-overflow crash for 1,001- and 1,002-digit
integers in the GitHub connection password re-confirmation request.

The pinned Rails oracle (`d7c7de92`) records 500 for these first requests in
`vectors/bot-ui-casting-followups.json`, under `size`. Rust still redirects to
password re-confirmation with 302, discarding optional replay parameters when
the actual session cookie would overflow. Integer digits remain exact and are
counted without JSON string quotes. Ordinary numeric strings retain their quotes.

The independent parameter-size boundary matches Rails: 2,037 integer digits
produce 2,048 serialized parameter bytes and remain storable; 2,038 digits
produce 2,049 bytes and are rejected for replay. The cookie preview adds the
real serialized session overhead rather than treating that parameter limit as
a promise that the cookie fits.

Regressions in `controllers/accounts/bots/casting_followups_tests.rs`:

- `ws11ui_casting_followups_integer_replay_size_counts_digits_without_quotes`
- `ws11ui_casting_followups_huge_integer_challenge_keeps_approved_302`

The Rails 500 stays in the corpus. This approved response difference is explicit
in the HTTP test; it is not replaced with a Rust-derived expectation.
