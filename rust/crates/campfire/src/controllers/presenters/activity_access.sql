SELECT activity_items.id FROM activity_items JOIN users ON users.id=activity_items.user_id
          LEFT JOIN messages AS activity_messages
            ON activity_messages.id = activity_items.source_id
            AND activity_items.source_type = 'Message'
          LEFT JOIN memberships AS activity_message_memberships
            ON activity_message_memberships.room_id = activity_messages.room_id
            AND activity_message_memberships.user_id = activity_items.user_id
          LEFT JOIN saved_items AS activity_saved_items
            ON activity_saved_items.id = activity_items.source_id
            AND activity_items.source_type = 'SavedItem'
          LEFT JOIN messages AS activity_saved_messages
            ON activity_saved_messages.id = activity_saved_items.message_id
          LEFT JOIN memberships AS activity_saved_memberships
            ON activity_saved_memberships.room_id = activity_saved_messages.room_id
            AND activity_saved_memberships.user_id = activity_items.user_id
          LEFT JOIN work_thread_events AS activity_work_events
            ON activity_work_events.id = activity_items.source_id
            AND activity_items.source_type = 'WorkThreadEvent'
          LEFT JOIN channel_threads AS activity_work_threads
            ON activity_work_threads.id = activity_work_events.channel_thread_id
          LEFT JOIN memberships AS activity_work_memberships
            ON activity_work_memberships.room_id = activity_work_threads.room_id
            AND activity_work_memberships.user_id = activity_items.user_id
          LEFT JOIN board_sla_nudges AS activity_sla_nudges
            ON activity_sla_nudges.id = activity_items.source_id
            AND activity_items.source_type = 'BoardSlaNudge'
          LEFT JOIN memberships AS activity_sla_memberships
            ON activity_sla_memberships.room_id = activity_sla_nudges.room_id
            AND activity_sla_memberships.user_id = activity_items.user_id
          LEFT JOIN huddle_grants AS activity_huddle_grants
            ON activity_huddle_grants.id = activity_items.source_id
            AND activity_items.source_type = 'HuddleGrant'
          LEFT JOIN memberships AS activity_huddle_memberships
            ON activity_huddle_memberships.room_id = activity_huddle_grants.room_id
            AND activity_huddle_memberships.user_id = activity_items.user_id
          LEFT JOIN events AS activity_events
            ON activity_events.id = activity_items.source_id
            AND activity_items.source_type = 'Event'
          LEFT JOIN memberships AS activity_event_memberships
            ON activity_event_memberships.room_id = activity_events.room_id
            AND activity_event_memberships.user_id = activity_items.user_id
          LEFT JOIN agent_approvals AS activity_approvals
            ON activity_approvals.id = activity_items.source_id
            AND activity_items.source_type = 'AgentApproval'
          LEFT JOIN agents AS activity_approval_agents
            ON activity_approval_agents.id = activity_approvals.agent_id
          LEFT JOIN users AS activity_approval_agent_users
            ON activity_approval_agent_users.id = activity_approval_agents.user_id
          LEFT JOIN agent_budget_notices AS activity_budget_notices
            ON activity_budget_notices.id = activity_items.source_id
            AND activity_items.source_type = 'AgentBudgetNotice'
          LEFT JOIN agents AS activity_budget_agents
            ON activity_budget_agents.id = activity_budget_notices.agent_id
          LEFT JOIN scheduled_messages AS activity_scheduled_messages
            ON activity_scheduled_messages.id = activity_items.source_id
            AND activity_items.source_type = 'ScheduledMessage'
          LEFT JOIN two_factor_credentials AS activity_two_factor_credentials
            ON activity_two_factor_credentials.id = activity_items.source_id
            AND activity_items.source_type = 'TwoFactorCredential'
          LEFT JOIN sessions AS activity_sign_in_sessions
            ON activity_sign_in_sessions.id = activity_items.source_id
            AND activity_items.source_type = 'Session'

WHERE users.status=0 AND users.role!=2 AND activity_items.user_id=? AND (          (activity_items.source_type = 'Message' AND activity_message_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'SavedItem' AND activity_saved_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'WorkThreadEvent' AND activity_work_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'BoardSlaNudge' AND activity_sla_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'HuddleGrant' AND activity_huddle_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'Event' AND activity_event_memberships.id IS NOT NULL)
          OR (activity_items.source_type = 'AgentApproval'
            AND activity_approvals.id IS NOT NULL
            AND activity_approval_agent_users.status = 0
            AND (activity_approval_agents.owner_id = activity_items.user_id
              OR users.role = 1))
          OR (activity_items.source_type = 'AgentBudgetNotice'
            AND activity_budget_notices.id IS NOT NULL
            AND (activity_budget_agents.owner_id = activity_items.user_id
              OR users.role = 1))
          OR (activity_items.source_type = 'ScheduledMessage'
            AND activity_scheduled_messages.id IS NOT NULL
            AND activity_scheduled_messages.user_id = activity_items.user_id)
          OR (activity_items.source_type = 'TwoFactorCredential'
            AND activity_two_factor_credentials.id IS NOT NULL
            AND activity_two_factor_credentials.user_id = activity_items.user_id)
          OR (activity_items.source_type = 'Session'
            AND activity_sign_in_sessions.id IS NOT NULL
            AND activity_sign_in_sessions.user_id = activity_items.user_id)
)
