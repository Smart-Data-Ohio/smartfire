class Room::MessagePusher
  attr_reader :room, :message

  def initialize(room:, message:)
    @room, @message = room, message
  end

  def push
    enqueue_payload_for_delivery build_payload, push_subscriptions_for_recipients
  end

  private
    def build_payload
      if room.direct?
        build_direct_payload
      else
        build_shared_payload
      end
    end

    def build_direct_payload
      {
        title: message.creator.name,
        body: message.plain_text_body,
        path: Rails.application.routes.url_helpers.room_path(room),
        tag: "room-#{room.id}"
      }
    end

    def build_shared_payload
      {
        title: room.name,
        body: "#{message.creator.name}: #{message.plain_text_body}",
        path: Rails.application.routes.url_helpers.room_path(room),
        tag: "room-#{room.id}"
      }
    end

    # The involvement scopes below find the candidate subscriptions; the
    # notification policy then drops recipients whose own settings (DND,
    # quiet hours) silence this message. One membership, user, and
    # exception lookup for the whole batch, never one per recipient.
    def push_subscriptions_for_recipients
      base = [
        push_subscriptions_for_users_involved_in_everything,
        push_subscriptions_for_mentionable_users(mentionee_ids),
        push_subscriptions_for_reply_author
      ].compact.reduce { |subscriptions, recipients| subscriptions.or(recipients) }&.distinct
      return Push::Subscription.none if base.nil?

      allowed_user_ids = policy_allowed_user_ids(base)
      base.where(user_id: allowed_user_ids)
    end

    def policy_allowed_user_ids(subscriptions)
      candidate_ids = subscriptions.pluck(:user_id).uniq
      return [] if candidate_ids.empty?

      memberships = room.memberships.where(user_id: candidate_ids).index_by(&:user_id)
      users = User.where(id: candidate_ids).index_by(&:id)
      exceptions = Notifications::Policy.dnd_exceptions_for(candidate_ids, message.creator)
      mentioned = mentionee_ids.to_set

      candidate_ids.select do |user_id|
        Notifications::Policy.new(
          recipient: users[user_id],
          sender: message.creator,
          kind: :room_message,
          room_membership: memberships[user_id],
          mentioned: mentioned.include?(user_id),
          reply_to_recipient: user_id == reply_author_id,
          dnd_exception: exceptions.include?(user_id)
        ).push?
      end
    end

    def push_subscriptions_for_users_involved_in_everything
      relevant_subscriptions.merge(Membership.involved_in_everything)
    end

    def push_subscriptions_for_mentionable_users(mentionees)
      relevant_subscriptions.merge(Membership.involved_in_mentions).where(user_id: mentionees)
    end

    # Reply authors should receive an opted-in direct notification even when
    # they follow the room at "mentions". A reply is one delivery regardless
    # of whether that user was also directly mentioned or follows everything.
    def push_subscriptions_for_reply_author
      return if reply_author_id.blank? || reply_author_id == message.creator_id || !message.reply_notify_author?

      relevant_subscriptions
        .merge(Membership.where.not(involvement: %w[ invisible nothing ]))
        .where(user_id: reply_author_id)
    end

    def mentionee_ids
      @mentionee_ids ||= message.mentionees.ids
    end

    def reply_author_id
      @reply_author_id ||= message.reply_to_message&.creator_id
    end

    def relevant_subscriptions
      Push::Subscription
        .joins(user: :memberships)
        .merge(Membership.visible.disconnected.where(room: room).where.not(user: message.creator))
    end

    def enqueue_payload_for_delivery(payload, subscriptions)
      Rails.configuration.x.web_push_pool.queue(payload, subscriptions)
    end
end
