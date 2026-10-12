CREATE TABLE soundboard_sounds (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 32),
  name_key TEXT NOT NULL UNIQUE,
  emoji TEXT,
  volume REAL NOT NULL DEFAULT 1.0 CHECK (volume BETWEEN 0.0 AND 1.0),
  uploader_id INTEGER NOT NULL REFERENCES users(id),
  created_at datetime(6) NOT NULL,
  updated_at datetime(6) NOT NULL
);
