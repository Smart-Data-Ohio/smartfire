CREATE TABLE oauth_assertion_jtis (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  oauth_client_id INTEGER NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
  jti_digest TEXT NOT NULL,
  expires_at datetime(6) NOT NULL,
  UNIQUE(oauth_client_id, jti_digest)
);
CREATE INDEX index_oauth_assertion_jtis_on_expires_at ON oauth_assertion_jtis(expires_at);
