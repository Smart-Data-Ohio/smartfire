CREATE TRIGGER personal_agent_grant_insert
BEFORE INSERT ON agent_grants
WHEN EXISTS (SELECT 1 FROM agents WHERE id=NEW.agent_id AND kind='personal')
 AND (NEW.capability NOT IN ('read_messages','post_messages','react','manage_threads')
      OR NEW.room_id IS NULL OR EXISTS (SELECT 1 FROM rooms WHERE id=NEW.room_id AND type='Rooms::Direct'))
BEGIN SELECT RAISE(ABORT, 'Personal agent grants must name a non-direct room and a chat capability'); END;

CREATE TRIGGER personal_agent_grant_update
BEFORE UPDATE OF agent_id, capability, room_id ON agent_grants
WHEN EXISTS (SELECT 1 FROM agents WHERE id=NEW.agent_id AND kind='personal')
 AND (NEW.capability NOT IN ('read_messages','post_messages','react','manage_threads')
      OR NEW.room_id IS NULL OR EXISTS (SELECT 1 FROM rooms WHERE id=NEW.room_id AND type='Rooms::Direct'))
BEGIN SELECT RAISE(ABORT, 'Personal agent grants must name a non-direct room and a chat capability'); END;

CREATE TRIGGER personal_agent_kind_update
BEFORE UPDATE OF kind ON agents
WHEN NEW.kind='personal' AND EXISTS (
 SELECT 1 FROM agent_grants g LEFT JOIN rooms r ON r.id=g.room_id WHERE g.agent_id=NEW.id
 AND (g.capability NOT IN ('read_messages','post_messages','react','manage_threads') OR g.room_id IS NULL OR r.type='Rooms::Direct'))
BEGIN SELECT RAISE(ABORT, 'Personal agent has incompatible grants'); END;
