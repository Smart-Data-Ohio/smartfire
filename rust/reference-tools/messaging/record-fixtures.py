#!/usr/bin/env python3
"""Record every behaviour-check fixture from pinned Rails, once, for the Rust-only suite.

behavior-check.py built each case's fixture by running behavior-fixtures.rb in the Rails
reference against a copy of the default seed. Rails is going away, so this runs each
distinct fixture invocation one last time and records its effect on that seed:

  fixtures/KEY/patch.sql   sqldiff from the default seed's database to the fixture's
  fixtures/KEY/db/*        files the fixture wrote beside the database (browser-fixture.json)
  fixtures/KEY/storage/*   storage files the fixture added

Applying patch.sql to a copy of the frozen default seed and copying the files back
reproduces the Rails-built fixture (the recorder checks that round trip with sqldiff).
Run from the repository root with the pinned reference image built (parity/bin/ci-seed).
"""
import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "rust"
PIN = (RUST / "parity/reference.sha").read_text().strip()
OUT = RUST / "test-support/behavior-fixtures"
SEED = RUST / "parity/.seed/default"
FIXTURES = RUST / "reference-tools/messaging/behavior-fixtures.rb"

source = (RUST / "reference-tools/messaging/behavior-check.py").read_text()
CASES = eval(source[source.index("CASES = {"):source.index("\n}\n") + 2].split("=", 1)[1].strip())


def pinned(path):
    return subprocess.check_output(["git", "show", f"{PIN}:{path}"], cwd=ROOT)


def invocations():
    """(kind, argument, extra files) for every case, mirroring behavior-check.py's fixture chain."""
    new_upload = {"uploading a fresh video in the thread composer", "late upload progress preserves a delivered attachment and reply preview"}
    for file, cases in CASES.items():
        for case in cases:
            extra = {}
            if file == "motion":
                yield ("motion", case, extra); continue
            if file == "channel_threads_controller":
                yield ("work-controller", case, extra); continue
            if file == "mobile_layout":
                yield ("mobile-layout", None, extra); continue
            if case in new_upload or file == "workspace_markdown" and case == "Markdown replies and file attachments remain usable":
                yield ("workspace-upload", None, extra); continue
            if file == "message_list_a11y":
                kind = "board-touch" if case.startswith("text fields") else "history" if case.startswith("paginated history") else "message_list"
                if case in cases[20:28]:
                    kind = "message_destinations"
                yield (kind, None, extra); continue
            if file == "search_forward_edit":
                yield ("search" if case.startswith("search tolerates") else "edit-card" if case.startswith("editing to add") else "forward", None, extra); continue
            if file == "unread_divider":
                yield ("unread-" + ["few", "many", "pill", "offpage", "menu"][cases.index(case)], None, extra); continue
            if file == "composer":
                yield ("composer-typing" if case.startswith("two typers") else "composer-drafts", None, extra); continue
            if file == "drive_attachments" or case == "From Google Drive starts the legacy picker flow":
                extra["google-calendar-test-helper.rb"] = pinned("test/test_helpers/google_calendar_test_helper.rb")
                yield ("drive", case, extra); continue
            if file == "composer_attach_menu":
                kind = "attach-share" if case.startswith("From Google Drive starts the enhanced") else "attach-menu"
                if kind == "attach-share":
                    extra["drive-share-mocks.rb"] = pinned("test/support/drive_share_mocks.rb")
                yield (kind, None, extra); continue
            if file == "boosting_messages":
                yield ("boosts", None, extra); continue
            if file in ("message_interactions", "message_actions_mobile", "message_toolbar"):
                yield ({"message_interactions": "interactions", "message_actions_mobile": "actions-mobile", "message_toolbar": "toolbar"}[file], None, extra); continue
            if file == "threads" and (case.startswith("opens a shared") or case.startswith("keeps an anchored")):
                yield ("thread-anchor" if case.startswith("opens a shared") else "thread-anchor-race", None, extra); continue
            if file == "threads" and case == "discusses a pull request from its card":
                yield ("thread-pr", None, extra); continue
            if file == "code_highlighting":
                extra["code-highlighting-reference.rb"] = pinned("test/system/code_highlighting_test.rb")
                extra["application-system-reference.rb"] = pinned("test/application_system_test_case.rb")
                yield ("highlight-thread" if case.startswith("thread code stays") else "highlight-search" if case.startswith("search results") else "highlight", None, extra); continue
            yield (None, None, extra)  # The plain default seed.


