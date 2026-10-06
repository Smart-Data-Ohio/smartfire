"""Saved-row assertions for the original action/toolbar behaviour declarations."""

MESSAGE = 607264868
AUTHOR = 773523953
DAVID = 127326141


def assert_action_rows(conn, seed, file, case, metadata):
    columns = "id,markdown_source,room_id,thread_id,reply_to_message_id,reply_notify_author"
    expected = seed.execute(f"SELECT {columns} FROM messages ORDER BY id").fetchall()
    actual = conn.execute(f"SELECT {columns} FROM messages ORDER BY id").fetchall()
    content = {
        "quick-react creates a boost from the toolbar": "👍",
        "keyboard users reach the toolbar from a focused message": "👍",
        "the emoji picker searches and reacts": "🔥",
        "the picker remembers recent reactions": "😀",
        "the picker Custom tab reacts with a workspace icon": ":acme:",
        "the picker reacts with a brand icon shortcode": ":openai:",
        "picker arrows move through options, Enter selects, and Escape returns focus": "😀",
        "groups emoji reactions, updates the live count, and highlights the current user": "👍",
    }.get(case)
    boosts = "SELECT message_id,booster_id,content FROM boosts ORDER BY message_id,booster_id,content"
    expected_boosts = seed.execute(boosts).fetchall()
    if content:
        expected_boosts.append((MESSAGE, DAVID if case.startswith("groups emoji") else AUTHOR, content))
    assert conn.execute(boosts).fetchall() == sorted(expected_boosts), "exact saved reaction and actor"
    if case.startswith("edits through"):
        expected = [(row[0], "Saved through the main composer", *row[2:]) if row[0] == MESSAGE else row for row in expected]
        assert conn.execute("SELECT edited_at IS NOT NULL FROM messages WHERE id=?", (MESSAGE,)).fetchone() == (1,)
    elif case.startswith("replies with notify"):
        expected = [row for row in expected if row[0] != MESSAGE]
        reply = conn.execute(f"SELECT {columns} FROM messages WHERE markdown_source='A reply without a notification'").fetchall()
        assert len(reply) == 1 and reply[0][2:] == (654632876, None, None, 0)
        assert conn.execute("SELECT creator_id FROM messages WHERE id=?", (reply[0][0],)).fetchone() == (AUTHOR,)
        expected.append(reply[0])
    elif case.startswith("copies message") or case.startswith("forwarding twice"):
        rows = conn.execute("SELECT id,forward_note,thread_id,forwarded_markdown,creator_id FROM messages WHERE forwarded_from_message_id=?", (MESSAGE,)).fetchall()
        assert len(rows) == 1, "only one forward is saved"
        copy = rows[0]
        assert copy[3:] == (0, AUTHOR)
        if case.startswith("copies message"):
            assert copy[1:3] == ("Forwarded from the interaction test", metadata["forward_thread_id"])
        body = "SELECT body FROM action_text_rich_texts WHERE record_type='Message' AND record_id=? AND name='body'"
        assert conn.execute(body, (copy[0],)).fetchall() == seed.execute(body, (MESSAGE,)).fetchall(), "forward retains the actual rendered snapshot"
        expected.append(next(row for row in actual if row[0] == copy[0]))
    assert actual == sorted(expected), f"{case}: unexpected message writes"
    assert conn.execute("SELECT COUNT(*) FROM channel_threads").fetchone() == seed.execute("SELECT COUNT(*) FROM channel_threads").fetchone(), "opening thread controls does not create a thread"
