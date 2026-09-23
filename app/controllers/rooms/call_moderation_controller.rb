class Rooms::CallModerationController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_call_room
  before_action :ensure_can_moderate

  # Hosts and administrators server-mute any member of a stage or voice room.
  # The mute revokes the member's grants in the same transaction (see
  # Membership), so the gateway removes them and their client rejoins
  # subscribe-only until they are unmuted; the token is the enforcement, and
  # the gateway's per-second grant check is the backstop for a grant that
  # somehow survived. Muting also ends the member's live stream in the same
  # transaction: their share dies with the revoked grant. Idempotent.
  def mute
    target = find_target
    return if performed?

    ActiveRecord::Base.transaction do
      target.server_mute!
      Stream.end_live_for_membership!(target)
    end

    broadcast_roster if @room.stage?
    broadcast_role_event_to_member(target)
    respond_with_roster_or_done
  end

  # Clearing a mute that was never set succeeds without doing anything.
  def unmute
    target = find_target
    return if performed?

    ActiveRecord::Base.transaction do
      target.server_unmute!
    end

    broadcast_roster if @room.stage?
    broadcast_role_event_to_member(target)
    respond_with_roster_or_done
  end

  # Drops the member from the call without touching their membership: their
  # grants are revoked, so the gateway removes them and the presence stacks
  # clear through the revocation callbacks. Unlike a mute, no rejoin event is
  # sent, so they stay out until they join again themselves. Also ends their
  # live stream, which cannot outlive the call.
  def disconnect
    target = find_target
    return if performed?

    ActiveRecord::Base.transaction do
      HuddleGrant.revoke_for_membership!(target)
      Stream.end_live_for_membership!(target)
    end

    respond_with_roster_or_done
  end

  private
    # Voice rooms have no host role, so only administrators moderate there.
    # Stage rooms add hosts. Administrators manage through membership like
    # everywhere else: a non-member administrator gets the same 404 as any
    # other non-member from the room scoping above.
    def ensure_can_moderate
      allowed = Current.user.administrator? || (@room.stage? && @membership.host?)
      head :forbidden unless allowed
    end

    def ensure_call_room
      head :not_found unless @room.stage? || @room.voice?
    end

    def find_target
      target = @room.memberships.find_by(id: params[:membership_id])
      return head :not_found unless target

      if target.id == @membership.id
        render plain: "You cannot moderate your own call session", status: :unprocessable_entity
        return nil
      end

      target
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

    # The huddle panel — and this target inside it — is permanent across
    # pages, so the rejoin signal reaches the affected browser wherever it
    # is. The persistent event is the only reconnect trigger, so delayed
    # delivery cannot reconnect twice.
    def broadcast_role_event_to_member(target)
      broadcast_append_to target.user, :rooms,
        target: "huddle_role_events",
        partial: "rooms/stage/role_event",
        locals: { room: @room, membership: target }
    end

    def respond_with_roster_or_done
      respond_to do |format|
        format.turbo_stream do
          if @room.stage?
            render turbo_stream: turbo_stream.replace([ @room, :stage_roster ],
              partial: "rooms/stage/roster",
              locals: { room: @room, viewer: @membership })
          else
            head :no_content
          end
        end
        format.html { redirect_to room_url(@room) }
        format.json { head :no_content }
      end
    end
end
