#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
PARITY_NAMESPACE=ws15g PARITY_OWNER=ws15g PARITY_RUNTIME=docker PARITY_IMAGE=${WS15G_SEED_IMAGE:-bbdfcdcb98b2} \
  "$ROOT/parity/bin/reference" runner --seed default --time 2026-03-02T16:00:00Z --freeze \
  -e GITHUB_SEED_FRAGMENTS=/work/vectors/github_seed_fragments.json "$ROOT/reference-tools/github/seed_fragments.rb"
python3 - "$ROOT" <<'PY'
import hashlib,json,pathlib,subprocess,sys
root=pathlib.Path(sys.argv[1])
v=json.loads((root/'vectors/github_seed_fragments.json').read_text())
for path,actual in v['sources'].items():
    expected=hashlib.sha256(subprocess.check_output(['git','-C',str(root.parent),'show','d7c7de92:'+path])).hexdigest()
    assert actual==expected,path
print('GitHub seed fragment source hashes: 4 pinned files match')
PY
