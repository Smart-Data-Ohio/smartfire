"""Encrypted nightly backup through the Rust container's real admin shim."""
from contextlib import closing
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[3]
IMAGE = os.environ.get("WS18_IMAGE")

FAKE_DOCKER = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
args=sys.argv[1:]
with open(os.environ["TRACE"], "a") as f:
    f.write(json.dumps(args) + "\n")
if args[0] == "ps":
    print("once-app-fixture")
elif args[0] == "inspect":
    fmt=args[args.index("--format")+1]
    if '"once"' in fmt:
        print(json.dumps({"host":"fixture.invalid", "image":os.environ["WS18_IMAGE"]}))
    elif ".Mounts" in fmt:
        print("fixture-volume")
    else:
        sys.exit("unexpected inspect")
elif args[:2] == ["volume", "inspect"]:
    print(os.environ["VOLUME"])
elif args[:2] == ["exec", "once-app-fixture"] and args[2:] == ["/rails/script/admin/prepare-backup"]:
    os.execv(os.environ["REAL_DOCKER"], ["docker", "exec", "ws18-nightly-backup", *args[2:]])
else:
    sys.exit("unexpected Docker operation " + repr(args))
'''


@unittest.skipUnless(IMAGE, "set WS18_IMAGE to a locally built image")
class BackupContainerTest(unittest.TestCase):
    def test_nightly_archive_roundtrips_with_rust_and_rails_image_checks(self):
        with tempfile.TemporaryDirectory(dir=ROOT / ".scratch", prefix="nightly-") as tmp:
            work = Path(tmp)
            volume = work / "volume"
            (volume / "db").mkdir(parents=True)
            (volume / "files").mkdir()
            shutil.copy2(ROOT / "rust/parity/.seed/default/db/production.sqlite3", volume / "db/production.sqlite3")
            (volume / "files/sentinel").write_text("ws18 upload preserved")
            with closing(sqlite3.connect(volume / "db/production.sqlite3")) as conn:
                conn.execute("CREATE TABLE ws18_backup_writes(id INTEGER PRIMARY KEY)")
                conn.commit()
            out, state, bin_dir = work / "out", work / "state", work / "bin"
            for directory in [out, state, bin_dir]:
                directory.mkdir()
            identity = work / "identity.txt"
            subprocess.run(["age-keygen", "-o", str(identity)], check=True, capture_output=True)
            recipient = next(line.removeprefix("# public key: ") for line in identity.read_text().splitlines() if line.startswith("# public key:"))
            (bin_dir / "docker").write_text(FAKE_DOCKER)
            (bin_dir / "docker").chmod(0o755)
            (bin_dir / "gcloud").write_text("#!/bin/sh\necho forbidden-cloud-command >&2\nexit 99\n")
            (bin_dir / "gcloud").chmod(0o755)
            real_docker = shutil.which("docker")
            subprocess.run([real_docker, "run", "-d", "--name", "ws18-nightly-backup", "--network", "none", "--memory", "768m",
                "--env-file", str(ROOT / "rust/parity/.env.reference"), "-v", f"{volume}:/rails/storage", IMAGE], check=True, capture_output=True)
            stopping, started = threading.Event(), threading.Event()
            writer = None
            try:
                writer_errors = []
                def write_rows():
                    try:
                        with closing(sqlite3.connect(volume / "db/production.sqlite3", timeout=5)) as conn:
                            while not stopping.is_set():
                                conn.execute("INSERT INTO ws18_backup_writes DEFAULT VALUES")
                                conn.commit()
                                started.set()
                                stopping.wait(0.002)
                    except Exception as error:
                        writer_errors.append(error)
                        started.set()
                writer = threading.Thread(target=write_rows)
                writer.start()
                self.assertTrue(started.wait(5), "writer did not start")
                env = {**os.environ, "WS18_IMAGE": IMAGE, "REAL_DOCKER": real_docker, "VOLUME": str(volume),
                    "TRACE": str(work / "docker-trace.jsonl"), "PATH": f"{bin_dir}:{os.environ['PATH']}",
                    "BACKUP_ENCRYPTION": "age", "BACKUP_AGE_RECIPIENT": recipient,
                    "BACKUP_VOLUME_DIR": "", "BACKUP_STATE_ROOT": str(state), "BACKUP_OUTPUT_DIR": str(out),
                    "BACKUP_LOCK_FILE": str(work / "backup.lock"), "BACKUP_RELEASE_LOCK_FILE": str(work / "release.lock"),
                    "BACKUP_DATETIME": "20260930-000000"}
                try:
                    result = subprocess.run([str(ROOT / "deploy/backups/campfire-backup.sh")], env=env, text=True, capture_output=True)
                finally:
                    stopping.set()
                    writer.join(10)
                self.assertFalse(writer.is_alive())
                self.assertEqual(writer_errors, [])
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                archive = out / "smartfire-backup-20260930-000000.tar.gz.age"
                self.assertEqual(list(out.iterdir()), [archive])
                calls = [json.loads(line) for line in (work / "docker-trace.jsonl").read_text().splitlines()]
                self.assertIn(["exec", "once-app-fixture", "/rails/script/admin/prepare-backup"], calls)
                # Restore checks run against separate disposable copies, with real Docker again.
                for index, image in enumerate([IMAGE, "ws6-reference-d7c7de92:latest"]):
                    restore = work / f"restore-{index}"
                    result = subprocess.run([str(ROOT / "deploy/backups/restore-check.sh"), "--backup", str(archive),
                        "--work-dir", str(restore), "--age-identity", str(identity), "--image", image,
                        "--container-name", f"ws18-restore-check-{index}", "--min-users", "1"], text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    stage = restore / "extracted/smartfire-backup-20260930-000000"
                    self.assertEqual((stage / "files/sentinel").read_text(), "ws18 upload preserved")
                    with closing(sqlite3.connect(stage / "production.sqlite3")) as conn:
                        self.assertEqual(conn.execute("PRAGMA integrity_check").fetchone()[0], "ok")
                        snapshot_count = conn.execute("SELECT count(*) FROM ws18_backup_writes").fetchone()[0]
                    with closing(sqlite3.connect(volume / "db/production.sqlite3")) as conn:
                        live_count = conn.execute("SELECT count(*) FROM ws18_backup_writes").fetchone()[0]
                    self.assertGreaterEqual(snapshot_count, 1)
                    self.assertLessEqual(snapshot_count, live_count)
                print("NIGHTLY BACKUP: real Rust shim with concurrent writer -> encrypted archive -> Rust and Rails image restore checks passed")
            finally:
                stopping.set()
                if writer is not None:
                    writer.join(10)
                subprocess.run([real_docker, "rm", "-f", "ws18-nightly-backup"], check=True, capture_output=True)


if __name__ == "__main__":
    unittest.main()
