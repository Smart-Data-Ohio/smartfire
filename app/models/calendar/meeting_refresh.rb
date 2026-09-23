module Calendar
  # Refreshes one member's cached meeting intervals from Google Calendar.
  # Runs from the opt-in controller, calendar push notifications, and the
  # periodic dispatcher; every path converges here so the throttle and
  # error handling stay in one place. Returns :ok, :error, :fresh
  # (fetched within the push throttle), or :skipped.
  #
  # Google failures never raise: the intervals clear (status reads as
  # off) and a gentle notice is stored for the settings page. The
  # 15-minute refresh cadence is the retry; failures stamp fetched_at
  # too, so a dead grant backs off instead of hot-looping.
  class MeetingRefresh
    LOOKBEHIND = 1.hour
    LOOKAHEAD = 24.hours
    PUSH_THROTTLE = 1.minute

    NOT_CONNECTED_MESSAGE = "Connect Google Calendar to show when you're in a meeting."
    RECONNECT_MESSAGE = "Google Calendar needs reconnecting before meeting status can update."
    UNREACHABLE_MESSAGE = "Google Calendar couldn't be reached; meeting status will retry."

    def self.refresh(user_id, now: Time.current)
      user = User.find_by(id: user_id)
      return :skipped unless user&.active? && user.meeting_status_enabled?

      account = user.google_account
      unless account&.usable? && account.calendar?
        store_error!(user, NOT_CONNECTED_MESSAGE, now:)
        return :error
      end

      cache = user.meeting_cache
      return :fresh if cache&.fetched_at && cache.fetched_at > now - PUSH_THROTTLE

      response = Google::Client.new(account).list_events(time_min: now - LOOKBEHIND, time_max: now + LOOKAHEAD)
      items = response.is_a?(Hash) ? response["items"] : nil
      store_intervals!(user, MeetingIntervals.from_items(items), now:)
      :ok
    rescue Google::Client::Error => error
      store_error!(user, message_for(error), now:)
      :error
    end

    def self.store_intervals!(user, intervals, now:)
      MeetingCache.upsert(
        {
          user_id: user.id,
          busy_intervals: intervals.map { |start_at, end_at| [ start_at.iso8601, end_at.iso8601 ] },
          fetched_at: now, fetch_error: nil
        },
        unique_by: :user_id
      )
    end

    def self.store_error!(user, message, now:)
      MeetingCache.upsert(
        { user_id: user.id, busy_intervals: [], fetched_at: now, fetch_error: message },
        unique_by: :user_id
      )
    end

    def self.message_for(error)
      case error
      when Google::Client::Unauthorized then RECONNECT_MESSAGE
      else UNREACHABLE_MESSAGE
      end
    end
    private_class_method :store_intervals!, :store_error!, :message_for
  end
end
