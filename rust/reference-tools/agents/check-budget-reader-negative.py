#!/usr/bin/env python3
"""Reject a wrong typed reader observable and always restore the exact oracle."""
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys

root = Path(__file__).resolve().parents[3]
logs = Path(sys.argv[1]).resolve()
logs.mkdir(parents=True, exist_ok=True)
assert os.environ.get('CARGO_TARGET_DIR'), 'supply an owned target'

def interrupted(*_):
    raise KeyboardInterrupt

signal.signal(signal.SIGTERM, interrupted)
path = root / 'rust/vectors/agent_budget_notice_reader.json'
original = path.read_bytes()
try:
    value = json.loads(original)
    value['results']['owner'][1]['cap_label'] = 'incorrect board-post label'
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    log = logs / 'budget-reader-negative.log'
    with log.open('w') as output:
        result = subprocess.run(['mise', 'exec', 'rust@1.98.1', '--', 'cargo', 'test', '--locked', '-j2', '--manifest-path', 'rust/Cargo.toml', '-p', 'campfire_db', 'ws11_next3_budget_notice_typed_reader_matches_rails', '--', '--test-threads=8'], cwd=root, stdout=output, stderr=subprocess.STDOUT)
    summaries = re.findall(r'^test result:.*$', log.read_text(), re.M)
    assert result.returncode == 101 and summaries, result.returncode
    assert '0 passed; 1 failed;' in summaries[-1], summaries
    print('WS11 typed budget reader negative control: ' + summaries[-1])
finally:
    path.write_bytes(original)
    assert path.read_bytes() == original
print('WS11 typed budget reader: wrong label rejected; exact oracle bytes restored')
