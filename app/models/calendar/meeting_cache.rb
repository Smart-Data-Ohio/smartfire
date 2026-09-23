module Calendar
  # One row per member opted into "In a meeting" status or calendar
  # out-of-office: the cached busy intervals and OOO intervals (ISO 8601
  # [start, end] pairs only — no titles, attendees, or any other event
  # detail), when they were fetched, the last fetch failure for the
  # settings page, and the last broadcast state so boundary flips announce
  # exactly once.
  class MeetingCache < ApplicationRecord
    self.table_name = "calendar_meeting_caches"

    belongs_to :user

    validates :user_id, uniqueness: true

    # True while now sits inside a cached busy interval. The start is
    # inclusive and the end exclusive, so a meeting reads as on at its
    # start minute and off at its end minute.
    def in_meeting?(now: Time.current)
      parsed_busy_intervals.any? { |start_at, end_at| start_at <= now && now < end_at }
    end

    # True while now sits inside a cached out-of-office interval, with
    # the same inclusive-start/exclusive-end reading as in_meeting?.
    def in_ooo?(now: Time.current)
      parsed_ooo_intervals.any? { |start_at, end_at| start_at <= now && now < end_at }
    end

    # The latest OOO interval end covering now: the "until" viewers see
    # while calendar OOO shows. Nil while not covered.
    def ooo_end_covering(now: Time.current)
      parsed_ooo_intervals.filter_map do |start_at, end_at|
        end_at if start_at <= now && now < end_at
      end.max
    end

    # Busy intervals as epoch-second [start, end] pairs for the layout's
    # meeting-quiet marker, so the sound controller re-evaluates the
    # gate on every play without a reload. Malformed pairs are skipped
    # with the same parsing as in_meeting?.
    def quiet_window_epochs
      parsed_busy_intervals.map { |start_at, end_at| [ start_at.to_i, end_at.to_i ] }
    end

    # OOO intervals as epoch-second pairs for the layout's OOO-quiet
    # marker, with the same live treatment as the meeting windows.
    def ooo_window_epochs
      parsed_ooo_intervals.map { |start_at, end_at| [ start_at.to_i, end_at.to_i ] }
    end

    # Claims the one follow-up refresh owed when a push notification
    # lands inside the throttle window: the conditional UPDATE wins only
    # when no follow-up was claimed within the window, so a burst of
    # pushes enqueues exactly one delayed job. Any completed fetch
    # clears the claim. Returns true when this call won and the caller
    # must enqueue.
    def claim_refresh_followup!(now: Time.current, window: MeetingRefresh::PUSH_THROTTLE)
      self.class.where(id:)
        .where("refresh_pending_at IS NULL OR refresh_pending_at <= ?", now - window)
        .update_all(refresh_pending_at: now, updated_at: Time.current) == 1
    end

    # Claims a boundary flip: the conditional UPDATE wins only when the
    # stored broadcast state differs, so concurrent dispatchers (or a
    # re-run) announce each flip exactly once. Returns true when this
    # call won and the caller must broadcast.
    def claim_broadcast!(in_meeting)
      self.class.where(id:)
        .where("in_meeting_broadcast IS NULL OR in_meeting_broadcast != ?", in_meeting)
        .update_all(in_meeting_broadcast: in_meeting, updated_at: Time.current) == 1
    end

    private
      def parsed_busy_intervals
        @parsed_busy_intervals ||= parse_pairs(busy_intervals)
      end

      def parsed_ooo_intervals
        @parsed_ooo_intervals ||= parse_pairs(ooo_intervals)
      end

      def parse_pairs(pairs)
        Array(pairs).filter_map do |pair|
          next unless pair.is_a?(Array)

          start_at = parse_time(pair[0])
          end_at = parse_time(pair[1])
          [ start_at, end_at ] if start_at && end_at
        end
      end

      def parse_time(value)
        Time.zone.parse(value.to_s)
      rescue ArgumentError, TypeError
        nil
      end
  end
end
