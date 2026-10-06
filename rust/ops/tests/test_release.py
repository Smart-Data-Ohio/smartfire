"""Local release decision harness for deploy/gcp/campfire-release.sh.

Runs the real script, phase by phase, against fakes for docker, once, curl and
the host tools. No real container, ONCE or cloud command runs. The fakes model
the one thing every decision hangs on: the live database's bytes in the
storage volume, which the candidate's `campfire db-migrate` and a serving
candidate can change. The real-Docker rehearsal of the same flow is
rust/ops/tests/simulate_release.sh.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy/gcp/campfire-release.sh"
CANDIDATE = "fixture/image@sha256:" + "1" * 64
PREVIOUS = "fixture/image@sha256:" + "2" * 64
ROLLBACK_TAG = "campfire-rollback:before-fixture"

FAKE = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
if name == "id":
    print("0")
    sys.exit(0)
work = pathlib.Path(os.environ["STATE_ROOT"])
state_file = work / "fake.json"
state = json.loads(state_file.read_text())
def save():
    state_file.write_text(json.dumps(state))
with open(os.environ["TRACE"], "a") as f:
    f.write(json.dumps([name, *args]) + "\n")
db = work / "volume/db/production.sqlite3"
candidate = os.environ["CANDIDATE"]
def serving():
    return state["running"] and not (state["image"] == candidate and os.environ.get("CANDIDATE_UNHEALTHY"))
def boot():
    crash = state["image"] == candidate and os.environ.get("CANDIDATE_CRASHES")
    state["running"] = not crash
    if state["running"] and state["image"] == candidate and os.environ.get("CANDIDATE_WRITES"):
        db.write_bytes(db.read_bytes() + b"+write")
    save()
def migrations(var):
    return [v for v in os.environ.get(var, os.environ.get("MIGRATIONS", "")).split(",") if v]
if name == "gcloud":
    sys.exit("cloud commands are forbidden in this harness")
if name == "docker":
    if args[0] == "ps":
        if "-a" in args or state["running"]:
            print("once-app-fixture")
    elif args[0] == "inspect":
        fmt = args[args.index("--format") + 1]
        if ".State.Running" in fmt:
            print("true" if state["running"] else "false")
        elif ".Mounts" in fmt:
            print("fixture-volume")
        elif '"once"' in fmt:
            print(json.dumps({"host": "fixture.invalid", "image": state["image"],
                "env": {"SECRET_KEY_BASE": "fixture"}, "autoUpdate": False}))
        elif fmt == "{{.Image}}":
            print("sha256:" + "9" * 64)
        else:
            sys.exit("unhandled inspect " + fmt)
    elif args[:2] == ["image", "inspect"]:
        if "--format" not in args:
            sys.exit(0)
        fmt = args[args.index("--format") + 1]
        if "net.smartdata.campfire.runtime" in fmt:
            print(os.environ.get("PREVIOUS_RUNTIME", "rust") if "2" * 64 in args[2] else os.environ.get("RUNTIME", "rust"))
        elif fmt == "{{.Architecture}}":
            print("amd64")
        elif fmt == "{{.Os}}":
            print("linux")
        elif fmt == "{{.Size}}":
            print("10485760")
        elif ".Config.Env" in fmt:
            print("GIT_REVISION=fixture")
        else:
            sys.exit("unhandled image inspect " + fmt)
    elif args[:2] == ["volume", "inspect"]:
        print(work / "volume")
    elif args[0] == "top":
        print("PID COMMAND")
        print("100 " + os.environ.get("PROCESSES", "/usr/local/bin/campfire server"))
    elif args[0] == "exec":
        backups = work / "volume/backups"
        backups.mkdir(exist_ok=True)
        (backups / "production.sqlite3").write_bytes(db.read_bytes())
    elif args[0] == "run":
        if state["running"]:
            sys.exit("a release container ran while the application was running")
        image = next(a for a in args if a in (candidate, os.environ["PREVIOUS"], os.environ["ROLLBACK_TAG"]))
        command = " ".join(args[args.index(image) + 1:])
        if image != candidate:
            if "db-check" not in command:
                sys.exit("the previous image may only check a copy: " + command)
            print("SCHEMA: previous image" if not os.environ.get("PREVIOUS_REFUSES") else "unknown migrations")
            sys.exit(1 if os.environ.get("PREVIOUS_REFUSES") else 0)
        if command.startswith("bash -c"):
            if os.environ.get("REHEARSAL_STATUS"):
                print("ERROR: injected rehearsal failure")
                sys.exit(int(os.environ["REHEARSAL_STATUS"]))
            for version in migrations("MIGRATIONS"):
                print("MIGRATED: " + version)
            print("MIGRATIONS: %d applied" % len(migrations("MIGRATIONS")))
            print("MATCH: 88 preexisting tables preserved")
            print("ADDITIVE: 0 tables, %d columns" % len(migrations("MIGRATIONS")))
            (work / "campfire-fixture/rehearsal/rehearsal-after.sqlite3").write_bytes(b"fixture")
        elif command == "campfire db-migrate /rails/storage/db/production.sqlite3":
            if "fixture-volume:/rails/storage" not in args:
                sys.exit("the live migration must mount the live volume")
            if os.environ.get("LIVE_STATUS"):
                print("ERROR: injected live migration failure")
                sys.exit(int(os.environ["LIVE_STATUS"]))
            applied = migrations("LIVE_MIGRATIONS")
            if applied:
                db.write_bytes(db.read_bytes() + b"+migrated")
            for version in applied:
                print("MIGRATED: " + version)
            print("MIGRATIONS: %d applied" % len(applied))
        else:
            sys.exit("unhandled candidate run " + command)
    elif args[0] not in ["rm", "login", "logout", "pull", "tag"]:
        sys.exit("unhandled docker " + repr(args))
elif name == "once":
    if args[0] == "stop":
        state["running"] = False
        save()
    elif args[0] == "start":
        boot()
    elif args[0] == "update":
        state["image"] = args[args.index("--image") + 1]
        boot()
    elif args[0] == "backup":
        pathlib.Path(args[2]).write_bytes(b"once backup")
    elif args[0] != "version":
        sys.exit("unhandled once " + repr(args))
elif name == "curl":
    print("200" if serving() else "000", end="")
elif name == "df":
    print("Filesystem 1M-blocks Used Available Use% Mounted on")
    print("fixture 100000 100 99900 1% /")
elif name == "du":
    print("1 " + args[-1])
elif name == "pgrep":
    sys.exit(1)
elif name == "systemctl":
    print("disabled" if args[0] == "is-enabled" else "inactive")
    sys.exit(1)
else:
    sys.exit("unhandled fake " + name)
'''


class Host:
    """A scratch VM: the volume, the release state root and the fakes."""

    def __init__(self, work):
        self.work = work
        self.volume = work / "volume"
        (self.volume / "db").mkdir(parents=True)
        (self.volume / "files").mkdir()
        (self.volume / "files/upload").write_text("kept")
        self.db.write_bytes(b"SQLite fixture database")
        self.state = work / "campfire-fixture"
        self.trace = work / "trace.jsonl"
        (work / "fake.json").write_text(json.dumps({"running": True, "image": PREVIOUS}))
        self.bin = work / "bin"
        self.bin.mkdir()
        for name in ["docker", "gcloud", "once", "curl", "id", "df", "du", "pgrep", "systemctl"]:
            fake = self.bin / name
            fake.write_text(FAKE)
            fake.chmod(0o755)

    @property
    def db(self):
        return self.volume / "db/production.sqlite3"

    def fake(self):
        return json.loads((self.work / "fake.json").read_text())

    def run(self, phase, **env):
        before = len(self.commands())
        environment = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}",
            "IMAGE_REF": CANDIDATE, "CANDIDATE": CANDIDATE, "PREVIOUS": PREVIOUS, "ROLLBACK_TAG": ROLLBACK_TAG,
            "RELEASE_LABEL": "fixture", "STATE_ROOT": str(self.work), "LOCK_FILE": str(self.work / "release.lock"),
            "ALLOW_BACKUP_WINDOW": "1", "HEALTH_TIMEOUT": "0", "FEED_DRAIN_TIMEOUT": "0",
            "OPEN_ROLES_PATHS": str(self.work / "no-feed"), "TRACE": str(self.trace), **env}
        result = subprocess.run(["bash", str(SCRIPT), phase], env=environment, input="token\n",
                                text=True, capture_output=True)
        return result, self.commands()[before:]

    def commands(self):
        if not self.trace.exists():
            return []
        return [json.loads(line) for line in self.trace.read_text().splitlines()]

    def json(self, name):
        return json.loads((self.state / name).read_text())

    def release(self, **env):
        """preflight, freeze and cutover, stopping at the first failure."""
        results = {}
        for phase in ["preflight", "freeze", "cutover"]:
            results[phase] = self.run(phase, **env)
            if results[phase][0].returncode:
                break
        return results


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class ReleaseTest(unittest.TestCase):
    def setUp(self):
        scratch = ROOT / ".scratch"
        scratch.mkdir(exist_ok=True)
        self._tmp = tempfile.TemporaryDirectory(prefix="release-", dir=scratch)
        self.host = Host(Path(self._tmp.name))

    def tearDown(self):
        self._tmp.cleanup()

    def assertOk(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_the_script_has_no_rails_paths_left(self):
        source = SCRIPT.read_text()
        for rails in ["bin/rails", "bin/start-app", "bundle exec", "db:prepare", "db:migrate", "puma", "resque"]:
            self.assertNotIn(rails, source)

    def test_a_release_without_migrations(self):
        results = self.host.release()
        for phase, (result, _) in results.items():
            with self.subTest(phase=phase):
                self.assertOk(result)
        self.assertEqual(self.host.json("preflight-result.json")["target_runtime"], "rust")
        self.assertEqual(self.host.json("preflight-result.json")["current_runtime"], "rust")
        rehearsal = self.host.json("freeze-result.json")["rehearsal"]
        self.assertEqual(rehearsal["migrations"], [])
        self.assertTrue(rehearsal["verified"])
        live = self.host.json("live-migration-result.json")
        self.assertEqual(live["applied"], [])
        self.assertEqual(live["frozen_live_database_sha256"], live["migrated_live_database_sha256"])
        self.assertEqual(self.host.fake(), {"running": True, "image": CANDIDATE})

    def test_a_release_with_migrations_rehearses_then_migrates_the_stopped_live_database(self):
        versions = "20261006120000,20261007120000"
        results = self.host.release(MIGRATIONS=versions)
        for phase, (result, _) in results.items():
            with self.subTest(phase=phase):
                self.assertOk(result)
        freeze = self.host.json("freeze-result.json")
        self.assertEqual(freeze["rehearsal"]["migrations"], versions.split(","))
        self.assertEqual(freeze["rehearsal"]["database_sha256"], freeze["frozen_live_database_sha256"])
        # The rehearsal ran on a copy, in a scratch directory, with no network.
        rehearsal = next(c for c in results["freeze"][1] if c[:2] == ["docker", "run"])
        self.assertIn("none", rehearsal)
        self.assertIn(f"{self.host.state}/rehearsal:/rails/storage", rehearsal)
        for step in ["campfire db-migrate", "campfire db-check", "campfire verify-additive-sqlite-migration"]:
            self.assertIn(step, rehearsal[-1])
        # The frozen bytes are kept, and the live database is migrated while stopped,
        # strictly between the freeze and the image switch.
        self.assertEqual((self.host.state / "frozen-live/db/production.sqlite3").read_bytes(), b"SQLite fixture database")
        cutover = results["cutover"][1]
        migrate = next(i for i, c in enumerate(cutover) if c[:2] == ["docker", "run"])
        update = next(i for i, c in enumerate(cutover) if c[:2] == ["once", "update"])
        self.assertLess(migrate, update)
        self.assertEqual(cutover[migrate][-2:], ["db-migrate", "/rails/storage/db/production.sqlite3"])
        self.assertIn("fixture-volume:/rails/storage", cutover[migrate])
        self.assertIn("none", cutover[migrate])
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database+migrated")
        self.assertEqual(self.host.json("deploy-result.json")["migrations"], versions.split(","))
        self.assertTrue(self.host.json("live-migration-result.json")["matches_rehearsal"])

    def test_preflight_refuses_anything_but_rust_to_rust(self):
        for env in [{"RUNTIME": "rails"}, {"RUNTIME": ""}, {"RUNTIME": "<no value>"}, {"PREVIOUS_RUNTIME": "rails"},
                    {"PREVIOUS_RUNTIME": ""}]:
            with self.subTest(env=env):
                result, _ = self.host.run("preflight", **env)
                self.assertEqual(result.returncode, 1)
                self.assertIn("this script only releases Rust to Rust", result.stderr)
                self.assertFalse((self.host.state / "preflight-result.json").exists())

    def test_later_phases_require_the_preflight_candidate_and_a_rust_record(self):
        self.assertOk(self.host.run("preflight")[0])
        other = "fixture/other@sha256:" + "3" * 64
        for phase in ["freeze", "cutover"]:
            with self.subTest(phase=phase):
                result, commands = self.host.run(phase, IMAGE_REF=other)
                self.assertEqual(result.returncode, 1)
                self.assertIn("does not match IMAGE_REF", result.stderr)
                self.assertEqual(commands, [], "refused before Docker or ONCE ran")
        record = self.host.json("preflight-result.json")
        del record["target_runtime"], record["current_runtime"]
        (self.host.state / "preflight-result.json").write_text(json.dumps(record))
        result, commands = self.host.run("freeze")
        self.assertEqual(result.returncode, 1)
        self.assertIn("did not record a Rust-to-Rust release", result.stderr)
        self.assertEqual(commands, [])

    def test_a_failed_rehearsal_refuses_and_brings_the_previous_image_back(self):
        results = self.host.release(MIGRATIONS="20261006120000", REHEARSAL_STATUS="2")
        result, commands = results["freeze"]
        self.assertEqual(result.returncode, 1)
        self.assertIn("refusing to cut over", result.stderr)
        self.assertNotIn("cutover", results)
        self.assertIn(["once", "start", "fixture.invalid"], commands)
        self.assertEqual(self.host.fake(), {"running": True, "image": PREVIOUS})
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database")
        self.assertFalse(self.host.json("rehearsal-result.json")["verified"])

    def test_cutover_refuses_a_database_that_moved_after_the_freeze(self):
        self.assertOk(self.host.run("preflight")[0])
        self.assertOk(self.host.run("freeze")[0])
        self.host.db.write_bytes(b"someone else wrote")
        result, commands = self.host.run("cutover")
        self.assertEqual(result.returncode, 10)
        self.assertIn("changed since the freeze", result.stderr)
        self.assertFalse(any(c[:2] in (["docker", "run"], ["once", "update"]) for c in commands))

    def test_cutover_refuses_without_a_rehearsal_of_this_database(self):
        self.assertOk(self.host.run("preflight")[0])
        self.assertOk(self.host.run("freeze")[0])
        freeze = self.host.json("freeze-result.json")
        freeze["rehearsal"]["database_sha256"] = "0" * 64
        (self.host.state / "freeze-result.json").write_text(json.dumps(freeze))
        result, commands = self.host.run("cutover")
        self.assertEqual(result.returncode, 10)
        self.assertIn("verified rehearsal", result.stderr)
        self.assertFalse(any(c[:2] in (["docker", "run"], ["once", "update"]) for c in commands))

    def test_simulated_failure_one_stops_before_the_migration(self):
        results = self.host.release(MIGRATIONS="20261006120000", CAMPFIRE_RELEASE_SIMULATE_FAILURE="1")
        result, commands = results["cutover"]
        self.assertEqual(result.returncode, 10)
        self.assertFalse(any(c[:2] in (["docker", "run"], ["once", "update"]) for c in commands))
        result, _ = self.host.run("rollback")
        self.assertOk(result)
        self.assertEqual(self.host.json("rollback-result.json")["action"], "image-rolled-back")
        self.assertEqual(self.host.fake(), {"running": True, "image": PREVIOUS})

    def test_a_failed_live_migration_never_switches_images_and_rolls_back(self):
        results = self.host.release(MIGRATIONS="20261006120000", LIVE_STATUS="2")
        result, commands = results["cutover"]
        self.assertEqual(result.returncode, 10)
        self.assertIn("db-migrate failed on the live database", result.stderr)
        self.assertFalse(any(c[:2] == ["once", "update"] for c in commands))
        result, _ = self.host.run("rollback")
        self.assertOk(result)
        self.assertEqual(self.host.json("rollback-result.json")["action"], "image-rolled-back")
        self.assertEqual(self.host.fake(), {"running": True, "image": PREVIOUS})

    def test_a_live_migration_that_differs_from_the_rehearsal_is_reverted(self):
        results = self.host.release(MIGRATIONS="20261006120000", LIVE_MIGRATIONS="20261006120000,20261007120000")
        result, commands = results["cutover"]
        self.assertEqual(result.returncode, 10)
        self.assertIn("but the rehearsal applied", result.stderr)
        self.assertFalse(any(c[:2] == ["once", "update"] for c in commands))
        self.assertFalse(self.host.json("live-migration-result.json")["matches_rehearsal"])
        result, _ = self.host.run("rollback")
        self.assertOk(result)
        self.assertEqual(self.host.json("rollback-result.json")["action"], "migration-reverted")
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database")

    def test_an_unhealthy_candidate_that_never_wrote_gets_the_frozen_bytes_back(self):
        results = self.host.release(MIGRATIONS="20261006120000", CANDIDATE_UNHEALTHY="1")
        self.assertEqual(results["cutover"][0].returncode, 10)
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database+migrated")
        result, commands = self.host.run("rollback")
        self.assertOk(result)
        rollback = self.host.json("rollback-result.json")
        self.assertEqual(rollback["action"], "migration-reverted")
        self.assertTrue(rollback["database_restored"])
        self.assertEqual(rollback["health"], "healthy")
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database")
        self.assertEqual(self.host.fake(), {"running": True, "image": PREVIOUS})
        self.assertIn(["once", "update", "fixture.invalid", "--image", PREVIOUS, "--auto-update=false"], commands)

    def test_an_unhealthy_candidate_without_migrations_rolls_back_the_image_only(self):
        results = self.host.release(CANDIDATE_CRASHES="1")
        self.assertEqual(results["cutover"][0].returncode, 10)
        result, _ = self.host.run("rollback")
        self.assertOk(result)
        self.assertEqual(self.host.json("rollback-result.json")["action"], "image-rolled-back")
        self.assertFalse(self.host.json("rollback-result.json")["database_restored"])

    def test_writes_after_the_migration_are_never_discarded(self):
        for refuses, action, image in [("", "refused-database-changed", PREVIOUS),
                                       ("1", "refused-database-incompatible", CANDIDATE)]:
            with self.subTest(action=action):
                self.tearDown()
                self.setUp()
                results = self.host.release(MIGRATIONS="20261006120000", CANDIDATE_UNHEALTHY="1", CANDIDATE_WRITES="1")
                self.assertEqual(results["cutover"][0].returncode, 10)
                written = self.host.db.read_bytes()
                self.assertEqual(written, b"SQLite fixture database+migrated+write")
                result, commands = self.host.run("rollback", PREVIOUS_REFUSES=refuses)
                self.assertEqual(result.returncode, 30, result.stdout + result.stderr)
                self.assertIn("OPERATOR ACTION REQUIRED", result.stderr)
                rollback = self.host.json("rollback-result.json")
                self.assertEqual(rollback["action"], action)
                self.assertFalse(rollback["database_restored"])
                self.assertEqual(self.host.fake()["image"], image)
                # The database bytes are whatever the writes left, plus nothing.
                self.assertTrue(self.host.db.read_bytes().startswith(written))
                check = next(c for c in commands if c[:2] == ["docker", "run"])
                self.assertIn(ROLLBACK_TAG, check)
                self.assertIn("none", check)
                self.assertNotIn("fixture-volume:/rails/storage", check, "the check reads a copy")

    def test_a_healthy_cutover_is_never_rolled_back(self):
        results = self.host.release(MIGRATIONS="20261006120000")
        self.assertOk(results["cutover"][0])
        result, commands = self.host.run("rollback")
        self.assertEqual(result.returncode, 30)
        self.assertEqual(self.host.json("rollback-result.json")["action"], "refused-application-healthy")
        self.assertFalse(any(c[:2] == ["once", "stop"] for c in commands))
        self.assertEqual(self.host.db.read_bytes(), b"SQLite fixture database+migrated")

    def test_a_missing_server_process_fails_after_health(self):
        results = self.host.release(PROCESSES="unrelated")
        result, _ = results["cutover"]
        self.assertEqual(result.returncode, 20)
        self.assertIn("read-only check(s) failed", result.stderr)

    def test_resume_reuses_a_rehearsal_only_for_the_same_database(self):
        self.assertOk(self.host.run("preflight")[0])
        self.assertOk(self.host.run("freeze")[0])
        (self.host.work / "fake.json").write_text(json.dumps({"running": True, "image": PREVIOUS}))
        result, commands = self.host.run("freeze", RESUME="1")
        self.assertOk(result)
        self.assertIn("already rehearsed", result.stdout)
        self.assertFalse(any(c[:2] == ["docker", "run"] for c in commands))
        (self.host.work / "fake.json").write_text(json.dumps({"running": True, "image": PREVIOUS}))
        self.host.db.write_bytes(b"SQLite fixture database with later writes")
        result, commands = self.host.run("freeze", RESUME="1")
        self.assertOk(result)
        self.assertIn("rehearsing again", result.stderr)
        self.assertTrue(any(c[:2] == ["docker", "run"] for c in commands))


if __name__ == "__main__":
    unittest.main()
