module Users::SidebarHelper
  # The sidebar row partial for a room outside its home section (the
  # Favourites section mixes every room kind in one list).
  def sidebar_room_partial_for(room)
    if room.direct?
      "users/sidebars/rooms/direct"
    elsif room.stage?
      "users/sidebars/rooms/stage"
    elsif room.voice?
      "users/sidebars/rooms/voice"
    elsif room.board?
      "users/sidebars/rooms/board"
    else
      "users/sidebars/rooms/shared"
    end
  end

  # Data attributes behind the shared room context menu (see
  # room_menu_controller.js), read off the row when it opens. Broadcast
  # renders pass no membership and read the new-room defaults.
  #
  # The delete flag mirrors User#can_delete_room? for the membership's
  # own viewer — never Current.user, which is the actor in broadcast
  # renders — and fails closed without a membership. The server still
  # authorizes every request; the flag only hides the menu item. It
  # compares ids against preloaded associations so sidebar rows never
  # query per row (see the sidebar query-count tests).
  def room_menu_data(room, membership, label: nil)
    viewer = menu_viewer_for(membership)

    {
      menu_categorizable: room.open? || room.closed?,
      menu_favorited: membership&.favorited? || false,
      menu_favorite_position: membership&.favorite_position,
      menu_muted: membership&.involved_in_muted? || false,
      menu_default_involvement: room.default_involvement,
      menu_category_id: membership&.room_category_id,
      menu_can_delete: menu_can_delete?(room, membership, viewer),
      menu_can_leave: membership.present?,
      menu_leave_url: leave_url_for(room),
      menu_open_room: room.open?,
      menu_direct_room: room.direct?,
      menu_room_label: label || room.name
    }
  end

  def sidebar_turbo_frame_tag(src: nil, &)
    turbo_frame_tag :user_sidebar, src: src, target: "_top", data: {
      turbo_permanent: true,
      controller: "rooms-list read-rooms turbo-frame",
      rooms_list_unread_class: "unread",
      action: "presence:present@window->rooms-list#read room:mark-unread@window->rooms-list#markUnread read-rooms:read->rooms-list#read turbo:frame-load->rooms-list#loaded refresh-room:visible@window->turbo-frame#reload".html_safe # otherwise -> is escaped
    }, &
  end

  # In-call huddle participants for all of the current user's rooms, loaded
  # once per render so sidebar rows never query per row. Sidebar stacks take
  # their room's entry as a preloaded `participants:` local; the header stack
  # falls back to HuddleGrant.participants_for when none is given.
  def huddle_participants_by_room_id
    return {} unless Huddle.configured?

    @huddle_participants_by_room_id ||= HuddleGrant.active.in_call
      .where(room_id: Current.user.memberships.select(:room_id))
      .includes(:user)
      .group_by(&:room_id)
      .transform_values { |grants| grants.filter_map(&:user).uniq.sort_by { |user| user.name.downcase } }
  end

  # Members of every direct room of the current user, loaded once per
  # render so DM rows never query per row. Rows exclude the current user
  # themselves in Ruby instead of scoping the association, which would
  # query again even when preloaded.
  def direct_room_members_by_room_id
    @direct_room_members_by_room_id ||= begin
      direct_room_ids = Current.user.memberships.joins(:room).where(room: { type: "Rooms::Direct" }).select(:room_id)
      Membership.where(room_id: direct_room_ids).includes(:user).group_by(&:room_id)
        .transform_values { |room_memberships| room_memberships.map(&:user) }
    end
  end

  private
    # Group DMs keep their own leave endpoint with last-member-destroys
    # semantics; every other room kind leaves through RoomsController.
    def leave_url_for(room)
      room.direct? ? leave_rooms_direct_path(room) : leave_room_path(room)
    end

    # The delete flag's viewer: page renders read Current (no query),
    # per-member broadcasts read the membership's own (already loaded)
    # user, and shared broadcast HTML without a membership gets none.
    def menu_viewer_for(membership)
      return nil if membership.nil?
      return Current.user if membership.user_id == Current.user&.id

      membership.user
    end

    # User#can_delete_room? by id comparison, so menu rows never query:
    # admins first, group DMs admin-only, everything else by creator id.
    # Menu rows are always persisted rooms, so the new-record branch of
    # can_administer? cannot apply here.
    def menu_can_delete?(room, membership, viewer)
      return false if viewer.nil?
      return true if viewer.administrator?
      return false if room.direct? && direct_group_capable?(room, membership)

      room.creator_id == viewer.id
    end

    # Group detection without a per-row COUNT: the sidebar preloads
    # every direct room's members once per render. Single-row renders
    # outside it (group broadcasts) fall back to the model instead.
    def direct_group_capable?(room, membership)
      if membership&.user_id == Current.user&.id && defined?(@direct_room_members_by_room_id) &&
          (members = @direct_room_members_by_room_id[room.id])
        members.size > 2 || room.name.present?
      else
        room.group_capable?
      end
    end
end
