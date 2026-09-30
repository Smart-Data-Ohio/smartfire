#!/usr/bin/env python3
"""Run alone: icon parity/CI tests must reject compiled regressions; restore source."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch" / "discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"))


def check(name, path, mutate, test):
    original = path.read_bytes()
    try:
        path.write_bytes(mutate(original))
        result = subprocess.run(
            ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
             "-p", "campfire", "--bin", "campfire", test, "--", "--nocapture"],
            cwd=ROOT / "rust", env=ENV, capture_output=True, text=True,
        )
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        failures = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode and summaries, f"{name}: no failing assertion; see log"
        assert any(t.endswith("::" + test) for t in failures), f"{name}: wrong test failed"
        print(f"{name}: {summaries[0]}", flush=True)
    finally:
        path.write_bytes(original)


def replace(old, new):
    def mutate(data):
        assert data.count(old) == 1, "ambiguous mutation"
        return data.replace(old, new)
    return mutate


source = ROOT / "rust/crates/campfire/src/rich_text.rs"
check("icons-vendor-drift", ROOT / "rust/crates/campfire/vendor/icons.yml", lambda b: b + b"\n",
      "vendored_icon_catalog_matches_reference")
check("icons-comparison", source,
      replace(b"Ok(reference) if reference == ICON_CONFIG.as_bytes()", b"Ok(reference) if !reference.is_empty()"),
      "changed_icon_catalog_is_rejected")
check("icons-ci-required", source,
      replace(b"std::io::ErrorKind::NotFound && !ci", b"std::io::ErrorKind::NotFound"),
      "missing_icon_reference_fails_in_ci")
check("icons-local-optional", source,
      replace(b"std::io::ErrorKind::NotFound && !ci", b"std::io::ErrorKind::NotFound && ci"),
      "missing_icon_reference_can_skip_locally")
print("WS8 icons discrimination: 4 compiled regressions detected; implementation restored")
