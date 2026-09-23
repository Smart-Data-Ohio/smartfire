module User::StatusSettings
  extend ActiveSupport::Concern

  PRESENCE_SETTINGS = %w[ auto dnd invisible ].freeze
  THEMES = %w[ light dark system ].freeze
  CUSTOM_STATUS_EMOJI_LIMIT = 8
  CUSTOM_STATUS_TEXT_LIMIT = 100
  CUSTOM_STATUS_EXPIRIES = %w[ minutes_30 hour_1 hours_4 today week never ].freeze
  CUSTOM_STATUS_EXPIRY_LABELS = {
    "minutes_30" => "30 minutes",
    "hour_1" => "1 hour",
    "hours_4" => "4 hours",
    "today" => "Today",
    "week" => "This week",
    "never" => "Never"
  }.freeze
  MINUTES_PER_DAY = 24 * 60

  included do
    has_many :keyword_alerts, dependent: :delete_all
    has_many :dnd_allowed_users, dependent: :delete_all
    has_many :dnd_allowed_people, through: :dnd_allowed_users, source: :allowed_user

    # The "Not set" form option submits a blank string; store it as nil so
    # every reader keeps testing blankness one way.
    normalizes :time_zone, with: ->(zone) { zone.presence }

    validates :presence_setting, inclusion: { in: PRESENCE_SETTINGS }
    validates :theme, inclusion: { in: THEMES }
    validates :custom_status_emoji, length: { maximum: CUSTOM_STATUS_EMOJI_LIMIT }, allow_nil: true
    validates :custom_status_text, length: { maximum: CUSTOM_STATUS_TEXT_LIMIT }, allow_nil: true
    validates :quiet_hours_start_minute, :quiet_hours_end_minute,
      numericality: { only_integer: true, greater_than_or_equal_to: 0, less_than: MINUTES_PER_DAY },
      allow_nil: true
    validate :time_zone_must_be_known, if: -> { time_zone.present? }
    validate :quiet_hours_must_be_complete, if: :quiet_hours_enabled?
  end

  # Manual Do Not Disturb, the DND presence, or the scheduled quiet-hours
  # window, whichever silences push and sounds right now. Inbox items
  # are still recorded.
  def dnd_active?(now: Time.current)
    dnd_enabled? || presence_setting == "dnd" || quiet_hours_active?(now:)
  end

  def quiet_hours_active?(now: Time.current)
    return false unless quiet_hours_enabled?
    return false if quiet_hours_start_minute.nil? || quiet_hours_end_minute.nil?
    return false if quiet_hours_start_minute == quiet_hours_end_minute

    minute = now.in_time_zone(time_zone_or_default).seconds_since_midnight / 60

    if quiet_hours_start_minute < quiet_hours_end_minute
      minute >= quiet_hours_start_minute && minute < quiet_hours_end_minute
    else
      minute >= quiet_hours_start_minute || minute < quiet_hours_end_minute
    end
  end

  # True while push and sounds stay silent for this sender: DND (manual or
  # scheduled) without a per-person exception. A nil sender carries no
  # exception, so reminders and other system pushes stay silent too.
  def notifications_muted?(sender: nil, now: Time.current)
    dnd_active?(now:) && !dnd_allows?(sender)
  end

  def dnd_allows?(sender)
    return false if sender.nil?

    sender_id = sender.is_a?(User) ? sender.id : sender
    return false if sender_id.nil?

    dnd_allowed_users.where(allowed_user_id: sender_id).exists?
  end

  # A custom status with an expiry in the past reads as blank without a
  # cleanup job: every reader goes through here.
  def custom_status_active?(now: Time.current)
    (custom_status_emoji.present? || custom_status_text.present?) &&
      (custom_status_expires_at.nil? || custom_status_expires_at > now)
  end

  def custom_status_display(now: Time.current)
    return nil unless custom_status_active?(now:)

    [ custom_status_emoji, custom_status_text ].compact_blank.join(" ")
  end

  # The status form submits an expiry preset instead of a timestamp. "Today"
  # and "this week" end at midnight in the user's own time zone.
  def custom_status_expires_in=(preset)
    preset = preset.to_s
    return if preset.blank?
    raise ArgumentError, "Unknown custom status expiry: #{preset}" unless CUSTOM_STATUS_EXPIRIES.include?(preset)

    now = Time.current
    self.custom_status_expires_at = case preset
    when "minutes_30" then now + 30.minutes
    when "hour_1" then now + 1.hour
    when "hours_4" then now + 4.hours
    when "today" then now.in_time_zone(time_zone_or_default).end_of_day
    when "week" then now.in_time_zone(time_zone_or_default).end_of_week
    when "never" then nil
    end
  end

  # Quiet hours render as time inputs ("HH:MM") and persist as minutes
  # since midnight, so the window survives time-zone changes.
  def quiet_hours_start
    minutes_to_clock_time(quiet_hours_start_minute)
  end

  def quiet_hours_start=(value)
    self.quiet_hours_start_minute = clock_time_to_minutes(value)
  end

  def quiet_hours_end
    minutes_to_clock_time(quiet_hours_end_minute)
  end

  def quiet_hours_end=(value)
    self.quiet_hours_end_minute = clock_time_to_minutes(value)
  end

  def time_zone_or_default
    time_zone.presence || Time.zone.name
  end

  # Effective presence from the manual setting plus the live lease state
  # (:online, :idle, or :offline). Invisible always reads offline; DND
  # reads as its own state while any lease is live.
  def effective_presence(lease_state)
    if presence_setting == "invisible"
      :offline
    elsif presence_setting == "dnd"
      lease_state == :offline ? :offline : :dnd
    else
      lease_state
    end
  end

  # Replace the keyword list from textarea lines: stripped, blank lines
  # dropped, de-duplicated case-insensitively, capped at MAX_PER_USER.
  # Returns true when the replacement validated and saved.
  def replace_keyword_alerts(lines)
    phrases = Array(lines).flat_map { |line| line.to_s.split("\n") }
      .map { |phrase| phrase.strip.gsub(/\s+/, " ") }
      .reject(&:blank?)
      .uniq { |phrase| phrase.downcase }
      .first(KeywordAlert::MAX_PER_USER + 1)

    KeywordAlert.transaction do
      keyword_alerts.delete_all
      phrases.each { |phrase| keyword_alerts.create!(phrase:) }
    end
    true
  rescue ActiveRecord::RecordInvalid => error
    errors.add(:base, error.record.errors.full_messages.to_sentence)
    false
  end

  private
    def time_zone_must_be_known
      unless ActiveSupport::TimeZone[time_zone].present?
        errors.add(:time_zone, "is not a valid time zone")
      end
    end

    def quiet_hours_must_be_complete
      if quiet_hours_start_minute.nil? || quiet_hours_end_minute.nil?
        errors.add(:quiet_hours_start, "needs a start and an end while quiet hours are on")
      end
    end

    def minutes_to_clock_time(minutes)
      return nil if minutes.nil?

      format("%02d:%02d", minutes / 60, minutes % 60)
    end

    def clock_time_to_minutes(value)
      match = value.to_s.match(/\A(\d{1,2}):(\d{2})(?::\d{2})?\z/)
      return nil if match.nil?

      hour, minute = match.captures.first(2).map(&:to_i)
      return nil if hour > 23 || minute > 59

      hour * 60 + minute
    end
end
