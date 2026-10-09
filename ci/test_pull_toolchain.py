import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("pull-toolchain.sh")
REFERENCE = "ghcr.io/smart-data-ohio/smartfire-toolchain:" + "a" * 64
DIGEST = "sha256:" + "b" * 64


class PullToolchainTest(unittest.TestCase):
    def pull(self, mode):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            docker = root / "docker"
            docker.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
with open(os.environ["CALLS"], "a") as file:
    file.write(json.dumps(args) + "\\n")
mode = os.environ["MODE"]
if args[:3] == ["buildx", "imagetools", "inspect"]:
    if mode == "missing":
        print("manifest unknown", file=sys.stderr)
        sys.exit(1)
    print(json.dumps({"digest": "bad" if mode == "invalid" else "sha256:" + "b" * 64}))
if args[0] == "pull" and mode == "pull-failed":
    sys.exit(1)
''')
            docker.chmod(0o755)
            output, calls = root / "output", root / "calls"
            result = subprocess.run(["bash", str(SCRIPT), REFERENCE], text=True, capture_output=True,
                env={**os.environ, "PATH": f"{root}:{os.environ['PATH']}", "GITHUB_OUTPUT": str(output),
                     "CALLS": str(calls), "MODE": mode})
            return result, output.read_text() if output.exists() else "", [json.loads(line) for line in calls.read_text().splitlines()]

    def test_pulls_the_resolved_digest_and_tags_the_local_ci_name(self):
        result, output, calls = self.pull("found")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "pulled=true\n")
        self.assertEqual(calls, [
            ["buildx", "imagetools", "inspect", REFERENCE, "--format", "{{json .Manifest}}"],
            ["pull", "--platform", "linux/amd64", f"ghcr.io/smart-data-ohio/smartfire-toolchain@{DIGEST}"],
            ["tag", f"ghcr.io/smart-data-ohio/smartfire-toolchain@{DIGEST}", "campfire-toolchain"],
        ])

    def test_missing_manifest_selects_the_local_fallback(self):
        result, output, calls = self.pull("missing")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, "pulled=false\n")
        self.assertEqual(len(calls), 1)
        self.assertIn("building the toolchain locally", result.stdout)

    def test_invalid_digest_and_failed_pull_cannot_report_success(self):
        for mode in ["invalid", "pull-failed"]:
            with self.subTest(mode=mode):
                result, output, calls = self.pull(mode)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(output, "")
                self.assertFalse(any(call[0] == "tag" for call in calls))


if __name__ == "__main__":
    unittest.main()
