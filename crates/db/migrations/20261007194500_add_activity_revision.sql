-- Counts use activity_item/access.sql. Keep these triggers in sync with its access dependencies.
-- Source deletions look up recipients before foreign-key cascades remove their links.
ALTER TABLE users ADD COLUMN activity_revision INTEGER NOT NULL DEFAULT 0;

CREATE TRIGGER activity_revision_activity_items_insert
AFTER INSERT ON activity_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.user_id;
END;

CREATE TRIGGER activity_revision_activity_items_delete
BEFORE DELETE ON activity_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.user_id;
END;

CREATE TRIGGER activity_revision_activity_items_update
AFTER UPDATE OF user_id, source_type, source_id, event_type, read_at, handled_at, updated_at ON activity_items
WHEN OLD.user_id IS NOT NEW.user_id OR OLD.source_type IS NOT NEW.source_type OR OLD.source_id IS NOT NEW.source_id OR OLD.event_type IS NOT NEW.event_type OR OLD.read_at IS NOT NEW.read_at OR OLD.handled_at IS NOT NEW.handled_at OR OLD.updated_at IS NOT NEW.updated_at
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.user_id) OR (id = NEW.user_id);
END;

CREATE TRIGGER activity_revision_memberships_insert
AFTER INSERT ON memberships
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.user_id;
END;

CREATE TRIGGER activity_revision_memberships_delete
BEFORE DELETE ON memberships
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.user_id;
END;

CREATE TRIGGER activity_revision_memberships_update
AFTER UPDATE OF id, user_id, room_id ON memberships
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.user_id) OR (id = NEW.user_id);
END;

CREATE TRIGGER activity_revision_messages_insert
AFTER INSERT ON messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = NEW.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_messages_delete
BEFORE DELETE ON messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = OLD.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = OLD.id)));
END;

CREATE TRIGGER activity_revision_messages_update
AFTER UPDATE OF id, room_id ON messages
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = OLD.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = OLD.id)))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = NEW.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = NEW.id))));
END;

CREATE TRIGGER activity_revision_saved_items_insert
AFTER INSERT ON saved_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_saved_items_delete
BEFORE DELETE ON saved_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_saved_items_update
AFTER UPDATE OF id, message_id ON saved_items
WHEN OLD.id IS NOT NEW.id OR OLD.message_id IS NOT NEW.message_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_work_thread_events_insert
AFTER INSERT ON work_thread_events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_work_thread_events_delete
BEFORE DELETE ON work_thread_events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_work_thread_events_update
AFTER UPDATE OF id, channel_thread_id ON work_thread_events
WHEN OLD.id IS NOT NEW.id OR OLD.channel_thread_id IS NOT NEW.channel_thread_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_channel_threads_insert
AFTER INSERT ON channel_threads
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = NEW.id));
END;

CREATE TRIGGER activity_revision_channel_threads_delete
BEFORE DELETE ON channel_threads
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = OLD.id));
END;

CREATE TRIGGER activity_revision_channel_threads_update
AFTER UPDATE OF id, room_id ON channel_threads
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_board_sla_nudges_insert
AFTER INSERT ON board_sla_nudges
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_board_sla_nudges_delete
BEFORE DELETE ON board_sla_nudges
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_board_sla_nudges_update
AFTER UPDATE OF id, room_id ON board_sla_nudges
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_huddle_grants_insert
AFTER INSERT ON huddle_grants
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_huddle_grants_delete
BEFORE DELETE ON huddle_grants
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_huddle_grants_update
AFTER UPDATE OF id, room_id ON huddle_grants
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_events_insert
AFTER INSERT ON events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_events_delete
BEFORE DELETE ON events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_events_update
AFTER UPDATE OF id, room_id ON events
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_agent_approvals_insert
AFTER INSERT ON agent_approvals
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_agent_approvals_delete
BEFORE DELETE ON agent_approvals
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_agent_approvals_update
AFTER UPDATE OF id, agent_id ON agent_approvals
WHEN OLD.id IS NOT NEW.id OR OLD.agent_id IS NOT NEW.agent_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_agent_budget_notices_insert
AFTER INSERT ON agent_budget_notices
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_agent_budget_notices_delete
BEFORE DELETE ON agent_budget_notices
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_agent_budget_notices_update
AFTER UPDATE OF id, agent_id ON agent_budget_notices
WHEN OLD.id IS NOT NEW.id OR OLD.agent_id IS NOT NEW.agent_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_scheduled_messages_insert
AFTER INSERT ON scheduled_messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_scheduled_messages_delete
BEFORE DELETE ON scheduled_messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_scheduled_messages_update
AFTER UPDATE OF id, user_id ON scheduled_messages
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_two_factor_credentials_insert
AFTER INSERT ON two_factor_credentials
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_two_factor_credentials_delete
BEFORE DELETE ON two_factor_credentials
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_two_factor_credentials_update
AFTER UPDATE OF id, user_id ON two_factor_credentials
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_sessions_insert
AFTER INSERT ON sessions
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = NEW.id));
END;

CREATE TRIGGER activity_revision_sessions_delete
BEFORE DELETE ON sessions
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = OLD.id));
END;

CREATE TRIGGER activity_revision_sessions_update
AFTER UPDATE OF id, user_id ON sessions
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_agents_insert
AFTER INSERT ON agents
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = NEW.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = NEW.id)));
END;

CREATE TRIGGER activity_revision_agents_delete
BEFORE DELETE ON agents
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = OLD.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = OLD.id)));
END;

CREATE TRIGGER activity_revision_agents_update
AFTER UPDATE OF id, owner_id, user_id ON agents
WHEN OLD.id IS NOT NEW.id OR OLD.owner_id IS NOT NEW.owner_id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = OLD.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = OLD.id)))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = NEW.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = NEW.id))));
END;

CREATE TRIGGER activity_revision_users_insert
AFTER INSERT ON users
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = NEW.id));
END;

CREATE TRIGGER activity_revision_users_delete
BEFORE DELETE ON users
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = OLD.id));
END;

CREATE TRIGGER activity_revision_users_update
AFTER UPDATE OF id, status, role ON users
WHEN OLD.id IS NOT NEW.id OR OLD.status IS NOT NEW.status OR OLD.role IS NOT NEW.role
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = OLD.id))) OR (id = NEW.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = NEW.id)));
END;
