# Full middleware/controller responses from our pinned Rails app, with fresh verified sessions.
require "active_support/testing/time_helpers"
require "action_dispatch/testing/integration"
include ActiveSupport::Testing::TimeHelpers

travel_to(Time.utc(2026, 3, 2, 16)) do
  ids = %w[david jason kevin bender].to_h { |name| [name, ActiveRecord::FixtureSet.identify(name)] }
  viewer = User.find(ids.fetch("david"))
  recipient = User.find(ids.fetch("jason"))
  session = viewer.sessions.create!(user_agent: "WS17", ip_address: "127.0.0.1", two_factor_verified_at: Time.current)
  request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
  request.cookie_jar.signed.permanent[:session_token] = { value: session.token, httponly: true, same_site: :lax }
  cookie = "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}"
  recipient_session = recipient.sessions.create!(user_agent: "WS17", ip_address: "127.0.0.1")
  cases = [
    { name: "live-status", attrs: { custom_status_emoji: "🚂", custom_status_text: "On a train <&>" }, lease: "live" },
    { name: "expired-status", attrs: { custom_status_text: "Old", custom_status_expires_at: 1.hour.ago.iso8601 }, lease: "live" },
    { name: "expired-lease", attrs: {}, lease: "expired" },
    { name: "idle-lease", attrs: {}, lease: "idle" },
    { name: "invisible", attrs: { presence_setting: "invisible" }, lease: "live" },
    { name: "dnd", attrs: { presence_setting: "dnd" }, lease: "live" },
    { name: "meeting", attrs: { meeting_status_enabled: true }, lease: nil, busy: true },
    { name: "custom-wins", attrs: { meeting_status_enabled: true, custom_status_text: "Own" }, lease: nil, busy: true },
    { name: "ooo", attrs: { ooo_until: "2026-03-04T12:00:00Z", ooo_note: "Back soon" }, lease: nil },
    { name: "invalid-ids", attrs: {}, lease: nil, requested_ids: ["#{ids['jason']}oops", "0x#{ids['kevin'].to_s(16)}", "nope"] },
    { name: "cap-before-dedup", attrs: {}, lease: nil, requested_ids: [ids['jason']] * 100 + [ids['kevin']] },
    { name: "cap-huge-integers", attrs: {}, lease: nil, requested_ids: ["9223372036854775808"] * 100 + [ids['jason']] }
  ]
  outputs = cases.map do |item|
    WorkspacePresenceLease.delete_all
    Calendar::MeetingCache.where(user: recipient).delete_all
    recipient.update_columns(presence_setting: "auto", custom_status_emoji: nil, custom_status_text: nil,
      custom_status_expires_at: nil, meeting_status_enabled: false, meeting_dnd_enabled: false,
      dnd_enabled: false, quiet_hours_enabled: false, ooo_until: nil, ooo_note: nil,
      ooo_calendar_enabled: false, **item[:attrs])
    if item[:lease]
      lease = WorkspacePresenceLease.establish(user: recipient, session: recipient_session)
      lease.update_columns(expires_at: 1.minute.ago) if item[:lease] == "expired"
      lease.update_columns(last_active_at: 11.minutes.ago) if item[:lease] == "idle"
    end
    if item[:busy]
      Calendar::MeetingCache.create!(user: recipient, busy_intervals: [[5.minutes.ago.iso8601, 55.minutes.from_now.iso8601]])
    end
    path = Rails.application.routes.url_helpers.presence_users_path(ids: item[:requested_ids] || [ids["jason"], ids["kevin"], ids["bender"], "nope"])
    client = ActionDispatch::Integration::Session.new(Rails.application)
    client.host! "campfire.test"
    client.get(path, headers: { "Cookie" => cookie, "Accept" => "application/json" })
    raise "unexpected HTTP #{client.response.status}: #{client.response.body}" unless client.response.status == 200
    item.merge(path:, body: client.response.body, content_type: client.response.content_type,
      retained_leases: WorkspacePresenceLease.count)
  end
  puts JSON.generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], cases: outputs)
end
