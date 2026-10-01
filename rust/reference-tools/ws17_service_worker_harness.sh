#!/usr/bin/env bash
# Execute the unchanged Rails harness against bytes returned by the Rust HTTP controller test.
# WS17_SERVICE_WORKER_OUTPUT must have been set to .scratch/service-worker-served.js for that run.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
HARNESS=$ROOT/.scratch/sw-harness
mkdir -p "$HARNESS/test/scripts" "$HARNESS/app/views/pwa"
cp "$ROOT/test/scripts/service_worker_harness.mjs" "$HARNESS/test/scripts/service_worker_harness.mjs"
cp "$ROOT/.scratch/service-worker-served.js" "$HARNESS/app/views/pwa/service_worker.js"
cmp "$ROOT/test/scripts/service_worker_harness.mjs" "$HARNESS/test/scripts/service_worker_harness.mjs"
cmp "$ROOT/app/views/pwa/service_worker.js" "$HARNESS/app/views/pwa/service_worker.js"
node "$HARNESS/test/scripts/service_worker_harness.mjs"

# Demonstrate that caching authenticated HTML/API responses is detected. Mutate only the
# scratch copy, then restore the exact Rust response and run the harness again.
python3 - "$HARNESS/app/views/pwa/service_worker.js" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
source = p.read_text()
anchor = 'return url.pathname === OFFLINE_URL || url.pathname.startsWith("/assets/")'
assert anchor in source
p.write_text(source.replace(anchor, "return true", 1))
PY
if node "$HARNESS/test/scripts/service_worker_harness.mjs" > "$ROOT/.scratch/service-worker-injected.log" 2>&1; then
  cp "$ROOT/.scratch/service-worker-served.js" "$HARNESS/app/views/pwa/service_worker.js"
  echo "authenticated-cache injection escaped" >&2
  exit 1
fi
cp "$ROOT/.scratch/service-worker-served.js" "$HARNESS/app/views/pwa/service_worker.js"
grep 'service worker harness check(s) failed' "$ROOT/.scratch/service-worker-injected.log"
node "$HARNESS/test/scripts/service_worker_harness.mjs"
