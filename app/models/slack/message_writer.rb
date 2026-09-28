module Slack
  # Writes one history or replies page to Smartfire records: messages with
  # converted markdown, boosts for reactions, pins, thread rows and
  # followers, plus every SlackImport::Record row, all in one transaction
  # so a crash never leaves unmapped records. Touches are disabled while
  # writing; the runner stamps room timestamps explicitly at the end.
  class MessageWriter
    # Join/leave, topic, purpose, rename, archive and pin notices, bot
    # membership notices, tombstones, deletions and huddle threads carry
    # no conversation content. thread_broadcast is skipped in history only:
    # the same message is kept once, as the thread reply.
    SKIPPED_SUBTYPES = %w[
      channel_join channel_leave channel_topic channel_purpose channel_name
      channel_archive channel_unarchive
      group_join group_leave group_topic group_purpose group_name
      group_archive group_unarchive
      pinned_item bot_add bot_remove tombstone message_deleted huddle_thread
    ].freeze
    HISTORY_SKIPPED_SUBTYPES = (SKIPPED_SUBTYPES + %w[ thread_broadcast ]).freeze

    def initialize(workspace:, run:, user_mapper:)
      @workspace = workspace
      @run = run
      @user_mapper = user_mapper
    end

    # Writes one conversations.history page of root messages. bounds holds
    # :oldest/:latest epoch floats (or nil). users maps Slack ids to
    # Smartfire users and is extended with on-demand fallbacks. Returns
    # counts plus the thread parents found on this page.
    def write_history_page(room:, conversation_id:, messages:, bounds:, users:)
      counts = fresh_counts
      thread_parents = []

      enrich_users_with_mentions!(users, messages)
      known = existing_message_keys(conversation_id, messages)
      queue_known_parents(conversation_id, messages, bounds, known, thread_parents)
      writable = messages.filter_map do |message|
        prepare_root(room, conversation_id, message, bounds:, users:, known:, counts:)
      end
      return { counts:, thread_parents: } if writable.empty?

      ActiveRecord::Base.no_touching do
        Message.transaction do
          writable.each do |planned|
            created = create_message!(room, planned, thread: nil, reply_to: nil)
            counts["messages"] += 1
            counts["files_linked"] += planned[:files_linked]
            record_truncation(conversation_id, planned)
            write_reactions(room, conversation_id, created, planned[:source], users:, counts:)
            write_pin(room, conversation_id, created, planned[:source], counts:)
            if parent_in_scope?(planned[:source], bounds)
              thread_parents << { "ts" => planned[:ts], "message_id" => created.id }
            end
          end
        end
      end

      { counts:, thread_parents: }
    end

    # Writes one conversations.replies page. In Direct rooms replies flatten
    # into ordinary messages linked to the parent; elsewhere they join a
    # ChannelThread created for the parent. thread_state tracks
    # {"thread_id" => id or nil} across pages of one thread.
    def write_replies_page(room:, conversation_id:, parent_ts:, parent_message_id:,
        messages:, bounds:, users:, direct:, thread_state:)
      counts = fresh_counts

      enrich_users_with_mentions!(users, messages)
      known = existing_message_keys(conversation_id, messages)
      replies = messages.filter_map do |message|
        next if message["ts"] == parent_ts # the echoed parent
        prepare_reply(room, conversation_id, message, bounds:, users:, known:, counts:)
      end

      ActiveRecord::Base.no_touching do
        Message.transaction do
          replies.each do |planned|
            if direct
              created = create_message!(room, planned,
                thread: nil, reply_to: parent_message_id)
            else
              thread = ensure_thread(room, conversation_id, parent_ts, parent_message_id, thread_state)
              created = create_message!(room, planned, thread:, reply_to: nil)
              counts["threads"] += 1 if thread_state["created_now"]
              thread_state["created_now"] = false
              follow_thread(thread, planned[:author], conversation_id:, parent_ts:)
            end
            counts["replies"] += 1
            counts["files_linked"] += planned[:files_linked]
            record_truncation(conversation_id, planned)
            write_reactions(room, conversation_id, created, planned[:source], users:, counts:)
            write_pin(room, conversation_id, created, planned[:source], counts:)
          end
        end
      end

      counts
    end

    # Dry-run scan of one history page: counts what an import would write
    # without writing anything, and converts up to samples_remaining
    # samples. Threads and replies come from reply_count; replies are never
    # read on a dry run.
    def dry_history_page(conversation_name:, messages:, bounds:, samples_remaining:)
      counts = fresh_counts
      samples = []
      users = @user_mapper.users_for(mentioned_ids(Array(messages)))

      Array(messages).each do |original|
        message = unwrap(original)
        ts = message["ts"].to_s
        unless ts.present? && !HISTORY_SKIPPED_SUBTYPES.include?(message["subtype"].to_s) &&
            in_bounds?(ts, bounds)
          counts["skipped"] += 1
          next
        end

        converted = Slack::MarkdownConverter.convert(message, users: display_names(users))
        if converted.markdown.strip.blank?
          counts["skipped"] += 1
          next
        end

        counts["messages"] += 1
        counts["files_linked"] += converted.files_linked
        if message["reply_count"].to_i.positive?
          counts["threads"] += 1
          counts["replies"] += message["reply_count"].to_i
        end
        if samples_remaining > samples.size
          samples << { "conversation" => conversation_name,
            "slack_text" => message["text"].to_s.truncate(200),
            "markdown" => converted.markdown.truncate(500) }
        end
      end

      { counts:, samples: }
    end

    # Finishes a thread after its last replies page: recomputes the reply
    # count once and stamps last_activity_at to the last reply time (or the
    # parent time when no reply was kept).
    def finish_thread(conversation_id, parent_ts)
      record = SlackImport::Record.find_by(slack_workspace_id: @workspace.id,
        slack_kind: "thread", slack_key: "#{conversation_id}:#{parent_ts}")
      return if record.nil?

      thread = ChannelThread.find_by(id: record.record_id)
      return if thread.nil?

      ChannelThread.refresh_messages_count(thread.id)
      last_reply_at = Message.where(thread_id: thread.id).maximum(:created_at) ||
        thread.parent_message&.created_at || thread.created_at
      thread.update_columns(last_activity_at: last_reply_at)
    end

    private
      def fresh_counts
        { "messages" => 0, "replies" => 0, "threads" => 0, "reactions" => 0,
          "pins" => 0, "files_linked" => 0, "skipped" => 0 }
      end

      def existing_message_keys(conversation_id, messages)
        keys = messages.filter_map { |message| message_key(conversation_id, message["ts"]) }
        return Set.new if keys.empty?

        SlackImport::Record.where(slack_workspace_id: @workspace.id,
          slack_kind: "message", slack_key: keys).pluck(:slack_key).to_set
      end

      def message_key(conversation_id, ts)
        "#{conversation_id}:#{ts}"
      end

      # Catch-up re-reads threads whose parents an earlier run imported:
      # the replies dedupe by key, so only late replies are created.
      def queue_known_parents(conversation_id, messages, bounds, known, thread_parents)
        candidates = messages.filter_map do |message|
          message = unwrap(message)
          ts = message["ts"].to_s
          next unless ts.present? && message["reply_count"].to_i.positive? &&
            in_bounds?(ts, bounds) && !HISTORY_SKIPPED_SUBTYPES.include?(message["subtype"].to_s)
          next unless known.include?(message_key(conversation_id, ts))

          ts
        end
        return if candidates.empty?

        rows = SlackImport::Record.where(slack_workspace_id: @workspace.id,
          slack_kind: "message",
          slack_key: candidates.map { |ts| message_key(conversation_id, ts) })
          .pluck(:slack_key, :record_id).to_h
        alive = Message.where(id: rows.values).pluck(:id).to_set
        candidates.each do |ts|
          id = rows[message_key(conversation_id, ts)]
          thread_parents << { "ts" => ts, "message_id" => id } if id && alive.include?(id)
        end
      end

      def prepare_root(room, conversation_id, message, bounds:, users:, known:, counts:)
        prepare(room, conversation_id, message, HISTORY_SKIPPED_SUBTYPES,
          bounds:, users:, known:, counts:)
      end

      def prepare_reply(room, conversation_id, message, bounds:, users:, known:, counts:)
        prepare(room, conversation_id, message, SKIPPED_SUBTYPES,
          bounds:, users:, known:, counts:)
      end

      def prepare(room, conversation_id, message, skipped_subtypes, bounds:, users:, known:, counts:)
        message = unwrap(message)
        ts = message["ts"].to_s
        return skip(counts) if ts.blank? || known.include?(message_key(conversation_id, ts))
        return skip(counts) if skipped_subtypes.include?(message["subtype"].to_s)
        return skip(counts) unless in_bounds?(ts, bounds)

        author = resolve_author(message, users)
        converted = Slack::MarkdownConverter.convert(message, users: display_names(users))
        return skip(counts) if converted.markdown.strip.blank?

        { ts:, key: message_key(conversation_id, ts), author:, source: message,
          markdown: converted.markdown, truncated: converted.truncated,
          files_linked: converted.files_linked, created_at: Time.at(Rational(ts.to_s)),
          edited_at: edited_at_for(message) }
      end

      def skip(counts)
        counts["skipped"] += 1
        nil
      end

      # message_changed wrappers carry the real message nested.
      def unwrap(message)
        if message["subtype"] == "message_changed" && message["message"].is_a?(Hash)
          message["message"].merge("ts" => message["message"]["ts"].presence || message["ts"])
        else
          message
        end
      end

      def in_bounds?(ts, bounds)
        # Rational, not Float: doubles cannot hold every microsecond past
        # the epoch exactly, and a message on its bound must compare true.
        seconds = Rational(ts.to_s)
        (bounds[:oldest].nil? || seconds >= bounds[:oldest]) &&
          (bounds[:latest].nil? || seconds <= bounds[:latest])
      end

      # A thread is only read when its parent is in scope: reply_count > 0
      # and the parent's own ts inside the bounds.
      def parent_in_scope?(message, bounds)
        message["reply_count"].to_i.positive? && in_bounds?(message["ts"].to_s, bounds)
      end

      def resolve_author(message, users)
        if message["user"].present?
          users[message["user"]] ||= @user_mapper.ensure_author(message["user"])
        else
          @user_mapper.bot_user_for(message)
        end
      end

      def display_names(users)
        users.transform_values(&:name)
      end

      # Mentions of any mapped workspace user render as @[Name], even when
      # that user is not a channel member (the renderer leaves non-member
      # tokens as text); only truly unknown ids fall back to @label. One
      # lookup per page for the mentioned ids missing from the member map.
      def enrich_users_with_mentions!(users, messages)
        missing = mentioned_ids(messages) - users.keys
        users.merge!(@user_mapper.users_for(missing)) if missing.any?
      end

      def mentioned_ids(messages)
        Array(messages).flat_map do |original|
          message = unwrap(original)
          parts = [ message["text"] ]
          Array(message["attachments"]).each do |attachment|
            parts.concat([ attachment["pretext"], attachment["text"], attachment["fallback"] ])
          end
          parts.join("\n").scan(/<@([A-Z0-9]+)(?:\|[^>]+)?>/)
        end.flatten.uniq
      end

      def edited_at_for(message)
        ts = message.dig("edited", "ts") || message["edited_ts"]
        Time.at(Rational(ts.to_s)) if ts.present?
      end

      def create_message!(room, planned, thread:, reply_to:)
        message = Message.new(room:, creator: planned[:author],
          markdown_source: planned[:markdown], created_at: planned[:created_at],
          edited_at: planned[:edited_at], importing: true)
        message.thread = thread if thread
        message.reply_to_message_id = reply_to if reply_to
        message.save!
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "message", slack_key: planned[:key],
          record: message, created_record: true)
        message
      end

      def record_truncation(conversation_id, planned)
        return unless planned[:truncated]

        @run.record_issue!("warning", "#{conversation_id}:#{planned[:ts]}",
          "Message exceeded #{Message::Markdown::SOURCE_LIMIT} characters and was truncated")
      end

      def write_reactions(room, conversation_id, message, source, users:, counts:)
        reactions = Array(source["reactions"])
        return if reactions.empty?

        truncated = []
        reactions.each do |reaction|
          base_name = reaction["name"].to_s.sub(/::skin-tone-\d+\z/, "")
          content = reaction_content(base_name)
          listed = Array(reaction["users"])
          # Slack truncates long reactor lists: count runs past the users
          # array. The listed users still import, with one issue per
          # message saying the rest never arrived.
          truncated << reaction["name"].to_s if reaction["count"].to_i > listed.size
          listed.each do |slack_user_id|
            key = "#{conversation_id}:#{source["ts"]}:#{base_name}:#{slack_user_id}"
            if content.nil?
              @run.record_issue!("warning", key,
                "Skipped reaction :#{reaction["name"]}: on #{conversation_id}:#{source["ts"]}: unknown emoji")
              next
            end
            next if reaction_recorded?(key)

            booster = users[slack_user_id] ||= @user_mapper.ensure_author(slack_user_id)
            boost = Boost.create!(message:, booster:, content:,
              created_at: message.created_at)
            SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
              slack_kind: "reaction", slack_key: key,
              record: boost, created_record: true)
            counts["reactions"] += 1
          end
        end
        if truncated.any?
          @run.record_issue!("warning", "#{conversation_id}:#{source["ts"]}",
            "Slack truncated the reaction list on #{conversation_id}:#{source["ts"]} (#{truncated.uniq.join(", ")}); imported the listed users only")
        end
      end

      # One lookup per reaction key; pages are small and reactions rare, so
      # no bulk preload.
      def reaction_recorded?(key)
        SlackImport::Record.exists?(slack_workspace_id: @workspace.id,
          slack_kind: "reaction", slack_key: key)
      end

      # Standard emoji resolve to their character, brand and workspace icons
      # to canonical :name:, and unknown custom emoji stay :name: when the
      # shortcode is valid; anything else is skipped with an issue.
      def reaction_content(name)
        case (icon = Icons.find(name))
        when Icons::Emoji then icon.character
        when Icons::Brand, Icons::Custom then ":#{icon.name}:"
        else
          candidate = ":#{name}:"
          candidate if name.match?(/\A[a-z0-9_]+\z/) && candidate.length <= 16
        end
      end

      def write_pin(room, conversation_id, message, source, counts:)
        pinned_to = Array(source["pinned_to"])
        return if pinned_to.empty? || !pinned_to.include?(conversation_id)
        return if pin_recorded?(conversation_id, source["ts"])

        if room.message_pins.count >= MessagePin::MAX_PER_ROOM
          @run.record_issue!("warning", "#{conversation_id}:#{source["ts"]}",
            "Room #{room.name} already has #{MessagePin::MAX_PER_ROOM} pins; extras skipped")
          return
        end

        timestamp = Time.current
        # insert skips the pin callbacks (badge broadcasts and the pin-note
        # path); message_id is unique so the row reads straight back.
        MessagePin.insert({ message_id: message.id, room_id: room.id,
          pinner_id: @run.user_id, created_at: message.created_at, updated_at: timestamp })
        pin_id = MessagePin.where(message_id: message.id).pick(:id)
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "pin", slack_key: "#{conversation_id}:#{source["ts"]}",
          record_type: "MessagePin", record_id: pin_id, created_record: true)
        counts["pins"] += 1
      end

      def pin_recorded?(conversation_id, ts)
        SlackImport::Record.exists?(slack_workspace_id: @workspace.id,
          slack_kind: "pin", slack_key: "#{conversation_id}:#{ts}")
      end

      def ensure_thread(room, conversation_id, parent_ts, parent_message_id, thread_state)
        if thread_state["thread_id"].present?
          return ChannelThread.find(thread_state["thread_id"])
        end

        record = SlackImport::Record.find_by(slack_workspace_id: @workspace.id,
          slack_kind: "thread", slack_key: "#{conversation_id}:#{parent_ts}")
        if record
          thread_state["thread_id"] = record.record_id
          return ChannelThread.find(record.record_id)
        end

        parent = Message.find(parent_message_id)
        thread = ChannelThread.create!(room:, creator: @run.user,
          parent_message: parent, last_activity_at: parent.created_at)
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "thread", slack_key: "#{conversation_id}:#{parent_ts}",
          record: thread, created_record: true)
        thread_state["thread_id"] = thread.id
        thread_state["created_now"] = true
        thread
      end

      # Reply authors follow the thread when they belong to the room;
      # authors who left the channel are skipped like ThreadMembership.join!
      # requires.
      def follow_thread(thread, author, conversation_id:, parent_ts:)
        key = "#{conversation_id}:#{parent_ts}:follow:#{author.id}"
        return if SlackImport::Record.exists?(slack_workspace_id: @workspace.id,
          slack_kind: "thread_membership", slack_key: key)
        return if ThreadMembership.exists?(thread_id: thread.id, user_id: author.id)
        return unless thread.room.memberships.exists?(user_id: author.id)

        follow = ThreadMembership.create!(thread:, user: author)
        SlackImport::Record.create!(slack_workspace: @workspace, slack_import: @run,
          slack_kind: "thread_membership", slack_key: key,
          record: follow, created_record: true)
      end
  end
end
