module Agents
  # Shared event polling and acking for GET /agents/events and the MCP
  # poll_events / ack_events tools. The endpoint-level read_messages gate
  # stays in the callers; this owns the row query, the per-row
  # readability rules, and the payload assembly, so both surfaces read
  # the same rows the same way.
  class EventPolling
    POLL_DEFAULT_LIMIT = 50
    POLL_MAX_LIMIT = 100

    def self.poll(agent:, since:, limit:, presenter:)
      new(agent, presenter).poll(since: since, limit: limit)
    end

    def self.ack(agent:, id:)
      event = agent.agent_events.deliverable
        .where.not(outcome: "suppressed")
        .find_by(id: id)
      return ServiceResult.fail("Event not found", status: :not_found) unless event

      # Approval decisions, GitHub completions, work assignments, and
      # slash-command invocations carry no message and are always ackable
      # by their own agent, under the workspace-wide form of the
      # capability check, matching polling.
      if AgentEvent::ALWAYS_READABLE_TYPES.include?(event.event_type) ||
          AgentEvent::WORK_DELIVERABLE_TYPES.include?(event.event_type) ||
          AgentEvent::SLASH_DELIVERABLE_TYPES.include?(event.event_type)
        unless agent.has_capability_anywhere?(:read_messages)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end
      else
        # Ack requires the row's message to be currently readable by the
        # agent under the same rule as polling: the message exists and
        # the agent's user is still a member of its room. A surviving
        # workspace grant alone is not enough.
        message = event.message
        unless message && Membership.exists?(user_id: agent.user_id, room_id: message.room_id)
          return ServiceResult.fail("Event not found", status: :not_found)
        end

        unless agent.can?(:read_messages, event.room)
          return ServiceResult.fail("Forbidden: agent lacks read_messages capability", status: :forbidden)
        end
      end

      event.acknowledged! unless event.acknowledged?

      ServiceResult.ok({ id: event.id, outcome: "acknowledged" })
    end

    def initialize(agent, presenter)
      @agent = agent
      @presenter = presenter
    end

    # Returns { events:, next_since: }: the agent's own deliverable rows
    # ordered by id, with the cursor (the last scanned row id, which the
    # client passes back as since). Rows the payload builders drop after
    # SQL filtering still advance the cursor, so a fully dropped page
    # returns no rows but never strands the client. Readability filters
    # in SQL before the limit applies, so revoked rows can never hide
    # newer readable rows.
    def poll(since:, limit:)
      since = since.to_i
      limit = [ (limit.presence || POLL_DEFAULT_LIMIT).to_i, 1 ].max
      limit = [ limit, POLL_MAX_LIMIT ].min

      events = @agent.agent_events.readable_by(@agent)
        .where("agent_events.id > ?", since)
        .where(outcome: %w[ pending delivered acknowledged ])
        .ordered
        .limit(limit)
        .includes(:room, :actor, message: [ :room, :rich_text_body, { creator: :avatar_attachment }, { thread: { pull_request_thread: :pull_request } } ])
        .to_a

      @approval_cache = AgentApproval.where(id: events.filter_map { |event| event.metadata.is_a?(Hash) && event.metadata["approval_id"] }).index_by(&:id)
      @thread_cache = ChannelThread.where(id: events.filter_map { |event| event.metadata.is_a?(Hash) && event.metadata["thread_id"] }).includes(:room, :work_owner, :tags, work_thread_links: %i[ github_pull_request event ]).index_by(&:id)
      preload_poll_access!(events)

      payload = events.filter_map { |event| poll_payload(event) }

      { events: payload, next_since: events.last&.id || since }
    end

    private
      # Membership and grant checks for every row of one poll, resolved in
      # three queries no matter how many rows the page holds. Mirrors
      # Agent#can?(:read_messages, room) exactly: an inactive agent reads
      # nothing, a legacy agent reads every member room, and anyone else
      # needs a room or workspace-wide grant.
      def preload_poll_access!(events)
        room_ids = events.filter_map(&:room_id).uniq
        @poll_agent_active = @agent.active?
        @poll_member_room_ids = Membership.where(user_id: @agent.user_id, room_id: room_ids).pluck(:room_id).to_set
        @poll_legacy = @agent.legacy_capabilities?

        granted = AgentGrant.active.where(agent_id: @agent.id, capability: "read_messages", room_id: [ room_ids, nil ].flatten)
          .pluck(:room_id)
        @poll_workspace_grant = granted.include?(nil)
        @poll_granted_room_ids = granted.compact.to_set
      end

      def poll_room_readable?(room)
        return false unless @poll_agent_active
        return false unless @poll_member_room_ids.include?(room.id)
        return true if @poll_legacy

        @poll_workspace_grant || @poll_granted_room_ids.include?(room.id)
      end

      def poll_payload(event)
        if event.event_type == "github_action_completed"
          return github_action_poll_payload(event)
        end

        if event.event_type == "fizzy_action_completed"
          return fizzy_action_poll_payload(event)
        end

        if event.event_type == "approval_decided" || event.message_id.nil? && event.metadata.is_a?(Hash) && event.metadata["approval_id"]
          return approval_poll_payload(event)
        end

        if AgentEvent::WORK_DELIVERABLE_TYPES.include?(event.event_type)
          return work_poll_payload(event)
        end

        if AgentEvent::SLASH_DELIVERABLE_TYPES.include?(event.event_type)
          return slash_command_poll_payload(event)
        end

        message = event.message
        room = event.room
        return if message.nil? || room.nil?
        return unless poll_room_readable?(room)

        # pull_request merges after compact so it stays an explicit null
        # outside PR threads instead of disappearing from the payload.
        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          hop: event.hop,
          room: { id: room.id, name: room.name },
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          message: @presenter.message_payload(message)
        }.compact.merge(pull_request: ::Github::PullRequestThread.payload_for_message(message, agent: @agent))
      end

      def work_poll_payload(event)
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        thread = @thread_cache&.dig(metadata["thread_id"]) || ChannelThread.find_by(id: metadata["thread_id"])
        room = event.room
        return if room.nil?
        return unless poll_room_readable?(room)

        # A deleted thread has no live payload to build, but its
        # work_unassigned row carries a pre-destroy snapshot taken while the
        # agent could still read it. Poll-only agents learn the deletion
        # from exactly that snapshot, marked thread_deleted, with no live
        # data beyond it. Assignment rows have no snapshot and stay dropped.
        work = if thread
          Agent::Delivery.work_payload(thread, assigned_by: metadata["assigned_by"], agent: @agent)
        else
          return unless event.event_type == "work_unassigned"

          snapshot = metadata["work_snapshot"]
          return unless snapshot.is_a?(Hash)

          snapshot
        end

        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          room: { id: room.id, name: room.name },
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          work: work,
          handoff: handoff_payload_for(event),
          thread_deleted: (true if thread.nil?)
        }.compact
      end

      # The context package travels snapshotted in the event metadata, so
      # polling never depends on the handoff row surviving. Other work
      # event types carry no handoff key.
      def handoff_payload_for(event)
        return unless event.event_type == "work_handed_off"

        snapshot = event.metadata.is_a?(Hash) ? event.metadata["handoff"] : nil
        snapshot.is_a?(Hash) ? snapshot.slice("id", "summary", "links", "open_questions", "sender_name", "receiver_agent_id") : nil
      end

      def slash_command_poll_payload(event)
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        room = event.room
        return if room.nil?
        return unless poll_room_readable?(room)

        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          room: { id: room.id, name: room.name },
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          thread_id: metadata["thread_id"],
          command: {
            name: metadata["command"],
            arguments: metadata["arguments"]
          }
        }.compact
      end

      def github_action_poll_payload(event)
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        room = event.room
        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          room: room ? { id: room.id, name: room.name } : nil,
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          github_action: {
            approval_id: metadata["approval_id"],
            action: metadata["action"],
            status: metadata["status"],
            url: metadata["url"],
            message: metadata["message"]
          }.compact
        }.compact
      end

      def fizzy_action_poll_payload(event)
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        room = event.room
        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          room: room ? { id: room.id, name: room.name } : nil,
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          fizzy_action: {
            approval_id: metadata["approval_id"],
            action: metadata["action"],
            status: metadata["status"],
            url: metadata["url"],
            message: metadata["message"]
          }.compact
        }.compact
      end

      def approval_poll_payload(event)
        metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
        approval = @approval_cache&.dig(metadata["approval_id"]) || AgentApproval.find_by(id: metadata["approval_id"])
        return if approval.nil? || approval.agent_id != @agent.id

        room = event.room
        {
          id: event.id,
          event_type: event.event_type,
          outcome: event.outcome,
          created_at: event.created_at&.utc,
          room: room ? { id: room.id, name: room.name } : nil,
          actor: event.actor ? { id: event.actor.id, name: event.actor.name } : nil,
          approval: {
            id: approval.id,
            approval_id: approval.id,
            action: approval.action,
            summary: approval.summary,
            status: approval.effective_status,
            decided_by: approval.decided_by&.name || metadata["decided_by"],
            note: approval.decision_note || metadata["note"],
            expires_at: approval.expires_at&.utc,
            room_id: approval.room_id
          }.compact
        }.compact
      end
  end
end
