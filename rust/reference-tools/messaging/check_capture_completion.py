#!/usr/bin/env python3
"""Move the real capture before completion; require its deterministic rejection.

The callback is queued on the test's current-thread runtime. The old comparator
must see an empty batch before any await. Fixture, producer and deadlines stay
unchanged. Sources are restored even when the negative control is rejected.
"""
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[3]
runner = sys.argv[1:]
if runner[:1] == ['--']:
    runner = runner[1:]
assert runner
path = ROOT / 'rust/crates/campfire/src/controllers/message_features/comparison_support.rs'
original = path.read_bytes()
try:
    text = original.decode()
    wait = '    frames(app, client, expected, context).await;\n'
    assertion = '    assert_eq!(json!(actual), *expected, "ordered publication differs from Rails: {context}");\n'
    assert text.count(wait) == 1 and text.count(assertion) == 1
    text = text.replace(wait, '').replace(assertion, assertion + wait)
    path.write_text(text)
    r = subprocess.run(runner + ['test','--locked','-p','campfire','--bin','campfire',
                                'ordered_capture_waits_for_actual_callback','-j2','--',
                                '--test-threads=4','--nocapture'], cwd=ROOT, text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(r.stdout, flush=True)
    assert r.returncode != 0 and 'ordered publication differs from Rails: deferred actual callback' in r.stdout
    assert 'left: Array []' in r.stdout
    print('WS8bm2 capture completion mutant: rejected deterministically before actual callback publication', flush=True)
finally:
    path.write_bytes(original)
