"""parity/bin/frozen-seeds: CI restores every seed the tests use, and rejects changed seeds."""

from importlib.machinery import SourceFileLoader
from importlib.util import module_from_spec, spec_from_loader
from pathlib import Path
import contextlib
import io
import re
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
loader = SourceFileLoader("frozen_seeds", str(ROOT / "parity/bin/frozen-seeds"))
frozen_seeds = module_from_spec(spec_from_loader("frozen_seeds", loader))
loader.exec_module(frozen_seeds)


class SeedCoverageTests(unittest.TestCase):
    def test_every_seed_the_tests_boot_is_frozen(self):
        # Include literal seed names in table-driven and conditional boot calls, too.
        known = {p.stem for p in (ROOT / "parity/seeds").glob("*.rb") if p.stem != "build"}
        required = set()
        for path in (ROOT / "crates").rglob("*.rs"):
            source = path.read_text()
            if re.search(r"\b(?:boot_seed\w*|seed_dir|boot_with_clients|boot_with_huddle_services)\s*\(", source):
                required.update(set(re.findall(r'"([a-z_]+)"', source)) & known)
        self.assertIn("agents_ui", required, "the inventory must include navigation/inbox tests")
        for seed in sorted(required):
            with self.subTest(seed=seed):
                self.assertIn(seed, frozen_seeds.SEEDS, f"the {seed} test seed is not frozen")


class CheckTests(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        shutil.copytree(frozen_seeds.FROZEN, root / "frozen")
        shutil.copyfile(frozen_seeds.ENV, root / "env")
        shutil.copyfile(frozen_seeds.MIGRATIONS, root / "migrations")
        self.root = root
        for name, value in (("FROZEN", root / "frozen"), ("MANIFEST", root / "frozen/manifest.json"),
                            ("ENV", root / "env"), ("MIGRATIONS", root / "migrations"), ("ROOT", root)):
            original = getattr(frozen_seeds, name)
            setattr(frozen_seeds, name, value)
            self.addCleanup(setattr, frozen_seeds, name, original)

    def assert_rejected(self, message):
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), self.assertRaises(SystemExit):
            frozen_seeds.check()
        self.assertIn(message, stderr.getvalue())

    def test_unchanged_seeds_pass_and_restore(self):
        with contextlib.redirect_stdout(io.StringIO()):
            frozen_seeds.restore(self.root / "restored")
        for seed in frozen_seeds.SEEDS:
            self.assertTrue((self.root / "restored" / seed / frozen_seeds.DATABASE).is_file())
            self.assertTrue((self.root / "restored" / seed / "storage").is_dir())

    def test_changed_file_is_rejected(self):
        labels = self.root / "frozen/default/labels.json"
        labels.write_text(labels.read_text() + " ")
        self.assert_rejected("default/labels.json: contents differ")

    def test_missing_and_added_files_are_rejected(self):
        (self.root / "frozen/agents_ui/labels.json").unlink()
        self.assert_rejected("added, removed or renamed")
        (self.root / "frozen/agents_ui/labels.json").write_text("{}")
        (self.root / "frozen/first_run/extra").write_text("")
        self.assert_rejected("added, removed or renamed")

    def test_changed_keys_are_rejected(self):
        env = self.root / "env"
        env.write_text(env.read_text().replace("SECRET_KEY_BASE=", "SECRET_KEY_BASE=x", 1))
        self.assert_rejected("SECRET_KEY_BASE")

    def test_a_new_migration_requires_migrating_the_seeds(self):
        with (self.root / "migrations").open("a") as migrations:
            migrations.write("29991231000000\n")
        self.assert_rejected("missing migrations 29991231000000")

    def test_seeds_ahead_of_the_build_are_rejected(self):
        migrations = self.root / "migrations"
        latest = max(migrations.read_text().split())
        migrations.write_text("".join(f"{v}\n" for v in migrations.read_text().split() if v != latest))
        self.assert_rejected(f"has migrations this build doesn't know: {latest}")

    def test_migrate_runs_db_migrate_on_each_seed_then_records_them(self):
        # A stand-in for `campfire db-migrate DATABASE` that applies one new migration.
        campfire = self.root / "campfire"
        campfire.write_text("#!/usr/bin/env python3\n"
                            "import sqlite3, sys\n"
                            "assert sys.argv[1] == 'db-migrate', sys.argv\n"
                            "conn = sqlite3.connect(sys.argv[2])\n"
                            "conn.execute(\"INSERT INTO schema_migrations (version) VALUES ('29991231000000')\")\n"
                            "conn.commit()\n")
        campfire.chmod(0o755)
        with (self.root / "migrations").open("a") as migrations:
            migrations.write("29991231000000\n")
        self.assert_rejected("missing migrations 29991231000000")
        with contextlib.redirect_stdout(io.StringIO()):
            frozen_seeds.migrate(campfire)
            frozen_seeds.check()
        for seed in frozen_seeds.SEEDS:
            database = self.root / "frozen" / seed / frozen_seeds.DATABASE
            self.assertIn("29991231000000", frozen_seeds.versions(database))
            self.assertEqual(sorted(p.name for p in database.parent.iterdir()), [database.name])


if __name__ == "__main__":
    unittest.main()
