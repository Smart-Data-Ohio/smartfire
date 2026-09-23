module FizzyAgentAuthentication
  extend ActiveSupport::Concern

  FIZZY_ID_FORMAT = /\A[A-Za-z0-9_-]+\z/

  private
    # Agent Fizzy endpoints are Bearer-only JSON: a bad or revoked token
    # is 401 from authentication itself, while a legacy bot key or a
    # human session is 403 here.
    def ensure_fizzy_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: agent token required" }, status: :forbidden
      end
    end

    # Reads run as the agent owner's own linked Fizzy account, so an
    # agent only ever sees what its owner can access in Fizzy.
    def ensure_owner_fizzy_account
      owner = Current.agent.owner
      if owner.nil?
        render json: { error: "Agent has no owner recorded" }, status: :unprocessable_entity
        return
      end

      @fizzy_account = owner.fizzy_connected_account
      unless @fizzy_account&.usable?
        render json: { error: "Agent owner has no usable Fizzy account" }, status: :unprocessable_entity
      end
    end

    # Room-less endpoints follow the approvals rule: the grant must be
    # workspace-wide, since the Fizzy call concerns no room.
    def ensure_fizzy_capability
      unless Current.agent.can?(:fizzy, nil)
        render json: { error: "Forbidden: agent lacks fizzy capability" }, status: :forbidden
      end
    end

    def ensure_fizzy_external_action
      unless Current.agent.can?(:external_action, nil)
        render json: { error: "Forbidden: agent lacks external_action capability" }, status: :forbidden
      end
    end

    def fizzy_client
      Fizzy::Client.new(token: @fizzy_account.access_token)
    end

    # The owner's stored Fizzy account unless the request names another
    # one the token can access (card URLs carry their own account).
    def resolved_fizzy_account_id
      params[:account_id].presence || @fizzy_account.fizzy_account_id
    end

    # Guards an agent-supplied Fizzy id before it is interpolated into an
    # API path. Renders 404 and returns nil when invalid.
    def validated_fizzy_id(value)
      value = value.to_s
      return value if value.match?(FIZZY_ID_FORMAT)

      render json: { error: "Not found in Fizzy" }, status: :not_found
      nil
    end

    # Maps Fizzy failures on agent reads to agent-facing statuses. A 401
    # disconnects the owner's account exactly like the member path, so
    # the profile offers a reconnect instead of failing silently. A 403
    # reads as 404, like the member card frame's minimal chip: the token
    # cannot see the resource, and the response must not say whether it
    # exists.
    def render_fizzy_read_error(error)
      case error
      when Fizzy::Client::Unauthorized
        @fizzy_account.mark_disconnected!("Fizzy rejected the linked token (401)")
        render json: { error: "Agent owner's Fizzy token was rejected" }, status: :unprocessable_entity
      when Fizzy::Client::NotFound, Fizzy::Client::Forbidden
        render json: { error: "Not found in Fizzy" }, status: :not_found
      when Fizzy::Client::Refused
        render json: { error: error.message }, status: :unprocessable_entity
      else
        render json: { error: error.message }, status: :bad_gateway
      end
    end
end
