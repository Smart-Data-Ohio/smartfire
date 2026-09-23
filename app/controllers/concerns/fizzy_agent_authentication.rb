module FizzyAgentAuthentication
  extend ActiveSupport::Concern

  private
    # Agent Fizzy endpoints are Bearer-only JSON: a bad or revoked token
    # is 401 from authentication itself, while a legacy bot key or a
    # human session is 403 here.
    def ensure_fizzy_agent_token
      unless authenticated_by.agent_token? && Current.agent
        render json: { error: "Forbidden: agent token required" }, status: :forbidden
      end
    end

    def render_fizzy_result(result)
      if result.ok?
        render json: result.payload, status: result.status
      else
        render json: result.failure_body, status: result.status
      end
    end
end
