CREATE TABLE user_blocks (
  blocker_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  blocked_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at datetime(6) NOT NULL,
  CHECK (blocker_id != blocked_id)
);
CREATE UNIQUE INDEX index_user_blocks_on_blocker_and_blocked ON user_blocks(blocker_id, blocked_id);
CREATE INDEX index_user_blocks_on_blocked_id ON user_blocks(blocked_id);
