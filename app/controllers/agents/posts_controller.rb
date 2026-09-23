class Agents::PostsController < ApplicationController
  include AgentAuthorization
  include AgentApiThrottle

  allow_agent_access only: %i[ index create ]

  # Bearer-only endpoint. Forgery protection stays on: Bearer agent already
  # bypass it through the Authentication concern, and a session-cookie request
  # that trips it gets the same 403 JSON that ensure_agent_token would return.
  rescue_from ActionController::InvalidAuthenticityToken, with: :reject_session_request

  before_action :set_room, only: %i[ index create ]
  before_action :ensure_agent_token, only: %i[ index create ]
  require_agent_capability :read_messages, only: :index
  require_agent_capability :post_messages, only: :create
  require_agent_capability :manage_threads, only: :create
  throttle_agent_api limit: 30, only: :create
  # After the capability checks so a member without a grant sees 403, not
  # a hint about what kind of room this is.
  before_action :ensure_board_room, only: %i[ index create ]

  # GET /rooms/:room_id/agents/posts (Bearer-only, JSON). The board's
  # posts as work payloads, newest activity first, max 100. status is a
  # single work status or open/done/all (default open); owner is a user
  # id, me, or agents; tag is a single tag. The filters live in
  # Agents::BoardPosts, shared with the MCP list_board_posts tool.
  def index
    no_store_response!

    result = Agents::BoardPosts.list(
      agent: Current.agent, room: @room,
      status: params[:status], owner: params[:owner], tag: params[:tag]
    )

    if result.ok?
      render json: result.payload.map { |thread| Agents::WorkPayload.for(thread, agent: Current.agent) }
    else
      render json: result.failure_body, status: result.status
    end
  end

  # POST /rooms/:room_id/agents/posts (Bearer-only, JSON). Creates a post
  # through the same path as the human new-post form: title (required),
  # body (Markdown for the first message, optional), tags (array or
  # comma-separated string), work_status (default in_progress), run_url
  # (https only), and owner_id (an eligible member or agent, defaulting
  # to the agent itself). Requires post_messages and manage_threads in
  # the board. Returns the work payload.
  def create
    no_store_response!

    result = Agents::BoardPosts.create(
      agent: Current.agent, room: @room,
      title: params[:title], body: params[:body], tags: params[:tags],
      work_status: params[:work_status], run_url: params[:run_url], owner_id: params[:owner_id]
    )

    if result.ok?
      render json: Agents::WorkPayload.for(result.payload, agent: Current.agent), status: :created
    elsif result.status == :not_found
      head :not_found
    else
      render json: result.failure_body, status: result.status
    end
  end

  private
    def set_room
      @room = Current.user.rooms.find_by(id: params[:room_id])

      head :not_found unless @room
    end

    def ensure_agent_token
      reject_session_request unless authenticated_by.agent_token?
    end

    def ensure_board_room
      return if @room.board?

      render json: { error: "Room is not a board" }, status: :unprocessable_entity
    end

    def reject_session_request
      render json: { error: "Forbidden: Bearer agent token required" }, status: :forbidden
    end
end
