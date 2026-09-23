class RoomsController < ApplicationController
  before_action :set_room, only: %i[ show destroy ]
  before_action :ensure_can_administer, only: %i[ destroy ]
  before_action :remember_last_room_visited, only: :show

  def index
    redirect_to room_url(Current.user.rooms.last)
  end

  def show
    @messages = Message::MentionPreloader.preload_for(find_messages)
  end

  def destroy
    room_label = @room.name
    Room.transaction { @room.begin_destroy! }
    enqueue_destroy
    AuditLog.record!(action: "room.destroy", target: @room, target_label: room_label,
      changes: { name: room_label })

    broadcast_remove_room
    redirect_to root_url
  end

  private
    # The enqueue runs after the marking transaction commits, so the job
    # never runs on pre-commit state. If the queue is down the room stays
    # marked deleted without a claim and the stuck-room sweep re-enqueues
    # its destroy — the destroy itself already happened, so the request
    # still succeeds.
    def enqueue_destroy
      Room::DestroyJob.perform_later(@room.id)
      @room.update_columns(destroy_enqueued_at: Time.current)
    rescue StandardError => error
      Rails.logger.error "Room destroy enqueue failed for room #{@room.id}: #{error.class}: #{error.message}"
    end

    def set_room
      if room = room_scope.find_by(id: params[:room_id] || params[:id])
        @room = room
      else
        redirect_to root_url, alert: "Room not found or inaccessible"
      end
    end

    # Subclasses narrow this to the room types they're allowed to act on, so that one
    # room namespace can't be used to reach another's rooms.
    def room_scope
      Current.user.rooms
    end

    def ensure_can_administer
      head :forbidden unless Current.user.can_administer?(@room)
    end

    def ensure_permission_to_create_rooms
      if Current.account.settings.restrict_room_creation_to_administrators? && !Current.user.administrator?
        head :forbidden
      end
    end

    def record_room_creation(room)
      AuditLog.record!(action: "room.create", target: room, changes: { name: room.name })
    end

    # Revises a room's membership like Memberships#revise and records who was
    # added and removed. Only the actual diff is logged: re-saving an
    # unchanged list writes no row.
    def revise_memberships_with_audit(room, granted:, revoked:)
      # Fresh plucks, not the cached associations: grant_to inserts with
      # insert_all and revoke destroys through scopes, both bypassing them.
      before_ids = room.memberships.pluck(:user_id)
      room.memberships.revise(granted: granted, revoked: revoked)
      after_ids = room.memberships.pluck(:user_id)

      granted_ids = after_ids - before_ids
      revoked_ids = before_ids - after_ids
      if granted_ids.present? || revoked_ids.present?
        names = User.where(id: granted_ids + revoked_ids).pluck(:id, :name).to_h
        AuditLog.record!(action: "room.membership.change", target: room,
          changes: {
            granted: granted_ids.filter_map { |id| names[id] },
            revoked: revoked_ids.filter_map { |id| names[id] }
          })
      end
    end

    def find_messages
      messages = @room.root_messages.with_rendering_details

      if show_first_message = messages.find_by(id: params[:message_id])
        @messages = messages.page_around(show_first_message)
      else
        @messages = messages.last_page
      end
    end

    def room_params
      params.require(:room).permit(:name, :icon_name)
    end

    def broadcast_remove_room
      broadcast_remove_to :rooms, target: [ @room, :list ]
    end
end
