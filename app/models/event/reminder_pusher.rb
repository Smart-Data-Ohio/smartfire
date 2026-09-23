class Event::ReminderPusher
  # A reminder this late is stale (the runner was down): the moment
  # passed, so stay silent instead of announcing an event already
  # underway or over. The dispatcher checks this before creating inbox
  # activity; the push checks it again in case time passed in between.
  STALE_AFTER_START = 5.minutes

  attr_reader :event

  def self.stale?(event, now: Time.current)
    (event.ends_at.present? && event.ends_at <= now) || event.starts_at < now - STALE_AFTER_START
  end

  def initialize(event:)
    @event = event
  end

  def push(now: Time.current)
    return if stale?(now)

    enqueue_payload_for_delivery build_payload(now), push_subscriptions_for_recipients
  end

  private
    def build_payload(now)
      body = "#{relative_start(now)}: #{event.title}"
      body += " in #{event.venue.name}" if event.venue.present?

      {
        title: event.room.direct? ? event.organizer.name : event.room.name,
        body:,
        path: Rails.application.routes.url_helpers.room_event_path(event.room, event),
        tag: "event-#{event.id}"
      }
    end

    def relative_start(now)
      minutes = ((event.starts_at - now) / 60).round

      if minutes <= 0
        "Starting now"
      else
        "Starts in #{minutes} #{'minute'.pluralize(minutes)}"
      end
    end

    def stale?(now)
      self.class.stale?(event, now:)
    end

    def push_subscriptions_for_recipients
      Push::Subscription.where(user_id: push_allowed_ids)
    end

    # Reminders carry no sender, so DND and quiet hours silence them with
    # no per-person exception. One user lookup for the whole batch.
    def push_allowed_ids
      ids = recipient_ids
      users = User.where(id: ids).index_by(&:id)

      ids.select do |id|
        Notifications::Policy.new(recipient: users[id], kind: :reminder).push?
      end
    end

    def recipient_ids
      User.active.without_bots
        .where(id: event.attendances.where(response: Event::NOTIFYING_RESPONSES).select(:user_id))
        .where(id: event.room.memberships.select(:user_id))
        .ids
    end

    def enqueue_payload_for_delivery(payload, subscriptions)
      Rails.configuration.x.web_push_pool.queue(payload, subscriptions)
    end
end
