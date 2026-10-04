"""Compare the real Rust operator's abort and partial progress with fresh Rails.

Run with a built campfire binary and the default parity seed. The environment
must freeze the wall clock at the vector instant (the pinned runtime supplies
libfaketime). No response/state/timestamp masking is used.
"""
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[1]
binary = Path(sys.argv[1]).resolve()
vector = json.loads((root / "vectors/twitter_backfill_attachment.json").read_text())
seed = root / "parity/.seed/default/db/production.sqlite3"


def state(db):
    references = list(map(list, db.execute("SELECT r.message_id,p.post_id FROM twitter_post_references r JOIN twitter_posts p ON p.id=r.twitter_post_id ORDER BY r.message_id,p.post_id")))
    posts = list(map(list, db.execute("SELECT post_id,url,CASE WHEN fetch_requested_at IS NOT NULL THEN strftime('%Y-%m-%dT%H:%M:%S',fetch_requested_at)||'.000000Z' END FROM twitter_posts ORDER BY post_id")))
    jobs = [row[0] for row in db.execute("SELECT p.post_id FROM background_jobs j JOIN twitter_posts p ON p.id=json_extract(j.arguments,'$.post_id') WHERE j.job_class='Twitter::FetchPostJob' ORDER BY p.post_id")]
    # Reject fractional timestamps before using the vector's exact whole-second format.
    for (at,) in db.execute("SELECT fetch_requested_at FROM twitter_posts WHERE fetch_requested_at IS NOT NULL"):
        assert at == "2026-03-02 16:00:00", at
    return dict(references=references, posts=posts, jobs=jobs)


with tempfile.TemporaryDirectory() as directory:
    path = Path(directory) / "production.sqlite3"
    shutil.copyfile(seed, path)
    db = sqlite3.connect(path)
    for row in vector["stored"]:
        db.execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,markdown_source,forward_note,created_at,updated_at) VALUES(?,486777696,127326141,?,?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')", (row["id"], f"backfill-{row['id']}", row["markdown"], row.get("forward_note")))
        db.execute("INSERT INTO action_text_rich_texts(name,record_type,record_id,body,created_at,updated_at) VALUES('body','Message',?,?,'2026-03-02 16:00:00','2026-03-02 16:00:00')", (row["id"], row["html"]))
    db.commit()
    source_rows = {name: db.execute(f"SELECT * FROM {name} ORDER BY id").fetchall() for name in ["messages", "action_text_rich_texts"]}
    for _ in range(2):
        result = subprocess.run([binary, "twitter-backfill-references", path], capture_output=True)
        assert result.returncode == vector["cli"]["exit_status"], result
        assert result.stdout == vector["cli"]["stdout"].encode(), result.stdout
        assert result.stderr == vector["cli"]["stderr"].encode(), result.stderr
        assert state(db) == vector["first"]["state"]
        snapshot = {name: db.execute(f"SELECT * FROM {name} ORDER BY id").fetchall() for name in ["messages", "action_text_rich_texts", "twitter_posts", "twitter_post_references", "background_jobs"]}
        assert {name: snapshot[name] for name in source_rows} == source_rows, "abort changed source messages/rich text"
        if _:
            assert snapshot == previous, "repeat changed complete persisted rows"
        previous = snapshot
print("Production Twitter attachment CLI: 2 aborts matched Rails exit/status, stdout/stderr bytes, exact partial state and unchanged repeat rows; 0 differences")
