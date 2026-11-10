ALTER TABLE agents ADD COLUMN wake_policy TEXT NOT NULL DEFAULT 'owner' CHECK (wake_policy IN ('owner','anyone'));
UPDATE agents SET wake_policy='anyone';
ALTER TABLE agents ADD COLUMN paused_at datetime;
ALTER TABLE agents ADD COLUMN paused_by_id INTEGER REFERENCES users(id) ON DELETE SET NULL;
