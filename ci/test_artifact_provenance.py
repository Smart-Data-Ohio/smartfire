"""Bind shared payload bytes to a real producer and consumer checkout."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PROVENANCE = ROOT / "ci/artifact_provenance.py"


class ArtifactProvenance(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        self.git_env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        # Git applies discovery ceilings to parents, not the current directory.
        self.git_env["GIT_CEILING_DIRECTORIES"] = str(self.root.parent)
        self.git("init", "--quiet", "--initial-branch=main")
        self.head = self.commit("first source")
        self.payload = self.root / "shared.tar.zst"
        self.payload.write_bytes(b"shared archive\x00\xff\n")
        self.sidecar = self.root / "shared.tar.zst.json"

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, env=self.git_env, text=True).strip()

    def commit(self, content):
        (self.root / "source").write_text(content)
        self.git("add", "source")
        self.git("-c", "user.name=CI test", "-c", "user.email=ci@example.test",
                 "commit", "--quiet", "-m", content)
        return self.git("rev-parse", "HEAD")

    def run_cli(self, command, *, head=None, payload=None):
        return subprocess.run([sys.executable, str(PROVENANCE), command,
                               str(payload or self.payload), "--head", head or self.head],
                              cwd=self.root, env=self.git_env, capture_output=True, text=True)

    def record(self):
        result = self.run_cli("record")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return json.loads(self.sidecar.read_text())

    def assert_rejected(self, result, message):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(message, result.stderr)

    def test_record_and_verify_bind_checkout_basename_and_real_payload_digest(self):
        metadata = self.record()
        self.assertEqual(metadata, dict(version=1, file=self.payload.name, head=self.head,
                                        sha256=hashlib.sha256(self.payload.read_bytes()).hexdigest()))
        result = self.run_cli("verify")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_large_image_hash_covers_bytes_across_read_boundaries(self):
        data = bytes(range(256)) * 8193
        self.payload.write_bytes(data)
        metadata = self.record()
        self.assertEqual(metadata["sha256"], hashlib.sha256(data).hexdigest())
        with self.payload.open("r+b") as payload:
            payload.seek(1024 * 1024 + 3)
            payload.write(b"corruption")
        self.assert_rejected(self.run_cli("verify"), "SHA256 mismatch")

    def test_old_payload_is_rejected_after_the_requested_branch_advances(self):
        self.record()
        new_head = self.commit("branch advances")
        self.assert_rejected(self.run_cli("verify", head=new_head), "producer HEAD mismatch")
        self.assert_rejected(self.run_cli("verify"), "checkout HEAD mismatch")
        self.git("checkout", "--quiet", "--detach", self.head)
        result = self.run_cli("verify")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_record_requires_the_actual_producer_head(self):
        self.assert_rejected(self.run_cli("record", head="0" * 40), "checkout HEAD mismatch")
        self.assertFalse(self.sidecar.exists())

    def test_missing_payload_or_metadata_is_rejected(self):
        self.assert_rejected(self.run_cli("verify"), "metadata")
        self.record()
        self.payload.unlink()
        self.assert_rejected(self.run_cli("verify"), "payload")
        self.assert_rejected(self.run_cli("record"), "payload")

    def test_malformed_metadata_is_rejected(self):
        valid = self.record()
        cases = [None, [], {**valid, "version": 2}, {**valid, "version": True},
                 {**valid, "file": "different.tar.zst"}, {**valid, "head": ""},
                 {**valid, "head": "g" * 40}, {**valid, "sha256": "a" * 63},
                 {**valid, "sha256": "g" * 64}, {key: value for key, value in valid.items()
                                                if key != "sha256"}]
        for metadata in cases:
            with self.subTest(metadata=metadata):
                self.sidecar.write_text(json.dumps(metadata))
                self.assert_rejected(self.run_cli("verify"), "metadata")
        self.sidecar.write_text("{not json")
        self.assert_rejected(self.run_cli("verify"), "metadata")

    def test_malformed_expected_heads_are_rejected(self):
        for head in ("main", "a" * 39, "a" * 41, "g" * 40):
            with self.subTest(head=head):
                self.assert_rejected(self.run_cli("record", head=head), "full 40-character Git SHA")
                self.assert_rejected(self.run_cli("verify", head=head), "full 40-character Git SHA")

    def test_expected_head_is_required_and_git_checkout_must_exist(self):
        result = subprocess.run([sys.executable, str(PROVENANCE), "record", str(self.payload)],
                                cwd=self.root, env=self.git_env, capture_output=True, text=True)
        self.assert_rejected(result, "--head")
        shutil.rmtree(self.root / ".git")
        self.assert_rejected(self.run_cli("record"), "cannot read checkout HEAD")

    def test_payload_metadata_cannot_be_reused_for_another_filename(self):
        self.record()
        other = self.root / "other.tar.zst"
        shutil.copyfile(self.payload, other)
        shutil.copyfile(self.sidecar, self.root / "other.tar.zst.json")
        self.assert_rejected(self.run_cli("verify", payload=other), "metadata file mismatch")


class CorrectnessSourcePin(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        (self.root / "ci").mkdir()
        shutil.copyfile(ROOT / "ci/correctness.sh", self.root / "ci/correctness.sh")
        (self.root / "ci/ignored_tests.py").write_text("raise SystemExit(0)\n")
        self.git("init", "--quiet", "--initial-branch=main")
        self.git("add", "ci")
        self.git("-c", "user.name=CI test", "-c", "user.email=ci@example.test",
                 "commit", "--quiet", "-m", "source")
        self.head = self.git("rev-parse", "HEAD")

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True).strip()

    def run_suite(self, source_sha=None):
        env = dict(os.environ)
        env.pop("CI_SOURCE_SHA", None)
        env.pop("CORRECTNESS_SHARD", None)
        env.pop("CORRECTNESS_ARCHIVE", None)
        if source_sha is not None:
            env["CI_SOURCE_SHA"] = source_sha
        return subprocess.run(["bash", "ci/correctness.sh", "unknown"], cwd=self.root,
                              env=env, capture_output=True, text=True)

    def test_moving_checkout_fails_before_suite_execution(self):
        (self.root / "source").write_text("later source")
        self.git("add", "source")
        self.git("-c", "user.name=CI test", "-c", "user.email=ci@example.test",
                 "commit", "--quiet", "-m", "later source")
        result = self.run_suite(self.head)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CI_SOURCE_SHA does not match checkout HEAD", result.stderr)
        self.assertFalse((self.root / "target/ci-receipts").exists())
        self.assertNotIn("Unknown correctness suite", result.stdout + result.stderr)

    def test_malformed_source_sha_fails_before_suite_execution(self):
        result = self.run_suite("main")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CI_SOURCE_SHA must be a full 40-character Git SHA", result.stderr)
        self.assertFalse((self.root / "target/ci-receipts").exists())

    def test_receipts_use_the_validated_head_and_local_runs_work_without_a_pin(self):
        for pin in (self.head, None):
            with self.subTest(pin=pin):
                result = self.run_suite(pin)
                self.assertNotEqual(result.returncode, 0)
                receipt = json.loads((self.root / "target/ci-receipts/unknown.json").read_text())
                self.assertEqual(receipt["head"], self.head)
                self.assertNotEqual(receipt["exit_code"], 0)

    def test_receipt_retains_the_head_validated_before_execution(self):
        (self.root / "ci/ignored_tests.py").write_text('''import subprocess
from pathlib import Path
Path("source").write_text("source changed during suite")
subprocess.check_call(["git", "add", "source"])
subprocess.check_call(["git", "-c", "user.name=CI test", "-c", "user.email=ci@example.test",
                       "commit", "--quiet", "-m", "suite advances checkout"])
''')
        result = self.run_suite(self.head)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotEqual(self.git("rev-parse", "HEAD"), self.head)
        receipt = json.loads((self.root / "target/ci-receipts/unknown.json").read_text())
        self.assertEqual(receipt["head"], self.head)


if __name__ == "__main__":
    unittest.main()
