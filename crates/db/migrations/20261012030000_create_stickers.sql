CREATE TABLE stickers (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL CHECK (length(name) BETWEEN 2 AND 30),
  name_key TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '',
  uploader_id INTEGER NOT NULL REFERENCES users(id),
  created_at datetime(6) NOT NULL,
  updated_at datetime(6) NOT NULL
);

-- The image uses the existing Active Storage attachment slot Sticker/image.
ALTER TABLE messages ADD COLUMN sticker_id INTEGER REFERENCES stickers(id) ON DELETE SET NULL;
-- Retain the name when the catalog entry is deleted, including on sticker-only messages.
ALTER TABLE messages ADD COLUMN sticker_name TEXT;
CREATE INDEX index_messages_on_sticker_id ON messages(sticker_id);
