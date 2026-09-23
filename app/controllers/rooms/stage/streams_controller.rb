class Rooms::Stage::StreamsController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_stage_room

  # Hosts and speakers go live with their screen at an explicit quality. One
  # room carries at most one live stream: starting while another is live
  # answers 409 naming the presenter, including when a concurrent start wins
  # the race and the partial unique index rejects this one. A server-muted
  # host or speaker answers 403: their share would die with the mute anyway.
  # Going live also requires an in-call huddle grant for the room — one the
  # gateway has seen recently, not just an active one — so a stream never
  # starts without a live call to publish over. The Stream callbacks broadcast
  # the header badge, sidebar dot, and per-viewer panel; the response only
  # swaps the actor's own panel without navigating.
  def create
    unless @membership.host? || @membership.speaker?
      return render plain: "Only hosts and speakers can go live", status: :forbidden
    end

    if @membership.server_muted?
      return render plain: "Muted members cannot go live", status: :forbidden
    end

    unless HuddleGrant.active.in_call.exists?(room_id: @room.id, membership_id: @membership.id)
      return render plain: "Join the stage before going live", status: :forbidden
    end

    unless Stream::QUALITIES.include?(params[:quality].to_s)
      return render plain: "Unknown stream quality", status: :unprocessable_entity
    end

    if (live = @room.live_stream)
      return render plain: "#{live.user.name} is already live", status: :conflict
    end

    begin
      stream = @room.streams.create!(membership: @membership, user: Current.user, quality: params[:quality])
    rescue ActiveRecord::RecordNotUnique
      live = @room.live_stream
      return render plain: "#{live&.user&.name || "Someone"} is already live", status: :conflict
    else
      # The presenting browser reads this for every later DELETE, so a
      # delayed end names this stream and can never kill someone else's
      # newer one.
      response.headers["X-Stream-Id"] = stream.id.to_s
    end

    respond_with_panel
  end

  # Ends the room's live stream. The presenter or any host or administrator
  # member can stop; anyone else gets 403 even when nothing is live, so the
  # endpoint never confirms stream state to listeners. Stopping an already
  # ended stream succeeds without doing anything. The actor travels with the
  # end so a host's stop can notify the presenter's browser.
  #
  # Both the Stop control and the presenting browser send the stream id they
  # mean to stop: a DELETE that names a stream which is no longer live ends
  # nothing, so a delayed stop can never kill someone else's newer stream.
  # The id is purely anti-staleness — stopping is still authorized by role —
  # and requests without one keep the historical end-whatever-is-live
  # behavior.
  def destroy
    stream = @room.live_stream

    unless @membership.host? || Current.user.administrator? || (stream && stream.membership_id == @membership.id)
      return render plain: "Only the presenter or a host can stop the stream", status: :forbidden
    end

    if params[:stream_id].present?
      stream.end!(ended_by: Current.user) if stream && stream.id.to_s == params[:stream_id].to_s
    else
      stream&.end!(ended_by: Current.user)
    end
    respond_with_panel
  end

  private
    def ensure_stage_room
      head :not_found unless @room.stage?
    end

    def respond_with_panel
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace([ @room, :stage_panel ],
            partial: "rooms/stage/panel_body",
            locals: { room: @room, membership: @membership, rejoin: false })
        end
        format.html { redirect_to room_url(@room) }
      end
    end
end
