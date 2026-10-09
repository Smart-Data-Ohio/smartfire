# Smartfire test seeds

The Rust tests and CI run against four committed, frozen seeds under `frozen/`: `default`,
`first_run`, `agents_ui` and `ledger_originals`. Each is a SQLite database
(`db/production.sqlite3`), an Active Storage tree (`storage/`) and a flat `labels.json` that maps
fixture labels to ids, cookies and other values the tests look up.

They were built once by the original Rails app, from the commit in `parity/reference.sha`, so
every row, attachment, variant and preview, cookie and encrypted column is byte-for-byte what
Rails wrote. The Rails app has since been removed from the repository, so the seeds are never
rebuilt: schema changes are applied to them with `campfire db-migrate`, the same way production
databases get them. The Ruby seed builders are in the git history (before the Rails removal) if
the provenance of a row is ever in question.

| Seed | Scenario |
|---|---|
| default | Markdown/reply/forward/reference/pin, active/closed/locked threads, polls/votes, saved/scheduled history, direct/group DMs, voice/stage/board, four work states/owner/tags/result, events, bot/agent/approval/ledger, inbox mentions/replies/invitations, non-default status/notification/category preferences, enrolled 2FA/remembered devices, admin and recorded integration cards |
| first_run | Empty schema; real first-run form |
| agents_ui | default plus an owner-managed agent, active/expired/revoked credentials, enforced and revoked grants, pending/denied/overdue approvals, failed webhook ledger delivery, live status and two work-thread steps. Labels use the `ui` or `agent_ui` suffix. It preserves default's no-grant legacy behavior. |
| ledger_originals | default plus the rows the ledger browser tests start from |

The media corpus covers rich text/code/tables/mentions, old SGIDs, sounds/unfurls, Twitter
cards, image/video/files, boosts, bot/deactivated authors, edits and a busy timeline. Variants
and previews were made with the same libvips/ffmpeg builds the production image ships. GitHub
PRs, Fizzy viewer caches, X posts, link embeds and LinkedIn excerpts have recorded rows fresh at
the frozen clock and synthetic credentials; nothing fetches live data.

## Clock and authentication

NOW is 2026-03-02 16:00:00 UTC; tests that need it freeze the app's clock there. Human users
have enrolled 2FA. Real remembered-device cookies are labelled `two_factor_cookies.<user>`, and
stable verified sessions `session_cookies.<user>`. The cookies and encrypted columns depend on
the test keys in `parity/.env.reference`, so those keys are part of the seeds' manifest.

## Using the frozen seeds

`frozen/manifest.json` records every file's SHA-256, a digest of the test keys in
`parity/.env.reference` and each seed's `schema_migrations`.

```sh
python3 parity/bin/frozen-seeds check              # hashes, keys and schema match this checkout
python3 parity/bin/frozen-seeds restore            # check, then copy them to parity/.seed/NAME
python3 parity/bin/frozen-seeds migrate CAMPFIRE   # run CAMPFIRE db-migrate on each, then record
python3 parity/bin/frozen-seeds record             # rewrite the manifest after a deliberate change
```

`check` fails when a seed file changed, appeared or disappeared, when the keys changed, when
`crates/db/src/schema_migrations.txt` has a migration the seeds lack, or when the seeds have one
the build doesn't know. After adding a migration, build `campfire` and run `migrate` with it:
the seeds get the schema change the same way production databases do, and the manifest is
rewritten. The `Rust seeds` job runs `check` and its unit tests (`parity/test_frozen_seeds.py`);
every other job restores the seeds through the setup action's `parity: seeds`.

The migration tool explicitly preserves pre-existing foreign-key violations: the default and
agents_ui seeds deliberately include an unrenderable message whose creator is missing. It uses
`db-migrate --preserve-existing-foreign-key-violations`; every new violation still rolls back the
migration. Normal `db-migrate DATABASE` remains strict.

Every app seed loader fails if its seed is missing and `CI` is set, even to an empty value.
Locally it may return early with a clear skip message, so run `frozen-seeds restore` first.
`first_run` is required by the account creation test, `agents_ui` by the navigation/inbox
tests, `ledger_originals` by the ledger browser tests, and `default` by the other request,
browser and cable tests.
