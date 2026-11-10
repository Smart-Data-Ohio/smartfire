ALTER TABLE mcp_event_deliveries RENAME TO mcp_event_deliveries_old;
CREATE TABLE mcp_event_deliveries (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  subscription_id TEXT NOT NULL REFERENCES mcp_event_subscriptions(id) ON DELETE CASCADE,
  agent_event_id INTEGER NOT NULL REFERENCES agent_events(id) ON DELETE CASCADE,
  webhook_id TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL CHECK(status IN ('pending','delivered','failed','suppressed','expired')),
  attempts INTEGER NOT NULL DEFAULT 0,
  next_attempt_at datetime NOT NULL,
  last_error TEXT,
  created_at datetime NOT NULL,
  updated_at datetime NOT NULL,
  body TEXT,
  UNIQUE(subscription_id, agent_event_id)
);
INSERT INTO mcp_event_deliveries SELECT * FROM mcp_event_deliveries_old;
UPDATE sqlite_sequence SET seq=MAX(seq,COALESCE((SELECT seq FROM sqlite_sequence WHERE name='mcp_event_deliveries_old'),0)) WHERE name='mcp_event_deliveries';
DELETE FROM sqlite_sequence WHERE name='mcp_event_deliveries' AND NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name='mcp_event_deliveries_old');
DROP TABLE mcp_event_deliveries_old;
CREATE INDEX mcp_deliveries_sweep ON mcp_event_deliveries(status, next_attempt_at);
