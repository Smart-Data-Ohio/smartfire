class Users::SidebarsController < ApplicationController
  DIRECT_PLACEHOLDERS = 20

  def show
    all_memberships = Current.user.memberships.visible.with_ordered_room.to_a
    # Favourites move out of their sections into Favourites; categorized
    # channels move under their category. Both sort in Ruby over the
    # already-loaded memberships, adding no queries.
    @favorite_memberships = all_memberships.select(&:favorited?)
      .sort_by { |membership| [ membership.favorite_position, membership.id ] }
    rest = all_memberships - @favorite_memberships
    @direct_memberships = extract_direct_memberships(rest)
    @voice_memberships  = rest.select { |m| m.room.voice? }
    @categorized_memberships = rest.select { |m| m.room_category_id.present? }
    @other_memberships  = rest - @direct_memberships - @voice_memberships - @categorized_memberships

    @room_categories = Current.user.room_categories.ordered.to_a

    @direct_placeholder_users = find_direct_placeholder_users

    preload_stage_streams(@other_memberships + @favorite_memberships)
  end

  private
    # The stage rows read room.live_stream for the live dot. Load every
    # stage room's live row (plus its presenter) once instead of one query
    # per row. Only Stage rooms carry the association, so only they are
    # preloaded; venues use the same call in Rooms::EventsController.
    def preload_stage_streams(memberships)
      stage_rooms = memberships.filter_map { |membership| membership.room if membership.room.stage? }
      ActiveRecord::Associations::Preloader.new(records: stage_rooms, associations: { live_streams: :user }).call
    end

    def extract_direct_memberships(all_memberships)
      all_memberships.select { |m| m.room.direct? }.sort_by { |m| m.room.updated_at }.reverse
    end

    def find_direct_placeholder_users
      exclude_user_ids = user_ids_already_in_direct_rooms_with_current_user.including(Current.user.id)
      User.active.where.not(id: exclude_user_ids).order(:created_at).limit([ DIRECT_PLACEHOLDERS - exclude_user_ids.count, 0 ].max)
    end

    def user_ids_already_in_direct_rooms_with_current_user
      Membership.where(room_id: Current.user.rooms.directs.pluck(:id)).pluck(:user_id).uniq
    end
end
