#!/usr/bin/env python3
"""Pick smoke-test URLs from a restored database, by id only (no names or content leave the DB).

Usage: targets.py DB VIEWER_ID PEER_ID > targets.json

Writes the room both users share (for the live-stream and write checks), a sample of the viewer's
rooms and DMs, user profiles, a board, events and threads, as URL paths for the smoke run.
"""
import json
import sqlite3
import sys

db = sqlite3.connect(f"file:{sys.argv[1]}?mode=ro", uri=True)
viewer, peer = int(sys.argv[2]), int(sys.argv[3])


def ids(query, *args):
    return [row[0] for row in db.execute(query, args)]


member_rooms = """select r.id from rooms r join memberships m on m.room_id = r.id
                  where m.user_id = ? and r.type = ? order by
                  (select count(*) from messages where room_id = r.id) desc, r.id limit ?"""
shared = ids("""select r.id from rooms r join memberships a on a.room_id = r.id and a.user_id = ?
                join memberships b on b.room_id = r.id and b.user_id = ? where r.type = 'Rooms::Open'
                order by (select count(*) from messages where room_id = r.id) asc, r.id limit 1""", viewer, peer)
open_rooms = ids(member_rooms, viewer, "Rooms::Open", 6)
closed_rooms = ids(member_rooms, viewer, "Rooms::Closed", 2)
directs = ids(member_rooms, viewer, "Rooms::Direct", 5)
boards = ids(member_rooms, viewer, "Rooms::Board", 2)
users = ids("select id from users where status = 0 and id != ? order by id limit 5", viewer)
events = [(r, e) for r, e in db.execute(
    """select e.room_id, e.id from events e join memberships m on m.room_id = e.room_id and m.user_id = ?
       order by e.id limit 3""", (viewer,))]
threads = [(r, t) for r, t in db.execute(
    """select t.room_id, t.id from channel_threads t join memberships m on m.room_id = t.room_id and m.user_id = ?
       order by t.id limit 3""", (viewer,))]

paths = ["/", "/users/me/profile", "/users/me/sessions", "/users/me/sidebar", "/searches",
         "/searches?q=the", "/activity", "/saved", "/scheduled_messages", "/work", "/switcher",
         "/account/edit", "/account/users", "/users", "/agents", "/room_categories"]
paths += [f"/rooms/{r}" for r in open_rooms + closed_rooms + directs + boards]
paths += [f"/rooms/opens/{r}/edit" for r in open_rooms[:2]]
paths += [f"/rooms/{r}/members" for r in open_rooms[:1]]
paths += [f"/rooms/{r}/files" for r in open_rooms[:1]]
paths += [f"/rooms/{r}/pins" for r in open_rooms[:1]]
paths += [f"/rooms/boards/{b}" for b in boards]
paths += [f"/rooms/boards/{b}/automations" for b in boards]
paths += [f"/users/{u}" for u in users] + [f"/users/{u}/card" for u in users[:2]]
paths += [f"/rooms/{r}/events" for r in sorted({r for r, _ in events})]
paths += [f"/rooms/{r}/events/{e}" for r, e in events]
paths += [f"/rooms/{r}/threads" for r in sorted({r for r, _ in threads})[:2]]
paths += [f"/rooms/{r}/threads/{t}" for r, t in threads]

json.dump({"shared_room": shared[0] if shared else open_rooms[0], "paths": paths}, sys.stdout, indent=1)
print()
