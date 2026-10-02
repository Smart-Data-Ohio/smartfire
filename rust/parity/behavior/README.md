# Message behavior parity

`parity/bin/behavior` drives the same assertions against an already-running Rails or Rust
server. It uses `Dockerfile.playwright` and the locked package file, the ordinary origin
proxy and seeded sessions. It creates no screenshots and compares no pixels. The browser
has no external network. Both apps are presented as `http://localhost:3999`.

Run from the worktree root against **isolated copies** of the default seed. The apps and
browser use `2026-03-02T16:00:00Z`; the browser uses UTC and a 1400×1400 viewport. Build
the seed independently with the pinned reference image:

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 \
  rust/parity/bin/seed build default first_run
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 \
  rust/parity/bin/reference up --seed default --port 52500 --time 2026-03-02T16:00:00Z --freeze
```

The reference's internal Docker network needs `parity/bin/forward-port`. `reference up`
starts it in the background. In an execution environment that terminates background
children when the launching command ends, keep the forwarder in a separate foreground
session (stop the old forwarder first if it is still listening):

```bash
ws8bm2_ip=$(docker inspect ws8bm2-reference-52500 --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}')
python3 rust/parity/bin/forward-port 52500 "$ws8bm2_ip"
```

For Rust, copy the built default seed into `.scratch/behavior-rust` while its server is
stopped, then start the compiled binary with the committed test environment. Do not use
a personal database:

```bash
cp -a rust/parity/.seed/default .scratch/behavior-rust
set -a
source rust/parity/.env.reference
set +a
CAMPFIRE_STORAGE_PATH="$PWD/.scratch/behavior-rust" \
CAMPFIRE_FILES_PATH="$PWD/.scratch/behavior-rust/storage" \
CAMPFIRE_FROZEN_TIME=2026-03-02T16:00:00Z HTTP_PORT=52501 TARGET_PORT=52503 \
  rust/target/debug/campfire
```

Prepare the source tests' model fixtures with Rails against both isolated storage copies,
before any browser sequence. `fixtures.rb` grants Bender membership, validates two agent
commands, clears the default seed's pre-existing pins/David saves, and makes the closed
poll's once-future deadline due through the same `update_columns` used by the source case.
It freezes its own Ruby clock explicitly, including when run with `--port` (`docker exec`
does not inherit Puma's libfaketime environment):

```bash
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 \
  rust/parity/bin/reference runner --storage .scratch/behavior-rust --time 2026-03-02T16:00:00Z --freeze \
  rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json
PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92 \
  rust/parity/bin/reference runner --port 52500 \
  rust/parity/behavior/fixtures.rb /work/parity/.seed/default/labels.json
```

The runner leaves app startup/reset/cleanup to the caller; it cleans its browser,
forwarder, fixture process and socket directory. Run each suite once on each freshly
prepared app. Use a short owned cache path for the Unix fixture socket. For Rust:

```bash
export PARITY_NAMESPACE=ws8bm2 PARITY_OWNER=ws8bm2 PARITY_IMAGE=ws8bm2-reference:d7c7de92
export PARITY_SCRATCH=/home/riels/.cache/rust-port/ws8bm2/browser
for ws8bm2_suite in scheduled slash search-files polls pins; do
  ws8bm2_args=()
  if [[ "$ws8bm2_suite" == slash ]]; then
    ws8bm2_args+=(--database /work/.scratch/behavior-rust/db/production.sqlite3 \
      --fixture-storage .scratch/behavior-rust)
  fi
  bash rust/parity/bin/behavior "$ws8bm2_suite" --target http://127.0.0.1:52501 --name Rust \
    --labels .seed/default/labels.json "${ws8bm2_args[@]}"
done
```

For Rails use port 52500/name Rails, database
`/work/rust/parity/.seed/.instances/52500/db/production.sqlite3` and fixture storage
`rust/parity/.seed/.instances/52500`. These paths are the independent app copies, never
a personal database. `--database` opens SQLite **read only** to prove event arguments and
custom status persisted. The test-only Unix fixture bridge allows one operation: register
a validated Rails agent command after page load, matching the source test's direct model
setup. It exposes no app endpoint. Browser submission then exercises each app's real HTTP
dispatcher; this does not claim parity of the WS11-owned registration REST API.

Coverage is all 45 source cases: scheduled 4, slash 26, search/files 4, polls 4 and
pins/saves 7. The seventh pins/saves case (`pins-dst`) needs separate freshly seeded
servers **and browser** at `2025-11-01T16:00:00Z`; its browser zone is New York. Run with
`--instant 2025-11-01T16:00:00Z --database <isolated app database>` to prove tomorrow's
09:00 resolves to `2025-11-02T14:00:00Z` after the fall-back.

Each case uses a new browser context. Public HTTP creates run each app's validations and
callbacks for polls, pins and messages. Assertions scope new messages by persisted
`data-message-id` or exclude earlier DOM IDs. Pin cases create fresh JZ-authored copies of the source message in the latest window,
so their live notes land in that window. Other old fixtures/closed polls use the anchored
room URL when the populated timeline has paged them out. The done assertion scopes the
saved row's status (the filter navigation already contains a "Done" link), so an in-flight
PATCH cannot be mistaken for its completed response. The Files
case creates a real attachment and Drive row. The quote case uses three users and checks
the lazy frame's private chip versus the member's full card. No provider network endpoint
is substituted by those stored fixture rows.

`SystemTestHelper#join_room`'s 15-second cable setup barrier, Capybara's two-second
selector default and the source cases' explicit ten-second waits are retained. The
source's intentional two-second delayed fetch remains an interleaving stimulus. There
are no timing retries, altered application assets, or skipped browser assertions.

The driver also waits (under its existing two-second setup bound) for the lazy
`composer`, `markdown-editor` and `markdown-autocomplete` controllers to connect before
emitting its first input. Cable readiness alone does not imply input-handler readiness.
The identical reference autocomplete controller installs handlers on connect but does
not search content filled before that connect; the one input event is lost.

`picker-readiness` holds the autocomplete module until the cable barrier is complete.
With `--legacy-setup`, it fills before releasing the module: both Rails and Rust fail
at the unchanged 2,000 ms picker assertion with no autocomplete request. Without that
flag, it releases the module, waits for controller readiness and fills: both pass. This
regression changes no application assets and does not retry or lengthen the picker wait.
