module Calendar
  # Refreshes one member's cached meeting and out-of-office intervals from
  # Google Calendar in a single events.list fetch. Runs from the opt-in
  # controller, calendar push notifications, and the periodic dispatchers;
  # every path converges here so the throttle and error handling stay in
  # one place. Returns :ok, :error, :fresh (fetched within the push
  # throttle), or :skipped.
  #
  # Google failures never raise: the intervals clear (status reads as
  # off) and a gentle notice is stored for the settings page. The
  # 15-minute refresh cadence is the retry; failures stamp fetched_at
  # too, so a dead grant backs off instead of hot-looping.
  class MeetingRefresh
    LOOKBEHIND = 1.hour
    LOOKAHEAD = 24.hours
    # Out-of-office spans (a vacation week) start further out than the next
    # meeting, so calendar-OOO members fetch a wider window in the same
    # request rather than a second one.
    OOO_LOOKAHEAD = 30.days
    PUSH_THROTTLE = 1.minute

    NOT_CONNECTED_MESSAGE = "Connect Google Calendar to show calendar status."
    RECONNECT_MESSAGE = "Google Calendar needs reconnecting before calendar status can update."
    UNREACHABLE_MESSAGE = "Google Calendar couldn't be reached; calendar status will retry."

    def self.refresh(user_id, now: Time.current)
      user = User.find_by(id: user_id)
      return :skipped unless user&.active? && (user.meeting_status_enabled? || user.ooo_calendar_enabled?)

      account = user.google_account
      unless account&.usable? && account.calendar?
        store_error!(user, NOT_CONNECTED_MESSAGE, now:)
        return :error
      end

      cache = user.meeting_cache
      return :fresh if cache&.fetched_at && cache.fetched_at > now - PUSH_THROTTLE

      lookahead = user.ooo_calendar_enabled? ? OOO_LOOKAHEAD : LOOKAHEAD
      response = Google::Client.new(account).list_events(time_min: now - LOOKBEHIND, time_max: now + lookahead)
      items = response.is_a?(Hash) ? response["items"] : nil
      store_intervals!(user, MeetingIntervals.from_items(items),
        OooIntervals.from_items(items, zone: user.time_zone_or_default), now:)
      :ok
    rescue Google::Client::Error => error
      store_error!(user, message_for(error), now:)
      :error
    end

    # Each interval set is stored only for its own opt-in: a meeting-only
    # member keeps no OOO intervals, and vice versa.
    def self.store_intervals!(user, busy, ooo, now:)
      MeetingCache.upsert(
        {
          user_id: user.id,
          busy_intervals: user.meeting_status_enabled? ? iso_pairs(busy) : [],
          ooo_intervals: user.ooo_calendar_enabled? ? iso_pairs(ooo) : [],
          fetched_at: now, fetch_error: nil
        },
        unique_by: :user_id
      )
    end

    def self.store_error!(user, message, now:)
      MeetingCache.upsert(
        { user_id: user.id, busy_intervals: [], ooo_intervals: [], fetched_at: now, fetch_error: message },
        unique_by: :user_id
      )
    end

    def self.iso_pairs(intervals)
      intervals.map { |start_at, end_at| [ start_at.iso8601, end_at.iso8601 ] }
    end

    def self.message_for(error)
      case error
      when Google::Client::Unauthorized then RECONNECT_MESSAGE
      else UNREACHABLE_MESSAGE
      end
    end
    private_class_method :store_intervals!, :store_error!, :iso_pairs, :message_for
  end
end
