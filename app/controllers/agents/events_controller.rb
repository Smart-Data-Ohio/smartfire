class Agents::EventsController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: %i[ index ack ]

  before_action :ensure_agent_token, only: %i[ index ack ]
  require_agent_capability :read_messages, only: :index
  throttle_agent_api limit: 120, only: %i[ index ack ]

  LEDGER_PER_PAGE = 50

  # GET /agents/events?since=<id>&limit=<n> (Bearer-only, JSON). Returns
  # the agent's own deliverable rows ordered by id as a bare JSON array,
  # with the cursor (the last scanned row id, which the client passes
  # back as since) in the X-Smartfire-Next-Since response header. Pass
  # ?envelope=1 for the { events, next_since } object form instead.
  # The query and payload assembly live in Agents::EventPolling, shared
  # with the MCP poll_events tool.
  def index
    no_store_response!

    result = Agents::EventPolling.poll(agent: Current.agent, since: params[:since], limit: params[:limit], presenter: self)
    response.set_header("X-Smartfire-Next-Since", result[:next_since].to_s)

    if params[:envelope] == "1"
      render json: { events: result[:events], next_since: result[:next_since] }
    else
      render json: result[:events]
    end
  end

  # POST /agents/events/:id/ack (Bearer-only, JSON). Idempotent. The row
  # lookup and its readability and capability checks live in
  # Agents::EventPolling, shared with the MCP ack_events tool.
  def ack
    no_store_response!

    result = Agents::EventPolling.ack(agent: Current.agent, id: params[:id])

    if result.ok?
      render json: result.payload
    elsif result.status == :not_found
      head :not_found
    else
      render json: result.failure_body, status: result.status
    end
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
end
