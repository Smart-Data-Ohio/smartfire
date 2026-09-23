module SlashCommands
  # Small documented time language for /remind and /event, resolving in
  # the invoker's own time zone (never the server zone). Supported forms:
  #
  #   in 20 minutes | in 1 hour | in 3 hours | in 2 days | in 1 week
  #   tomorrow at 9am | tomorrow 9:30pm
  #   today at 3pm | at 3pm | at 15:00
  #   friday 5pm | next friday | monday
  #   2026-10-01 15:00 (parsed in the user's zone)
  #
  # A bare weekday means the next occurrence at 9am; "next <weekday>"
  # means the one a week after that. A bare time means today, or
  # tomorrow when it already passed.
  module TimeParser
    WEEKDAYS = %w[ sunday monday tuesday wednesday thursday friday saturday ].freeze
    TIME_OF_DAY = /(?<hour>\d{1,2})(?::(?<minute>\d{2}))?\s*(?<meridiem>am|pm)?/i
    ISO_DATETIME = /(?<iso>\d{4}-\d{2}-\d{2}[ T]\d{1,2}:\d{2}(?::\d{2})?)/
    LEADING_PATTERNS = [
      /\A#{ISO_DATETIME}/,
      /\Ain\s+(?<amount>\d+)\s*(?<unit>minutes?|mins?|hours?|hrs?|days?|weeks?)\b/i,
      /\Atomorrow(?:\s+at)?\s+#{TIME_OF_DAY}/i,
      /\Atomorrow\b/i,
      /\Atoday(?:\s+at)?\s+#{TIME_OF_DAY}/i,
      /\Aat\s+#{TIME_OF_DAY}/i,
      /\Anext\s+(?<weekday>#{WEEKDAYS.join("|")})(?:\s+#{TIME_OF_DAY})?/i,
      /\A(?<weekday>#{WEEKDAYS.join("|")})(?:\s+#{TIME_OF_DAY})?/i
    ].freeze
    TRAILING_PATTERNS = [
      /#{ISO_DATETIME}\s*\z/,
      /\bin\s+(?<amount>\d+)\s*(?<unit>minutes?|mins?|hours?|hrs?|days?|weeks?)\s*\z/i,
      /\btomorrow(?:\s+at)?\s+#{TIME_OF_DAY}\s*\z/i,
      /\btomorrow\s*\z/i,
      /\btoday(?:\s+at)?\s+#{TIME_OF_DAY}\s*\z/i,
      /\bat\s+#{TIME_OF_DAY}\s*\z/i,
      /\bnext\s+(?<weekday>#{WEEKDAYS.join("|")})(?:\s+#{TIME_OF_DAY})?\s*\z/i,
      /\b(?<weekday>#{WEEKDAYS.join("|")})(?:\s+#{TIME_OF_DAY})?\s*\z/i
    ].freeze
    DEFAULT_MORNING_HOUR = 9

    class << self
      # Parses a whole string as a time, or nil. Falls back to the
      # zone-aware datetime parser for explicit dates ("2026-10-01 15:00").
      def parse(text, zone:, now: Time.current)
        zone = ActiveSupport::TimeZone[zone] || Time.zone
        zoned_now = now.in_time_zone(zone)
        match = LEADING_PATTERNS.filter_map { |pattern| text.to_s.strip.match(pattern) }.first

        time = match ? time_from_match(match, zone:, now: zoned_now) : zone.parse(text.to_s.strip)
        time if time.is_a?(ActiveSupport::TimeWithZone)
      rescue ArgumentError, TypeError
        nil
      end

      # Splits "/remind <when> <text>": leading time, trailing text.
      # Returns [ time, text ] or nil when no leading time parses.
      def split_leading_time(text, zone:, now: Time.current)
        zone = ActiveSupport::TimeZone[zone] || Time.zone
        zoned_now = now.in_time_zone(zone)
        match = LEADING_PATTERNS.filter_map { |pattern| text.to_s.strip.match(pattern) }.first
        return unless match

        time = time_from_match(match, zone:, now: zoned_now)
        rest = text.to_s.strip[match.end(0)..].to_s.strip
        [ time, rest.presence ] if time
      end

      # Splits "/event <title> <when>": leading title, trailing time.
      # Returns [ title, time ] with time possibly nil (title only).
      def split_trailing_time(text, zone:, now: Time.current)
        zone = ActiveSupport::TimeZone[zone] || Time.zone
        zoned_now = now.in_time_zone(zone)
        match = TRAILING_PATTERNS.filter_map { |pattern| text.to_s.strip.match(pattern) }.first

        if match
          time = time_from_match(match, zone:, now: zoned_now)
          title = text.to_s.strip[0...match.begin(0)].to_s.strip
          # A bare time ("Friday") is a title, not a when: events need
          # one, and a when-only form cannot exist without it.
          title.blank? ? [ text.to_s.strip, nil ] : [ title, time ]
        else
          [ text.to_s.strip.presence, nil ]
        end
      end

      private
        def group(match, name)
          match.names.include?(name.to_s) ? match[name] : nil
        end

        def time_from_match(match, zone:, now:)
          if group(match, :iso).present?
            zone.parse(group(match, :iso))
          elsif group(match, :amount).present?
            amount = group(match, :amount).to_i
            return if amount <= 0

            case group(match, :unit).to_s.downcase
            when /\Amin/ then now + amount.minutes
            when /\Ahr/, /\Ahour/ then now + amount.hours
            when /\Aday/ then now + amount.days
            when /\Aweek/ then now + amount.weeks
            end
          elsif group(match, :weekday).present?
            weekday_time(match, zone:, now:)
          elsif match[0].match?(/tomorrow/i)
            apply_time_of_day((now + 1.day).beginning_of_day, match)
          elsif match[0].match?(/(?:today|at)\b/i)
            time = apply_time_of_day(now.beginning_of_day, match)
            time && time <= now ? time + 1.day : time
          end
        rescue ArgumentError, TypeError
          nil
        end

        def weekday_time(match, zone:, now:)
          target = WEEKDAYS.index(group(match, :weekday).to_s.downcase)
          return unless target

          day = now.beginning_of_day
          delta = (target - day.wday) % 7
          candidate = delta.zero? ? apply_time_of_day(day, match) : nil
          if match[0].match?(/next\b/i) || (candidate.nil? || candidate <= now) && delta.zero?
            delta += 7
          end
          apply_time_of_day(day + delta.days, match)
        end

        def apply_time_of_day(day, match)
          hour = group(match, :hour)&.to_i
          return day.change(hour: DEFAULT_MORNING_HOUR) if hour.nil?

          minute = group(match, :minute)&.to_i || 0
          return if hour > 23 || minute > 59

          meridiem = group(match, :meridiem)
          if meridiem
            return if hour < 1 || hour > 12
            hour = hour % 12
            hour += 12 if meridiem.casecmp("pm").zero?
          end

          day.change(hour:, min: minute)
        rescue ArgumentError
          nil
        end
    end
  end
end
