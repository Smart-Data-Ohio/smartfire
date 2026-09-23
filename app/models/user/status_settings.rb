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
  OOO_NOTE_LIMIT = 140
  OOO_PRESETS = %w[ tomorrow monday week custom ].freeze
  OOO_PRESET_LABELS = {
    "tomorrow" => "Until tomorrow",
    "monday" => "Until Monday",
    "week" => "1 week",
    "custom" => "Custom date and time"
  }.freeze

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
    validates :ooo_note, length: { maximum: OOO_NOTE_LIMIT }, allow_nil: true
    validate :ooo_until_must_be_future, if: -> { will_save_change_to_ooo_until? && ooo_until.present? }
    validates :quiet_hours_start_minute, :quiet_hours_end_minute,
      numericality: { only_integer: true, greater_than_or_equal_to: 0, less_than: MINUTES_PER_DAY },
      allow_nil: true
    validate :time_zone_must_be_known, if: -> { time_zone.present? }
    validate :quiet_hours_must_be_complete, if: :quiet_hours_enabled?
  end

  # Manual Do Not Disturb, the DND presence, or the scheduled quiet-hours
  # window, whichever silences push and sounds right now. Inbox items
  # are still recorded. A manual DND with an expiry in the past reads as
  # off without a cleanup job, the same way custom statuses do.
  def dnd_active?(now: Time.current)
    manual_dnd_active?(now:) || presence_setting == "dnd" || quiet_hours_active?(now:)
  end

  def manual_dnd_active?(now: Time.current)
    dnd_enabled? && (dnd_until.nil? || dnd_until > now)
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

  # Out of office, manual or from Google Calendar. A manual OOO is set
  # with an end in the status settings or with /ooo; it reads as off once
  # the end passes, with no cleanup job. The OOO dispatcher broadcasts
  # each flip and clears the expired columns. A calendar OOO covers the
  # member while now sits inside a cached outOfOffice interval. When both
  # overlap, the later end wins; clearing the manual one early ends only
  # it, and a covering calendar interval keeps showing with its own end.
  OOO_STATUS = "🌴 Out of office"

  def manual_ooo_active?(now: Time.current)
    ooo_until.present? && ooo_until > now
  end

  # Calendar out of office: the member opted in and now sits inside a
  # cached outOfOffice interval. Reads the cache row only (preloaded with
  # `includes(:meeting_cache)` on list paths); never touches Google.
  def calendar_ooo_active?(now: Time.current)
    ooo_calendar_enabled? && !!meeting_cache&.in_ooo?(now:)
  end

  def out_of_office?(now: Time.current)
    manual_ooo_active?(now:) || calendar_ooo_active?(now:)
  end

  # The "until" viewers see: the later end wins when manual and calendar
  # OOO overlap, or nil while not out of office.
  def ooo_until_effective(now: Time.current)
    [
      (ooo_until if manual_ooo_active?(now:)),
      (meeting_cache.ooo_end_covering(now:) if calendar_ooo_active?(now:))
    ].compact.max
  end

  # Whether the OOO label shows for this member. Precedence, top wins:
  # invisible hides everything inferred; OOO wins over a custom status,
  # manual DND, and the meeting label. OOO quiet itself (notifications
  # pausing) never suppresses the label, or the two would cancel out.
  def ooo_status_visible?(now: Time.current)
    out_of_office?(now:) && presence_setting != "invisible"
  end

  # The OOO status line: the label, the return date, and the member's own
  # note if their manual OOO set one. The date renders in the OOO
  # member's own zone, so every viewer — and every broadcast render —
  # sees the same date. Calendar details never leave the server.
  def ooo_status_text(now: Time.current)
    return nil unless ooo_status_visible?(now:)

    text = "#{OOO_STATUS} until #{ooo_until_date(now:)}"
    text += " — #{ooo_note}" if manual_ooo_active?(now:) && ooo_note.present?
    text
  end

  def ooo_until_date(now: Time.current)
    ooo_until_effective(now:)&.in_time_zone(time_zone_or_default)&.to_date&.to_fs(:long)
  end

  # Claims an OOO boundary flip: the conditional UPDATE wins only when the
  # stored broadcast state differs, so concurrent dispatchers (or a
  # re-run) announce each flip exactly once. Returns true when this call
  # won and the caller must broadcast. Flipping to false also clears a
  # manual OOO that already ended — and only one that already ended, so a
  # set racing the sweep keeps its new end.
  def claim_ooo_broadcast!(active, now: Time.current)
    scope = self.class.where(id: id).where("ooo_broadcast IS NULL OR ooo_broadcast != ?", active)

    if active
      scope.update_all(ooo_broadcast: true, updated_at: Time.current) == 1
    else
      scope.where("ooo_until IS NULL OR ooo_until <= ?", now)
        .update_all(ooo_broadcast: false, ooo_until: nil, ooo_note: nil, updated_at: Time.current) == 1
    end
  end

  # Quiet while out of office for Notifications::Policy: unless the member
  # asked to keep being notified, OOO reads exactly as DND, with the same
  # starred-people exception. Inbox items are still recorded.
  def ooo_dnd_active?(now: Time.current)
    out_of_office?(now:) && !ooo_notify_enabled?
  end

  # The status form submits an OOO preset instead of a timestamp. Presets
  # run to the end of the day in the member's own time zone; "Monday" is
  # the next one (a week out on Mondays). Custom parses a datetime-local
  # value in the member's zone, or nil when it is blank or unparseable
  # (the future check then rejects it). Raises ArgumentError on unknown
  # presets.
  def ooo_preset_until(preset, custom_value = nil, now: Time.current)
    preset = preset.to_s
    raise ArgumentError, "Unknown OOO preset: #{preset}" unless OOO_PRESETS.include?(preset)

    zoned = now.in_time_zone(time_zone_or_default)

    case preset
    when "tomorrow" then zoned.tomorrow.end_of_day
    when "monday"
      days_until = (1 - zoned.wday) % 7
      days_until = 7 if days_until.zero?
      (zoned + days_until.days).end_of_day
    when "week" then (zoned + 1.week).end_of_day
    when "custom" then parse_ooo_custom(custom_value)
    end
  end

  # The automatic meeting label. One string everywhere it shows (profile
  # badge, member panel, DM tooltips): the 📅 is the calendar icon, so
  # JSON surfaces render it with no HTML.
  IN_MEETING_STATUS = "📅 In a meeting"

  # True while the member opted into meeting status and now sits inside
  # a cached busy interval. Reads the cache row only (preloaded with
  # `includes(:meeting_cache)` on list paths); never touches Google.
  def in_meeting?(now: Time.current)
    meeting_status_enabled? && !!meeting_cache&.in_meeting?(now:)
  end

  # Whether the meeting label shows for this member. Precedence, top
  # wins: invisible hides everything inferred; out of office wins over the
  # automatic label; an active custom status wins too; any manual DND (the
  # toggle, DND presence, quiet hours) wins as well, since DND already
  # signals unavailability. Meeting auto-DND itself never suppresses the
  # label, or the two features would cancel each other. The presence dot
  # and label are unaffected: only the status line changes.
  def meeting_status_visible?(now: Time.current)
    in_meeting?(now:) && !out_of_office?(now:) && !custom_status_active?(now:) && !dnd_active?(now:) &&
      presence_setting != "invisible"
  end

  # The status line beside the member's name: out of office, their custom
  # status, the meeting label, or nothing. Every surface (badge, member
  # panel, DM tooltips) reads through here.
  def status_text_display(now: Time.current)
    ooo_status_text(now:) || custom_status_display(now:) || (IN_MEETING_STATUS if meeting_status_visible?(now:))
  end

  # Quiet-during-meetings for Notifications::Policy. Only when meeting
  # status itself is on; unlike the label, this ignores custom statuses
  # and manual DND — quiet applies through the whole busy interval, and
  # the "Allow during DND" people still get through.
  def meeting_dnd_active?(now: Time.current)
    meeting_dnd_enabled? && in_meeting?(now:)
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
    def ooo_until_must_be_future
      errors.add(:ooo_until, "must be in the future") if ooo_until <= Time.current
    end

    def parse_ooo_custom(value)
      return nil if value.blank?

      ActiveSupport::TimeZone[time_zone_or_default]&.parse(value.to_s)
    rescue ArgumentError, TypeError
      nil
    end

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
