ALTER TABLE channel_threads ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public';

CREATE TABLE thread_access_memberships (
  thread_id INTEGER NOT NULL REFERENCES channel_threads(id) ON DELETE CASCADE,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  added_by_id INTEGER REFERENCES users(id) ON DELETE SET NULL,
  created_at DATETIME(6) NOT NULL,
  PRIMARY KEY (thread_id, user_id)
);
CREATE INDEX index_thread_access_memberships_on_user_id ON thread_access_memberships(user_id);

CREATE TRIGGER channel_thread_visibility_immutable
BEFORE UPDATE OF visibility ON channel_threads
WHEN NEW.visibility != OLD.visibility
BEGIN
  SELECT RAISE(ABORT, 'Thread visibility cannot change after creation');
END;

CREATE TRIGGER channel_thread_visibility_valid
BEFORE INSERT ON channel_threads
WHEN NEW.visibility NOT IN ('public', 'private')
BEGIN
  SELECT RAISE(ABORT, 'Invalid thread visibility');
END;
