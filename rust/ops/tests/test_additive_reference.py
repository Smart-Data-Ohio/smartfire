"""Differential against OUR pinned Rails verifier; requires an explicitly supplied binary."""
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
BINARY = os.environ.get("WS18_BINARY")
REFERENCE = os.environ.get("WS18_REFERENCE_IMAGE", "ws6-reference-d7c7de92:latest")


@unittest.skipUnless(BINARY, "set WS18_BINARY to the built campfire binary")
class AdditiveReferenceTest(unittest.TestCase):
    def test_full_schema_preservation_cases_match_rails(self):
        cases = {
            "unchanged": "",
            "additive": "ALTER TABLE ws18_data ADD COLUMN optional TEXT; CREATE TABLE ws18_added(id INTEGER);",
            "row": "UPDATE ws18_data SET data='changed';",
            "delete": "DELETE FROM ws18_data;",
            "type": "UPDATE ws18_data SET data=CAST(data AS TEXT);",
            "table": "DROP TABLE ws18_data;",
            "column": "ALTER TABLE ws18_data DROP COLUMN data;",
            "index": "DROP INDEX ws18_index;",
            "trigger": "DROP TRIGGER ws18_trigger;",
            "fk_added": "DROP TABLE ws18_children; CREATE TABLE ws18_children(a INTEGER,b INTEGER,FOREIGN KEY(a) REFERENCES users(id),FOREIGN KEY(b) REFERENCES messages(id));",
            "fk_action": "DROP TABLE ws18_children; CREATE TABLE ws18_children(a INTEGER,b INTEGER,FOREIGN KEY(a) REFERENCES users(id) ON DELETE CASCADE);",
            "fk_removed": "DROP TABLE ws18_children; CREATE TABLE ws18_children(a INTEGER,b INTEGER);",
            "same_file": "",
        }
        scratch = ROOT / ".scratch"
        scratch.mkdir(exist_ok=True)
        for name, sql in cases.items():
            with self.subTest(case=name), tempfile.TemporaryDirectory(dir=scratch) as tmp:
                work = Path(tmp)
                before, after = work / "before.sqlite3", work / "after.sqlite3"
                with sqlite3.connect(before) as conn:
                    conn.executescript((ROOT / "rust/crates/db/src/schema.sql").read_text())
                    conn.executescript("CREATE TABLE ws18_data(id INTEGER PRIMARY KEY,data BLOB); INSERT INTO ws18_data VALUES(1,X'00ff'); CREATE INDEX ws18_index ON ws18_data(id); CREATE TRIGGER ws18_trigger AFTER INSERT ON ws18_data BEGIN SELECT 1; END; CREATE TABLE ws18_children(a INTEGER,b INTEGER,FOREIGN KEY(a) REFERENCES users(id));")
                after.write_bytes(before.read_bytes())
                with sqlite3.connect(after) as conn:
                    conn.executescript(sql)
                target = before if name == "same_file" else after
                ours = subprocess.run([BINARY, "verify-additive-sqlite-migration", str(before), str(target)], text=True, capture_output=True)
                target_name = "before.sqlite3" if name == "same_file" else "after.sqlite3"
                rails = subprocess.run(["docker", "run", "--rm", "--name", f"ws18-verifier-{name}",
                    "--network", "none", "-v", f"{work}:/fixtures:ro", "--entrypoint", "bash", REFERENCE,
                    "-c", f"bundle exec script/admin/verify-additive-sqlite-migration /fixtures/before.sqlite3 /fixtures/{target_name}"],
                    text=True, capture_output=True)
                self.assertEqual((ours.returncode, ours.stdout, ours.stderr), (rails.returncode, rails.stdout, rails.stderr))


if __name__ == "__main__":
    unittest.main()
