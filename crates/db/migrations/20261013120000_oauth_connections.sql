CREATE TABLE oauth_clients (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  client_id TEXT NOT NULL UNIQUE, registration TEXT NOT NULL CHECK (registration IN ('cimd','dcr')),
  client_name TEXT NOT NULL, client_uri TEXT, redirect_uris TEXT NOT NULL,
  auth_methods TEXT NOT NULL DEFAULT '["none"]', jwks_uri TEXT, metadata TEXT,
  fetched_at datetime(6), refetch_after datetime(6), blocked_at datetime(6),
  created_at datetime(6) NOT NULL, updated_at datetime(6) NOT NULL);
CREATE TABLE oauth_connections (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  agent_credential_id INTEGER NOT NULL UNIQUE REFERENCES agent_credentials(id) ON DELETE CASCADE,
  oauth_client_id INTEGER NOT NULL REFERENCES oauth_clients(id) ON DELETE RESTRICT,
  resource TEXT NOT NULL, scope TEXT NOT NULL,
  auth_method TEXT CHECK (auth_method IN ('none','private_key_jwt')),
  expires_at datetime(6) NOT NULL, created_at datetime(6) NOT NULL);
CREATE INDEX index_oauth_connections_on_oauth_client_id ON oauth_connections(oauth_client_id);
CREATE TABLE oauth_authorizations (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  public_id TEXT NOT NULL UNIQUE,
  oauth_client_id INTEGER NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  redirect_uri TEXT NOT NULL, state TEXT, code_challenge TEXT NOT NULL, scope TEXT NOT NULL,
  resource TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'pending'
    CHECK (status IN ('pending','approved','denied','exchanged')), code_digest TEXT UNIQUE,
  oauth_connection_id INTEGER REFERENCES oauth_connections(id) ON DELETE CASCADE,
  expires_at datetime(6) NOT NULL, created_at datetime(6) NOT NULL, updated_at datetime(6) NOT NULL);
CREATE INDEX index_oauth_authorizations_on_user_id ON oauth_authorizations(user_id);
CREATE INDEX index_oauth_authorizations_on_oauth_connection_id ON oauth_authorizations(oauth_connection_id);
CREATE TABLE oauth_tokens (id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  oauth_connection_id INTEGER NOT NULL REFERENCES oauth_connections(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('access','refresh')), token_digest TEXT NOT NULL UNIQUE,
  expires_at datetime(6) NOT NULL, used_at datetime(6), replaced_by_id INTEGER,
  revoked_at datetime(6), created_at datetime(6) NOT NULL);
CREATE INDEX index_oauth_tokens_on_oauth_connection_id ON oauth_tokens(oauth_connection_id);
