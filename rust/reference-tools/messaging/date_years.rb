require "json"
inputs = ["+2026-03-05 14:30", "-2026-03-05 14:30", "-0001-03-05 14:30", "0000-03-05 14:30", "+002026-03-05T14:30:00Z", "-002026-03-05T14:30:00Z", "002026-03-05 14:30", "9999-03-05 14:30", "-9999-03-05 14:30", "+2026/03/05 14:30", "-2026/03/05 14:30", "2026-3-5 14:30:00.000000999", "05 Mar -0001 14:30", "March 5 -0001 14:30", "05-Mar-2026 14:30", "Thu, 05 Mar 2026 14:30:00 GMT", "2026-03-05T14:30:00+053030", "2026-03-05T14:30:00-00:00", "2026-03-08T02:30:00", "2026-11-01T01:30:00", "2026-03-05T14:30:00,123456789Z", "2026-03-05T14:30:00.123456789+05:30", "2026-03-05T24:00:00.1", "2026-03-05T23:59:60", "2026-03-05T14:30:00+25:00", "2026-03-05T14:30:00+05:99", "2026-03-05T14:30:00+05:30:99", "2026-03-05T14:30:00-05", "2026-03-05T14:30:00 EST", "2026-03-05T14:30:00 India Standard Time"]
cases = %w[UTC America/New_York Europe/Berlin Pacific/Apia].flat_map do |zone|
  Time.use_zone(zone) do
    inputs.map do |input|
      begin
        time = Time.zone.parse(input)
        { zone:, input:, parts: Date._parse(input, false), micros: time && (time.to_r * 1_000_000).to_i, result: time&.utc&.iso8601(6) }
      rescue ArgumentError, TypeError, RangeError => error
        { zone:, input:, parts: Date._parse(input, false), error: error.class.name }
      end
    end
  end
end
ActionController::Base.allow_forgery_protection = false
Rails.application.config.hosts.clear
user = User.find(127326141)
room = Room.find(486777696)
message = room.root_messages.create!(creator: user, markdown_source: "Signed year HTTP reference", client_message_id: "signed-year-http-reference")
request = ActionDispatch::Request.new(Rails.application.env_config.merge("HTTP_HOST" => "campfire.test", "rack.input" => StringIO.new))
request.cookie_jar.signed[:session_token] = user.sessions.where.not(two_factor_verified_at: nil).first!.token
headers = { "Cookie" => "session_token=#{Rack::Utils.escape(request.cookie_jar[:session_token])}" }
browser = ActionDispatch::Integration::Session.new(Rails.application)
browser.host! "campfire.test"
requests = cases.map do |row|
  user.update_columns(time_zone: row.fetch(:zone))
  input = row.fetch(:input)
  responses = [["/saved", { message_id: message.id, saved_item: { remind_at: input } }],
    ["/rooms/#{room.id}/scheduled_messages", { scheduled_message: { markdown_source: "Signed year schedule", send_at: input } }]].map do |path, params|
    browser.post(path, params:, headers: headers.dup, as: :json)
    { path:, input: params, status: browser.response.status, body: browser.response.body }
  end
  { zone: row.fetch(:zone), responses: }
end
rows = {
  "messages" => ActiveRecord::Base.connection.select_all("SELECT * FROM messages WHERE id=#{message.id}").to_a,
  "action_text_rich_texts" => ActiveRecord::Base.connection.select_all("SELECT * FROM action_text_rich_texts WHERE record_type='Message' AND record_id=#{message.id}").to_a
}
File.write(ARGV.fetch(0), JSON.pretty_generate(reference: "d7c7de92", cases:, rows:, requests:) + "\n")
puts "WS8bm2 signed-year Rails oracle: #{cases.size} signed/expanded-year/offset/fraction/DST cases in 4 zones"
puts "WS8bm2 signed-year Rails HTTP oracle: #{requests.size * 2} actual reminder/scheduled responses in 4 zones"
