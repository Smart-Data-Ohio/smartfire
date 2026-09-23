class Rooms::Stage::HandsController < ApplicationController
  include RoomScoped

  HAND_RAISE_LIMIT = 10
  HAND_RAISE_WINDOW = 1.minute

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_stage_room

  # Listeners raise their own hand. Speakers and hosts have no hand to raise.
  # Raises are idempotent and throttled per membership: a double raise keeps
  # the first timestamp and skips the roster broadcast, and hammering the
  # endpoint answers 429.
  def create
    unless @membership.listener?
      return render plain: "Only listeners can raise a hand", status: :unprocessable_entity
    end

    if hand_raise_throttled?
      return render plain: "Slow down and try again", status: :too_many_requests
    end

    broadcast_roster if @membership.raise_hand!
    respond_with_controls
  end

  # Clears a raised hand: your own, or — with a membership_id parameter —
  # another member's, for hosts and administrators. Clearing a hand that was
  # never raised succeeds without doing anything.
  def destroy
    target = target_membership
    return if performed?

    target.lower_hand!
    broadcast_roster
    respond_with_controls
  end

  private
    def target_membership
      if params[:membership_id].present?
        unless @membership.host? || Current.user.administrator?
          render plain: "Only hosts can lower another member's hand", status: :forbidden
          return nil
        end

        target = @room.memberships.find_by(id: params[:membership_id])
        head :not_found unless target
        target
      else
        @membership
      end
    end

    def ensure_stage_room
      head :not_found unless @room.stage?
    end

    # Per-membership minute-bucketed counter. Null stores (test env default)
    # answer nil from increment, which counts as unthrottled.
    def hand_raise_throttled?
      key = "stage_hand_raise/#{@room.id}/#{@membership.id}/#{Time.current.to_i / HAND_RAISE_WINDOW.to_i}"
      Rails.cache.increment(key, 1, expires_in: HAND_RAISE_WINDOW).to_i > HAND_RAISE_LIMIT
    end

    # Host action forms render only for viewers who may use them, so each
    # member gets their own roster on their own stream. Stage rooms are
    # small; a loop is fine.
    def broadcast_roster
      @room.memberships.includes(:user).each do |member|
        broadcast_replace_to member.user, :rooms,
          target: [ @room, :stage_roster ],
          partial: "rooms/stage/roster",
          locals: { room: @room, viewer: member }
      end
    end

    def respond_with_controls
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace([ @room, :stage_controls ],
            partial: "rooms/stage/controls",
            locals: { room: @room, membership: @membership })
        end
        format.html { redirect_to room_url(@room) }
      end
    end
end
