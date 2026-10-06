# Smartfire deterministic seeds

The seeds the Rust tests use are committed and frozen (see "Frozen seeds" at the end); the
rest of this file records how the Rails app built them.

Seeds came from the **root Smartfire Rails app** and its current fixtures/models. Each is a
SQLite database, Active Storage tree and flat labels.json. Both targets copy the same snapshot.
Content/IDs/times/password digests/blob keys are stable; encrypted columns and pending browser
cookies use real random IVs. Rebuilds are semantically deterministic, not claimed to be
byte-identical SQLite files. Build once and copy to both targets.

```sh
export PARITY_NAMESPACE=ws19 PARITY_OWNER=ws19
rust/parity/bin/reference build
rust/parity/bin/seed build default unread first_run live_rooms imports
rust/parity/bin/reference up --seed default --port 49001 --time 2026-03-02T16:00:00Z --freeze
rust/parity/bin/check-inventory --seed default --port 49001
rust/parity/bin/reference runner --seed default --time 2026-03-02T16:00:00Z --freeze rust/reference-tools/campfire/verify_parity_seed.rb
rust/parity/bin/reference down --port 49001
```

The Parity::Seed DSL supplies fixtures, based_on, at, label and deterministic media helpers.
default.rb retains the media corpus then loads smartfire.rb. Loaded-layer guards allow
smartfire to be built directly too. Explicitly build the five gate seeds; older optional
custom_styles/restricted/crowd fixture tools do not add states to this inventory.

| Seed | Scenario |
|---|---|
| default | Markdown/reply/forward/reference/pin, active/closed/locked threads, polls/votes, saved/scheduled history, direct/group DMs, voice/stage/board, four work states/owner/tags/result, events, bot/agent/approval/ledger, inbox mentions/replies/invitations, non-default status/notification/category preferences, enrolled 2FA/remembered devices, admin and recorded integration cards |
| unread | default plus sidebar unread badges, viewed from profile so room presence does not mark them read |
| first_run | Empty schema; real first-run form |
| live_rooms | default plus participant, speaker and live stream; synthetic gateway env from reference_env.* labels on both targets |
| imports | default plus workspace/personal Slack previews and failed run, with recorded conversation/sample payloads |
| agents_ui | default plus an owner-managed agent, active/expired/revoked credentials, enforced and revoked grants, pending/denied/overdue approvals, failed webhook ledger delivery, live status and two work-thread steps. Labels use the `ui` or `agent_ui` suffix. Required by the navigation/inbox request tests and agent page captures; it preserves default's no-grant legacy behavior. |

The retained media corpus covers rich text/code/tables/mentions, old SGIDs, sounds/unfurls,
Twitter cards, image/video/files, boosts, bot/deactivated authors, edits and a busy timeline.
Variants/previews are preprocessed in the reference's libvips/ffmpeg runtime. External images
resolve to fixture bytes. GitHub PRs, Fizzy viewer caches, X posts, link embeds and LinkedIn
excerpts have recorded rows fresh at the frozen clock and synthetic credentials. They never
fetch live data while building or capturing.

## Clock and authentication

NOW is 2026-03-02 16:00:00 UTC. Writes use explicit instants. Seed runner containers have no
network and discard jobs; callbacks cannot fetch or execute queued work. Captures freeze app
and browser wall clocks while monotonic scheduling continues. Locale/time zone/fonts/browser
versions are fixed.

Human users have enrolled 2FA. Real remembered-device cookies are labelled
two_factor_cookies.<user>. Gate page contexts use stable verified Session rows and
session_cookies.<user> so concurrent logins cannot add unordered session/audit/inbox rows.
Pending challenge cookies come from a real Rails password POST; enrollment has a deterministic
setup secret. Clear Rails' execution context after the nested integration request before
additional seed layers run.

Default memberships start read. Room presence clears unread state and can race initial sidebar
rendering; unread.rb observes badges from profile instead. Every mutating capture gets a fresh
snapshot copy.

## Reference processes and network

The repository root Dockerfile builds the app. The parity layer leaves Rails source unchanged
and runs a private local Redis, Thrust/Puma with embedded Action Cable, and Resque Pool with one
default worker and one slack_import worker. Render-triggered local jobs can settle without a
CPU-dependent worker count.

Periodic and the huddle reconciler are omitted. Their settled outcomes (scheduled history,
fresh caches, participants/activity) are seeded: running them against a frozen clock would
repeatedly mutate state or attempt external connections. Their execution needs separate
integration tests. Runtime Resque remains real and app containers use an internal bridge with
no Internet route. Capture containers have --network none; host-loopback forwarding supplies
only ingress. Package downloads are build-time operations.

The parity-only cable initializer uses one worker so unsubscribe/subscribe commands execute in
order. A migration timestamp initializer allows later-dated migrations at the frozen clock.
libfaketime is built without FAKE_PTHREAD so waits remain real and Puma/Resque do not spin.
Namespace images/containers/network with PARITY_NAMESPACE and ownership with PARITY_OWNER.
Native/host-browser modes are conveniences, not acceptance runs.

## Frozen seeds (what the Rust tests and CI use)

Rails no longer runs anywhere in the Rust workflow. The `default`, `first_run`, `agents_ui` and
`ledger_originals` seeds were built once by the Rails app as described above (from the pin in
`parity/reference.sha`, validated by its seed validator at the frozen clock) and committed
under `frozen/`, with `frozen/manifest.json` recording every file's SHA-256, a digest of the
test keys in `parity/.env.reference` and each seed's `schema_migrations`.

```sh
python3 rust/parity/bin/frozen-seeds check              # hashes, keys and schema match this checkout
python3 rust/parity/bin/frozen-seeds restore            # check, then copy them to parity/.seed/NAME
python3 rust/parity/bin/frozen-seeds migrate CAMPFIRE   # run CAMPFIRE db-migrate on each, then record
python3 rust/parity/bin/frozen-seeds record             # rewrite the manifest after a deliberate change
```

`check` fails when a seed file changed, appeared or disappeared, when the keys changed, when
`crates/db/src/schema_migrations.txt` has a migration the seeds lack, or when the seeds have one
the build doesn't know. After adding a migration, build `campfire` and run `migrate` with it:
the seeds get the schema change the same way production databases do, and the manifest is
rewritten. The `Rust seeds` job runs `check` and its unit tests (`parity/test_frozen_seeds.py`);
every other job restores the seeds through the setup action's `parity: seeds`.

Every app seed loader fails if its seed is missing and `CI` is set, even to an empty value.
Locally it may return early with a clear skip message, so run `frozen-seeds restore` first.
`first_run` is required by the account creation test, `agents_ui` by the navigation/inbox
tests, `ledger_originals` by the ledger browser tests, and `default` by the other request,
browser and cable tests.
