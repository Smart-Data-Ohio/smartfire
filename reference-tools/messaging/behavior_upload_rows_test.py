import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from behavior_upload_rows import WEBP_VARIATION_DIGEST, assert_upload_rows

PROGRESS = 'late upload progress preserves a delivered attachment and reply preview'
VIDEO = 'uploading a fresh video in the thread composer'


class UploadRowsTest(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.sources = Path(directory.name)
        (self.sources / 'test/fixtures/files').mkdir(parents=True)
        for name in ['moon.jpg', 'alpha-centuri.mov']:
            (self.sources / 'test/fixtures/files' / name).write_bytes(b'image')

    def fixture(self, video=False):
        schema = """
          CREATE TABLE messages(id INTEGER,creator_id INTEGER,room_id INTEGER,thread_id INTEGER,reply_to_message_id INTEGER,reply_notify_author INTEGER);
          CREATE TABLE channel_threads(id INTEGER,name TEXT,creator_id INTEGER,room_id INTEGER);
          CREATE TABLE thread_memberships(thread_id INTEGER,user_id INTEGER);
          CREATE TABLE active_storage_blobs(id INTEGER,filename TEXT,byte_size INTEGER,key TEXT);
          CREATE TABLE active_storage_attachments(record_type TEXT,name TEXT,record_id INTEGER,blob_id INTEGER);
          CREATE TABLE active_storage_variant_records(id INTEGER,blob_id INTEGER,variation_digest TEXT);
        """
        seed, conn = sqlite3.connect(':memory:'), sqlite3.connect(':memory:')
        self.addCleanup(seed.close)
        self.addCleanup(conn.close)
        for db in (seed, conn):
            db.executescript(schema)
            db.execute('INSERT INTO messages VALUES(607264868,773523953,654632876,NULL,NULL,1)')
        if video:
            conn.executescript(f"""
              INSERT INTO channel_threads VALUES(5,'Video upload',773523953,654632876);
              INSERT INTO thread_memberships VALUES(5,773523953);
              INSERT INTO messages VALUES(908005740,773523953,654632876,5,NULL,1);
              INSERT INTO active_storage_blobs VALUES(9,'alpha-centuri.mov',5,'real-key');
              INSERT INTO active_storage_attachments VALUES('Message','attachment',908005740,9);
              INSERT INTO active_storage_blobs VALUES(10,'preview.png',3,'preview-key');
              INSERT INTO active_storage_attachments VALUES('ActiveStorage::Blob','preview_image',9,10);
              INSERT INTO active_storage_variant_records VALUES(1,10,'{WEBP_VARIATION_DIGEST}');
              INSERT INTO active_storage_blobs VALUES(11,'preview.webp',3,'variant-key');
              INSERT INTO active_storage_attachments VALUES('ActiveStorage::VariantRecord','image',1,11);
            """)
        else:
            conn.execute('INSERT INTO messages VALUES(908005740,773523953,654632876,NULL,607264868,1)')
            conn.execute("INSERT INTO active_storage_blobs VALUES(9,'moon.jpg',5,'real-key')")
            conn.execute("INSERT INTO active_storage_attachments VALUES('Message','attachment',908005740,9)")
        return seed, conn

    def verify(self, conn, seed, case=PROGRESS, *, contents=b'image'):
        with patch('behavior_upload_rows.uploaded_bytes', return_value=contents):
            assert_upload_rows(conn, seed, case, Path('disk'), self.sources)

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
        with patch('behavior_upload_rows.uploaded_bytes', side_effect=FileNotFoundError):
            with self.assertRaises(FileNotFoundError):
                assert_upload_rows(conn, seed, PROGRESS, Path('disk'), self.sources)

    def test_processed_video_preview_and_variant_pass(self):
        seed, conn = self.fixture(video=True)
        self.verify(conn, seed, VIDEO)

    def test_missing_preview_or_variant_fails(self):
        for mutation in ("DELETE FROM active_storage_attachments WHERE name='preview_image'",
                         "UPDATE active_storage_variant_records SET variation_digest='other'",
                         "DELETE FROM active_storage_attachments WHERE record_type='ActiveStorage::VariantRecord'"):
            seed, conn = self.fixture(video=True)
            conn.execute(mutation)
            with self.subTest(mutation=mutation), self.assertRaises(AssertionError):
                self.verify(conn, seed, VIDEO)


if __name__ == '__main__':
    unittest.main()
