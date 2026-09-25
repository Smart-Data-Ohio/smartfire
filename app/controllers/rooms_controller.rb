class RoomsController < ApplicationController
  before_action :set_room, only: %i[ show destroy leave ]
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
    # A non-member opening an open room's URL sees the join page instead
    # of the redirect: open rooms stay discoverable by link after leaving.
    return render :join if @join_preview

    @messages = Message::MentionPreloader.preload_for(find_messages)
    set_unread_divider unless @room.board?
    # The other DM members for the out-of-office notice above the
    # composer, rendered per viewer and never fragment-cached.
    @ooo_notice_members = @room.direct? ?
      @room.users.active.without_bots.where.not(id: Current.user.id).includes(:meeting_cache).ordered.to_a : []
  end

  def destroy
    room_label = @room.name
    Room.transaction { @room.begin_destroy! }
    enqueue_destroy
    AuditLog.record!(action: "room.destroy", target: @room, target_label: room_label,
      changes: { name: room_label })

    broadcast_remove_room

    # The sidebar menu deletes through fetch: like the involvement
    # toggle, JSON takes no redirect.
    respond_to do |format|
      format.html { redirect_to root_url, notice: ("Deleted ##{room_label}" if room_label.present?) }
      format.json { render json: { deleted: true, room_id: @room.id } }
    end
  end

  # Leaves the room without deleting it, even when the last member
  # leaves; only group DMs keep their destroy-the-empty-room semantics.
  # Rooms::DirectsController inherits this for its own leave route.
  def leave
    room_label = @room.name

    if @room.direct?
      if @room.leave(Current.user) == :destroyed
        AuditLog.record!(action: "room.destroy", target: @room, target_label: room_label,
          changes: { name: room_label })
        enqueue_destroy
        broadcast_remove_room
      else
        record_self_leave
      end
    else
      @room.memberships.find_by!(user_id: Current.user.id).destroy!
      record_self_leave
    end

    respond_to do |format|
      format.html { redirect_to root_url }
      format.json { render json: { left: true, room_id: @room.id } }
    end
  rescue ActiveRecord::RecordNotDestroyed
    respond_to do |format|
      format.html { redirect_to room_url(@room), alert: "Couldn't leave #{room_label}." }
      format.json { render json: { error: "Couldn't leave this room." }, status: :unprocessable_entity }
    end
  end

  # Rejoins an open room after leaving it. Private rooms need someone to
  # re-add the member, and every other room kind refuses.
  def join
    room = Room.alive.opens.find_by(id: params[:id])

    if room.nil?
      redirect_to root_url, alert: "Room not found or inaccessible"
    elsif Current.user.memberships.exists?(room_id: room.id)
      redirect_to room_url(room)
    else
      membership = room.memberships.create!(user: Current.user, involvement: room.default_involvement)
      broadcast_prepend_to Current.user, :rooms, target: :shared_rooms,
        partial: "users/sidebars/rooms/shared", locals: { room: room, membership: membership, unread: false }
      redirect_to room_url(room)
    end
  rescue ActiveRecord::RecordNotUnique
    # A double submit raced the first join: the membership exists now.
    redirect_to room_url(room)
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
      elsif action_name == "show" && (joinable = joinable_open_room)
        @room = joinable
        @join_preview = true
      else
        redirect_to root_url, alert: "Room not found or inaccessible"
      end
    end

    # An alive open room the viewer is not a member of. Only show falls
    # back to it; destroy and leave stay member-scoped above.
    def joinable_open_room
      Room.alive.opens.find_by(id: params[:room_id] || params[:id])
    end

    # Subclasses narrow this to the room types they're allowed to act on, so that one
    # room namespace can't be used to reach another's rooms.
    def room_scope
      Current.user.rooms
    end

    def ensure_can_administer
      head :forbidden unless Current.user.can_delete_room?(@room)
    end

    # A member leaving under their own power: the actor is the leaver,
    # and the change shape matches the admin membership revision above.
    def record_self_leave
      AuditLog.record!(action: "room.membership.change", target: @room,
        changes: { revoked: [ Current.user.name ] })
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

      @thread_reply_counts = ChannelThread.board_reply_counts(@messages.filter_map(&:channel_thread))
      @messages
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
      return unless membership&.unread?

      first_unread = membership.first_unread_message
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
