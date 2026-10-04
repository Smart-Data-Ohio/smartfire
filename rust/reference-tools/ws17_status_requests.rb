# Actual pinned HTTP writes, stored values, enqueue arguments and complete broadcast frames.
require "active_support/testing/time_helpers"
require "action_dispatch/testing/integration"
include ActiveSupport::Testing::TimeHelpers
ActionController::Base.allow_forgery_protection = false
ActiveJob::Base.queue_adapter = :test
ActionCable.server.config.cable = {"adapter" => "test"}
ActiveRecord::Base.logger = nil
Rails.logger = Logger.new($stderr)
user = User.find_by!(email_address: "david@37signals.com")
base = {presence_setting: "auto", custom_status_emoji: nil, custom_status_text: nil,
  custom_status_expires_at: nil, dnd_enabled: false, quiet_hours_enabled: false,
  meeting_status_enabled: false, meeting_dnd_enabled: false, ooo_until: nil, ooo_note: nil,
  ooo_calendar_enabled: false, ooo_notify_enabled: false, ooo_broadcast: nil, time_zone: "UTC"}
busy = [["2026-03-02T15:55:00Z", "2026-03-02T16:55:00Z"]]
ooo = [["2026-03-02T15:55:00Z", "2026-03-04T16:00:00Z"]]
manual = {ooo_until: "2026-03-03T16:00:00Z", ooo_note: "Back soon"}
rows = [
  {name: "updates_presence", params: {presence_setting: "dnd", custom_status_emoji: "🚂", custom_status_text: "On a train", custom_status_expires_in: "hour_1"}},
  {name: "clears_custom", attrs: {custom_status_text: "Train", custom_status_emoji: "🚂", custom_status_expires_at: "2026-03-03T16:00:00Z"}, params: {clear_custom_status: "1"}},
  {name: "invalid_presence", params: {presence_setting: "away"}},
  {name: "requires_sign_in", anonymous: true, params: {presence_setting: "dnd"}},
  {name: "meeting_on", params: {meeting_status_enabled: "1"}},
  {name: "meeting_off", attrs: {meeting_status_enabled: true}, busy:, params: {meeting_status_enabled: "0"}},
  {name: "meeting_off_badge", attrs: {meeting_status_enabled: true}, busy:, params: {meeting_status_enabled: "0"}},
  {name: "meeting_off_no_cache", attrs: {meeting_status_enabled: true}, params: {meeting_status_enabled: "0"}},
  {name: "unrelated_meeting", params: {presence_setting: "dnd"}},
  {name: "manual_on", params: {ooo_preset: "tomorrow", ooo_note: "Back <soon> & safe"}},
  {name: "custom_zone", attrs: {time_zone: "Pacific Time (US & Canada)"}, params: {ooo_preset: "custom", ooo_until_custom: "2026-03-09T15:30"}},
  {name: "unknown_ooo", params: {presence_setting: "dnd", ooo_preset: "someday"}},
  {name: "blank_ooo", params: {presence_setting: "dnd", ooo_preset: "custom", ooo_until_custom: ""}},
  {name: "past_ooo", params: {presence_setting: "dnd", ooo_preset: "custom", ooo_until_custom: "2026-03-01T15:30"}},
  {name: "long_note", params: {ooo_preset: "tomorrow", ooo_note: "x"*141}},
  {name: "manual_clear", attrs: manual, params: {clear_ooo: "1"}},
  {name: "manual_clear_calendar", attrs: manual.merge(ooo_calendar_enabled: true), ooo:, params: {clear_ooo: "1"}},
  {name: "note_edit", attrs: manual, params: {ooo_note: "Slower than hoped"}},
  {name: "calendar_on", params: {ooo_calendar_enabled: "1"}},
  {name: "calendar_off", attrs: {ooo_calendar_enabled: true}, ooo:, params: {ooo_calendar_enabled: "0"}},
  {name: "calendar_off_keep", attrs: {meeting_status_enabled: true, ooo_calendar_enabled: true}, ooo:, busy:, params: {ooo_calendar_enabled: "0"}},
  {name: "meeting_off_keep", attrs: {meeting_status_enabled: true, ooo_calendar_enabled: true}, ooo:, busy:, params: {meeting_status_enabled: "0"}},
  {name: "unrelated_calendar", params: {ooo_note: "Back soon"}},
  {name: "both_on", params: {meeting_status_enabled: "1", ooo_calendar_enabled: "1"}},
  {name: "both_off", attrs: {meeting_status_enabled: true, ooo_calendar_enabled: true}, ooo:, busy:, params: {meeting_status_enabled: "0", ooo_calendar_enabled: "0"}},
  {name: "calendar_off_and_note", attrs: manual.merge(ooo_calendar_enabled: true), ooo:, params: {ooo_calendar_enabled: "0", ooo_note: "Changed"}},
  {name: "unknown_expiry", params: {presence_setting: "dnd", custom_status_text: "Unsaved <edit>", custom_status_expires_in: "unknown", meeting_status_enabled: "1"}},
  {name: "invalid_lengths", params: {custom_status_emoji: "😀"*9, custom_status_text: "é"*101}},
  {name: "clear_zero_is_present", attrs: manual.merge(custom_status_text: "Train"), params: {clear_custom_status: "0", clear_ooo: "0"}},
  {name: "nil_boolean", params: {meeting_status_enabled: nil}},
  {name: "blank_note_clears", attrs: manual, params: {ooo_note: " "}},
  {name: "boolean_status", params: {custom_status_text: true, custom_status_emoji: false, ooo_note: true}},
  {name: "false_note", attrs: manual, params: {ooo_note: false}},
  {name: "hash_note", attrs: manual, params: {ooo_note: {x: "Away <&>"}}},
  {name: "unknown_compound_preset", params: {ooo_preset: ["tomorrow"], presence_setting: "dnd"}},
  {name: "unpermitted_flags", params: {presence_setting: "auto", dnd_enabled: "1", ooo_notify_enabled: "1", theme: "dark", time_zone: "America/Los_Angeles"}},
]
fields = %w[presence_setting custom_status_emoji custom_status_text custom_status_expires_at meeting_status_enabled ooo_calendar_enabled ooo_until ooo_note ooo_broadcast]
output = []
seeded_failures = {}
claims = []
travel_to(Time.utc(2026, 3, 2, 16)) do
  session = user.sessions.create!(user_agent: "WS17", ip_address: "127.0.0.1", two_factor_verified_at: Time.current)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed.permanent[:session_token] = {value: session.token, httponly: true, same_site: :lax}
  cookie = "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"
  %w[status notification_settings].each do |endpoint|
    client = ActionDispatch::Integration::Session.new(Rails.application)
    client.host! "campfire.test"
    params = endpoint == "status" ? {presence_setting: "away"} : {quiet_hours_enabled: "1", quiet_hours_start: "", quiet_hours_end: ""}
    client.patch("/users/me/#{endpoint}", params: JSON.generate(user: params),
      headers: {"CONTENT_TYPE" => "application/json", "HTTP_ACCEPT" => "text/html", "Cookie" => cookie})
    seeded_failures[endpoint] = client.response.status
  end
  # The named Rails tests use users.yml: David has no confirmed two-factor credential.
  # Keep the observed seeded failures above, then apply that actual fixture state for these cases.
  user.two_factor_credential&.update_columns(confirmed_at: nil)
  rows.each do |row|
    Calendar::MeetingCache.where(user:).delete_all
    user.update_columns(**base.merge(row.fetch(:attrs, {})))
    if row[:busy] || row[:ooo]
      Calendar::MeetingCache.create!(user:, busy_intervals: row.fetch(:busy, []), ooo_intervals: row.fetch(:ooo, []), fetched_at: Time.current)
    end
    ActiveJob::Base.queue_adapter.enqueued_jobs.clear
    frames = []
    subscriber = ActiveSupport::Notifications.subscribe("broadcast.action_cable") do |*args|
      payload = args.last
      frames << {stream: payload[:broadcasting], html: payload[:message]}
    end
    client = ActionDispatch::Integration::Session.new(Rails.application)
    client.host! "campfire.test"
    headers = {"CONTENT_TYPE" => "application/json", "HTTP_ACCEPT" => "text/html"}
    headers["Cookie"] = cookie unless row[:anonymous]
    client.patch("/users/me/status", params: JSON.generate(user: row[:params]), headers:)
    ActiveSupport::Notifications.unsubscribe(subscriber)
    user.reload
    cache = user.meeting_cache
    output << row.merge(status: client.response.status, location: client.response.location,
      stored: user.attributes.slice(*fields).transform_values { |v| v.is_a?(Time) ? v.iso8601(6) : v },
      cache: cache && {busy_intervals: cache.busy_intervals, ooo_intervals: cache.ooo_intervals},
      jobs: ActiveJob::Base.queue_adapter.enqueued_jobs.map { |j| {class: j[:job].name, args: j[:args]} }, frames:)
  end
  [nil, false, true].product([nil, "2026-03-02T15:59:00Z", "2026-03-02T16:01:00Z"], [false, true]).each do |stored, until_time, active|
    user.update_columns(ooo_broadcast: stored, ooo_until: until_time, ooo_note: "Keep or clear")
    won = user.claim_ooo_broadcast!(active)
    user.reload
    claims << {stored:, until_time:, active:, won:, after: {broadcast: user.ooo_broadcast, until_time: user.ooo_until&.iso8601(6), note: user.ooo_note}}
  end
end
puts JSON.generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: "2026-03-02T16:00:00Z", seeded_failures:, claims:, rows: output)
