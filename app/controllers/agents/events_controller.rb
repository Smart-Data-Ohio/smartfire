class Agents::EventsController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: %i[ index ack ]

  before_action :ensure_agent_token, only: %i[ index ack ]
  before_action :set_ack_event, only: :ack
  require_agent_capability :read_messages, only: %i[ index ack ]
  throttle_agent_api limit: 120, only: %i[ index ack ]

  LEDGER_PER_PAGE = 50
  POLL_DEFAULT_LIMIT = 50
  POLL_MAX_LIMIT = 100

  # GET /agents/events?since=<id>&limit=<n> (Bearer-only, JSON). Returns
  # the agent's own deliverable rows ordered by id as a bare JSON array,
  # with the cursor (the last scanned row id, which the client passes
  # back as since) in the X-Smartfire-Next-Since response header. Pass
  # ?envelope=1 for the { events, next_since } object form instead.
  # Rows the Ruby payload builders drop after SQL filtering (a revoked
  # message, a deleted thread without a snapshot) still advance the
  # cursor, so a fully dropped page returns no rows but never strands
  # the client. Readability (message exists, membership, read grant)
  # filters in SQL before the limit applies, so revoked rows can never
  # hide newer readable rows. Approval decision rows carry no message
  # and render an approval payload instead; GitHub completion rows
  # render a github_action payload instead, and work rows render a work
  # payload instead. A deleted thread's work_unassigned row renders its
  # pre-destroy snapshot marked thread_deleted.
  def index
    no_store_response!

    agent = Current.agent
    since = params[:since].to_i
    limit = [ (params[:limit].presence || POLL_DEFAULT_LIMIT).to_i, 1 ].max
    limit = [ limit, POLL_MAX_LIMIT ].min

    events = agent.agent_events.readable_by(agent)
      .where("agent_events.id > ?", since)
      .where(outcome: %w[ pending delivered acknowledged ])
      .ordered
      .limit(limit)
      .includes(:room, :actor, message: [ :room, :rich_text_body, { creator: :avatar_attachment }, { thread: { pull_request_thread: :pull_request } } ])
      .to_a

    @approval_cache = AgentApproval.where(id: events.filter_map { |event| event.metadata.is_a?(Hash) && event.metadata["approval_id"] }).index_by(&:id)
    @thread_cache = ChannelThread.where(id: events.filter_map { |event| event.metadata.is_a?(Hash) && event.metadata["thread_id"] }).includes(:room, :work_owner, :tags, work_thread_links: %i[ github_pull_request event ]).index_by(&:id)
    preload_poll_access!(agent, events)

    payload = events.filter_map { |event| poll_payload(event) }
    next_since = events.last&.id || since
    response.set_header("X-Smartfire-Next-Since", next_since.to_s)

    if params[:envelope] == "1"
      render json: { events: payload, next_since: next_since }
    else
      render json: payload
    end
  end

  # POST /agents/events/:id/ack (Bearer-only, JSON). Idempotent.
  def ack
    no_store_response!

    @agent_event.acknowledged! unless @agent_event.acknowledged?
    render json: { id: @agent_event.id, outcome: "acknowledged" }
  end

  # GET /agents/:id/events (HTML). Activity ledger for admins and the
  # agent's owner. Paginated, filterable by outcome. Bearer agent tokens
  # are denied by default; this is a session-authenticated management page.
  def ledger
    @agent = Agent.find(params[:id])
    @bot = @agent.user

    unless Current.user.administrator? || @agent.owner == Current.user
      head :forbidden
      return
    end

    @outcome_filter = params[:outcome].presence_in(AgentEvent::OUTCOMES)
    @page = [ params[:page].to_i, 1 ].max

    scope = @agent.agent_events.recent_first.includes(:room, :actor, :message)
    scope = scope.where(outcome: @outcome_filter) if @outcome_filter

    @events = scope.limit(LEDGER_PER_PAGE + 1).offset((@page - 1) * LEDGER_PER_PAGE).to_a
    @has_next = @events.size > LEDGER_PER_PAGE
    @events.pop if @has_next
  end

  private
    def ensure_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
      end
    end

    def set_ack_event
      @agent_event = Current.agent.agent_events.deliverable
        .where.not(outcome: "suppressed")
        .find_by(id: params[:id])

      unless @agent_event
        head :not_found
        return
      end

      # Approval decisions, GitHub completions, and work assignments carry
      # no message and are always ackable by their own agent. Clearing the
      # room forces the capability check below to the workspace-wide form,
      # matching the polling endpoint.
      if AgentEvent::ALWAYS_READABLE_TYPES.include?(@agent_event.event_type) || AgentEvent::WORK_DELIVERABLE_TYPES.include?(@agent_event.event_type)
        @room = nil
        @message = nil
        return
      end

      # Ack requires the row's message to be currently readable by the agent
      # under the same rule as polling: the message exists and the agent's
      # user is still a member of its room. A surviving workspace grant
      # alone is not enough.
      message = @agent_event.message
      unless message && Membership.exists?(user_id: Current.agent.user_id, room_id: message.room_id)
        head :not_found
        return
      end

      # Lets AgentAuthorization check the event's room.
      @room = @agent_event.room
      @message = message
    end

    # Membership and grant checks for every row of one poll, resolved in
    # three queries no matter how many rows the page holds. Mirrors
    # Agent#can?(:read_messages, room) exactly: an inactive agent reads
    # nothing, a legacy agent reads every member room, and anyone else
    # needs a room or workspace-wide grant.
    def preload_poll_access!(agent, events)
      room_ids = events.filter_map(&:room_id).uniq
      @poll_agent_active = agent.active?
      @poll_member_room_ids = Membership.where(user_id: agent.user_id, room_id: room_ids).pluck(:room_id).to_set
      @poll_legacy = agent.legacy_capabilities?

      granted = AgentGrant.active.where(agent_id: agent.id, capability: "read_messages", room_id: [ room_ids, nil ].flatten)
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

      if event.event_type == "approval_decided" || event.message_id.nil? && event.metadata.is_a?(Hash) && event.metadata["approval_id"]
        return approval_poll_payload(event)
      end

      if AgentEvent::WORK_DELIVERABLE_TYPES.include?(event.event_type)
        return work_poll_payload(event)
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
        message: message_payload(message)
      }.compact.merge(pull_request: Github::PullRequestThread.payload_for_message(message))
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
        Agent::Delivery.work_payload(thread, assigned_by: metadata["assigned_by"])
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
        thread_deleted: (true if thread.nil?)
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

    def approval_poll_payload(event)
      metadata = event.metadata.is_a?(Hash) ? event.metadata : {}
      approval = @approval_cache&.dig(metadata["approval_id"]) || AgentApproval.find_by(id: metadata["approval_id"])
      return if approval.nil? || approval.agent_id != Current.agent.id

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
