#!/usr/bin/env python3
"""Read-only prevalence counts for the cutover, from a restored database. Prints counts only.

(a) Rich-text bodies embedding Active Storage blobs (<action-text-attachment sgid=...> whose
    signed GlobalID names ActiveStorage::Blob), which the Rust port renders as the
    missing-attachment marker: affected records by type, distinct rooms, newest created_at.
    The SGID payload is base64 JSON readable without the key; only the model name is used.
(b) Sessions that could still hold an AES-256-CBC encrypted cookie: sessions last active before
    GCM_SWITCH (UTC date) that have not expired. Members' sessions never expire; administrators'
    expire after ADMIN_IDLE_DAYS of inactivity (config/initializers/session_lifetimes.rb).

Usage: counts.py DB GCM_SWITCH [ADMIN_IDLE_DAYS=7]
"""
import base64
import collections
import html
import json
import re
import sqlite3
import sys

db = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
gcm_switch = sys.argv[2]
admin_idle_days = int(sys.argv[3]) if len(sys.argv) > 3 else 7

tag_re = re.compile(r"<action-text-attachment\b[^>]*>", re.I)
sgid_re = re.compile(r'\bsgid="([^"]+)"')


def sgid_model(sgid):
    data = html.unescape(sgid).split("--")[0]
    for decode in (base64.b64decode, base64.urlsafe_b64decode):
        try:
            gid = json.loads(decode(data + "=" * (-len(data) % 4)))["_rails"]["data"]
            return gid.split("/")[3] if gid.startswith("gid://") else "unknown"
        except Exception:
            continue
    return "undecodable"


models = collections.Counter()
affected = []
for record_type, record_id, body, created_at in db.execute(
        "select record_type, record_id, body, created_at from action_text_rich_texts"):
    blob = False
    for tag in tag_re.findall(body or ""):
        match = sgid_re.search(tag)
        model = sgid_model(match.group(1)) if match else "(no sgid: content attachment)"
        models[model] += 1
        blob = blob or model == "ActiveStorage::Blob"
    if blob:
        affected.append((record_type, record_id, created_at))

print("(a) rich-text records:", db.execute("select count(*) from action_text_rich_texts").fetchone()[0])
print("    attachment tags by kind:", dict(models))
print("    records embedding ActiveStorage::Blob:", len(affected), dict(collections.Counter(a[0] for a in affected)))
message_ids = [a[1] for a in affected if a[0] == "Message"]
rooms = 0
if message_ids:
    rooms = db.execute("select count(distinct room_id) from messages where id in (%s)" % ",".join("?" * len(message_ids)),
                       message_ids).fetchone()[0]
print("    distinct rooms:", rooms, "| newest created_at:", max((a[2] for a in affected), default="n/a"))

total, first, live = db.execute(f"""
    select count(*), min(s.created_at),
           sum(case when u.role = 1 and s.last_active_at < datetime('now', '-{admin_idle_days} days') then 0 else 1 end)
    from sessions s join users u on u.id = s.user_id""").fetchone()
before = db.execute(f"""
    select count(*) from sessions s join users u on u.id = s.user_id
    where s.last_active_at < ? and not (u.role = 1 and s.last_active_at < datetime('now', '-{admin_idle_days} days'))""",
                    (gcm_switch,)).fetchone()[0]
print(f"(b) sessions: {total} (oldest created_at {first}); unexpired now: {live}; "
      f"unexpired and last active before the GCM switch ({gcm_switch}): {before}")
