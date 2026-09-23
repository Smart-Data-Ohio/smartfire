class Rooms::Stage::RolesController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_stage_room
  before_action :ensure_can_manage_stage

  STAGE_ROLES = %w[ listener speaker host ].freeze

  # Hosts and administrators change any member's stage role. A change that
  # crosses the publish boundary (to or from listener) revokes the member's
  # huddle grants in the same transaction, and three broadcasts deliver it:
  # a per-viewer roster goes to every member's own rooms stream, a
  # personalized panel goes to the affected member's stream, and a rejoin
  # event goes to the persistent target in their huddle panel, which is
  # present on every page. The persistent event is the only reconnect
  # trigger, so delayed delivery cannot reconnect twice; the panel carries no
  # trigger. A host↔speaker change keeps the grants and the call, so it
  # broadcasts the roster and panel but no rejoin event: rejoining would
  # drop the member's share and end their live stream. A demotion to
  # listener also ends the member's live stream in the same transaction.
  def update
    target = @room.memberships.find_by(id: params[:membership_id])
    return head :not_found unless target

    # Same rank rule as call moderation: only administrators change an
    # administrator's stage role.
    if target.user.administrator? && !Current.user.administrator? && target.user != Current.user
      return render plain: "Only administrators can change an administrator's stage role", status: :forbidden
    end

    unless STAGE_ROLES.include?(params[:stage_role].to_s)
      return render plain: "Unknown stage role", status: :unprocessable_entity
    end

    begin
      ActiveRecord::Base.transaction do
        target.change_stage_role!(params[:stage_role])
        Stream.end_live_for_membership!(target) if target.listener?
      end
    rescue ActiveRecord::RecordInvalid => error
      return render plain: error.record.errors.full_messages.to_sentence, status: :unprocessable_entity
    end

    crossed_publish_boundary = publish_boundary_crossed?(target)

    broadcast_roster
    broadcast_panel_to_member(target)
    broadcast_role_event_to_member(target) if crossed_publish_boundary
    respond_with_roster
  end

  private
    # Administrators manage through membership like everyone else: an
    # administrator who is not a member of the room gets the same 404 as any
    # other non-member from the room scoping above.
    def ensure_can_manage_stage
      head :forbidden unless @membership.host? || Current.user.administrator?
    end

    def ensure_stage_room
      head :not_found unless @room.stage?
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

    def broadcast_panel_to_member(target)
      broadcast_replace_to target.user, :rooms,
        target: [ @room, :stage_panel ],
        partial: "rooms/stage/panel_body",
        locals: { room: @room, membership: target.reload, rejoin: false }
    end

    # Read from previous_changes before the panel broadcast reloads the
    # record: only a to-or-from-listener crossing needs a fresh token.
    def publish_boundary_crossed?(target)
      was, now = target.previous_changes["stage_role"]
      (was == "listener") != (now == "listener")
    end

    # The stage panel only exists on that stage's page, but the huddle panel
    # — and this target inside it — is permanent across pages. A demoted
    # speaker who navigated elsewhere still gets the rejoin signal here.
    def broadcast_role_event_to_member(target)
      broadcast_append_to target.user, :rooms,
        target: "huddle_role_events",
        partial: "rooms/stage/role_event",
        locals: { room: @room, membership: target }
    end

    def respond_with_roster
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace([ @room, :stage_roster ],
            partial: "rooms/stage/roster",
            locals: { room: @room, viewer: @membership })
        end
        format.html { redirect_to room_url(@room) }
      end
    end
end
