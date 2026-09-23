module Calendar
  # One row per member opted into "In a meeting" status: the cached busy
  # intervals (ISO 8601 [start, end] pairs only — no titles, attendees,
  # or any other event detail), when they were fetched, the last fetch
  # failure for the settings page, and the last broadcast state so
  # boundary flips announce exactly once.
  class MeetingCache < ApplicationRecord
    self.table_name = "calendar_meeting_caches"

    belongs_to :user

    validates :user_id, uniqueness: true

    # True while now sits inside a cached busy interval. The start is
    # inclusive and the end exclusive, so a meeting reads as on at its
    # start minute and off at its end minute.
    def in_meeting?(now: Time.current)
      parsed_intervals.any? { |start_at, end_at| start_at <= now && now < end_at }
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
      def parsed_intervals
        @parsed_intervals ||= Array(busy_intervals).filter_map do |pair|
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
