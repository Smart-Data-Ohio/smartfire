class Agents::StreamingMessagesController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: %i[ create update finalize ]

  # Bearer-only endpoint. Forgery protection stays on: bearer tokens already
  # bypass it through the Authentication concern, and a session-cookie request
  # that trips it gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :set_room, only: :create
  before_action :ensure_agent_token
  require_agent_capability :post_messages, only: :create
  throttle_agent_api limit: 60, only: %i[ create finalize ]
  throttle_agent_api limit: 240, only: :update

  # POST /rooms/:room_id/agents/streaming_messages (Bearer-only, JSON).
  # Starts a stream: a message in a `streaming` state the agent appends
  # to or replaces until it finalizes. Accepts a top-level thread_id
  # like the messages endpoint; the response carries thread_id and
  # streaming. Counts against the daily message budget.
  def create
    no_store_response!

    result = Agents::Streaming.start(
      agent: Current.agent,
      room: @room,
      thread_id: params[:thread_id],
      attributes: stream_message_attributes
    )

    render_stream_result result
  rescue ActiveRecord::RecordNotFound
    head :not_found
  end

  # PATCH /agents/streaming_messages/:id (Bearer-only, JSON). Appends
  # `append` to the stream, or replaces the whole body with
  # `markdown_source` when no append is given. The save always lands;
  # the broadcast coalesces to about 4 per second per message.
  def update
    no_store_response!

    result = Agents::Streaming.update(
      agent: Current.agent,
      id: params[:id],
      append: params[:append],
      markdown_source: params[:markdown_source]
    )

    render_stream_result result
  end

  # POST /agents/streaming_messages/:id/finalize (Bearer-only, JSON).
  # Ends the stream, firing every deferred side effect exactly once.
  # Idempotent: finalizing an already-final message succeeds without
  # repeating side effects.
  def finalize
    no_store_response!

    render_stream_result Agents::Streaming.finalize(agent: Current.agent, id: params[:id])
  end

  private
    def set_room
      @room = Current.user.rooms.find_by(id: params[:room_id])

      head :not_found unless @room
    end

    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def reject_session_request
      render json: { error: "Forbidden: #{"Bearer"} agent token required" }, status: :forbidden
    end

    def stream_message_attributes
      params.require(:message).permit(
        :markdown_source, :client_message_id, :reply_to_message_id, :reply_notify_author
      ).to_h.symbolize_keys
    end

    def render_stream_result(result)
      if result.ok?
        message = result.payload
        render json: message_payload(message).merge(
          thread_id: message.thread_id, streaming: message.streaming?
        ), status: result.status
      elsif result.status == :not_found
        head :not_found
      elsif result.payload
        render json: result.payload, status: result.status
      else
        render json: result.failure_body, status: result.status
      end
    end
end