def key(kind, argument):
    if argument is None:
        return kind
    return kind + "--" + re.sub(r"[^a-z0-9]+", "-", argument.lower()).strip("-")[:60] + "-" + hashlib.sha256(argument.encode()).hexdigest()[:8]


def sequence_last(patch):
    """sqldiff writes sqlite_sequence rows by rowid; inserting the tables' rows creates them
    too, so replay them last and by replacement."""
    lines = patch.decode().splitlines(keepends=True)
    sequence = [line.replace("INSERT INTO sqlite_sequence", "INSERT OR REPLACE INTO sqlite_sequence")
                for line in lines if "sqlite_sequence" in line]
    return "".join([line for line in lines if "sqlite_sequence" not in line] + sequence).encode()


def files_under(directory):
    return {path.relative_to(directory).as_posix(): path for path in directory.rglob("*") if path.is_file()} if directory.exists() else {}


def record(kind, argument, extra, env):
    name = key(kind, argument)
    target = OUT / name
    with tempfile.TemporaryDirectory(prefix="record-", dir=ROOT / ".scratch") as scratch:
        fixture = Path(scratch) / "fixture"
        shutil.copytree(SEED, fixture)
        for filename, contents in extra.items():
            (fixture / "db" / filename).write_bytes(contents)
        command = [str(RUST / "parity/bin/reference"), "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z",
                   "--freeze", str(FIXTURES), kind] + ([argument] if argument is not None else [])
        subprocess.run(command, cwd=ROOT, env=env, check=True)
        database = fixture / "db/production.sqlite3"
        with sqlite3.connect(database) as conn:
            conn.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        conn.close()
        patch = sequence_last(subprocess.check_output(["sqldiff", str(SEED / "db/production.sqlite3"), str(database)]))
        shutil.rmtree(target, ignore_errors=True)
        (target / "db").mkdir(parents=True)
        (target / "patch.sql").write_bytes(patch)
        for relative, path in files_under(fixture / "db").items():
            if relative.startswith("production.sqlite3") or relative in extra:
                continue
            (target / "db" / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target / "db" / relative)
        before = files_under(SEED / "storage")
        for relative, path in files_under(fixture / "storage").items():
            if relative in before and before[relative].read_bytes() == path.read_bytes():
                continue
            assert relative not in before, f"{name}: fixture changed an existing storage file {relative}"
            (target / "storage" / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target / "storage" / relative)
        assert set(files_under(SEED / "storage")) <= set(files_under(fixture / "storage")), f"{name}: fixture deleted storage"
        # Round trip: the frozen seed plus the patch is the Rails fixture.
        replay = Path(scratch) / "replay.sqlite3"
        shutil.copyfile(SEED / "db/production.sqlite3", replay)
        with sqlite3.connect(replay) as conn:
            conn.executescript(patch.decode())
        conn.close()
        residue = subprocess.check_output(["sqldiff", str(replay), str(database)])
        assert not residue.strip(), f"{name}: patch doesn't reproduce the fixture:\n{residue.decode()[:2000]}"
    print(f"recorded {name}: {len(patch)} bytes of SQL", flush=True)
    return name


if __name__ == "__main__":
    (ROOT / ".scratch").mkdir(exist_ok=True)
    env = dict(os.environ, PARITY_CPUS="2", PARITY_NAMESPACE="rails-free-record", PARITY_OWNER="rails-free-record",
               PARITY_RUNTIME="docker", TMPDIR=str(ROOT / ".scratch"), CAMPFIRE_REFERENCE=str(ROOT))
    env["PARITY_IMAGE"] = os.environ["RECORD_IMAGE"]
    planned = list(invocations())
    named = [(file, case) for file, cases in CASES.items() for case in cases]
    assert len(planned) == len(named)
    only = set(sys.argv[1:])
    index = {}
    seen = set()
    for (file, case), (kind, argument, extra) in zip(named, planned):
        name = key(kind, argument) if kind else None
        index.setdefault(file, {})[case] = name
        if name and name not in seen and (not only or name in only):
            seen.add(record(kind, argument, extra, env))
    (OUT / "index.json").write_text(json.dumps({"reference": PIN, "seed": "parity/seeds/frozen/default", "cases": index}, indent=2, ensure_ascii=False) + "\n")
    print(f"recorded {len(seen)} fixtures for {sum(len(c) for c in CASES.values())} cases", flush=True)
