"""Contract tests for deploy/backups/campfire-backup.sh.

Each test builds a fake ONCE storage volume (a temp SQLite database plus an
uploads tree) and runs the real script against it in BACKUP_VOLUME_DIR mode,
producing the encrypted archive in a temp output directory. Only the
host/cloud boundary is stubbed: a poison `gcloud` proves the script never
shells out to the cloud, and a stubbed `df` drives the free-space refusal.
sqlite, tar, gzip, age and gpg are the real binaries.

The gpg path always runs (gpg ships on CI runners). The age path runs when an
`age` binary is on PATH and skips otherwise; pass extra directories via
BACKUP_TEST_EXTRA_PATH (colon-separated) to exercise it locally.
"""
from contextlib import closing, contextmanager
import fcntl
import glob
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import tempfile
import threading
import time
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "deploy/backups/campfire-backup.sh"
DB_SENTINEL = "sentinel-db-row-7f3a91"
FILE_SENTINEL = "sentinel-file-data-9c2e44"

# The script must never invoke gcloud: the workflow uploads, not the VM.
# Any call fails loudly and is recorded for the assertion.
GCLOUD_POISON = """#!/usr/bin/env bash
echo "gcloud $*" >> "$STUB_LOG"
echo "poison gcloud must never be invoked" >&2
exit 9
"""


def search_path(*prepend):
    parts = [*prepend, os.environ.get("BACKUP_TEST_EXTRA_PATH"), os.environ.get("PATH")]
    return ":".join(part for part in parts if part is not None)


def stub_path_env(dirs=None):
    return {"PATH": search_path(str(dirs["bin"])) if dirs else search_path()}


