# Rooms for direct message chats between users. These act as a singleton, so a single set of users will
# always refer to the same direct room.
class Rooms::Direct < Room
  # Ad hoc group DMs hold at most this many members, the acting user included.
  MAX_MEMBERS = 10
  # Member first names listed in the default group name before the "+N" remainder.
  DEFAULT_NAME_PREVIEW_COUNT = 3
  # A system note in the last minute suppresses the next rename note.
  RENAME_NOTE_WINDOW = 1.minute

  class OverCapacity < StandardError; end
  class NotAGroup < StandardError; end

  validates :name, length: { maximum: 100 }, allow_nil: true

  class << self
    def find_or_create_for(users)
      find_for(users) || begin
        create_for({}, users: users)
      rescue ActiveRecord::RecordNotUnique
        # A concurrent create won the member key; reuse its room.
        find_for(users) || raise
      end
    end

    # Every direct room, one-to-one or group, is found by the hash of its
    # exact member set. Rooms created before the member key (or whose key
    # was left null) fall back to a scan of unkeyed rooms only, which is
    # normally empty.
    def find_for(users)
      ids = users.pluck(:id)
      alive.directs.find_by(direct_member_key: member_key_for(ids)) || find_unkeyed_for(ids)
    end

    def create_for(attributes, users:)
      list = Array(users)
      super(attributes.merge(direct_member_key: member_key_for(list.map(&:id))), users: list)
    end

    # Stable hash of the exact member set. The migration backfill duplicates
    # this computation in plain SQL; keep the two in sync.
    def member_key_for(user_ids)
      "dm:#{Digest::SHA256.hexdigest(user_ids.map(&:to_i).sort.join(","))}"
    end

    private
      def find_unkeyed_for(ids)
        wanted = ids.map(&:to_i).sort
        alive.directs.where(direct_member_key: nil).includes(:users).detect do |room|
          room.user_ids.sort == wanted
        end
      end
  end

  # A live group: three or more current members. One-to-one history stays
  # private — adding members to a one-to-one DM would expose it to someone
  # new — so groups are always born from the selection path, never by
  # widening a one-to-one conversation.
  def group?
    memberships.size > 2
  end

  # Group administration (rename, add) needs a live group, or a custom name
  # proving the room already lived as one: a group that shrank to two
  # members keeps its identity instead of collapsing into a one-to-one DM.
  def group_capable?
    group? || name.present?
  end

  def default_involvement
    "everything"
  end

  # Display name for direct rooms. A custom group name wins; otherwise the
  # other members' names, with groups previewing first names ("Riel, Jon,
  # Chris +2"). Pass preloaded members to avoid a query per row.
  def direct_display_name(for_user: nil, members: nil)
    return name if name.present?

    list = members ? members.sort_by { |user| user.name.downcase } : users.ordered.to_a
    list -= [ for_user ] if for_user
    return for_user&.name if list.empty?

    if list.one?
      list.first.name
    else
      firsts = list.first(DEFAULT_NAME_PREVIEW_COUNT).map { |user| user.name.split.first }
      remainder = list.size - firsts.size
      remainder.positive? ? "#{firsts.join(", ")} +#{remainder}" : firsts.join(", ")
    end
  end

  # Adds users to a group DM, posting who did it. Raises OverCapacity past
  # the member cap and NotAGroup for a one-to-one DM: widening one would
  # expose its private history, so groups start from the selection path.
  def add_members(users, added_by:)
    raise NotAGroup unless group_capable?

    fresh = Array(users).reject { |user| user_ids.include?(user.id) }
    raise OverCapacity if memberships.size + fresh.size > MAX_MEMBERS
    return [] if fresh.empty?

    transaction do
      memberships.grant_to(fresh)
      post_system_note("added #{fresh.map(&:name).to_sentence} to the group", creator: added_by)
    end
    broadcast_directory_updates!(newcomers: fresh)

    fresh
  end

  # Renames the group, or clears the custom name back to the default when
  # blank. One-to-one DMs always show the other member's name. Rename
  # notes are rate-limited to one per room per minute — a burst of
  # renames still lands the latest name, but only the first note —
  # measured by any recent system note so membership churn in the same
  # window collapses into one line too.
  def rename(new_name, renamed_by:)
    raise NotAGroup unless group_capable?

    clean = new_name.to_s.strip
    transaction do
      update!(name: clean.presence)
      unless recent_system_note?
        if clean.present?
          post_system_note("renamed the group to #{clean}", creator: renamed_by)
        else
          post_system_note("cleared the group name", creator: renamed_by)
        end
      end
    end
    broadcast_directory_updates!
  end

  # Removes a member. The group keeps working for everyone left; the last
  # member out destroys the room instead of leaving an empty one behind.
  # Returns :left or :destroyed so the controller can finish the destroy.
  def leave(user)
    outcome = transaction do
      memberships.find_by!(user_id: user.id).destroy!

      if memberships.exists?
        post_system_note("left the group", creator: user)
        :left
      else
        begin_destroy!
        :destroyed
      end
    end
    broadcast_directory_updates! if outcome == :left

    outcome
  end

  # Recomputes the member-set key after members change. A mutated group
  # whose new set collides with another room's key keeps its own history
  # under a room-suffixed key: only the create/open-from-selection path
  # ever reuses a room, membership changes never merge two rooms.
  def refresh_direct_member_key!
    return if destroyed? || deleted?

    candidate = self.class.member_key_for(memberships.pluck(:user_id))
    candidate = "#{candidate}##{id}" if self.class.alive.directs.where.not(id: id).exists?(direct_member_key: candidate)
    update_column(:direct_member_key, candidate) unless direct_member_key == candidate
  rescue ActiveRecord::RecordNotUnique
    update_column(:direct_member_key, "#{candidate}##{id}")
  end

  private
    def recent_system_note?
      messages.where(system_note: true).where(created_at: RENAME_NOTE_WINDOW.ago..).exists?
    end

    # Re-renders every remaining member's sidebar row and room header with
    # explicit locals, so the broadcast never depends on request state. New
    # members get a prepended row instead; the leaver's row was already
    # removed by their membership destroy.
    def broadcast_directory_updates!(newcomers: [])
      participants = Huddle.configured? ? HuddleGrant.participants_for(self) : []
      fresh_memberships = Membership.where(room_id: id).includes(:user).to_a
      newcomer_ids = newcomers.map(&:id).to_set

      fresh_memberships.each do |membership|
        member = membership.user
        row_members = fresh_memberships.map(&:user).reject { |user| user.id == member.id }
        row_locals = { membership:, members: row_members, participants:, huddleable: Huddle.configured? }

        if newcomer_ids.include?(member.id)
          broadcast_prepend_to member, :rooms, target: :direct_rooms,
            partial: "users/sidebars/rooms/direct", locals: row_locals
        else
          broadcast_replace_to member, :rooms, target: [ self, :list ],
            partial: "users/sidebars/rooms/direct", locals: row_locals
        end

        broadcast_replace_to member, :rooms, target: [ self, :header ],
          partial: "rooms/show/header_identity", locals: { room: self, for_user: member }
      end
    end

    # Notes are quiet system notes (see the contract on Message): they
    # render as one compact centered line with the actor's name, broadcast
    # into open timelines, and skip unread, push, agents, inbox, and
    # search. The note text only — the presentation owns the actor name.
    # Plain Action Text, never Markdown: member and group names render
    # literally instead of being parsed as formatting.
    def post_system_note(text, creator:)
      messages.create!(creator: creator, system_note: true, body: text).tap(&:broadcast_create)
    end
end
