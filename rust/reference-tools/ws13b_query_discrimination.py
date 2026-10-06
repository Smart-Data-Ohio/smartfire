#!/usr/bin/env python3
"""Detect compiled SQL, transaction-lock and association-cache regressions."""
import os
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-query-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
mutations = [
    ("per-viewer-reload", "models/huddle_notices.rs", "    for membership in memberships {", "    for membership in memberships {\n        let _ = human(tx.conn(), membership.user_id)?;", 2),
    ("deferred-lock", "database.rs", 'conn.execute_batch("BEGIN IMMEDIATE TRANSACTION")?;', 'conn.execute_batch("BEGIN DEFERRED TRANSACTION")?;', 2),
    ("preload-cache-bypassed", "models/stage.rs", "Some(streams) => Ok(streams.first().cloned()),", "Some(_streams) => Stream::live_for_room(conn, self.room.id),", 2),
    ("nonstage-stream-lookup", "models/huddle_grant.rs", "        if last_stage_grant {\n", "        if last_stage_grant || true {\n", 1),
]
environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1", CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
detected = set()
for name, file, before, after, expected in mutations:
    path = ROOT / "rust/crates/db/src" / file
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["cargo", "test", "--locked", "-j4", "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", "campfire_db", "huddle_query_assertions_test", "--", "--test-threads=4"], cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = next((s for s in re.findall(r"^test result: FAILED\..*$", output, re.M) if f"{expected} failed;" in s), None)
        assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
        detected.update(re.findall(r"^test tests::huddle_query_assertions_test::(\w+) \.\.\. FAILED$", output, re.M))
        print(f"{name}: {summary}", flush=True)
    finally:
        path.write_text(original)
assert len(detected) == 7, detected
print(f"WS13b query discrimination: {len(mutations)} compiled regressions detected across 7 original remainder traces; sources restored", flush=True)
