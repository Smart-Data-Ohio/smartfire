#!/usr/bin/env python3
"""Replay the full relative-consumer oracle, retaining the owner-blocked failure.

This diagnostic is not a passing parity gate or a fixture mask. The normal test
explicitly covers the 56 unblocked requests. This runs all 80 against the real
shared message renderer and requires the first New York broadcast to differ.
"""
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[3]
runner = sys.argv[1:]
if runner[:1] == ['--']:
    runner = runner[1:]
assert runner
parent = ROOT / 'rust/crates/campfire/src/controllers/message_features.rs'
module = parent.parent / 'message_features/relative_renderer_probe.rs'
assert not module.exists()
original = parent.read_bytes()
try:
    module.write_text(r'''#[tokio::test]
async fn relative_renderer_owner_probe() {
    let vector = serde_json::from_str(include_str!("../../../../../vectors/messaging/relative_consumers.json")).unwrap();
    super::container_input_tests::compare_feature_input_requests(vector).await;
}
''')
    parent.write_bytes(original + b'\n#[cfg(test)]\nmod relative_renderer_probe;\n')
    r = subprocess.run(runner + ['test','--locked','-p','campfire','--bin','campfire',
                                'relative_renderer_owner_probe','-j2','--',
                                '--test-threads=4','--nocapture'], cwd=ROOT, text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(r.stdout, flush=True)
    assert r.returncode != 0 and 'ordered publication differs from Rails: container actual publications America/New_York/slash_0' in r.stdout
    assert 'avatar?v=20260302160000' in r.stdout and 'avatar?v=20260302110000' in r.stdout
    assert '2026-03-02T16:00:00Z' in r.stdout and '2026-03-02T11:00:00-05:00' in r.stdout
    print('WS8bm2 relative renderer: owner-blocked UTC timestamp/avatar mismatch reproduced against real Rails frames; no parity credit', flush=True)
finally:
    parent.write_bytes(original)
    module.unlink(missing_ok=True)
