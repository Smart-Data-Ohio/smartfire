module Calendar
  # Pure derivation of out-of-office intervals from Google events.list items
  # for calendar OOO status. Only eventType, start, and end are read (the
  # client's fields mask keeps titles, descriptions, and every other event
  # detail out of the response entirely) and nothing but [start, end] pairs
  # is kept. Only events with eventType "outOfOffice" count; cancelled
  # ones never do. Unlike meeting busy intervals, all-day (date-only) OOO
  # events count — a vacation week is the main use case — with the dates
  # resolving in the member's own zone. Malformed items are skipped, never
  # raised: a shape change at Google must not break presence.
  module OooIntervals
    OUT_OF_OFFICE = "outOfOffice"

    def self.from_items(items, zone:)
      time_zone = ActiveSupport::TimeZone[zone] || Time.zone
      Array(items).filter_map do |item|
        interval_for(item, zone: time_zone) if ooo?(item)
      end.sort_by(&:first)
    end

    def self.ooo?(item)
      return false unless item.is_a?(Hash)
      return false unless item["eventType"] == OUT_OF_OFFICE
      return false if item["status"] == "cancelled"

      item.dig("start", "dateTime").present? || item.dig("start", "date").present?
    end

    def self.interval_for(item, zone:)
      if item.dig("start", "dateTime").present?
        start_at = parse_time(item.dig("start", "dateTime"))
        end_at = parse_time(item.dig("end", "dateTime"))
      else
        # All-day spans carry calendar dates with no zone; Google's end
        # date is already exclusive.
        start_at = parse_day(item.dig("start", "date"), zone:)
        end_at = parse_day(item.dig("end", "date"), zone:)
      end
      return nil if start_at.nil? || end_at.nil? || end_at <= start_at

      [ start_at, end_at ]
    end

    def self.parse_time(value)
      Time.zone.parse(value.to_s)
    rescue ArgumentError, TypeError
      nil
    end

    def self.parse_day(value, zone:)
      return nil if value.blank?

      zone.parse(value.to_s)&.beginning_of_day
    rescue ArgumentError, TypeError
      nil
    end
    private_class_method :ooo?, :interval_for, :parse_time, :parse_day
  end
end
