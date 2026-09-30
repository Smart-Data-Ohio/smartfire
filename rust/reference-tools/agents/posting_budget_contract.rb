require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026, 3, 2, 16) do
  bot = User.find(394959859)
  agent = bot.agent
  room = Room.find(486777696)
  existing = room.messages.create!(creator: bot, body: "Original", client_message_id: "ws11-retry")
  agent.update!(daily_message_cap: 1)
  client = ActionDispatch::Integration::Session.new(Rails.application)
  client.host! "campfire.test"
  client.https!
  path = Rails.application.routes.url_helpers.room_bot_messages_path(room, "#{bot.id}-BenderToken1")
  raw = %({"message":{"client_message_id":"ws11-retry","body":"Replacement"}})
  client.post path, params: raw, headers: { "CONTENT_TYPE" => "application/json" }
  replay = { status: client.response.status, original_id: existing.id, returned_id: client.response.headers["Location"].split('/').last.to_i, notices: AgentBudgetNotice.count }
  client.post path, params: %({"attachment":123,"message":{"client_message_id":"ws11-retry"}}), headers: { "CONTENT_TYPE" => "application/json" }
  malformed_replay = { status: client.response.status, returned_id: client.response.headers["Location"].split('/').last.to_i }
  client.post path, params: %({"attachment":123,"message":{"client_message_id":"ws11-new-bad-attachment"}}), headers: { "CONTENT_TYPE" => "application/json" }
  malformed_overflow = { status: client.response.status }
  client.post path, params: "Over budget", headers: { "CONTENT_TYPE" => "text/plain" }
  overflow = { status: client.response.status, body: client.response.body, retry_after_header: client.response.headers["Retry-After"] }
  client.post path, params: "Over budget again", headers: { "CONTENT_TYPE" => "text/plain" }
  notices = { notices: AgentBudgetNotice.count, inbox_items: ActivityItem.where(source_type: 'AgentBudgetNotice', event_type: 'agent_budget_exceeded', user_id: agent.owner_id).count }
  reset = [ ['UTC', Time.utc(2026,3,2,16)], ['Eastern Time (US & Canada)', Time.utc(2026,3,8,6,30)], ['America/New_York', Time.utc(2026,11,1,5,30)], ['America/New_York', Time.utc(2026,11,1,6,30)], ['UTC', Time.utc(2026,3,2,23,59,59,999_999)] ].map do |zone, now|
    Time.use_zone(zone) do
      current = now.in_time_zone
      { zone: zone, now: now.iso8601(6), day: current.to_date.to_s, start: current.beginning_of_day.utc.iso8601(6), end: current.end_of_day.utc.iso8601(6), retry_after: Agents::Budgets.seconds_until_reset(now: current) }
    end
  end
  puts JSON.pretty_generate(reference_pin: 'd7c7de92', replay: replay, malformed_replay: malformed_replay, malformed_overflow: malformed_overflow, overflow: overflow, notices: notices, reset: reset)
end
