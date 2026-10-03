"""Injection checks for every source the image/seed cache promises to track."""

from pathlib import Path
import re
import shlex
import shutil
import tempfile
import unittest

from ci_seed import IMAGE_INPUTS, SEED_INPUTS, cache_keys

ROOT = Path(__file__).resolve().parent.parent


class SeedCoverageTests(unittest.TestCase):
    def test_ci_builds_validates_and_caches_every_test_seed(self):
        # Include literal seed names in table-driven and conditional boot calls, too.
        known = {p.stem for p in (ROOT / "parity/seeds").glob("*.rb") if p.stem != "build"}
        required = set()
        for path in (ROOT / "crates").rglob("*.rs"):
            source = path.read_text()
            if re.search(r"\b(?:boot_seed\w*|seed_dir|boot_with_clients|boot_with_huddle_services)\s*\(", source):
                required.update(set(re.findall(r'"([a-z_]+)"', source)) & known)
        self.assertIn("agents_ui", required, "the inventory must include navigation/inbox tests")
        script = (ROOT / "parity/bin/ci-seed").read_text()
        built = set(shlex.split(re.search(r'"\$ROOT/parity/bin/seed" build ([^\n]+)', script)[1]))
        validated = set(shlex.split(re.search(r"for seed in ([^;]+); do", script)[1]))
        workflow = (ROOT.parent / ".github/workflows/rust.yml").read_text()
        for seed in sorted(required):
            with self.subTest(seed=seed):
                self.assertIn(seed, built, f"CI does not build the {seed} test seed")
                self.assertIn(seed, validated, f"CI does not validate the {seed} test seed")
                self.assertEqual(workflow.count(f"rust/parity/.seed/{seed}\n"), 2,
                                 f"CI must restore and save the {seed} test seed")


class CacheIdentityTests(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name) / "rust"
        for name in SEED_INPUTS:
            source = ROOT / name
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if source.is_dir():
                shutil.copytree(source, target)
            else:
                shutil.copyfile(source, target)

    def test_every_input_changes_the_seed_key(self):
        before = cache_keys(self.root)
        for name in SEED_INPUTS:
            source = self.root / name
            files = sorted(p for p in source.rglob("*") if p.is_file()) if source.is_dir() else [source]
            for file in files:
                with self.subTest(input=file.relative_to(self.root)):
                    content = file.read_bytes()
                    if name == "parity/reference.sha":
                        file.write_text("a" * 40 + "\n")
                    else:
                        file.write_bytes(content + b"\n# cache identity injection\n")
                    after = cache_keys(self.root)
                    self.assertNotEqual(before["seed_key"], after["seed_key"])
                    if name in IMAGE_INPUTS:
                        self.assertNotEqual(before["image_key"], after["image_key"])
                    else:
                        self.assertEqual(before["image_key"], after["image_key"])
                    file.write_bytes(content)

    def test_checkout_schema_and_migrations_invalidate_both_caches(self):
        self.assertIn("../db/schema.rb", IMAGE_INPUTS)
        self.assertIn("../db/migrate", IMAGE_INPUTS)
        before = cache_keys(self.root)
        migration = self.root.parent / "db/migrate/20990101000000_added_nullable_column.rb"
        migration.write_text("# a new Rails migration\n")
        after = cache_keys(self.root)
        self.assertNotEqual(before["image_key"], after["image_key"])
        self.assertNotEqual(before["seed_key"], after["seed_key"])
        migration.unlink()
        self.assertEqual(before, cache_keys(self.root))

    def test_added_and_removed_seed_files_invalidate(self):
        before = cache_keys(self.root)
        added = self.root / "parity/seeds/new.rb"
        added.write_text("# new seed\n")
        self.assertNotEqual(before["seed_key"], cache_keys(self.root)["seed_key"])
        added.unlink()
        (self.root / "parity/seeds/default.rb").unlink()
        self.assertNotEqual(before["seed_key"], cache_keys(self.root)["seed_key"])

    def test_same_contents_with_different_mtime_reuse_the_cache(self):
        before = cache_keys(self.root)
        import os
        os.utime(self.root / "parity/bin/seed", (1, 1))
        self.assertEqual(before, cache_keys(self.root))

    def test_invalid_pin_and_missing_inputs_fail(self):
        for pin in ("d7c7de92", "main", "x" * 40):
            with self.subTest(pin=pin):
                (self.root / "parity/reference.sha").write_text(pin)
                with self.assertRaises(ValueError):
                    cache_keys(self.root)
        shutil.copyfile(ROOT / "parity/reference.sha", self.root / "parity/reference.sha")
        (self.root / "parity/.env.reference").unlink()
        with self.assertRaises(ValueError):
            cache_keys(self.root)

    def test_symlink_inputs_are_rejected(self):
        source = self.root / "parity/seeds/default.rb"
        source.unlink()
        source.symlink_to("first_run.rb")
        with self.assertRaises(ValueError):
            cache_keys(self.root)


if __name__ == "__main__":
    unittest.main()
