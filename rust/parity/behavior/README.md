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

The runner leaves app startup/reset/cleanup to the caller; it cleans its own browser,
forwarder and socket directory. Execute each suite once per app, starting the sequence
from fresh seed copies. Repeating the files case on the same database creates duplicate
upload fixtures, so reset before repeating a sequence.

```bash
for ws8bm2_app in Rails Rust; do
  if [[ "$ws8bm2_app" == Rails ]]; then ws8bm2_port=52500; else ws8bm2_port=52501; fi
  for ws8bm2_suite in scheduled slash search-files; do
    PARITY_NAMESPACE=ws8bm2 bash rust/parity/bin/behavior "$ws8bm2_suite" \
      --target "http://127.0.0.1:$ws8bm2_port" --name "$ws8bm2_app" \
      --labels .seed/default/labels.json
  done
done
```

Coverage is 4/4 scheduled, 20/26 slash and 4/4 search/files cases. The remaining slash
cases are registered-agent listing, argument insertion without execution, immediate
argumentless execution, registration after page load, ephemeral execution with persisted
arguments, and persisted custom status. Polls (4) and pins/saves (7) remain to be added.

Each named case uses a new browser context. Public HTTP creates replace the reference
tests' direct model fixture creation and run each app's callbacks. Assertions scope new
messages by persisted `data-message-id` or exclude earlier DOM IDs; message DOM IDs use
`client_message_id`, not the database ID. The Files case creates a real attachment and a
Drive row, then uses the room-shell Files link and filters. The quote case uses three
real users and checks the lazy frame's private chip versus the member's full card.

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
