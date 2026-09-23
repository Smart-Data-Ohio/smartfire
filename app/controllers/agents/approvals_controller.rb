class Agents::ApprovalsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ index show create destroy for_agent ]
  allow_bot_access only: %i[ for_agent ]

  before_action :ensure_agent_token, only: %i[ index show create destroy ]
  throttle_agent_api limit: 120, only: %i[ index show ]
  throttle_agent_api limit: 60, only: %i[ create destroy ]

  HTML_PER_PAGE = 50

  # GET /agents/approvals?status=pending (Bearer-only, JSON). Lists the
  # agent's own rows, newest first, max 100. The lookup lives in
  # Agents::Approvals, shared with the MCP tools.
  def index
    no_store_response!

    render_approval_result Agents::Approvals.list(agent: Current.agent, status: params[:status])
  end

  # GET /agents/approvals/:id (Bearer-only, JSON).
  def show
    no_store_response!

    render_approval_result Agents::Approvals.show(agent: Current.agent, id: params[:id])
  end

  # POST /agents/approvals (Bearer-only, JSON). A repeated external_id
  # returns the existing row with 200 instead of a duplicate.
  def create
    no_store_response!

    render_approval_result Agents::Approvals.create(agent: Current.agent, fields: approval_request_fields, credential: current_credential)
  end

  # DELETE /agents/approvals/:id (Bearer-only, JSON). Cancels a pending
  # request; 422 once decided or expired.
  def destroy
    no_store_response!

    render_approval_result Agents::Approvals.cancel(agent: Current.agent, id: params[:id])
  end

  # GET /agents/:id/approvals (HTML). Approval history for admins and the
  # agent's owner. Paginated, filterable by status. Bearer agent tokens
  # and non-deciders get 404.
  def for_agent
    if authenticated_by.agent_token? || authenticated_by.bot_key? || Current.user&.bot?
      head :not_found
      return
    end

    @agent = Agent.find_by(id: params[:id])
    @bot = @agent&.user
    unless @agent && @bot && decider?(@agent, Current.user)
      head :not_found
      return
    end

    @status_filter = params[:status].presence_in(AgentApproval::STATUSES)
    @page = [ params[:page].to_i, 1 ].max

    scope = AgentApproval.where(agent_id: @agent.id).order(id: :desc).includes(:room, :decided_by)
    scope = Agents::Approvals.apply_effective_status_filter(scope, @status_filter) if @status_filter

    @approvals = scope.limit(HTML_PER_PAGE + 1).offset((@page - 1) * HTML_PER_PAGE).to_a
    @approvals.each(&:expire_if_due!)
    if @status_filter == "pending"
      @approvals.select!(&:pending_effective?)
    elsif @status_filter == "expired"
      @approvals.select!(&:expired_effective?)
    end
    @has_next = @approvals.size > HTML_PER_PAGE
    @approvals.pop if @has_next
  end

  private
    def ensure_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
      end
    end

    def render_approval_result(result)
      if result.ok?
        render json: result.payload, status: result.status
      elsif result.status == :not_found
        head :not_found
      else
        render json: result.failure_body, status: result.status
      end
    end

    def decider?(agent, user)
      return false unless user&.active? && !user.bot?
      return false unless agent.user&.active?

      user.administrator? || agent.owner_id == user.id
    end

    # Accepts a nested approval object or top-level fields. Top-level
    # `action` collides with the routing param, so the raw body is read
    # for it and `approval_action` works as an alias.
    def approval_request_fields
      nested = params[:approval]
      nested_hash = case nested
      when ActionController::Parameters then nested.to_unsafe_h
      when Hash then nested
      else {}
      end
      nested_hash = nested_hash.transform_keys(&:to_s) if nested_hash.any?

      body_params = request.request_parameters
      body_hash = body_params.is_a?(Hash) ? body_params : {}

      payload = if nested_hash.key?("payload")
        nested_value(nested, "payload")
      elsif body_hash.key?("payload")
        body_hash["payload"]
      else
        params[:payload]
      end

      {
        "action" => nested_hash["action"].presence || body_hash["action"].presence || params[:approval_action].presence,
        "summary" => nested_hash["summary"].presence || params[:summary].presence || body_hash["summary"].presence,
        "room_id" => nested_hash["room_id"].presence || params[:room_id].presence || body_hash["room_id"].presence,
        "payload" => payload,
        "external_id" => nested_hash["external_id"].presence || params[:external_id].presence || body_hash["external_id"].presence,
        "expires_at" => nested_hash["expires_at"].presence || params[:expires_at].presence || body_hash["expires_at"].presence,
        "expires_in" => nested_hash["expires_in"].presence || nested_hash["expires_in_seconds"].presence ||
          params[:expires_in].presence || params[:expires_in_seconds].presence ||
          body_hash["expires_in"].presence || body_hash["expires_in_seconds"].presence
      }
    end

    def nested_value(nested, key)
      if nested.is_a?(ActionController::Parameters)
        nested.key?(key) ? nested[key] : nil
      elsif nested.is_a?(Hash)
        nested[key] || nested[key.to_sym]
      end
    end

    def current_credential
      scheme, token = request.authorization.to_s.split(" ", 2)
      return nil unless scheme&.casecmp?("Bearer") && token.present?

      AgentCredential.find_by(token_digest: AgentCredential.digest(token.strip))
    end
end
