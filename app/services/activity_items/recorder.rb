module ActivityItems
  class Recorder
    EVENT_PRIORITY = {
      "keyword_alert" => 0,
      "thread_activity" => 1,
      "work_update" => 1,
      "work_assignment" => 1,
      "reply" => 2,
      "mention" => 3
    }.freeze

    GROUPABLE_EVENT_TYPES = %w[ thread_activity work_update ].freeze

    class << self
      # This is the source hook for message creation and future source types.
      # Mentions, replies, and assignments are idempotent per recipient +
      # source, but thread_activity and work_update refresh the
      # recipient's existing unhandled item for the thread in place —
      # unread again, back at the top — so repeat calls are not no-ops.
      def record_message!(message)
        new(message).record_message!
      end

      # Future sources such as work events can call this directly. The source
      # remains responsible for exposing current recipient preferences through
      # `activity_recipient_ids` when it has rules beyond room membership.
      # skip_source_check is for the one caller whose recipients the source
      # cannot authorize: a board post's first message notifies room members
      # following everything, who are not thread members yet, so the board
      # posts controller authorizes each recipient itself and the recorder
      # still applies grouping and the unique-index guard.
      def record!(recipient:, source:, event_type:, skip_source_check: false)
        new(source).record!(recipient:, event_type:, skip_source_check:)
      end
    end

    def initialize(source)
      @source = source
    end

    def record_message!
      return [] unless @source.is_a?(Message) && @source.persisted?

      message_candidates.filter_map do |recipient, event_type|
        record!(recipient:, event_type:)
      end
    end

    def record!(recipient:, event_type:, skip_source_check: false)
      event_type = event_type.to_s
      validate_event_type!(event_type)
      return unless source_persisted?
      return unless ActivityItem.active_human?(recipient)
      return if @source.respond_to?(:creator_id) && @source.creator_id == recipient.id
      return unless skip_source_check || source_allows_recipient?(recipient)

      # A burst of the same kind of update for one thread keeps a single
      # item, repointed at the newest event so it describes the latest
      # change, and surfaces as unread again.
      if (grouped_item = find_groupable_item(recipient, event_type))
        grouped_item.update!(source: @source, read_at: nil, updated_at: Time.current)
        return grouped_item
      end

      ActivityItem.create_or_find_by!(
        user_id: recipient.id,
        source_type: source_type,
        source_id: @source.id
      ) { |item| item.event_type = event_type }
    end

    private
      # Computed once per record call. Every candidate is re-checked through
      # source_allows_recipient?, so recomputing here would re-query
      # memberships once per recipient.
      def message_candidates
        @message_candidates ||= @source.thread ? thread_message_candidates : room_message_candidates
      end

      # The notification policy picks each candidate's winner (mention over
      # reply over keyword); the candidates are the mentionees, the reply
      # author, and the keyword matches, with one membership load for all
      # of them.
      def room_message_candidates
        candidates = {}
        mentioned = mention_ids.to_set

        candidate_ids.each do |user_id|
          membership = room_memberships[user_id]
          next if membership.nil?

          event_type = Notifications::Policy.new(
            recipient: membership.user,
            sender: @source.creator,
            kind: :room_message,
            room_membership: membership,
            mentioned: mentioned.include?(user_id),
            reply_to_recipient: user_id == reply_author_id && @source.reply_notify_author?,
            keyword_matched: room_keyword_matched_ids.include?(user_id)
          ).inbox_event_type

          choose_candidate(candidates, membership.user, event_type) if event_type
        end
        candidates
      end

      def thread_message_candidates
        candidates = {}
        mentioned = mention_ids.to_set
        matched = thread_keyword_matched_ids

        thread_memberships.each do |thread_membership|
          event_type = Notifications::Policy.new(
            recipient: thread_membership.user,
            sender: @source.creator,
            kind: :thread_message,
            room_membership: room_memberships[thread_membership.user_id],
            thread_membership:,
            mentioned: mentioned.include?(thread_membership.user_id),
            reply_to_recipient: thread_membership.user_id == reply_author_id && @source.reply_notify_author?,
            keyword_matched: matched.include?(thread_membership.user_id)
          ).inbox_event_type

          choose_candidate(candidates, thread_membership.user, event_type) if event_type
        end
        candidates
      end

      def candidate_ids
        mention_ids | [ reply_author_id ].compact | room_keyword_matched_ids.to_a
      end

      # Only the memberships the candidate check can read: the thread's
      # members for thread messages, the mentionees plus the reply author
      # plus the keyword matches for root messages. A root post to a large
      # room used to load every room membership to notify at most a handful
      # of members.
      def room_memberships
        @room_memberships ||=
          begin
            ids = @source.thread ? thread_memberships.map(&:user_id) : candidate_ids
            if ids.empty?
              {}
            else
              @source.room.memberships.includes(:user).where(user_id: ids).index_by(&:user_id)
            end
          end
      end

      def thread_memberships
        @thread_memberships ||= @source.thread.memberships.includes(:user).to_a
      end

      def mention_ids
        @mention_ids ||= @source.mentionees.ids
      end

      def reply_author_id
        @reply_author_id ||= @source.reply_to_message&.creator_id
      end

      # Invisible members get nothing from the room. Members with
      # notifications off still get direct mentions, but no replies or
      # followed-thread activity.
      def mentionable_room_membership?(membership)
        membership.present? && !membership.involved_in_invisible? && ActivityItem.active_human?(membership.user)
      end

      # Keyword alerts match once per message over the candidate
      # recipients: thread messages reuse the already-loaded memberships,
      # and root messages match only members holding an alert (a join from
      # keyword_alerts into the room's memberships), so a keyword alert
      # somewhere never loads the whole roster. Every distinct phrase
      # compiles into its own pattern checked independently. A keyword
      # never overrides a mention, reply, or thread item for the same
      # message (the policy picks the winner), and muted or invisible
      # members match nothing. Members with room notifications off still
      # match: a keyword is an explicit opt-in, like a mention.
      def room_keyword_matched_ids
        @room_keyword_matched_ids ||= begin
          text = @source.plain_text_body

          if text.blank?
            Set.new
          else
            holder_ids = KeywordAlert.where(user_id: @source.room.memberships.select(:user_id)).distinct.pluck(:user_id)

            if holder_ids.empty?
              Set.new
            else
              phrases = KeywordAlert.where(user_id: holder_ids).pluck(:user_id, :phrase)
              Notifications::KeywordMatcher.matching_user_ids(phrases_by_user(phrases), text).to_set
            end
          end
        end
      end

      def thread_keyword_matched_ids
        @thread_keyword_matched_ids ||= begin
          text = @source.plain_text_body
          eligible = thread_memberships.filter_map do |membership|
            room_membership = room_memberships[membership.user_id]
            next unless mentionable_room_membership?(room_membership)
            next if membership.involved_in_nothing?

            membership
          end

          if text.blank? || eligible.empty?
            Set.new
          else
            phrases = KeywordAlert.where(user_id: eligible.map(&:user_id)).pluck(:user_id, :phrase)
            Notifications::KeywordMatcher.matching_user_ids(phrases_by_user(phrases), text).to_set
          end
        end
      end

      def phrases_by_user(rows)
        rows.group_by(&:first).transform_values { |pairs| pairs.map(&:last) }
      end

      def choose_candidate(candidates, recipient, event_type)
        return unless recipient
        return unless ActivityItem.active_human?(recipient)
        return if recipient.id == @source.creator_id

        previous = candidates[recipient]
        if previous.nil? || EVENT_PRIORITY.fetch(event_type) > EVENT_PRIORITY.fetch(previous)
          candidates[recipient] = event_type
        end
      end

      # A burst of thread or work updates collapses into the recipient's
      # existing unhandled item for the thread instead of stacking a second
      # row beside it. The grouped item is refreshed in place (unread again,
      # back at the top of the updated_at ordering) and keeps its type and
      # source. Mentions, replies, and assignments always record their own
      # item; a later mention never merges into an earlier thread item.
      def find_groupable_item(recipient, event_type)
        return unless GROUPABLE_EVENT_TYPES.include?(event_type)

        thread_id = groupable_thread_id
        return unless thread_id

        ActivityItem
          .where(user_id: recipient.id, handled_at: nil, event_type: event_type)
          .where(
            "(activity_items.source_type = :message AND activity_items.source_id IN " \
            "(SELECT id FROM messages WHERE thread_id = :thread_id)) OR " \
            "(activity_items.source_type = :work_event AND activity_items.source_id IN " \
            "(SELECT id FROM work_thread_events WHERE channel_thread_id = :thread_id))",
            message: Message.polymorphic_name, work_event: "WorkThreadEvent", thread_id:
          )
          .order(updated_at: :desc, id: :desc)
          .first
      end

      def groupable_thread_id
        if @source.is_a?(Message)
          @source.thread_id
        elsif @source.is_a?(WorkThreadEvent)
          @source.channel_thread_id
        end
      end

      def source_type
        @source.class.base_class.name
      end

      def source_persisted?
        @source.respond_to?(:persisted?) && @source.persisted?
      end

      def source_allows_recipient?(recipient)
        if @source.respond_to?(:activity_recipient_ids)
          return @source.activity_recipient_ids.include?(recipient.id)
        end

        if @source.is_a?(Message)
          return message_candidates.keys.any? { |candidate| candidate.id == recipient.id }
        end

        # Sources with different access rules must expose
        # `activity_recipient_ids`; an unknown polymorphic source is never
        # displayable merely because it happens to respond to `room`.
        false
      end

      def validate_event_type!(event_type)
        return if ActivityItem::EVENT_TYPES.include?(event_type)

        raise ArgumentError, "Unknown activity event type: #{event_type}"
      end
  end
end
