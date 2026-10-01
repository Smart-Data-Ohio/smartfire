#!/usr/bin/env bash
# The approved ten-file Rails oracle overlay. Every other Rails input stays at the pin.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT="$ROOT/../.scratch/status-image"
mkdir -p "$OUT"
python3 "$ROOT/reference-tools/users/verify_post_pin.py"
python3 - "$ROOT" "$OUT/Dockerfile" <<'PY'
from pathlib import Path
import json,sys
root=Path(sys.argv[1]);ledger=json.loads((root/'reference-tools/users/post-pin/source-hashes.json').read_text())
lines=['FROM ws8br2-reference:d7c7de92','LABEL parity.owner="ws8br2" parity.status_reference="2e20b24c"']
for source in ledger:
    target=source if source.startswith(('app/','config/')) else 'app/views/'+source
    lines.append('COPY '+source+' /rails/'+target)
lines.append('RUN rm -rf /rails/public/assets && SECRET_KEY_BASE_DUMMY=1 SKIP_TELEMETRY=1 bin/rails assets:precompile')
Path(sys.argv[2]).write_text('\n'.join(lines)+'\n')
PY
docker build --label parity.owner=ws8br2 -t ws8br2-reference:d7c7de92-status-2e20b24c \
  -f "$OUT/Dockerfile" "$ROOT/reference-tools/users/post-pin"
printf 'WS8br2 status oracle image: d7c7de92 plus exactly the 10 approved 2e20b24c source files; assets precompiled\n'
