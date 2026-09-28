module Slack
  # Maps users.list members onto Smartfire users (shared decision 3):
  # active humans match by email or become claimable active placeholders,
  # while deleted members, guests, bots and Slackbot become deactivated
  # placeholders that keep their names on history but can never sign in.
  # Every mapping is recorded as a slack_kind "user" Record so later runs
  # (workspace or personal) reuse it instead of duplicating users.
  class UserMapper
    SLACKBOT_ID = "USLACKBOT"
    SLACKBOT_NAME = "Slackbot"

    def initialize(workspace:, run:)
      @workspace = workspace
      @run = run
    end

    # Maps one users.list page in a single transaction. Returns a stats
    # delta with matched/placeholders/deactivated/bots/total counts.
    def map_page(members)
      members = Array(members)
      delta = fresh_delta
      return delta if members.empty?

      known_keys = existing_keys(members.filter_map { |member| member["id"] })
      fresh = members.reject { |member| member["id"].blank? || known_keys.include?(member["id"]) }
      email_index = matchable_users(fresh)

      SlackImport::Record.transaction do
        fresh.each do |member|
          map_member(member, email_index, delta)
        end
      end
      delta
    end

    # Dry-run preview of one page: the same decisions without writing.
    def preview_page(members)
      members = Array(members)
      delta = fresh_delta
      return delta if members.empty?

      known_keys = existing_keys(members.filter_map { |member| member["id"] })
      email_index = matchable_users(members.reject { |member| known_keys.include?(member["id"]) })

      members.each do |member|
        next if member["id"].blank? || known_keys.include?(member["id"])

        preview_member(member, email_index, delta)
      end
      delta
    end

    # Smartfire users for Slack user ids, through the mapping table.
    def users_for(slack_ids)
      ids = Array(slack_ids).compact_blank.uniq
      return {} if ids.empty?

      user_ids = records_for(ids).pluck(:slack_key, :record_id).to_h
      return {} if user_ids.empty?

      User.where(id: user_ids.values).index_by(&:id).then do |users_by_id|
        user_ids.filter_map do |slack_id, user_id|
          [ slack_id, users_by_id[user_id] ] if users_by_id[user_id]
        end.to_h
      end
    end

    # Safety net for message authors missing from the mapping (members
    # pruned from users.list after mapping): a deactivated placeholder.
    def ensure_author(slack_id)
      users_for([ slack_id ])[slack_id] ||
        record_new_placeholder(slack_id, name: "Unknown Slack user")
    end

    # Bot authors on messages without a `user` field, keyed by bot_id or
    # username so every bot maps once.
    def bot_user_for(message)
      key = self.class.bot_key(message)
      users_for([ key ])[key] ||
        record_new_placeholder(key, name: message["username"].presence || "Slack bot")
    end

    def self.bot_key(message)
      if message["bot_id"].present?
        "bot:#{message["bot_id"]}"
      else
        "botname:#{message["username"].presence || "unknown"}"
      end
    end

    # Display-name fallback chain for previews and unknown users.
    def self.display_name_for(member)
      profile = member["profile"] || {}
      profile["real_name"].presence || profile["display_name"].presence ||
        member["real_name"].presence || member["name"].presence
    end

    private
      def fresh_delta
        { "matched" => 0, "placeholders" => 0, "deactivated" => 0, "bots" => 0, "total" => 0 }
      end

      def existing_keys(slack_ids)
        return Set.new if slack_ids.empty?

        records_for(slack_ids).pluck(:slack_key).to_set
      end

      def records_for(slack_ids)
        SlackImport::Record.where(slack_workspace_id: @workspace.id,
          slack_kind: "user", slack_key: slack_ids)
      end

      # Matchable humans by downcased email, including deactivated users.
      def matchable_users(members)
        emails = members.filter_map do |member|
          email_for(member) unless placeholder_only?(member)
        end.uniq
        return {} if emails.empty?

        User.where("LOWER(email_address) IN (?)", emails.map(&:downcase))
          .order(:id).index_by { |user| user.email_address.to_s.downcase }
      end

      def map_member(member, email_index, delta)
        user, bucket, created = build_mapping(member, email_index)
        record_mapping(member["id"], user, created_record: created)
        count(delta, bucket)
        user
      end

      def preview_member(member, email_index, delta)
        _, bucket, _ = build_mapping(member, email_index, dry_run: true)
        count(delta, bucket)
      end

      def count(delta, bucket)
        delta[bucket] += 1
        delta["total"] += 1
      end

      # Returns [user_or_nil, stats_bucket, created_record].
      def build_mapping(member, email_index, dry_run: false)
        # The connection owner IS their Slack user: OAuth proved it, so the
        # id maps to them even when the emails differ. Without this every
        # run would mint a duplicate placeholder for its own starter.
        if member["id"] == owner_slack_id && owner_user
          return [ owner_user, "matched", false ]
        end

        if bot_like?(member)
          user = dry_run ? nil : create_placeholder(name: bot_name(member), status: :deactivated)
          return [ user, "bots", true ]
        end

        if placeholder_only?(member)
          user = dry_run ? nil : create_placeholder(name: member_name(member), status: :deactivated)
          return [ user, "deactivated", true ]
        end

        email = email_for(member)
        if email.blank?
          user = dry_run ? nil : create_placeholder(name: member_name(member), status: :deactivated)
          @run.record_issue!("warning", "user:#{member["id"]}",
            "Slack user #{member_name(member)} has no email address; imported as deactivated") unless dry_run
          return [ user, "deactivated", true ]
        end

        if (user = email_index[email.downcase])
          return [ user, "matched", false ]
        end

        user = dry_run ? nil : create_active_placeholder(member, email)
        [ user, "placeholders", true ]
      end

      def record_mapping(slack_key, user, created_record:)
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "user", slack_key:, record: user, created_record:)
      end

      def create_active_placeholder(member, email)
        profile = member["profile"] || {}
        user = User.new(
          name: member_name(member),
          email_address: email,
          time_zone: valid_time_zone(member["tz"]),
          bio: profile["title"].presence,
          google_email_link_allowed: link_allowed?(email))
        user.save!
        user
      end

      def create_placeholder(name:, status:)
        user = User.new(name:, status:)
        user.skip_open_room_grant = true
        user.save!
        user
      end

      def record_new_placeholder(slack_key, name:)
        user = create_placeholder(name:, status: :deactivated)
        record_mapping(slack_key, user, created_record: true)
        user
      end

      # Deleted members, guests and bots never match by email: they always
      # become deactivated placeholders.
      def placeholder_only?(member)
        member["deleted"] || member["is_restricted"] || member["is_ultra_restricted"] ||
          bot_like?(member)
      end

      def bot_like?(member)
        member["id"] == SLACKBOT_ID || member["is_bot"] || member["is_app_user"]
      end

      def bot_name(member)
        return SLACKBOT_NAME if member["id"] == SLACKBOT_ID

        member_name(member)
      end

      def member_name(member)
        self.class.display_name_for(member).presence || "Slack user"
      end

      def email_for(member)
        (member.dig("profile", "email").presence || member["email"].presence)&.strip
      end

      def valid_time_zone(name)
        zone = ActiveSupport::TimeZone[name.to_s]
        zone&.name
      end

      # Placeholders are claimable by first Google sign-in only when their
      # domain is allowlisted, mirroring the sign-in check.
      def link_allowed?(email)
        domain = email.to_s.strip.downcase.split("@").last.to_s
        domain.present? && Google::SignIn.allowed_domains.include?(domain)
      end

      def owner_slack_id
        @run.slack_connection&.slack_user_id
      end

      def owner_user
        @run.slack_connection&.user
      end
  end
end
