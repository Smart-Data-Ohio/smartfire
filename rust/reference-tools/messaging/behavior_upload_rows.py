"""Persisted identities and real disk bytes for the two pinned upload regressions."""
import base64
import hashlib
from behavior_upload_bytes import uploaded_bytes

# ActiveStorage::Variation#digest of preview(format: :webp): SHA1 of Marshal.dump({format: :webp}).
WEBP_VARIATION_DIGEST = base64.b64encode(hashlib.sha1(b"\x04\x08{\x06:\x0bformat:\x09webp").digest()).decode()


def attached_blob(conn, record_type, name, record_id):
    rows = conn.execute("SELECT b.id,b.key FROM active_storage_blobs b JOIN active_storage_attachments a ON a.blob_id=b.id "
                        "WHERE a.record_type=? AND a.name=? AND a.record_id=?", (record_type, name, record_id)).fetchall()
    assert len(rows) == 1, f'one {record_type} {name} attachment: {rows}'
    return rows[0]


def assert_video_preview(conn, blob_id, storage):
    """sending_messages_test.rb:49-52: the processed preview image and its webp variant exist on disk."""
    preview_id, preview_key = attached_blob(conn, 'ActiveStorage::Blob', 'preview_image', blob_id)
    assert uploaded_bytes(storage, preview_key), 'preview image stored'
    variants = conn.execute('SELECT id FROM active_storage_variant_records WHERE blob_id=? AND variation_digest=?',
                            (preview_id, WEBP_VARIATION_DIGEST)).fetchall()
    assert len(variants) == 1, f'one webp preview variant record: {variants}'
    _, variant_key = attached_blob(conn, 'ActiveStorage::VariantRecord', 'image', variants[0][0])
    assert uploaded_bytes(storage, variant_key), 'preview variant stored'


def assert_upload_rows(conn, seed, case, storage, sources):
    video = case.startswith('uploading a fresh video')
    old_messages = seed.execute('SELECT * FROM messages ORDER BY id').fetchall()
    old_ids = {row[0] for row in old_messages}
    assert [row for row in conn.execute('SELECT * FROM messages ORDER BY id') if row[0] in old_ids] == old_messages
    new_ids = [row[0] for row in conn.execute('SELECT id FROM messages') if row[0] not in old_ids]
    assert len(new_ids) == 1, 'exactly one persisted browser upload'
    row = conn.execute('SELECT creator_id,room_id,thread_id,reply_to_message_id,reply_notify_author FROM messages WHERE id=?', new_ids).fetchone()
    assert row[:2] == (773523953, 654632876)
    if video:
        assert row[2] is not None and row[3] is None
        assert conn.execute('SELECT name,creator_id,room_id FROM channel_threads WHERE id=?', (row[2],)).fetchone() == ('Video upload', 773523953, 654632876)
        assert conn.execute('SELECT COUNT(*) FROM messages WHERE thread_id=?', (row[2],)).fetchone()[0] == 1
        assert conn.execute('SELECT COUNT(*) FROM thread_memberships WHERE thread_id=? AND user_id=773523953', (row[2],)).fetchone()[0] == 1
    else:
        assert row[2:] == (None, 607264868, 1), 'preserved reply identity and notify default'
    blobs = conn.execute("SELECT b.filename,b.byte_size,b.key,b.id FROM active_storage_blobs b JOIN active_storage_attachments a ON a.blob_id=b.id WHERE a.record_type='Message' AND a.name='attachment' AND a.record_id=?", new_ids).fetchall()
    filename = 'alpha-centuri.mov' if video else 'moon.jpg'
    expected = (sources / 'test/fixtures/files' / filename).read_bytes()
    assert len(blobs) == 1 and blobs[0][:2] == (filename, len(expected))
    assert uploaded_bytes(storage, blobs[0][2]) == expected, 'original pinned upload bytes persisted'
    if video:
        assert_video_preview(conn, blobs[0][3], storage)
