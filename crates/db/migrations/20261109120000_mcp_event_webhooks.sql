CREATE TABLE mcp_event_subscriptions (
  id TEXT PRIMARY KEY NOT NULL,
  agent_id INTEGER NOT NULL REFERENCES agents(id),
  agent_credential_id INTEGER NOT NULL REFERENCES agent_credentials(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  arguments TEXT NOT NULL,
  url TEXT NOT NULL,
  secret TEXT NOT NULL,
  previous_secret TEXT,
  previous_secret_expires_at datetime,
  refresh_before datetime NOT NULL,
  verified_at datetime,
  watermark_event_id INTEGER NOT NULL,
  last_delivery_at datetime,
  last_error TEXT,
  failed_since datetime,
  created_at datetime NOT NULL,
  updated_at datetime NOT NULL,
  UNIQUE(agent_credential_id, url, name, arguments)
);
CREATE INDEX mcp_subscriptions_agent ON mcp_event_subscriptions(agent_id, name, refresh_before);
CREATE TABLE mcp_event_deliveries (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  subscription_id TEXT NOT NULL REFERENCES mcp_event_subscriptions(id) ON DELETE CASCADE,
  agent_event_id INTEGER NOT NULL REFERENCES agent_events(id) ON DELETE CASCADE,
  webhook_id TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL CHECK(status IN ('pending','delivered','failed','suppressed')),
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at datetime NOT NULL,
  last_error TEXT,
  created_at datetime NOT NULL,
  updated_at datetime NOT NULL,
  UNIQUE(subscription_id, agent_event_id)
);
CREATE INDEX mcp_deliveries_sweep ON mcp_event_deliveries(status, next_attempt_at);
