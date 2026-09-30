"""Real Docker/Rails failure injections; run after ci-seed image/build/validate."""

import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
COMMAND = ROOT / "parity/bin/ci-seed"


class SeedGateTests(unittest.TestCase):
    def setUp(self):
        scratch = ROOT / "parity/.ci/injections"
        scratch.mkdir(parents=True, exist_ok=True)
        self.scratch = tempfile.TemporaryDirectory(dir=scratch)
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)

    def test_missing_seed_is_rejected(self):
        result = self.validate(self.root)
        self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_corrupt_cached_seed_is_rejected_by_rails(self):
        for name in ("default", "first_run"):
            shutil.copytree(ROOT / "parity/.seed" / name, self.root / name)
        database = self.root / "default/db/production.sqlite3"
        with sqlite3.connect(database) as connection:
            connection.execute("DELETE FROM message_pins")
            connection.commit()
        connection.close()
        result = self.validate(self.root)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn('"pins": false', result.stdout)
        self.assertIn("parity seed verification failed", result.stdout)

    def test_restored_image_with_wrong_revision_is_rejected(self):
        image = f"ws19b-injected-reference-{os.getpid()}"
        dockerfile = self.root / "Dockerfile"
        dockerfile.write_text("FROM ws19b-ci-reference\nENV GIT_REVISION=wrong-reference-pin\n")
        subprocess.run(["docker", "build", "-q", "-t", image, str(self.root)], check=True, capture_output=True)
        self.addCleanup(subprocess.run, ["docker", "image", "rm", image], check=True, capture_output=True)
        result = subprocess.run([COMMAND, "check-image"], env={**os.environ, "PARITY_IMAGE": image},
                                text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("reference image has the wrong Rails revision", result.stdout)

    def validate(self, seed_dir):
        return subprocess.run([COMMAND, "validate"], env={**os.environ, "PARITY_SEED_DIR": str(seed_dir)},
                              text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


if __name__ == "__main__":
    unittest.main()
