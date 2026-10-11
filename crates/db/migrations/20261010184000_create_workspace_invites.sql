CREATE TABLE workspace_invites (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  creator_id INTEGER NOT NULL REFERENCES users(id),
  token_digest TEXT NOT NULL UNIQUE,
  expires_at datetime(6),
  max_uses INTEGER CHECK (max_uses IS NULL OR max_uses IN (1, 5, 10, 25, 50, 100)),
  uses INTEGER NOT NULL DEFAULT 0 CHECK (uses >= 0),
  revoked_at datetime(6),
  created_at datetime(6) NOT NULL,
  updated_at datetime(6) NOT NULL
);
