module Calendar
  # Pure derivation of busy intervals from Google events.list items for
  # "In a meeting" status. Only start/end times are read; titles,
  # descriptions, and attendee identities never arrive (the client's
  # fields mask excludes them) and nothing but [start, end] pairs is
  # kept. Cancelled, declined-by-self, "Show as: Free" (transparent),
  # and all-day (date-only start) events never count as busy. Tentative
  # and needs-action events do, matching Google's own free/busy.
  # Malformed items are skipped, never raised: a shape change at Google
  # must not break presence.
  module MeetingIntervals
    DECLINED = "declined"

    def self.from_items(items)
      Array(items).filter_map do |item|
        interval_for(item) if busy?(item)
      end.sort_by(&:first)
    end

    def self.busy?(item)
      return false unless item.is_a?(Hash)
      return false if item["status"] == "cancelled"
      return false if item["eventType"] == "outOfOffice"
      return false if item["transparency"] == "transparent"
      return false if item.dig("start", "dateTime").blank?
      return false if declined_by_self?(item)

      true
    end

    def self.declined_by_self?(item)
      Array(item["attendees"]).any? do |attendee|
        attendee.is_a?(Hash) && attendee["self"] && attendee["responseStatus"] == DECLINED
      end
    end

    def self.interval_for(item)
      start_at = parse_time(item.dig("start", "dateTime"))
      end_at = parse_time(item.dig("end", "dateTime"))
      return nil if start_at.nil? || end_at.nil? || end_at <= start_at

      [ start_at, end_at ]
    end

    def self.parse_time(value)
      Time.zone.parse(value.to_s)
    rescue ArgumentError, TypeError
      nil
    end
    private_class_method :busy?, :declined_by_self?, :interval_for, :parse_time
  end
end
