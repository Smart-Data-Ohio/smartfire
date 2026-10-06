"""Real, locally built image checks. Only ws18 containers and ports are used."""
import concurrent.futures
from contextlib import closing
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import tempfile
import time
import unittest
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
IMAGE = os.environ.get("WS18_IMAGE")
SEED = ROOT / "rust/parity/.seed/default"


def docker(*args):
    return subprocess.check_output(["docker", *args], text=True).strip()


@unittest.skipUnless(IMAGE, "set WS18_IMAGE to a locally built image")
class ImageTest(unittest.TestCase):
    def test_contract(self):
        metadata = json.loads(docker("image", "inspect", IMAGE))[0]["Config"]
        self.assertEqual(metadata["Labels"].get("net.smartdata.campfire.runtime"), "rust")
        self.assertEqual(metadata["User"], "1000:1000")
        self.assertEqual(metadata["WorkingDir"], "/rails")
        self.assertEqual(metadata["Cmd"], ["bin/boot"])
        self.assertEqual(set(metadata["ExposedPorts"]), {"80/tcp", "443/tcp"})
        self.assertNotIn("Healthcheck", metadata)
        docker("run", "--rm", "--name", "ws18-hook-contract", "--network", "none", IMAGE,
               "bash", "-c", "test -x /hooks/pre-backup && test -x /hooks/post-restore && test -x /rails/script/admin/prepare-backup")

    def test_online_backup_and_restore_with_path_overrides(self):
        self.assertTrue((SEED / "db/production.sqlite3").is_file(), "build the default parity seed first")
        with tempfile.TemporaryDirectory(dir=ROOT / ".scratch", prefix="image-backup-") as tmp:
            work = Path(tmp)
            (work / "db").mkdir()
            (work / "appendonlydir").mkdir()
            marker = work / "appendonlydir/keep-until-cutover"
            marker.write_text("retained")
            shutil.copy2(SEED / "db/production.sqlite3", work / "db/production.sqlite3")
            for overrides, database, backup in [
                (["-e", "CAMPFIRE_STORAGE=/custom"], "db/production.sqlite3", "backups/production.sqlite3"),
                (["-e", "CAMPFIRE_STORAGE=/ignored", "-e", "CAMPFIRE_STORAGE_PATH=/custom",
                  "-e", "CAMPFIRE_DATABASE_PATH=/custom/db/renamed.sqlite3", "-e", "CAMPFIRE_BACKUPS_PATH=/custom/snapshots"],
                 "db/renamed.sqlite3", "snapshots/renamed.sqlite3"),
            ]:
                shutil.copy2(SEED / "db/production.sqlite3", work / database)
                options = ["--rm", "--network", "none", "-v", f"{work}:/custom", "-e", "SECRET_KEY_BASE_DUMMY=1", *overrides]
                docker("run", "--name", "ws18-prepare-backup", *options, IMAGE, "/rails/script/admin/prepare-backup")
                with closing(sqlite3.connect(work / backup)) as conn:
                    self.assertEqual(conn.execute("PRAGMA integrity_check").fetchone()[0], "ok")
                    self.assertGreater(conn.execute("SELECT count(*) FROM users").fetchone()[0], 0)
                completed = (work / backup).read_bytes()
                failure = subprocess.run(["docker", "run", "--name", "ws18-backup-failure", *options,
                    "-e", f"CAMPFIRE_DATABASE_PATH=/custom/missing/{Path(database).name}", IMAGE,
                    "/rails/script/admin/prepare-backup"], text=True, capture_output=True)
                self.assertNotEqual(failure.returncode, 0)
                self.assertEqual((work / backup).read_bytes(), completed)
                self.assertFalse((work / "missing" / Path(database).name).exists())
                (work / database).unlink()
                docker("run", "--name", "ws18-post-restore", *options, IMAGE, "/hooks/post-restore")
                self.assertEqual((work / database).read_bytes(), (work / backup).read_bytes())
                self.assertEqual(marker.read_text(), "retained")
                output = docker("run", "--name", "ws18-check-restored", *options, IMAGE, "campfire", "db-check", f"/custom/{database}")
                self.assertIn("migration versions accepted (read-only)", output)
                output = docker("run", "--rm", "--name", "ws18-check-readonly", "--network", "none",
                    "-v", f"{work}:/custom:ro", IMAGE, "campfire", "db-check", "--immutable", f"/custom/{database}")
                self.assertIn("migration versions accepted (read-only)", output)
            print("IMAGE BACKUP: default and explicit paths restored; Redis AOF retained")

    def test_seeded_server_and_rss(self):
        self.assertTrue((SEED / "db/production.sqlite3").is_file(), "build the default parity seed first")
        with tempfile.TemporaryDirectory(dir=ROOT / ".scratch", prefix="image-server-") as tmp:
            work = Path(tmp)
            shutil.copytree(SEED / "db", work / "db")
            shutil.copytree(SEED / "storage", work / "files")
            name = "ws18-image-server"
            docker("run", "-d", "--name", name, "--memory", "768m", "--cpus", "2",
                   "-p", "127.0.0.1:51800:80", "--env-file", str(ROOT / "rust/parity/.env.reference"),
                   "-v", f"{work}:/rails/storage", IMAGE)
            try:
                base = "http://127.0.0.1:51800"
                def request(path):
                    with urllib.request.urlopen(base + path, timeout=10) as response:
                        self.assertEqual(response.status, 200)
                        return response.read()
                for attempt in range(60):
                    try:
                        request("/up")
                        break
                    except OSError:
                        if attempt == 59:
                            self.fail(docker("logs", name))
                        time.sleep(0.5)
                self.assertIn(b"<html", request("/session/new"))
                def rss():
                    for line in docker("exec", name, "cat", "/proc/1/status").splitlines():
                        if line.startswith("VmRSS:"):
                            return line.split()[1]
                    self.fail("VmRSS absent")
                idle = rss()
                with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
                    list(pool.map(request, ["/up", "/session/new"] * 40))
                print(f"IMAGE HTTP: /up=200 /session/new=200; 80 requests succeeded; RSS idle={idle} kB after-load={rss()} kB")
            finally:
                docker("rm", "-f", name)


if __name__ == "__main__":
    unittest.main()
