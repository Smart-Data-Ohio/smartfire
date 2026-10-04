#!/usr/bin/env python3
"""Change real Saved/Scheduled producers; keep fixtures and comparators untouched.

The complete HTTP section assertions must reject both changed room labels.
The send-now control is the unchanged 91cb61210 producer, recorded in the report.
Run with a native Cargo wrapper after any other Cargo command has finished.
"""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
runner = sys.argv[1:]
if runner[:1] == ["--"]:
    runner = runner[1:]
assert runner
originals = {}
try:
    for name, old, new in [
        ("saved_items", "        room_name,\n        author_name:",
         '        room_name: format!("producer-mutant:{room_name}"),\n        author_name:'),
        ("scheduled_messages", "        id: row.id,\n        room_name,",
         '        id: row.id,\n        room_name: format!("producer-mutant:{room_name}"),'),
    ]:
        path = ROOT / f"rust/crates/campfire/src/controllers/{name}.rs"
        raw = path.read_bytes()
        source = raw.decode()
        assert source.count(old) == 1, name
        originals[path] = raw
        path.write_text(source.replace(old, new))
    result = subprocess.run(runner + [
        "test", "--locked", "-p", "campfire", "--bin", "campfire",
        "controllers::message_features::review_229_tests", "-j2", "--",
        "--test-threads=4", "--nocapture",
    ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    print(result.stdout, flush=True)
    assert result.returncode != 0
    assert re.search(r"running 3 tests\b", result.stdout)
    assert "FAILED. 1 passed; 2 failed;" in result.stdout
    for path in ["/saved", "/scheduled_messages"]:
        witness = f"real feature section differs from fresh Rails: {path}"
        assert witness in result.stdout, witness
        print(f"WS8bm2 review229 producer mutant: rejected at {witness}", flush=True)
finally:
    for path, raw in originals.items():
        path.write_bytes(raw)
