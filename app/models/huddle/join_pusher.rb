class Huddle::JoinPusher
  # At most one join push per huddle per viewer per window: every join
  # still shows the in-app banner, but the push is throttled so a busy
  # group call does not buzz the outsider's phone per joiner.
  PUSH_THROTTLE_WINDOW = 10.minutes

  def initialize(grant:, recipient:, room_membership:, dnd_exception: nil)
    @grant = grant
    @recipient = recipient
    @room_membership = room_membership
    @dnd_exception = dnd_exception
  end

  def push
    return unless grant && room && recipient && joiner
    # Members who switched huddle inbox items off also skip join pushes;
    # the in-app banner still shows (broadcast separately by the notifier).
    return unless recipient.inbox_preferences.huddle_invitations
    return unless policy.push?

    subscriptions = push_subscriptions_for_recipient
    return unless subscriptions.exists?

    return unless claim_throttle!

    enqueue_payload_for_delivery build_payload, subscriptions
  end

  private
    attr_reader :grant, :recipient, :room_membership

    def policy
      Notifications::Policy.new(
        recipient:, sender: joiner, kind: :huddle_join,
        room_membership:, dnd_exception: @dnd_exception
      )
    end

    def build_payload
      {
        title: "#{joiner.name} joined your huddle",
        body: "Join from the conversation",
        path: Rails.application.routes.url_helpers.room_path(room),
        tag: "huddle-#{room.id}"
      }
    end

    # Mirrors Huddle::InvitationPusher: only opted-in, disconnected
    # memberships get push. Connected sessions already see the in-app
    # banner live.
    def push_subscriptions_for_recipient
      Push::Subscription
        .joins(user: :memberships)
        .merge(Membership.visible.disconnected.where(room:, user: recipient).where.not(involvement: "nothing"))
    end

    # Claims the throttle window with one conditional UPDATE, so two
    # concurrent joins push only once. Claimed only when a push is
    # actually going out: DND, connected, and subscription-less viewers
    # return before this and never burn the window.
    def claim_throttle!
      Membership.where(id: room_membership.id)
        .where("last_huddle_join_push_at IS NULL OR last_huddle_join_push_at < ?", PUSH_THROTTLE_WINDOW.ago)
        .update_all(last_huddle_join_push_at: Time.current) == 1
    end

    def room
      grant.room
    end

    def joiner
      grant.user
    end

    def enqueue_payload_for_delivery(payload, subscriptions)
      Rails.configuration.x.web_push_pool.queue(payload, subscriptions)
    end
end
