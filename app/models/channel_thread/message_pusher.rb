class ChannelThread::MessagePusher
  attr_reader :thread, :message

  def initialize(thread:, message:)
    @thread, @message = thread, message
  end

  def push
    deliveries = recipients
    return if deliveries.empty?

    subscribed_ids = Push::Subscription.where(user_id: deliveries.map { |recipient| recipient[:user].id }).distinct.pluck(:user_id).to_set

    deliveries.each do |recipient|
      enqueue_payload_for_user(recipient[:user], recipient[:payload]) if subscribed_ids.include?(recipient[:user].id)
    end
  end

  private
    # One membership and user preload for the whole thread, then the
    # notification policy decides per recipient (follow state, mentions,
    # DND, quiet hours). A replying follower gets the reply payload; an
    # unfollowed reply author gets nothing at all.
    def recipients
      thread_memberships = thread.memberships.includes(:user).where.not(user_id: message.creator_id).to_a
      return [] if thread_memberships.empty?

      room_memberships = thread.room.memberships.where(user_id: thread_memberships.map(&:user_id)).index_by(&:user_id)
      exceptions = Notifications::Policy.dnd_exceptions_for(thread_memberships.map(&:user_id), message.creator)
      reply_payload = build_reply_payload
      thread_payload = build_thread_payload
      candidates = {}

      thread_memberships.each do |membership|
        policy = Notifications::Policy.new(
          recipient: membership.user,
          sender: message.creator,
          kind: :thread_message,
          room_membership: room_memberships[membership.user_id],
          thread_membership: membership,
          mentioned: mention_ids.include?(membership.user_id),
          reply_to_recipient: membership.user_id == reply_author_id && message.reply_notify_author?,
          dnd_exception: exceptions.include?(membership.user_id)
        )
        next unless policy.push?

        payload = (membership.user_id == reply_author_id && message.reply_notify_author?) ? reply_payload : thread_payload
        candidates[membership.user_id] = { user: membership.user, payload: }
      end

      candidates.values
    end

    def mention_ids
      @mention_ids ||= message.mentionees.ids.to_set
    end

    def reply_author_id
      @reply_author_id ||= message.reply_to_message&.creator_id
    end

    def build_thread_payload
      {
        title: thread.name,
        body: "#{message.creator.name}: #{message.plain_text_body}",
        path: Rails.application.routes.url_helpers.room_path(thread.room, thread: thread.id, message_id: message.id)
      }
    end

    def build_reply_payload
      {
        title: "Reply in #{thread.name}",
        body: "#{message.creator.name}: #{message.plain_text_body}",
        path: Rails.application.routes.url_helpers.room_path(thread.room, thread: thread.id, message_id: message.id)
      }
    end

    def enqueue_payload_for_user(user, payload)
      Rails.configuration.x.web_push_pool.queue(payload, Push::Subscription.where(user_id: user.id))
    end
end
