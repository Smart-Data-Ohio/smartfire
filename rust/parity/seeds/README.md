# Smartfire deterministic seeds

Seeds come from the **root Smartfire Rails app** and its current fixtures/models. Each is a
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

## Rust CI seeds

The Rust test workflow builds `default`, `first_run` and `agents_ui` from the full Rails SHA in
`parity/reference.sha` (currently `78b9b1546`). `parity/bin/ci-seed prepare` archives that commit
into ignored `parity/.ci/reference`, then overlays the checkout's `db/schema.rb` and
`db/migrate/` so seeds match the schema required by the Rust build. Rails behavior,
fixtures and bundle stay pinned; migrations still run only through Rails.
The overlay must remain compatible with the pinned application and Rails runtime.
Incompatible changes, such as removed or renamed columns, new required values, or
migrations that need newer application code, require a pin bump and revalidation.
`ci-seed image` builds or loads the canonical reference image, `ci-seed build` creates all three
seeds, and `ci-seed validate` runs the Rails validator at the frozen seed clock every time.
The same four commands can be run locally from any directory.

The image cache holds a Docker archive. Its exact identity includes the Rails pin (covering
reference code, fixtures, Dockerfile and bundle inputs), the checkout's schema and migrations,
parity Docker inputs and the build/cache tooling. The seed identity also includes all seed scripts, the fixed environment
and the Rails validator. There are no fallback keys. Local image archives are reused only
when their recorded image cache key matches all current inputs and their SHA-256 checksum
matches the receipt; stale archives or missing receipts trigger a rebuild.
The image's embedded Rails revision is
checked after load, and cached seeds still undergo Rails validation in every run: the `Rust seeds`
job validates the exact cache entry the test and correctness jobs restore, and both gates
(`Rust port`, `Rust correctness`) fail unless it succeeded. A job whose exact entry is missing
builds and validates its own seeds.
Only pushes to main save caches; PRs read them and use no application secrets.

Every app seed loader fails if its seed is missing and `CI` is set, even to an empty value.
Locally it may return early with a clear skip message. `first_run` is required by the account
creation test, `agents_ui` by the navigation/inbox tests, and `default` by the other request and cable tests. The CI setup summary
prints elapsed time and both cache-hit flags for cold/warm comparisons.
