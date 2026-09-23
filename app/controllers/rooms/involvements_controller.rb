class Rooms::InvolvementsController < ApplicationController
  include RoomScoped

  def show
    @involvement = @membership.involvement
  end

  def update
    @membership.update! involvement: params[:involvement]
    # Muting clears the unread state: a muted room only goes unread on
    # mention, so anything unread from before the mute is stale.
    @membership.read if @membership.involved_in_muted?

    broadcast_visibility_changes
    redirect_to room_involvement_url(@room)
  end

  private
    def broadcast_visibility_changes
      case
      when @room.direct?
        broadcast_replace_muted_row if muted_transition?
      when @membership.involved_in_invisible?
        broadcast_remove_to @membership.user, :rooms, target: [ @room, :list ]
      when @membership.involvement_previously_was.inquiry.invisible?
        if @room.stage?
          broadcast_prepend_to @membership.user, :rooms, target: :stage_rooms, partial: "users/sidebars/rooms/stage", locals: { room: @room, membership: @membership }
        elsif @room.voice?
          broadcast_prepend_to @membership.user, :rooms, target: :voice_rooms, partial: "users/sidebars/rooms/voice", locals: { room: @room, membership: @membership }
        elsif @room.board?
          broadcast_prepend_to @membership.user, :rooms, target: :board_rooms, partial: "users/sidebars/rooms/board", locals: { room: @room, membership: @membership }
        else
          broadcast_prepend_to @membership.user, :rooms, target: :shared_rooms, partial: "users/sidebars/rooms/shared", locals: { room: @room, membership: @membership }
        end
      else
        broadcast_replace_muted_row if muted_transition?
      end
    end

    def muted_transition?
      @membership.involved_in_muted? != @membership.involvement_previously_was.inquiry.muted?
    end

    # A mute or unmute redims the sidebar row in place: the row stays
    # visible either way, so it is replaced rather than removed.
    def broadcast_replace_muted_row
      if @room.direct?
        broadcast_replace_to @membership.user, :rooms, target: [ @room, :list ],
          partial: "users/sidebars/rooms/direct", locals: { membership: @membership }
      else
        broadcast_replace_to @membership.user, :rooms, target: [ @room, :list ],
          partial: row_partial_for(@room), locals: { room: @room, unread: @membership.unread?, membership: @membership }
      end
    end

    def row_partial_for(room)
      if room.stage?
        "users/sidebars/rooms/stage"
      elsif room.voice?
        "users/sidebars/rooms/voice"
      elsif room.board?
        "users/sidebars/rooms/board"
      else
        "users/sidebars/rooms/shared"
      end
    end
end
