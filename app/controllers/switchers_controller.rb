# One JSON endpoint behind the Ctrl/Cmd+K quick switcher: the user's
# accessible rooms, the people they can open a DM with, and recent
# threads across their rooms. The client fuzzy-filters and ranks
# recents locally, so this serves the full scoped lists in a constant
# number of queries.
class SwitchersController < ApplicationController
  RECENT_THREAD_LIMIT = 15

  def show
    respond_to do |format|
      format.json do
        render json: {
          rooms: switcher_rooms,
          people: switcher_people,
          threads: switcher_threads
        }
      end
    end
  end

  private
    def switcher_rooms
      memberships = Current.user.memberships.visible.includes(:room).with_ordered_room.to_a
      members_by_room_id = direct_members_by_room_id(memberships.filter_map { |membership| membership.room_id if membership.room.direct? })

      memberships.map do |membership|
        room = membership.room
        {
          id: room.id,
          name: switcher_room_name(room, members_by_room_id),
          kind: switcher_room_kind(room, members_by_room_id),
          url: room_path(room),
          icon_name: room.icon_name,
          unread: membership.unread?,
          muted: membership.involved_in_muted?,
          favorite: membership.favorited?
        }
      end
    end

    # Full names for direct rooms, computed from one preloaded lookup
    # instead of room_display_name's query per room.
    def direct_members_by_room_id(room_ids)
      return {} if room_ids.empty?

      Membership.where(room_id: room_ids).includes(:user).group_by(&:room_id)
        .transform_values { |room_memberships| room_memberships.map(&:user) }
    end

    def switcher_room_name(room, members_by_room_id)
      return room.name unless room.direct?

      names = members_by_room_id.fetch(room.id, []).reject { |user| user.id == Current.user.id }.map(&:name)
      names.to_sentence.presence || Current.user.name
    end

    def switcher_room_kind(room, members_by_room_id)
      if room.direct?
        members_by_room_id.fetch(room.id, []).size > 2 ? "group" : "dm"
      elsif room.voice?
        "voice"
      elsif room.stage?
        "stage"
      elsif room.board?
        "board"
      else
        "channel"
      end
    end

    def switcher_people
      users = User.active.without_bots.where.not(id: Current.user.id).order(:name).to_a
      dm_url_by_user_id = dm_url_by_user_id(users.map(&:id))

      users.map do |user|
        {
          id: user.id,
          name: user.name,
          avatar_url: helpers.fresh_user_avatar_url(user),
          dm_url: dm_url_by_user_id[user.id]
        }
      end
    end

    # Existing two-person DM room URL per user id, so the switcher
    # navigates straight there instead of creating a duplicate room.
    def dm_url_by_user_id(user_ids)
      return {} if user_ids.empty?

      direct_room_ids = Current.user.memberships.joins(:room).where(room: { type: "Rooms::Direct" }).select(:room_id)
      two_person_ids = Membership.where(room_id: direct_room_ids).group(:room_id).having("COUNT(*) = 2").count.keys
      return {} if two_person_ids.empty?

      Membership.where(room_id: two_person_ids, user_id: user_ids).where.not(user_id: Current.user.id)
        .pluck(:room_id, :user_id).to_h { |room_id, user_id| [ user_id, room_path(room_id) ] }
    end

    def switcher_threads
      ChannelThread.joins(:room).merge(Room.alive).where(room_id: Current.user.rooms.select(:id))
        .includes(:room).order(last_activity_at: :desc).limit(RECENT_THREAD_LIMIT).map do |thread|
        {
          id: thread.id,
          name: thread.name,
          room_name: thread.room.name,
          room_id: thread.room_id,
          url: room_url(thread.room, thread: thread.id)
        }
      end
    end
end
