# Golden vectors

JSON files recorded from Smartfire's original Rails app (Rails main, `load_defaults 8.2`) running
in production mode inside its pinned image (`parity/reference.sha`). Tests in the Rust crates read
them; never edit them by hand.

They use the fixed `SECRET_KEY_BASE` from `parity/.env.reference` (recorded in each file as
`secret_key_base`). A second secret, `rotated_secret_key_base`, stands in for a rotated
`SECRET_KEY_BASE` in the "wrong secret" cases. The clock is frozen at `now` (2026-01-01T12:00:00Z).

## Frozen

The Rails app and the recorders that wrote these files (`reference-tools/run.sh` and its Ruby
scripts) have been removed from the repository, so the vectors can't be regenerated. They pin what
Rails did; change one only together with the behavior it pins, and say why in the commit. The
recorders are in the git history from before the Rails removal.

Where the app draws randomness (AR encryption IVs, JWT ids, RSA test keys, webhook secrets) the file
records the value drawn, so re-recording would change those bytes but not what the tests assert.
