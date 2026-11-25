CREATE TABLE user_email_addresses (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  email_address VARCHAR NOT NULL,
  created_at DATETIME(6) NOT NULL,
  updated_at DATETIME(6) NOT NULL
);
CREATE UNIQUE INDEX index_user_email_addresses_on_email_address
  ON user_email_addresses (email_address COLLATE NOCASE);
CREATE INDEX index_user_email_addresses_on_user_id ON user_email_addresses (user_id);