def capture(args, env=None, cwd=None):
    """Open3.capture2e: run with stdin closed, stdout+stderr merged."""
    full = None if env is None else {**os.environ, **env}
    result = subprocess.run(args, env=full, cwd=cwd, stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return result.stdout, result


def sqlite_query(db, sql):
    out, status = capture(["sqlite3", str(db), sql])
    if status.returncode != 0:
        raise RuntimeError(f"sqlite3 failed: {out}")
    return out


def sqlite_count(db, table):
    return int(sqlite_query(db, f'SELECT count(*) FROM "{table}";').strip())


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def glob_paths(pattern):
    return sorted(glob.glob(str(pattern), recursive=True))


def age_available():
    return shutil.which("age", path=stub_path_env()["PATH"]) is not None


class CampfireBackupTest(unittest.TestCase):
    def test_backs_up_a_live_database_while_a_writer_keeps_writing(self):
        with self.backup_env() as (env, dirs):
            seed = self.seed_volume(dirs["volume"], messages=50)
            stop = threading.Event()
            warmed_up = threading.Event()
            writer_errors = []

            def write_rows():
                try:
                    with closing(sqlite3.connect(seed, timeout=5, isolation_level=None)) as db:
                        db.execute("INSERT INTO messages(body) VALUES ('writer-warmup')")
                        warmed_up.set()
                        while not stop.is_set():
                            db.execute("INSERT INTO messages(body) VALUES ('writer-row')")
                            time.sleep(0.002)
                except BaseException as error:
                    writer_errors.append(error)
                    warmed_up.set()

            writer = threading.Thread(target=write_rows)
            writer.start()
            warmed_up.wait()

            try:
                if writer_errors:
                    raise writer_errors[0]
                _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
                self.assertEqual(0, status.returncode, "backup failed while the writer was running")
            finally:
                stop.set()
                writer.join()
            if writer_errors:
                raise writer_errors[0]

            live_count = sqlite_count(seed, "messages")
            snapshot = self.extract_snapshot(dirs, "20260923-090000", age=False)
            self.assertEqual("ok", sqlite_query(snapshot["db"], "PRAGMA integrity_check;").strip())
            snapshot_count = sqlite_count(snapshot["db"], "messages")
            self.assertGreaterEqual(snapshot_count, 51, f"snapshot lost committed rows: {snapshot_count}")
            self.assertLessEqual(snapshot_count, live_count, "snapshot has rows from the future")
            self.assertEqual(FILE_SENTINEL, Path(snapshot["file"]).read_text().strip())

    def test_output_bytes_are_encrypted_and_round_trip_through_decryption(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=5)
            _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertEqual(0, status.returncode)

            blob = self.output_blob(dirs, "20260923-090000", "gpg")
            self.assertNotIn(DB_SENTINEL.encode(), blob, "database contents leaked into the output")
            self.assertNotIn(FILE_SENTINEL.encode(), blob, "file contents leaked into the output")
            _out, gzip_status = capture(["gzip", "-t", self.output_path(dirs, "20260923-090000", "gpg")])
            self.assertNotEqual(0, gzip_status.returncode, "output is plain gzip, not encrypted")

            snapshot = self.extract_snapshot(dirs, "20260923-090000", age=False)
            self.assertIn(DB_SENTINEL.encode(), Path(snapshot["db"]).read_bytes())

    def test_age_encryption_round_trips_when_age_is_available(self):
        if not age_available():
            self.skipTest("age is not on PATH")

        with self.backup_env(encryption="age") as (env, dirs):
            self.seed_volume(dirs["volume"], messages=5)
            _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertEqual(0, status.returncode)

            blob = self.output_blob(dirs, "20260923-090000", "age")
            self.assertTrue(blob.startswith(b"age-encryption.org/"), "missing age header")
            self.assertNotIn(DB_SENTINEL.encode(), blob)
            self.assertNotIn(FILE_SENTINEL.encode(), blob)

            snapshot = self.extract_snapshot(dirs, "20260923-090000", age=True)
            self.assertEqual("ok", sqlite_query(snapshot["db"], "PRAGMA integrity_check;").strip())
            manifest = json.loads(Path(snapshot["manifest"]).read_text())
            self.assertEqual("20260923-090000", manifest.get("stamp"))
            self.assertEqual(sha256_file(snapshot["db"]), (manifest.get("database") or {}).get("sha256"))

    def test_manifest_describes_the_backup_and_the_database_checksum_matches(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=3)
            _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertEqual(0, status.returncode)

            snapshot = self.extract_snapshot(dirs, "20260923-090000", age=False)
            manifest = json.loads(Path(snapshot["manifest"]).read_text())
            self.assertEqual("20260923-090000", manifest.get("stamp"))
            self.assertEqual("gpg", manifest.get("encryption"))
            self.assertEqual(1, (manifest.get("files") or {}).get("count"))
            self.assertEqual(sha256_file(snapshot["db"]), (manifest.get("database") or {}).get("sha256"))

    def test_output_dir_mode_produces_only_encrypted_output_and_prints_its_path_and_checksum(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=2)
            out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertEqual(0, status.returncode, f"backup failed: {out}")

            expected = self.output_path(dirs, "20260923-090000", "gpg")
            self.assertEqual([expected], glob_paths(dirs["out"] / "*"),
                "the output directory must hold exactly the encrypted archive")
            backup_file = re.search(r"^BACKUP_FILE=(.*)$", out, re.M)
            self.assertEqual(expected, backup_file and backup_file.group(1).strip(),
                f"BACKUP_FILE line missing or wrong:\n{out}")
            backup_sha = re.search(r"^BACKUP_SHA256=(.*)$", out, re.M)
            self.assertEqual(sha256_file(expected), backup_sha and backup_sha.group(1).strip(),
                f"BACKUP_SHA256 line missing or wrong:\n{out}")

            # No plaintext anywhere: the unencrypted tarball is removed and the
            # work directory goes with the EXIT trap.
            self.assertEqual([], glob_paths(dirs["root"] / "**/*.tar.gz"),
                "an unencrypted tarball survived the run")
            self.assertEqual([], glob_paths(dirs["state"] / "campfire-backup.*"),
                "the work directory survived the run")
            self.assertTrue((dirs["state"] / "campfire-nightly-last.json").is_file(),
                "the run summary was not written")

            # The script never uploads: a poison gcloud fails the run if invoked.
            self.assertEqual([], self.poison_log(dirs), "the script shelled out to gcloud")

    def test_accepts_the_output_directory_from_backup_output_dir(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            out, status = self.run_backup(env, dirs,
                {"BACKUP_DATETIME": "20260923-090000", "BACKUP_OUTPUT_DIR": str(dirs["out"])},
                [])
            self.assertEqual(0, status.returncode, f"backup failed: {out}")
            self.assertTrue(os.path.isfile(self.output_path(dirs, "20260923-090000", "gpg")))

    def test_refuses_an_output_directory_inside_its_own_work_directory(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            inside = str(dirs["state"] / "campfire-backup.20260923-090000" / "sub")
            _out, status = self.run_backup(env, dirs,
                {"BACKUP_DATETIME": "20260923-090000"}, ["--output-dir", inside])
            self.assertNotEqual(0, status.returncode,
                "an output dir inside the work dir would be deleted by the trap")

    def test_exits_75_when_a_release_holds_the_release_lock(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            lock_path = env["BACKUP_RELEASE_LOCK_FILE"]
            with open(lock_path, "w") as held:
                try:
                    fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    locked = True
                except OSError:
                    locked = False
                self.assertTrue(locked, "could not take the test lock")
                out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
                self.assertEqual(75, status.returncode,
                    f"expected EX_TEMPFAIL, got {status.returncode}:\n{out}")
                self.assertIn("a release holds", out)

    def test_refuses_to_start_without_enough_free_space(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=5)
            self.write_df_stub(dirs, avail_bytes=1024)
            out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertNotEqual(0, status.returncode, "the backup must not start on a nearly-full disk")
            self.assertIn("bytes free", out)
            self.assertEqual([], glob_paths(dirs["state"] / "campfire-backup.*"),
                "a refused run must not stage anything")

    def test_prunes_stale_work_directories_at_start_and_keeps_the_rest(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            stale = dirs["state"] / "campfire-backup.20200101-000000"
            fresh = dirs["state"] / "campfire-backup.fresh"
            other = dirs["state"] / "unrelated-old-dir"
            for directory in [stale, fresh, other]:
                directory.mkdir(parents=True, exist_ok=True)
                (directory / "leftover").write_text("x")
            # The backup script runs as a subprocess on the real clock, so the
            # stale mtimes come from real time too.
            old = time.time() - (3 * 24 * 3600)
            for path in [stale, stale / "leftover", other, other / "leftover"]:
                os.utime(path, (old, old))

            _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertEqual(0, status.returncode)

            self.assertFalse(stale.exists(), "a work dir older than a day must be pruned")
            self.assertTrue(fresh.is_dir(), "a fresh work dir must be kept")
            self.assertTrue(other.is_dir(), "a non-matching dir must be kept")

    def test_missing_database_exits_non_zero(self):
        with self.backup_env() as (env, dirs):
            out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "20260923-090000"})
            self.assertNotEqual(0, status.returncode)
            self.assertIn("database file not found", out)

    def test_bad_recipient_exits_non_zero(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            _out, status = self.run_backup(env, dirs, {
                "BACKUP_DATETIME": "20260923-090000",
                "BACKUP_GPG_RECIPIENT": "nobody-knows-this-key@example.com"})
            self.assertNotEqual(0, status.returncode)

    def test_malformed_stamp_exits_non_zero(self):
        with self.backup_env() as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            _out, status = self.run_backup(env, dirs, {"BACKUP_DATETIME": "yesterday-teatime"})
            self.assertNotEqual(0, status.returncode)

    def test_example_placeholder_age_recipient_exits_non_zero(self):
        if not age_available():
            self.skipTest("age is not on PATH")

        with self.backup_env(encryption="age") as (env, dirs):
            self.seed_volume(dirs["volume"], messages=1)
            out, status = self.run_backup(env, dirs, {
                "BACKUP_DATETIME": "20260923-090000",
                "BACKUP_AGE_RECIPIENT": "REPLACE-age1-public-recipient"})
            self.assertNotEqual(0, status.returncode)
            self.assertIn("placeholder", out)

    # --- helpers -----------------------------------------------------------

    @contextmanager
    def backup_env(self, encryption="gpg"):
        self.assert_path("sqlite3")
        with tempfile.TemporaryDirectory(prefix="campfire-backup-test") as tmp:
            root = Path(tmp)
            dirs = {
                "root": root,
                "volume": root / "vol",
                "state": root / "state",
                "out": root / "out",
                "bin": root / "bin",
            }
            for directory in dirs.values():
                directory.mkdir(parents=True, exist_ok=True)
            self.write_gcloud_poison(dirs)

            env = {
                "BACKUP_ENCRYPTION": encryption,
                "BACKUP_VOLUME_DIR": str(dirs["volume"]),
                "BACKUP_STATE_ROOT": str(dirs["state"]),
                "BACKUP_LOCK_FILE": str(root / "backup.lock"),
                "BACKUP_RELEASE_LOCK_FILE": str(root / "release.lock"),
                "STUB_LOG": str(root / "calls.log"),
            }
            home = None
            if encryption == "age":
                env["BACKUP_AGE_RECIPIENT"] = self.age_recipient(root)
                self.age_identity = root / "age-key.txt"
            else:
                home = root / "gnupg"
                home.mkdir()
                home.chmod(0o700)
                self.generate_gpg_key(home)
                env["BACKUP_GPG_HOME"] = str(home)
                env["BACKUP_GPG_RECIPIENT"] = "backup-test@example.com"
                self.gpg_home = home

            try:
                yield env, dirs
            finally:
                if home is not None:
                    # Stop this homedir's gpg-agent rather than leaving it to
                    # notice its socket is gone.
                    subprocess.run(["gpgconf", "--homedir", str(home), "--kill", "gpg-agent"],
                        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def run_backup(self, env, dirs, overrides, args=None):
        if args is None:
            args = ["--output-dir", str(dirs["out"])]
        full = {**stub_path_env(dirs), **env, **{k: v for k, v in overrides.items() if v is not None}}
        for key, value in overrides.items():
            if value is None:
                full.pop(key, None)
        return capture([str(SCRIPT), *args], env=full)

    def write_gcloud_poison(self, dirs):
        stub = dirs["bin"] / "gcloud"
        stub.write_text(GCLOUD_POISON)
        stub.chmod(0o755)

    def write_df_stub(self, dirs, avail_bytes):
        stub = dirs["bin"] / "df"
        stub.write_text(f"#!/usr/bin/env bash\nprintf 'Avail\\n{avail_bytes}\\n'\n")
        stub.chmod(0o755)

    def poison_log(self, dirs):
        log = dirs["root"] / "calls.log"
        if not log.is_file():
            return []
        return [line.strip() for line in log.read_text().splitlines(keepends=True)]

    def seed_volume(self, volume, messages):
        db_dir = volume / "db"
        files_dir = volume / "files/ab"
        db_dir.mkdir(parents=True, exist_ok=True)
        files_dir.mkdir(parents=True, exist_ok=True)
        db = str(db_dir / "production.sqlite3")
        sqlite_query(db, "PRAGMA journal_mode=WAL;")
        sqlite_query(db, f"""CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE rooms(id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE messages(id INTEGER PRIMARY KEY, body TEXT);
INSERT INTO users(name) VALUES ('{DB_SENTINEL}');
INSERT INTO rooms(name) VALUES ('general');
""")
        for n in range(messages):
            sqlite_query(db, f"INSERT INTO messages(body) VALUES ('seed-{n}');")
        (files_dir / "blob1").write_text(FILE_SENTINEL)
        return db

    def output_path(self, dirs, datetime, ext):
        return str(dirs["out"] / f"smartfire-backup-{datetime}.tar.gz.{ext}")

    def output_blob(self, dirs, datetime, ext):
        return Path(self.output_path(dirs, datetime, ext)).read_bytes()

    def extract_snapshot(self, dirs, datetime, age):
        """Decrypts the output archive and extracts it, mirroring the documented
        restore path (decrypt -> tar -> SHA256SUMS), and returns the paths."""
        work = dirs["root"] / "extracted"
        work.mkdir(parents=True, exist_ok=True)
        plain = str(work / "backup.tar.gz")
        if age:
            _out, status = capture(["age", "--decrypt", "--identity", str(self.age_identity),
                "--output", plain, self.output_path(dirs, datetime, "age")], env=stub_path_env(dirs))
            self.assertEqual(0, status.returncode, "age decryption failed")
        else:
            _out, status = capture(["gpg", "--batch", "--yes", "--homedir", str(self.gpg_home),
                "--decrypt", "--output", plain, self.output_path(dirs, datetime, "gpg")])
            self.assertEqual(0, status.returncode, "gpg decryption failed")
        _out, status = capture(["tar", "-xzf", plain, "-C", str(work)])
        self.assertEqual(0, status.returncode, "tar extraction failed")
        stage = work / f"smartfire-backup-{datetime}"
        sums, status = capture(["sha256sum", "-c", "SHA256SUMS"], cwd=str(stage))
        self.assertEqual(0, status.returncode, f"SHA256SUMS failed:\n{sums}")
        return {"db": str(stage / "production.sqlite3"),
                "file": str(stage / "files/ab/blob1"),
                "manifest": str(stage / "manifest.json")}

    def generate_gpg_key(self, home):
        _out, status = capture(["gpg", "--batch", "--pinentry-mode", "loopback",
            "--passphrase", "", "--homedir", str(home),
            "--quick-generate-key", "backup-test@example.com", "rsa2048", "encr", "never"])
        if status.returncode != 0:
            raise RuntimeError("gpg key generation failed")

    def age_recipient(self, root):
        key_file = str(root / "age-key.txt")
        _out, status = capture(["age-keygen", "-o", key_file], env=stub_path_env())
        if status.returncode != 0:
            raise RuntimeError("age-keygen failed")
        match = re.search(r"^# public key: (age1[0-9a-z]+)", Path(key_file).read_text(), re.M)
        if not match:
            raise RuntimeError("no age recipient")
        return match.group(1)

    def assert_path(self, tool, prepend=None):
        path = search_path(*([prepend] if prepend else []))
        self.assertIsNotNone(shutil.which(tool, path=path),
            f"{tool} is not on PATH (needed by the backup test)")


if __name__ == "__main__":
    unittest.main()
