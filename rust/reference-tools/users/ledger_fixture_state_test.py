import sqlite3
import unittest
from ledger_fixture_state import snapshot, restore

class FixtureStateTests(unittest.TestCase):
    def test_restores_original_rows_sequences_and_fts_after_a_browser_write(self):
        with sqlite3.connect(':memory:') as db:
            db.executescript('CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,body TEXT);'
                             'CREATE VIRTUAL TABLE messages_index USING fts5(body);'
                             "INSERT INTO messages VALUES(14,'fixture');"
                             "INSERT INTO messages_index(rowid,body) VALUES(14,'fixture');")
            original = snapshot(db)
            db.executescript("UPDATE messages SET body='changed';"
                             "INSERT INTO messages(body) VALUES('case-specific');"
                             "DELETE FROM messages_index;"
                             "INSERT INTO messages_index(rowid,body) VALUES(15,'case-specific');")
            restore(db, original)
            self.assertEqual(db.execute('SELECT * FROM messages').fetchall(), [(14, 'fixture')])
            self.assertEqual(db.execute('SELECT rowid,body FROM messages_index').fetchall(), [(14, 'fixture')])
            self.assertEqual(db.execute("SELECT rowid FROM messages_index WHERE messages_index MATCH 'fixture'").fetchall(), [(14,)])
            self.assertEqual(db.execute("SELECT seq FROM sqlite_sequence WHERE name='messages'").fetchone(), (14,))

if __name__ == '__main__':
    unittest.main()
