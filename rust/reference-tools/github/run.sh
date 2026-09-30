#!/usr/bin/env bash
# Run GitHub vectors/rollback verification in our pinned Rails image; no network requests.
set -euo pipefail
RUST_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TASK_ROOT=$(cd "$RUST_ROOT/.." && pwd)
SCRATCH="$TASK_ROOT/.scratch/ws15g"
mkdir -p "$SCRATCH"
MODE=${1:-generate}
case "$MODE" in generate|verify|webhooks) ;; *) echo "usage: $0 [generate|verify|webhooks]" >&2; exit 2 ;; esac
python3 - "$TASK_ROOT" "$SCRATCH/reference-hashes.json" <<'PY'
import hashlib, json, pathlib, subprocess, sys
paths = ['app/models/github/app.rb', 'app/models/github/write_client.rb', 'app/models/github_connected_account.rb', 'app/controllers/github/app_connections_controller.rb', 'config/initializers/active_record_encryption.rb', 'config/application.rb', 'Gemfile.lock', 'app/controllers/github/webhooks_controller.rb', 'app/models/github/webhook_delivery.rb', 'app/models/github/notifier.rb']
hashes = {p: hashlib.sha256(subprocess.check_output(['git', '-C', sys.argv[1], 'show', 'd7c7de92:' + p])).hexdigest() for p in paths}
pathlib.Path(sys.argv[2]).write_text(json.dumps(hashes))
PY
rm -f "$SCRATCH/$MODE.sqlite3" "$SCRATCH/$MODE.sqlite3-wal" "$SCRATCH/$MODE.sqlite3-shm"
GITHUB_FIXTURE_SECRET=$(sed -n 's/^SECRET_KEY_BASE=//p' "$RUST_ROOT/parity/.env.reference")
SCRIPT=generate.rb
if [[ "$MODE" == verify ]]; then SCRIPT=verify_rust.rb; fi
if [[ "$MODE" == webhooks ]]; then SCRIPT=webhooks.rb; fi
docker run --rm --name "ws15g-github-$MODE" --entrypoint '' --user "$(id -u):$(id -g)" \
  -e RAILS_ENV=production -e RAILS_LOG_LEVEL=fatal -e SECRET_KEY_BASE="$GITHUB_FIXTURE_SECRET" \
  -e DISABLE_SSL=1 -e SKIP_TELEMETRY=true -e DATABASE_URL="sqlite3:/work/scratch/$MODE.sqlite3" \
  -e GITHUB_REFERENCE_HASHES=/work/scratch/reference-hashes.json -e GITHUB_VECTOR_PATH=/work/vectors/github.json \
  -e GITHUB_WEBHOOK_VECTOR_PATH=/work/vectors/github_webhooks.json \
  -e GITHUB_RUST_OUTPUT=/work/scratch/rust-account.json \
  -v "$SCRATCH:/work/scratch" -v "$RUST_ROOT/reference-tools/github:/work/scripts:ro" -v "$RUST_ROOT/vectors:/work/vectors" \
  "${WS15G_REFERENCE_IMAGE:-ws6-reference-d7c7de92}" bin/rails runner "/work/scripts/$SCRIPT"
