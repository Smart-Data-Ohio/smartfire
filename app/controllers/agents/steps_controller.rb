class Agents::StepsController < ApplicationController
  include AgentApiThrottle

  allow_agent_access only: %i[ create update ]

  # Bearer-only endpoint. Forgery protection stays on: bearer tokens already
  # bypass it through the Authentication concern, and a session-cookie request
  # that trips it gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :ensure_agent_token
  throttle_agent_api limit: 60, only: %i[ create update ]

  # POST /agents/steps (Bearer-only, JSON). Attaches a structured step to
  # the agent's own message (message_id) or to a work thread it owns
  # (thread_id): name, status, input/output summaries, duration. Message
  # steps need post_messages, thread steps need manage_threads.
  def create
    no_store_response!

    render_step_result Agents::Steps.create(agent: Current.agent, fields: step_fields)
  end

  # PATCH /agents/steps/:id (Bearer-only, JSON). Updates the step's
  # fields; each updates only when its key is given. Steps never move
  # between parents.
  def update
    no_store_response!

    render_step_result Agents::Steps.update(agent: Current.agent, id: params[:id], fields: step_fields)
  end

  private
    def ensure_agent_token
      unless authenticated_by.agent_token? && Current.agent
        reject_session_request
      end
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    def step_fields
      params.permit(:message_id, :thread_id, :name, :status, :input_summary, :output_summary, :duration_ms)
        .to_h.transform_keys(&:to_s)
    end

    def render_step_result(result)
      if result.ok?
        render json: Agents::Steps.step_payload(result.payload), status: result.status
      elsif result.status == :not_found
        head :not_found
      elsif result.payload
        render json: result.payload, status: result.status
      else
        render json: result.failure_body, status: result.status
      end
    end
end
