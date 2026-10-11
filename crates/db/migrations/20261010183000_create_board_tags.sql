CREATE TABLE board_tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  emoji TEXT,
  position INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL,
  updated_at DATETIME NOT NULL
);
CREATE UNIQUE INDEX index_board_tags_on_room_id_and_name ON board_tags(room_id, lower(name));
ALTER TABLE rooms ADD COLUMN tags_required BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE rooms ADD COLUMN default_board_tag_id INTEGER REFERENCES board_tags(id) ON DELETE SET NULL;
