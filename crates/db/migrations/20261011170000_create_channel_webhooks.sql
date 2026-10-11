CREATE TABLE channel_webhooks (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  room_id INTEGER NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
  creator_id INTEGER NOT NULL REFERENCES users(id),
  author_id INTEGER NOT NULL UNIQUE REFERENCES users(id),
  name TEXT NOT NULL,
  token_digest TEXT NOT NULL UNIQUE,
  last_used_at datetime(6),
  created_at datetime(6) NOT NULL,
  updated_at datetime(6) NOT NULL
);
CREATE INDEX index_channel_webhooks_on_room_id ON channel_webhooks(room_id);
CREATE TABLE channel_webhook_messages (
  message_id INTEGER PRIMARY KEY NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  webhook_id INTEGER REFERENCES channel_webhooks(id) ON DELETE SET NULL,
  name TEXT NOT NULL,
  username TEXT,
  embeds_json TEXT NOT NULL CHECK (json_valid(embeds_json))
);
CREATE TRIGGER channel_webhooks_revoke_author AFTER DELETE ON channel_webhooks BEGIN
  DELETE FROM memberships WHERE user_id=OLD.author_id;
  DELETE FROM active_storage_attachments WHERE record_type='ChannelWebhook' AND record_id=OLD.id;
END;
