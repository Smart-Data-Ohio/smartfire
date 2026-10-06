#!/usr/bin/env python3
"""Prove the thirteen declarations use compiled WS17 decisions and real adapters."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch/ws13b-policy-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
mutations = [
    ("quiet-policy-bypassed", "db/src/models/notification_policy.rs", "self.base_push() && (!recipient.quiet_for_push(self.now) || self.dnd_exception)", "self.base_push() && (!recipient.quiet_for_push(self.now) || true)", "campfire_db", "huddle_"),
    ("caller-allowance-removed", "db/src/models/notification_policy.rs", "|| self.dnd_exception)", "|| false)", "campfire_db", "huddle_"),
    ("quiet-override-removed", "db/src/models/huddle_invitations.rs", "return Ok(!check(&recipient));", "let _ = check(&recipient);", "campfire_db", "huddle_ring_policy_seam_test::policy_quiet_override"),
    ("push-bridge-removed", "db/src/models/notification_push.rs", "match request.kind {", "return Ok(false); #[allow(unreachable_code)] match request.kind {", "campfire_db", "huddle_join_push_sequences_test"),
    ("ring-handler-unregistered", "campfire/src/jobs/huddle.rs", "    registry.register(ring);", "    // deliberately missing ring handler", "campfire", "registered_ring_worker_uses_current_ws17_policy_and_exact_rails_frames"),
]
env = dict(os.environ, CARGO_BUILD_JOBS="2", TMPDIR=str(ROOT / ".scratch"),
           CARGO_TARGET_DIR=str(ROOT / "rust/target"), CI="1",
           CABLE_TEST_PORT_RANGE="53000-53049", MAIL_TEST_PORT_RANGE="53050-53099")
for name, relative, before, after, package, test in mutations:
    path = ROOT / "rust/crates" / relative
    original = path.read_text()
    try:
        assert original.count(before) == 1, (name, original.count(before))
        path.write_text(original.replace(before, after, 1))
        result = subprocess.run(["cargo", "test",
                                 "--locked", "--manifest-path", str(ROOT / "rust/Cargo.toml"),
                                 "-p", package, test, "--", "--test-threads=8"],
                                cwd=ROOT, env=env, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summary = re.search(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode != 0 and summary and "could not compile" not in output and "panicked at" in output, output[-5000:]
        print(f"{name}: {summary.group()}", flush=True)
    finally:
        path.write_text(original)
print(f"WS13b real-policy discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
