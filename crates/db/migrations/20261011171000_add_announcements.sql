ALTER TABLE rooms ADD COLUMN announcement INTEGER NOT NULL DEFAULT 0;

CREATE TABLE announcement_follows (
  source_room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
  target_room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
  creator_id INTEGER NOT NULL REFERENCES users(id),
  created_at datetime(6) NOT NULL,
  PRIMARY KEY (source_room_id, target_room_id),
  CHECK (source_room_id != target_room_id)
);
CREATE INDEX announcement_follows_on_target ON announcement_follows(target_room_id);

CREATE TABLE announcement_publications (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  source_message_id INTEGER UNIQUE REFERENCES messages(id) ON DELETE SET NULL,
  publisher_id INTEGER NOT NULL REFERENCES users(id),
  published_at datetime(6) NOT NULL
);

CREATE TABLE announcement_deliveries (
  publication_id INTEGER NOT NULL REFERENCES announcement_publications(id) ON DELETE CASCADE,
  target_room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
  message_id INTEGER UNIQUE REFERENCES messages(id) ON DELETE CASCADE,
  PRIMARY KEY (publication_id, target_room_id)
);
CREATE INDEX announcement_deliveries_on_target ON announcement_deliveries(target_room_id);
