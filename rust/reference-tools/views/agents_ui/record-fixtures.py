#!/usr/bin/env python3
"""Record the agents-UI system fixtures from pinned Rails, once, for the Rust-only browser suite.

system_behavior.py ran a Rails fixture script (system_fixture.rb, budget_fixture.rb,
work_fixture.rb, inbox_browser_fixture.rb) on a private Rails instance booted from the
agents_ui seed, then copied that instance's database to Rust. This records each
scenario's effect one last time:

  test-support/agents-ui-fixtures/SCENARIO/patch.sql    sqldiff from the agents_ui seed
  test-support/agents-ui-fixtures/SCENARIO/labels.json  the fixture's printed labels

Applying patch.sql to a copy of the frozen agents_ui seed reproduces the Rails instance's
database (checked here with sqldiff). Run from the repository root with the pinned
reference image built (parity/bin/ci-seed).
"""
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "users"))
from reference_runtime import ReferenceNetwork  # noqa: E402

ROOT = Path(__file__).resolve().parents[4]
RUST = ROOT / "rust"
OUT = RUST / "test-support/agents-ui-fixtures"
SEED = RUST / "parity/.seed/agents_ui"
REFERENCE = RUST / "parity/bin/reference"
SCENARIOS = {
    "pages": ("system_fixture.rb", "2026-03-02T16:00:00Z"),
    "budget": ("budget_fixture.rb", "2026-03-03T16:00:00Z"),
    "work": ("work_fixture.rb", "2026-03-02T16:00:00Z"),
    "inbox": ("inbox_browser_fixture.rb", "2026-03-02T16:00:00Z"),
    "inbox-filter": ("inbox_browser_fixture.rb", "2026-03-02T16:00:00Z"),
}


def sequence_last(patch):
    """sqldiff writes sqlite_sequence rows by rowid; inserting the tables' rows creates them
    too, so replay them last and by replacement."""
    lines = patch.decode().splitlines(keepends=True)
    sequence = [line.replace("INSERT INTO sqlite_sequence", "INSERT OR REPLACE INTO sqlite_sequence")
                for line in lines if "sqlite_sequence" in line]
    return "".join([line for line in lines if "sqlite_sequence" not in line] + sequence).encode()


def record(scenario, port):
    script, frozen_time = SCENARIOS[scenario]
    with tempfile.TemporaryDirectory(prefix="record-agents-ui-", dir=ROOT / ".scratch") as scratch:
        work = Path(scratch)
        shutil.copytree(SEED, work / "seeds/agents_ui")
        env = dict(os.environ, PARITY_NAMESPACE="rails-free-record-ui", PARITY_OWNER="rails-free-record",
                   PARITY_SEED_DIR=str(work / "seeds"), PARITY_RUNTIME="docker", WS11UI_INBOX_CASE=scenario,
                   PARITY_IMAGE=os.environ["RECORD_IMAGE"])
        network = ReferenceNetwork(env["PARITY_NAMESPACE"], work.name, env["PARITY_OWNER"])
        env["PARITY_NETWORK"] = network.name
        subprocess.run([str(REFERENCE), "up", "--seed", "agents_ui", "--port", str(port), "--time", frozen_time, "--freeze"],
                       cwd=ROOT, env=env, check=True)
        try:
            labels = subprocess.check_output([str(REFERENCE), "runner", "--port", str(port), "--time", frozen_time, "--freeze",
                                              str(RUST / "reference-tools/views/agents_ui" / script), scenario],
                                             cwd=ROOT, env=env, text=True)
            json.loads(labels)
            instance = work / f"seeds/.instances/{port}/db/production.sqlite3"
            snapshot = work / "snapshot.sqlite3"
            # The same online backup system_behavior.py took of the running instance.
            with sqlite3.connect(instance) as source, sqlite3.connect(snapshot) as target:
                source.backup(target)
            source.close()
            target.close()
        finally:
            subprocess.run([str(REFERENCE), "down", "--port", str(port)], cwd=ROOT, env=env, check=False, stdout=subprocess.DEVNULL)
            network.close()
        patch = sequence_last(subprocess.check_output(["sqldiff", str(SEED / "db/production.sqlite3"), str(snapshot)]))
        replay = work / "replay.sqlite3"
        shutil.copyfile(SEED / "db/production.sqlite3", replay)
        with sqlite3.connect(replay) as conn:
            conn.executescript(patch.decode())
        conn.close()
        residue = subprocess.check_output(["sqldiff", str(replay), str(snapshot)])
        assert not residue.strip(), f"{scenario}: patch doesn't reproduce the instance:\n{residue.decode()[:2000]}"
        before = {p.relative_to(SEED / "storage") for p in (SEED / "storage").rglob("*") if p.is_file()}
        after = {p.relative_to(work / f"seeds/.instances/{port}/storage") for p in (work / f"seeds/.instances/{port}/storage").rglob("*") if p.is_file()} \
            if (work / f"seeds/.instances/{port}/storage").exists() else before
        assert after <= before | set(), f"{scenario}: fixture added storage files {sorted(map(str, after - before))}"
    target = OUT / scenario
    shutil.rmtree(target, ignore_errors=True)
    target.mkdir(parents=True)
    (target / "patch.sql").write_bytes(patch)
    (target / "labels.json").write_text(json.dumps(json.loads(labels), indent=2, sort_keys=True) + "\n")
    print(f"recorded {scenario}: {len(patch)} bytes of SQL, labels {sorted(json.loads(labels))}", flush=True)


if __name__ == "__main__":
    (ROOT / ".scratch").mkdir(exist_ok=True)
    for index, scenario in enumerate(sys.argv[1:] or SCENARIOS):
        record(scenario, 49610 + index)
