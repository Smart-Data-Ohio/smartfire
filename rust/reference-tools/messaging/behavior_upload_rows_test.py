import sqlite3
import unittest
from pathlib import Path
from unittest.mock import patch
from behavior_upload_rows import assert_upload_rows


class UploadRowsTest(unittest.TestCase):
    def fixture(self):
        schema = """
          CREATE TABLE messages(id INTEGER,creator_id INTEGER,room_id INTEGER,thread_id INTEGER,reply_to_message_id INTEGER,reply_notify_author INTEGER);
          CREATE TABLE active_storage_blobs(id INTEGER,filename TEXT,byte_size INTEGER,key TEXT);
          CREATE TABLE active_storage_attachments(record_type TEXT,name TEXT,record_id INTEGER,blob_id INTEGER);
        """
        seed, conn = sqlite3.connect(':memory:'), sqlite3.connect(':memory:')
        self.addCleanup(seed.close)
        self.addCleanup(conn.close)
        for db in (seed, conn):
            db.executescript(schema)
            db.execute('INSERT INTO messages VALUES(607264868,773523953,654632876,NULL,NULL,1)')
        conn.execute('INSERT INTO messages VALUES(908005740,773523953,654632876,NULL,607264868,1)')
        conn.execute("INSERT INTO active_storage_blobs VALUES(9,'moon.jpg',5,'real-key')")
        conn.execute("INSERT INTO active_storage_attachments VALUES('Message','attachment',908005740,9)")
        return seed, conn

    def verify(self, conn, seed, *, contents=b'image'):
        with patch('behavior_upload_rows.subprocess.check_output', return_value=b'image'), \
                patch('behavior_upload_rows.uploaded_bytes', return_value=contents):
            assert_upload_rows(conn, seed, 'late upload progress preserves a delivered attachment and reply preview', Path('disk'), Path('root'), 'pin')

    def test_real_saved_upload_passes(self):
        seed, conn = self.fixture()
        self.verify(conn, seed)

    def test_wrong_reply_identity_and_modified_history_fail(self):
        for mutation in ('UPDATE messages SET reply_to_message_id=NULL WHERE id=908005740',
                         'UPDATE messages SET creator_id=7 WHERE id=607264868'):
            seed, conn = self.fixture()
            conn.execute(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                self.verify(conn, seed)

    def test_missing_or_different_disk_bytes_fail(self):
        seed, conn = self.fixture()
        with self.assertRaisesRegex(AssertionError, 'original pinned upload bytes'):
            self.verify(conn, seed, contents=b'wrong')
        with patch('behavior_upload_rows.subprocess.check_output', return_value=b'image'), \
                patch('behavior_upload_rows.uploaded_bytes', side_effect=FileNotFoundError):
            with self.assertRaises(FileNotFoundError):
                assert_upload_rows(conn, seed, 'late upload progress preserves a delivered attachment and reply preview', Path('disk'), Path('root'), 'pin')
