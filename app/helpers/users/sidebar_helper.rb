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
  def room_menu_data(room, membership)
    {
      menu_categorizable: room.open? || room.closed?,
      menu_favorited: membership&.favorited? || false,
      menu_favorite_position: membership&.favorite_position,
      menu_muted: membership&.involved_in_muted? || false,
      menu_default_involvement: room.default_involvement,
      menu_category_id: membership&.room_category_id
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

  # Two-person DM ids for the current user, counted once: only those DMs can
  # huddle, so only their rows render a stack. One grouped count instead of a
  # COUNT per row.
  def two_person_direct_room_ids
    return [].to_set unless Huddle.configured?

    @two_person_direct_room_ids ||= begin
      direct_room_ids = Current.user.memberships.joins(:room).where(room: { type: "Rooms::Direct" }).select(:room_id)
      Membership.where(room_id: direct_room_ids).group(:room_id).count.select { |_, count| count == 2 }.keys.to_set
    end
  end
end
