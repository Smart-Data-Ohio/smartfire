module Slack
  # Resolves one Slack conversation to its Smartfire room: public channels
  # to Open rooms, private channels to Closed rooms, ims to one-to-one
  # Direct rooms, and group DMs to Direct or Closed rooms by size. Channels
  # merge into an alive same-type room with the same name unless
  # room_targets says otherwise; memberships are only ever written in rooms
  # the import created.
  class ConversationMapper
    Result = Data.define(:action, :room, :created_record, :skip_reason)

    def initialize(workspace:, run:)
      @workspace = workspace
      @run = run
    end

    def self.conversation_type(conversation)
      if conversation["is_im"]
        "im"
      elsif conversation["is_mpim"]
        "mpim"
      elsif conversation["is_private"]
        "private_channel"
      else
        "public_channel"
      end
    end

    # Import-mode resolution: creates or merges the room, writes memberships
    # for created rooms, and records the conversation mapping, all in one
    # transaction. member_ids are Slack user ids; users maps the known ones
    # to Smartfire users.
    def resolve(conversation, member_ids:, users:)
      target = room_target_for(conversation["id"])
      return skip("skipped by room target") if target == "skip"

      type = self.class.conversation_type(conversation)
      if type == "im"
        return resolve_im(conversation, target, member_ids:, users:)
      elsif type == "mpim"
        return resolve_mpim(conversation, target, member_ids:, users:)
      end

      if target.is_a?(Integer)
        return resolve_room_target(conversation, target)
      end

      resolve_channel(conversation, type, member_ids:, users:, force_new: target == "new")
    end

    # Dry-run preview: the same target without writing anything.
    def preview(conversation, member_ids:, users:)
      target = room_target_for(conversation["id"])
      return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "skipped") if target == "skip"

      type = self.class.conversation_type(conversation)
      if type == "im"
        return preview_im(conversation, target, member_ids:, users:)
      elsif type == "mpim"
        return preview_mpim(conversation, target, member_ids:, users:)
      end

      if target.is_a?(Integer)
        room = alive_channel_room(target)
        if room.nil?
          return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "invalid room target")
        end
        return Result.new(action: "merge", room:, created_record: false, skip_reason: nil)
      end

      name = channel_room_name(conversation)
      room = target == "new" ? nil : merge_room(channel_room_type(conversation), name)
      Result.new(action: room ? "merge" : "create", room:, created_record: room.nil?, skip_reason: nil)
    end

    # Stand-in member for dry-run previews, which resolve targets without
    # writing users. Negative ids never match a real room's member key, so
    # previews without an existing room read as "create".
    DryUser = Struct.new(:id, :name)

    def self.dry_users_for(member_ids)
      Array(member_ids).each_with_index.map do |slack_id, index|
        [ slack_id, DryUser.new(-(index + 1), "") ]
      end.to_h
    end

    # Room name a conversation would map to, for stats before resolution.
    def describe_target(conversation, member_count: 0)
      type = self.class.conversation_type(conversation)
      if type == "mpim" && member_count > Rooms::Direct::MAX_MEMBERS
        conversation["name"].presence || "Group DM"
      elsif type == "im"
        "Direct message"
      elsif type == "mpim"
        "Group DM"
      else
        channel_room_name(conversation)
      end
    end

    private
      def skip(reason)
        Result.new(action: "skip", room: nil, created_record: false, skip_reason: reason)
      end

      def room_target_for(conversation_id)
        targets = @run.options["room_targets"]
        return nil unless targets.is_a?(Hash)

        value = targets[conversation_id]
        return value if value == "new" || value == "skip"
        return value.to_i if value.to_s.match?(/\A\d+\z/)

        nil
      end

      # -- channels -----------------------------------------------------

      def channel_room_type(conversation)
        conversation["is_private"] ? "Rooms::Closed" : "Rooms::Open"
      end

      def channel_room_name(conversation)
        name = conversation["name"].presence || "slack-channel"
        conversation["is_archived"] ? "#{name} (archived)" : name
      end

      def merge_room(type, name)
        Room.alive.where(type:).where("LOWER(name) = ?", name.downcase).order(:id).first
      end

      def alive_channel_room(id)
        room = Room.alive.find_by(id:)
        room if room.is_a?(Rooms::Open) || room.is_a?(Rooms::Closed)
      end

      def resolve_channel(conversation, type, member_ids:, users:, force_new:)
        room_class = channel_room_type(conversation).constantize
        name = channel_room_name(conversation)
        archived = conversation["is_archived"]

        room = nil
        SlackImport::Record.transaction do
          room = force_new ? nil : merge_room(room_class.sti_name, name)
          if room
            record_conversation(conversation["id"], room, created_record: false)
            return Result.new(action: "merge", room:, created_record: false, skip_reason: nil)
          end

          room = room_class.create!(name:, creator: @run.user)
          grant_missing_members(room, member_ids:, users:)
          record_conversation(conversation["id"], room, created_record: true)
        end

        # The Open-room grant of every active user lands in an
        # after_save_commit, so invisibility and membership records apply
        # only once the creation transaction has committed.
        apply_channel_involvement(room, member_ids:, users:, archived:)
        record_memberships(conversation["id"], room, users, member_ids)
        Result.new(action: "create", room:, created_record: true, skip_reason: nil)
      end

      def resolve_room_target(conversation, room_id)
        room = alive_channel_room(room_id)
        if room.nil?
          @run.record_issue!("error", "channel:#{conversation["id"]}",
            "Room target #{room_id} for ##{conversation["name"]} is not an alive Open or Closed room; skipped")
          return skip("invalid room target")
        end

        record_conversation(conversation["id"], room, created_record: false)
        Result.new(action: "merge", room:, created_record: false, skip_reason: nil)
      end

      # Slack members missing from a room the import created (deactivated
      # members on Open rooms, everyone on Closed rooms) get the room
      # default. Never called for merged rooms.
      def grant_missing_members(room, member_ids:, users:)
        member_users = member_ids.filter_map { |id| users[id] }.uniq(&:id)
        existing_ids = room.memberships.where(user_id: member_users.map(&:id)).pluck(:user_id).to_set
        fresh = member_users.reject { |user| existing_ids.include?(user.id) }
        room.memberships.grant_to(fresh) if fresh.any?
      end

      # Everyone else in a created Open room goes invisible; archived rooms
      # go fully invisible. Runs after commit (see resolve_channel).
      def apply_channel_involvement(room, member_ids:, users:, archived:)
        invisible_ids = if archived
          room.memberships.pluck(:user_id)
        elsif room.open?
          slack_ids = member_ids.filter_map { |id| users[id]&.id }.to_set
          room.memberships.pluck(:user_id).reject { |id| slack_ids.include?(id) }
        else
          []
        end
        room.memberships.where(user_id: invisible_ids).update_all(involvement: :invisible) if invisible_ids.any?
      end

      def record_memberships(conversation_id, room, users, member_ids)
        # Map Smartfire user ids back to Slack ids for stable keys; users
        # the Open-room grant pulled in get user-based keys instead.
        slack_by_user_id = users.invert.transform_keys(&:id)
        member_set = member_ids.to_set
        timestamp = Time.current
        record_rows = room.memberships.pluck(:id, :user_id).map do |membership_id, user_id|
          slack_id = slack_by_user_id[user_id]
          key = if slack_id && member_set.include?(slack_id)
            "#{conversation_id}:#{slack_id}"
          else
            "#{conversation_id}:user-#{user_id}"
          end
          { slack_workspace_id: @workspace.id, slack_import_id: @run.id,
            slack_kind: "membership", slack_key: key,
            record_type: "Membership", record_id: membership_id,
            created_record: true, created_at: timestamp, updated_at: timestamp }
        end
        SlackImport::Record.insert_all(record_rows) if record_rows.any?
      end

      def record_conversation(conversation_id, room, created_record:)
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "conversation", slack_key: conversation_id,
          record: room, created_record:)
      end

      # -- DMs ----------------------------------------------------------

      def dm_peer_ids(conversation, member_ids:, users:)
        others = member_ids - [ @run.slack_connection&.slack_user_id ]
        if others.empty? && conversation["user"].present? &&
            conversation["user"] != @run.slack_connection&.slack_user_id
          others = [ conversation["user"] ]
        end
        others
      end

      def resolve_im(conversation, target, member_ids:, users:)
        peers = dm_peer_ids(conversation, member_ids:, users:)
        if target.is_a?(Integer)
          @run.record_issue!("error", "channel:#{conversation["id"]}",
            "Room targets only apply to channels; this DM keeps its own Direct room")
          return resolve_im(conversation, nil, member_ids:, users:)
        end
        return skip("self DM") if peers.empty?

        peer_users = peers.filter_map { |id| users[id] }
        return skip("DM peer is not mapped") if peer_users.empty?

        if peers.include?(Slack::UserMapper::SLACKBOT_ID)
          return skip("Slackbot DM")
        end

        members = ([ @run.user ] + peer_users).uniq(&:id)
        return skip("self DM") if members.one?

        room = nil
        created = false
        SlackImport::Record.transaction do
          room = Current.set(user: @run.user) { Rooms::Direct.find_or_create_for(members) }
          created = room.previously_new_record?
          record_conversation(conversation["id"], room, created_record: created)
          record_dm_memberships(conversation["id"], room) if created
        end
        Result.new(action: created ? "create" : "merge", room:, created_record: created, skip_reason: nil)
      end

      def preview_im(conversation, target, member_ids:, users:)
        peers = dm_peer_ids(conversation, member_ids:, users:)
        return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "self DM") if peers.empty?
        return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "Slackbot DM") if peers.include?(Slack::UserMapper::SLACKBOT_ID)

        peer_users = peers.filter_map { |id| users[id] }
        return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "DM peer not found") if peer_users.empty?

        members = ([ @run.user ] + peer_users).uniq(&:id)
        return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "self DM") if members.one?

        room = Rooms::Direct.find_for(members)
        Result.new(action: room ? "merge" : "create", room:, created_record: room.nil?, skip_reason: nil)
      end

      def resolve_mpim(conversation, target, member_ids:, users:)
        if target.is_a?(Integer)
          @run.record_issue!("error", "channel:#{conversation["id"]}",
            "Room targets only apply to channels; this group DM keeps its own room")
          return resolve_mpim(conversation, nil, member_ids:, users:)
        end

        members = member_ids.filter_map { |id| users[id] }.uniq(&:id)
        if members.size < 2
          @run.record_issue!("warning", "channel:#{conversation["id"]}",
            "Group DM #{conversation["name"] || conversation["id"]} has fewer than 2 mapped members; skipped")
          return skip("too few members")
        end

        SlackImport::Record.transaction do
          if members.size <= Rooms::Direct::MAX_MEMBERS
            room = Current.set(user: @run.user) { Rooms::Direct.find_or_create_for(members) }
            created = room.previously_new_record?
            record_conversation(conversation["id"], room, created_record: created)
            record_dm_memberships(conversation["id"], room) if created
            Result.new(action: created ? "create" : "merge", room:, created_record: created, skip_reason: nil)
          else
            name = mpim_closed_name(conversation, users)
            room = merge_room("Rooms::Closed", name)
            if room
              record_conversation(conversation["id"], room, created_record: false)
              Result.new(action: "merge", room:, created_record: false, skip_reason: nil)
            else
              room = Rooms::Closed.create!(name:, creator: @run.user)
              room.memberships.grant_to(members)
              record_membership_rows(conversation["id"], room)
              record_conversation(conversation["id"], room, created_record: true)
              Result.new(action: "create", room:, created_record: true, skip_reason: nil)
            end
          end
        end
      end

      def preview_mpim(conversation, target, member_ids:, users:)
        members = member_ids.filter_map { |id| users[id] }.uniq(&:id)
        if members.size < 2
          return Result.new(action: "skip", room: nil, created_record: false, skip_reason: "too few members")
        end

        if members.size <= Rooms::Direct::MAX_MEMBERS
          room = Rooms::Direct.find_for(members)
          Result.new(action: room ? "merge" : "create", room:, created_record: room.nil?, skip_reason: nil)
        else
          room = merge_room("Rooms::Closed", mpim_closed_name(conversation, users))
          Result.new(action: room ? "merge" : "create", room:, created_record: room.nil?, skip_reason: nil)
        end
      end

      def mpim_closed_name(conversation, users)
        names = users.values.map(&:name).reject(&:blank?).sort_by(&:downcase)
        if names.empty?
          conversation["name"].presence || "Group DM"
        elsif names.size <= 4
          names.join(", ")
        else
          "#{names.first(3).join(", ")} +#{names.size - 3}"
        end
      end

      def record_dm_memberships(conversation_id, room)
        record_membership_rows(conversation_id, room)
      end

      def record_membership_rows(conversation_id, room)
        timestamp = Time.current
        rows = room.memberships.pluck(:id, :user_id).map do |membership_id, user_id|
          { slack_workspace_id: @workspace.id, slack_import_id: @run.id,
            slack_kind: "membership", slack_key: "#{conversation_id}:user-#{user_id}",
            record_type: "Membership", record_id: membership_id,
            created_record: true, created_at: timestamp, updated_at: timestamp }
        end
        SlackImport::Record.insert_all(rows) if rows.any?
      end
  end
end
