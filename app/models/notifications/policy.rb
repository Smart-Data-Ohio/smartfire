module Notifications
  # The one place deciding whether a new message, reminder, or huddle
  # invitation pushes (and sounds) for one recipient, and which inbox
  # item it records: every push path and the inbox recorder call this
  # with preloaded memberships instead of re-deciding the rules
  # themselves. The recorder batches keyword matching once per message
  # and then asks for each candidate's winner here.
  #
  # Inbox rules:
  # - Invisible room memberships get nothing, ever.
  # - A muted thread (involvement "nothing") gets nothing, not even mentions.
  # - Mentions win over replies, replies over followed-thread activity,
  #   and thread activity over keyword matches; only the winner is recorded.
  # - Unfollowed threads (involvement "mentions") record mentions but not
  #   replies: replies are a follower benefit.
  # - At room level, mentions still record with notifications off
  #   (involvement "nothing"), but replies do not.
  # - A muted room (involvement "muted") records mentions and keyword
  #   alerts only; replies and followed-thread activity record nothing.
  # - Keyword matches record for every non-invisible room member.
  # - DND and quiet hours never suppress inbox items.
  #
  # Push and sound rules:
  # - Push mirrors the inbox winner, except keyword matches (inbox only)
  #   and room-level "everything" followers, who get push for every
  #   message without an inbox item.
  # - A muted room pushes mentions only: replies, thread activity, and
  #   keyword alerts stay silent, and DND still applies on top.
  # - DND (manual, presence, or quiet hours) suppresses push and sounds
  #   for everything except messages from people the recipient starred
  #   with "Allow during DND". Reminders carry no sender, so they stay silent.
  # - Quiet-during-meetings (opt-in, and only while meeting status is on)
  #   suppresses push and sounds during busy intervals exactly like DND,
  #   with the same starred-people exception. Inbox items are still
  #   recorded, as with every other DND form.
  class Policy
    KINDS = %i[ room_message thread_message reminder huddle ].freeze

    attr_reader :recipient, :sender, :kind, :room_membership, :thread_membership,
      :now, :mentioned, :reply_to_recipient, :keyword_matched

    def initialize(recipient:, sender: nil, kind: :room_message,
        room_membership: nil, thread_membership: nil,
        mentioned: false, reply_to_recipient: false, keyword_matched: false,
        dnd_exception: nil, now: Time.current)
      raise ArgumentError, "Unknown notification kind: #{kind}" unless KINDS.include?(kind)

      @recipient = recipient
      @sender = sender
      @kind = kind
      @room_membership = room_membership
      @thread_membership = thread_membership
      @mentioned = mentioned
      @reply_to_recipient = reply_to_recipient
      @keyword_matched = keyword_matched
      @dnd_exception = dnd_exception
      @now = now
    end

    # Which inbox item this event records for the recipient, or nil for
    # none. Reminders and huddles record their items in their own flows.
    def inbox_event_type
      return nil unless active_human_recipient?
      return nil if room_invisible?

      case kind
      when :room_message then room_inbox_event_type
      when :thread_message then thread_inbox_event_type
      else nil
      end
    end

    def push?
      return false if recipient.nil?

      base_push? && !muted_for_push?
    end
    alias sound? push?

    # DND exceptions for a batch of recipients in one query, so pushers
    # filter without a lookup per recipient. Returns the allowed ids.
    def self.dnd_exceptions_for(user_ids, sender)
      return Set.new if sender.nil? || user_ids.blank?

      DndAllowedUser.where(user_id: user_ids, allowed_user_id: sender.id).pluck(:user_id).to_set
    end

    private
      def room_inbox_event_type
        return nil if room_membership.nil?
        return "mention" if mentioned
        return "reply" if reply_to_recipient && room_replies_enabled?
        return "keyword_alert" if keyword_matched

        nil
      end

      def thread_inbox_event_type
        return nil if thread_membership.nil? || thread_membership.involved_in_nothing?
        return nil if room_membership.nil?

        return "mention" if mentioned && thread_mentions_enabled?
        return "reply" if reply_to_recipient && thread_membership.involved_in_everything? && room_replies_enabled?
        return "thread_activity" if thread_membership.involved_in_everything? && room_replies_enabled?
        return "keyword_alert" if keyword_matched

        nil
      end

      def base_push?
        case kind
        when :reminder, :huddle then true
        when :room_message then room_base_push?
        when :thread_message then thread_base_push?
        end
      end

      def room_base_push?
        return false if room_membership.nil? || room_invisible? || room_membership.involved_in_nothing?
        return true if room_membership.involved_in_everything?
        return true if mentioned && room_mentions_enabled?
        return true if reply_to_recipient && room_replies_enabled?

        false
      end

      def thread_base_push?
        return false if room_membership.nil? || thread_membership.nil?
        return false if room_invisible? || room_membership.involved_in_nothing?
        return false if thread_membership.involved_in_nothing?
        return mentioned && thread_mentions_enabled? if room_membership.involved_in_muted?
        return true if thread_membership.involved_in_everything?
        return true if mentioned && thread_mentions_enabled?

        false
      end

      def muted_for_push?
        return false unless quiet_now?

        if @dnd_exception.nil?
          !recipient.dnd_allows?(sender)
        else
          !@dnd_exception
        end
      end

      # Manual DND, DND presence, and quiet hours silence push and
      # sounds, and so does quiet-during-meetings for members who opted
      # into it: during a busy interval they read exactly as DND, with
      # the same starred-people exception. Callers preloading users for
      # a batch add `includes(:meeting_cache)` so the meeting check
      # stays off the hot path.
      def quiet_now?
        recipient.respond_to?(:dnd_active?) &&
          (recipient.dnd_active?(now:) || meeting_quiet?)
      end

      def meeting_quiet?
        recipient.respond_to?(:meeting_dnd_active?) && recipient.meeting_dnd_active?(now:)
      end

      def active_human_recipient?
        recipient&.active? && !recipient.bot?
      end

      def room_invisible?
        room_membership&.involved_in_invisible?
      end

      # Muted rooms still allow mention pushes; replies and thread
      # activity stay silent through room_replies_enabled?.
      def room_mentions_enabled?
        room_membership&.involved_in_mentions? || room_membership&.involved_in_everything? ||
          room_membership&.involved_in_muted?
      end

      def room_replies_enabled?
        room_membership&.involved_in_mentions? || room_membership&.involved_in_everything?
      end

      def thread_mentions_enabled?
        thread_membership&.involved_in_mentions? || thread_membership&.involved_in_everything?
      end
  end
end
