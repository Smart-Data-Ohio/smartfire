class RoomsController < ApplicationController
  before_action :set_room, only: %i[ show destroy ]
  before_action :ensure_can_administer, only: %i[ destroy ]
  before_action :remember_last_room_visited, only: :show

  def index
    redirect_to room_url(Current.user.rooms.last)
  end

  # Messages beyond this many unread scroll the room to the "New
  # messages" divider on open; fewer unread keep today's scroll to the
  # bottom with the divider above them.
  UNREAD_DIVIDER_SCROLL_THRESHOLD = 5

  def show
    @messages = Message::MentionPreloader.preload_for(find_messages)
    set_unread_divider unless @room.board?
  end

  def destroy
    Room.transaction { @room.begin_destroy! }
    enqueue_destroy

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

    def find_messages
      messages = @room.root_messages.with_rendering_details

      if show_first_message = messages.find_by(id: params[:message_id])
        @messages = messages.page_around(show_first_message)
      else
        @messages = messages.last_page
      end
    end

    # Locates the "New messages" divider for the current membership. The
    # room always opens on its usual page (last, or anchored around the
    # requested message); when the first unread message is on it, the
    # view renders the divider above it and scrolls to it when the
    # unread count is large. When the first unread fell off the page,
    # the jump pill links to it instead of re-paging the room out from
    # under the last-page contract other pages rely on.
    def set_unread_divider
      membership = Current.user.memberships.find_by(room_id: @room.id)
      first_unread = membership&.first_unread_message
      return if first_unread.nil?

      @unread_count = membership.unread_count_from(first_unread)

      if @messages.any? { |message| message.id == first_unread.id }
        @unread_divider_message_id = first_unread.id
        @scroll_to_unread_divider = true if @unread_count > UNREAD_DIVIDER_SCROLL_THRESHOLD
      else
        @jump_to_unread_url = room_path(@room, message_id: first_unread.id)
      end
    end

    def room_params
      params.require(:room).permit(:name, :icon_name)
    end

    def broadcast_remove_room
      broadcast_remove_to :rooms, target: [ @room, :list ]
    end
end
